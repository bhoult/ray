//! The GPU half of the renderer: a wgpu compute shader that traces the bands
//! of rows it is given (lib.wgsl, the CPU renderer ported per pixel) and hands
//! back each pixel's samples, which the CPU accumulates and post-processes with
//! its own rows. The scene is uploaded when it changes; the shader is rebuilt
//! only when the scene's distance-program structures change (codegen.rs).

pub mod codegen;

use crate::scene::Scene;
use std::sync::Arc;
use std::time::Instant;

/// What one frame of GPU work needs besides the scene.
pub struct FrameParams {
    /// the uniform block of lib.wgsl's `Frame`, as words
    pub words: Vec<u32>,
    /// padded frame width (pixels per row)
    pub width: u32,
    /// runs of the scene buffer that change every frame (a fractal's detail,
    /// the anchor tables): (first word, words)
    pub patches: Vec<(u32, Vec<u32>)>,
}

/// A pipeline being built on a thread: for which generated shader text,
/// since when, and where it arrives.
struct Compile {
    generated: String,
    started: Instant,
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<Result<wgpu::ComputePipeline, String>>>,
}

/// A submitted frame: its output buffer and how many rows it has.
pub struct Pending {
    pub index: wgpu::SubmissionIndex,
    pub rows: u32,
    pub width: u32,
    pub submitted: Instant,
    /// set when the readback mapping succeeded
    mapped: Arc<std::sync::atomic::AtomicBool>,
}

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub name: String,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    pipeline: Option<wgpu::ComputePipeline>,
    /// a shader being built in the background (`background`), and the
    /// generated text the current scene needs
    compiling: Option<Compile>,
    want: String,
    /// build shaders on a thread and report not ready meanwhile (the viewer
    /// renders on the CPU until then: a cold build takes seconds); off, the
    /// build blocks (headless renders)
    pub background: bool,
    /// the generated part of the current shader
    generated: String,
    scene_buf: Option<wgpu::Buffer>,
    terrain_buf: Option<wgpu::Buffer>,
    bands_buf: Option<wgpu::Buffer>,
    out_buf: Option<wgpu::Buffer>,
    staging: Option<wgpu::Buffer>,
    uniform: wgpu::Buffer,
    bind: Option<wgpu::BindGroup>,
    pub layout_info: codegen::Layout,
    /// per object, its anchor table's header word in the scene buffer (codegen::Upload::tables)
    pub tables: Vec<u32>,
    /// the terrain maps in the terrain buffer (codegen::Upload::terrain_same)
    terrains: Vec<std::sync::Arc<crate::scene::terrain::Terrain>>,
    scene_id: u64,
    pub error: Option<String>,
    pub compile_ms: f32,
    /// distinct distance-program structures in the shader
    pub structures: usize,
}

/// The device limits ray asks for: everything the adapter offers (large
/// terrain maps need big storage buffers).
pub fn limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    adapter.limits()
}

impl Gpu {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, name: String) -> Gpu {
        // errors are reported, never fatal: a bad shader falls back to the CPU
        device.on_uncaptured_error(Arc::new(|e| eprintln!("ray: GPU error: {e}")));
        let storage = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ray"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                storage(1, true),
                storage(2, true),
                storage(3, true),
                storage(4, false),
                storage(5, true),
            ],
        });
        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("ray"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ray frame"),
            size: 1024,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Gpu {
            device,
            queue,
            name,
            layout,
            pipeline_layout,
            pipeline: None,
            compiling: None,
            want: String::new(),
            background: false,
            generated: String::new(),
            scene_buf: None,
            terrain_buf: None,
            bands_buf: None,
            out_buf: None,
            staging: None,
            uniform,
            bind: None,
            layout_info: codegen::Layout::default(),
            tables: Vec::new(),
            terrains: Vec::new(),
            scene_id: u64::MAX,
            error: None,
            compile_ms: 0.0,
            structures: 0,
        }
    }

    /// A device of its own (headless rendering).
    pub fn headless() -> Result<Gpu, String> {
        // Vulkan (probing every backend took longer at start), else any
        let find = |backends: wgpu::Backends| {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::from_env().unwrap_or(backends), ..Default::default() });
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() }))
        };
        let adapter = find(wgpu::Backends::PRIMARY).or_else(|_| find(wgpu::Backends::all())).map_err(|e| format!("no GPU adapter: {e}"))?;
        let name = adapter.get_info().name;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ray"),
            required_limits: limits(&adapter),
            // wgpu's default takes host memory in 128 MB blocks, and pins them
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            ..Default::default()
        }))
        .map_err(|e| format!("no GPU device: {e}"))?;
        Ok(Gpu::new(device, queue, name))
    }

    fn buffer(&self, label: &str, words: &[u32], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let b = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (words.len().max(4) * 4) as u64,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&b, 0, bytemuck_words(words));
        b
    }

    /// Uploads `scene` if it changed and rebuilds the shader if its program
    /// structures changed. False when the GPU cannot render it (see `error`),
    /// or not yet: its shader is still being built (`compiling`).
    pub fn prepare(&mut self, scene: &Scene, scene_id: u64) -> bool {
        self.poll();
        if scene_id == self.scene_id {
            // (the bind group comes with the first dispatch)
            return self.pipeline.is_some() && self.scene_buf.is_some() && self.error.is_none();
        }
        self.scene_id = scene_id;
        let up = codegen::build(scene, &self.terrains);
        let max = self.device.limits().max_storage_buffer_binding_size as usize;
        if up.terrain.len() * 4 > max || up.scene.len() * 4 > max {
            // (the bigger of the two is the one that does not fit)
            let mb = (up.terrain.len().max(up.scene.len()) * 4) >> 20;
            self.error = Some(format!("scene too large for one GPU buffer ({mb} MB, at most {} MB)", max >> 20));
            self.pipeline = None;
            return false;
        }
        self.want = up.generated.clone();
        if up.generated != self.generated || self.pipeline.is_none() {
            // these structures need their own pipeline: none until it is built
            self.pipeline = None;
            if self.compiling.as_ref().is_none_or(|c| c.generated != up.generated) {
                self.compiling = Some(self.compile(&up));
            }
            if !self.background {
                let c = self.compiling.take().unwrap();
                let r = c.rx.lock().unwrap().recv().unwrap_or_else(|_| Err("shader build failed".into()));
                self.install(c, r);
            }
        }
        let storage = wgpu::BufferUsages::STORAGE;
        self.scene_buf = Some(self.buffer("ray scene", &up.scene, storage));
        if !up.terrain_same || self.terrain_buf.is_none() {
            self.terrain_buf = Some(self.buffer("ray terrain", &up.terrain, storage));
            self.terrains = up.terrains.clone();
        }
        self.layout_info = up.layout;
        self.tables = up.tables.clone();
        self.bind = None;
        if self.pipeline.is_some() {
            self.error = None;
        }
        self.pipeline.is_some() && self.error.is_none()
    }

    /// Starts building the pipeline for `up`'s shader on a thread.
    fn compile(&self, up: &codegen::Upload) -> Compile {
        let (device, layout, source) = (self.device.clone(), self.pipeline_layout.clone(), up.shader());
        let (tx, rx) = std::sync::mpsc::channel();
        let started = Instant::now();
        let _ = std::thread::Builder::new().name("shader".into()).spawn(move || {
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("ray"), source: wgpu::ShaderSource::Wgsl(source.into()) });
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ray"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });
            let r = match pollster::block_on(device.pop_error_scope()) {
                Some(e) => Err(format!("shader: {e}")),
                None => Ok(pipeline),
            };
            let _ = tx.send(r);
        });
        Compile { generated: up.generated.clone(), started, rx: std::sync::Mutex::new(rx) }
    }

    /// Takes a finished background build, if it is the one the scene needs.
    fn poll(&mut self) {
        let Some(c) = &self.compiling else { return };
        let r = c.rx.lock().unwrap().try_recv();
        match r {
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            r => {
                let c = self.compiling.take().unwrap();
                if c.generated == self.want {
                    self.install(c, r.unwrap_or_else(|_| Err("shader build failed".into())));
                }
            }
        }
    }

    fn install(&mut self, c: Compile, r: Result<wgpu::ComputePipeline, String>) {
        match r {
            Ok(p) => {
                self.pipeline = Some(p);
                self.compile_ms = c.started.elapsed().as_secs_f32() * 1000.0;
                self.structures = c.generated.matches("_t(p0").count();
                self.generated = c.generated;
                self.error = None;
                if self.background {
                    eprintln!("ray: GPU shader built in {:.0} ms ({} structures); the GPU joins in", self.compile_ms, self.structures);
                }
            }
            Err(e) => {
                self.error = Some(e);
                self.pipeline = None;
                self.generated.clear();
            }
        }
    }

    /// Whether a shader is being built in the background.
    pub fn compiling(&self) -> bool {
        self.compiling.is_some()
    }

    /// Makes sure the band, output and staging buffers hold `bands` bands of
    /// `width` pixels.
    fn reserve(&mut self, bands: usize, width: u32) {
        let out_bytes = (bands * 16 * width as usize * 8).max(64) as u64;
        let grow = |b: &Option<wgpu::Buffer>, size: u64| b.as_ref().is_none_or(|b| b.size() < size);
        if grow(&self.bands_buf, (bands * 4).max(16) as u64) {
            self.bands_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ray bands"),
                size: (bands.max(64) * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.bind = None;
        }
        if grow(&self.out_buf, out_bytes) {
            self.out_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ray out"),
                size: out_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            self.staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ray readback"),
                size: out_bytes,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.bind = None;
        }
        if self.bind.is_none() {
            fn entry(binding: u32, b: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
                wgpu::BindGroupEntry { binding, resource: b.as_entire_binding() }
            }
            self.bind = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ray"),
                layout: &self.layout,
                entries: &[
                    entry(0, &self.uniform),
                    entry(1, self.scene_buf.as_ref().unwrap()),
                    entry(2, self.terrain_buf.as_ref().unwrap()),
                    entry(3, self.bands_buf.as_ref().unwrap()),
                    entry(4, self.out_buf.as_ref().unwrap()),
                    entry(5, self.scene_buf.as_ref().unwrap()),
                ],
            }));
        }
    }

    /// Traces `bands` (band indices, 16 rows each) and starts the readback.
    pub fn dispatch(&mut self, params: &FrameParams, bands: &[u32]) -> Option<Pending> {
        if self.pipeline.is_none() || self.scene_buf.is_none() {
            return None;
        }
        self.reserve(bands.len(), params.width);
        let rows = bands.len() as u32 * 16;
        self.queue.write_buffer(&self.uniform, 0, bytemuck_words(&params.words));
        if let Some(scene) = &self.scene_buf {
            for (at, v) in &params.patches {
                self.queue.write_buffer(scene, *at as u64 * 4, bytemuck_words(v));
            }
        }
        self.queue.write_buffer(self.bands_buf.as_ref()?, 0, bytemuck_words(bands));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ray") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("ray"), timestamp_writes: None });
            pass.set_pipeline(self.pipeline.as_ref()?);
            pass.set_bind_group(0, self.bind.as_ref(), &[]);
            pass.dispatch_workgroups(params.width.div_ceil(8), rows.div_ceil(8), 1);
        }
        let bytes = rows as u64 * params.width as u64 * 8;
        enc.copy_buffer_to_buffer(self.out_buf.as_ref()?, 0, self.staging.as_ref()?, 0, bytes);
        let submitted = Instant::now();
        let index = self.queue.submit([enc.finish()]);
        let mapped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = mapped.clone();
        self.staging.as_ref()?.slice(..bytes).map_async(wgpu::MapMode::Read, move |r| {
            flag.store(r.is_ok(), std::sync::atomic::Ordering::Release);
        });
        Some(Pending { index, rows, width: params.width, submitted, mapped })
    }

    /// Waits for a submitted frame (blocking); the instant it finished.
    pub fn wait(&self, p: &Pending) -> Instant {
        let _ = self.device.poll(wgpu::PollType::Wait { submission_index: Some(p.index.clone()), timeout: None });
        Instant::now()
    }

    /// The finished frame's pixels: (RGB9E5, object) pairs, `rows` x `width`;
    /// None when the GPU failed it.
    pub fn read<R>(&self, p: &Pending, f: impl FnOnce(&[u32]) -> R) -> Option<R> {
        if !p.mapped.load(std::sync::atomic::Ordering::Acquire) {
            return None;
        }
        let bytes = p.rows as u64 * p.width as u64 * 8;
        let staging = self.staging.as_ref()?;
        let r = {
            let view = staging.slice(..bytes).get_mapped_range();
            let words: &[u32] = words_of(&view);
            f(words)
        };
        staging.unmap();
        Some(r)
    }
}

fn bytemuck_words(w: &[u32]) -> &[u8] {
    // SAFETY: u32 has no padding and any alignment of u8 is fine
    unsafe { std::slice::from_raw_parts(w.as_ptr() as *const u8, std::mem::size_of_val(w)) }
}

fn words_of(b: &[u8]) -> &[u32] {
    assert!((b.as_ptr() as usize).is_multiple_of(4) && b.len().is_multiple_of(4));
    // SAFETY: mapped GPU memory is at least 4-byte aligned (checked) and every bit pattern is a u32
    unsafe { std::slice::from_raw_parts(b.as_ptr() as *const u32, b.len() / 4) }
}

/// RGB9E5 to linear RGB.
#[inline(always)]
pub fn rgb9e5(v: u32) -> [f32; 3] {
    let e = (v >> 27) as i32 - 24;
    let s = f32::from_bits(((e + 127).clamp(1, 254) as u32) << 23);
    [(v & 511) as f32 * s, ((v >> 9) & 511) as f32 * s, ((v >> 18) & 511) as f32 * s]
}
