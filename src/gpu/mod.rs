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
    /// the frame's mode and flags (the kernel built for them draws it)
    pub spec: Spec,
}

/// A mode and lighting flags (lib.wgsl's Frame `mode` and `flags`) that a
/// kernel is built for: lib.wgsl's SPEC_MODE and SPEC_FLAGS, so that the
/// driver drops the code they turn off. The general kernel reads them from
/// the frame and draws any; it has to reserve registers for the costliest
/// path whatever the mode.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Spec {
    pub mode: u32,
    pub flags: u32,
}

/// wgpu's runtime checks, but for its loop counters: it would add one to
/// every loop, so that a loop that never ends cannot hang the driver, and
/// they cost (a fifth of a path-traced showcase's time, nearly half of
/// nurbs-booleans': docs/GPU_PERFORMANCE.md).
/// lib.wgsl's loops all have hard bounds: a count, or MAX_VISITS for the
/// BVH walks. Its bounds checks stay: off, they measured no faster, and
/// where the platform does not bound buffer reads itself they are wgpu's.
const CHECKS: wgpu::ShaderRuntimeChecks = wgpu::ShaderRuntimeChecks { bounds_checks: true, force_loop_bounding: false };

/// A pipeline being built on a thread: for which generated shader text and
/// mode (None: the general kernel), since when, and where it arrives.
struct Compile {
    generated: String,
    spec: Option<Spec>,
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
    /// the general kernel, and the ones built for a mode (None: its build
    /// failed, and the general one draws it)
    pipeline: Option<wgpu::ComputePipeline>,
    special: std::collections::HashMap<Spec, Option<wgpu::ComputePipeline>>,
    /// the kernels being built (at most one general and one for a mode),
    /// the generated text the current scene needs, and the last frame's mode
    compiling: Vec<Compile>,
    want: String,
    spec: Option<Spec>,
    /// build shaders on a thread and report not ready meanwhile (the viewer
    /// renders on the CPU until then: a cold build takes seconds), then the
    /// kernel for the mode, while the general one draws; off, the build
    /// blocks, and is only of the kernel for the mode (headless renders)
    pub background: bool,
    /// build kernels for the mode (off: the general kernel draws every mode;
    /// RAY_GENERAL_KERNEL in the environment, for comparing the two)
    pub specialize: bool,
    /// the generated part of the current kernels' shader
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
    /// and the flame grids after them
    grids: Vec<std::sync::Arc<crate::scene::flame::FlameGrid>>,
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
            special: Default::default(),
            compiling: Vec::new(),
            want: String::new(),
            spec: None,
            background: false,
            specialize: std::env::var_os("RAY_GENERAL_KERNEL").is_none(),
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
            grids: Vec::new(),
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

    /// Uploads `scene` if it changed, and builds the kernels its program
    /// structures and `spec` need. False when the GPU cannot render it (see
    /// `error`), or not yet: its shader is still being built (`compiling`).
    pub fn prepare(&mut self, scene: &Scene, scene_id: u64, spec: Spec) -> bool {
        self.poll();
        self.spec = Some(spec);
        if scene_id == self.scene_id {
            self.kernels(spec);
            // (the bind group comes with the first dispatch)
            return self.kernel(spec).is_some() && self.scene_buf.is_some() && self.error.is_none();
        }
        self.scene_id = scene_id;
        let up = codegen::build(scene, &self.terrains, &self.grids);
        let max = self.device.limits().max_storage_buffer_binding_size as usize;
        if up.terrain.len() * 4 > max || up.scene.len() * 4 > max {
            // (the bigger of the two is the one that does not fit)
            let mb = (up.terrain.len().max(up.scene.len()) * 4) >> 20;
            self.error = Some(format!("scene too large for one GPU buffer ({mb} MB, at most {} MB)", max >> 20));
            self.pipeline = None;
            self.special.clear();
            return false;
        }
        self.want = up.generated.clone();
        if up.generated != self.generated || self.pipeline.is_none() && self.special.is_empty() {
            // these structures need kernels of their own: none until one is built
            self.pipeline = None;
            self.special.clear();
            self.error = None;
        }
        self.kernels(spec);
        let storage = wgpu::BufferUsages::STORAGE;
        self.scene_buf = Some(self.buffer("ray scene", &up.scene, storage));
        if !up.terrain_same || self.terrain_buf.is_none() {
            self.terrain_buf = Some(self.buffer("ray terrain", &up.terrain, storage));
            self.terrains = up.terrains.clone();
            self.grids = up.grids.clone();
        }
        self.layout_info = up.layout;
        self.tables = up.tables.clone();
        self.bind = None;
        self.kernel(spec).is_some() && self.error.is_none()
    }

    /// The kernel that draws `spec`: its own, else the general one.
    fn kernel(&self, spec: Spec) -> Option<&wgpu::ComputePipeline> {
        self.special.get(&spec).and_then(Option::as_ref).or(self.pipeline.as_ref())
    }

    /// Builds what `spec` needs and has not got: in the background the
    /// general kernel first (it draws whatever the mode) and the one for
    /// `spec` beside it; headless, the one for `spec` now, or the general one
    /// if that cannot be built.
    fn kernels(&mut self, spec: Spec) {
        if self.error.is_some() {
            return;
        }
        let missing = self.specialize && !self.special.contains_key(&spec);
        if self.background {
            if self.pipeline.is_none() {
                self.start(None);
            }
            if missing {
                self.start(Some(spec));
            }
            return;
        }
        if missing {
            self.build_now(Some(spec));
        }
        if self.kernel(spec).is_none() {
            self.build_now(None);
        }
    }

    /// Starts building the kernel for `spec` (None: the general one) on a
    /// thread, unless one of its sort is being built already: then the next
    /// frame asks again.
    fn start(&mut self, spec: Option<Spec>) {
        // (a build for structures no longer wanted cannot be stopped: its
        // result is dropped)
        let want = &self.want;
        self.compiling.retain(|c| c.generated == *want);
        if self.compiling.iter().any(|c| c.spec.is_some() == spec.is_some()) {
            return;
        }
        let (device, layout, generated) = (self.device.clone(), self.pipeline_layout.clone(), self.want.clone());
        let (tx, rx) = std::sync::mpsc::channel();
        let started = Instant::now();
        let _ = std::thread::Builder::new().name("shader".into()).spawn(move || {
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let source = codegen::shader(&generated);
            let desc = wgpu::ShaderModuleDescriptor { label: Some("ray"), source: wgpu::ShaderSource::Wgsl(source.into()) };
            // SAFETY: every loop in lib.wgsl ends (CHECKS)
            let module = unsafe { device.create_shader_module_trusted(desc, CHECKS) };
            let constants = spec.map_or(Vec::new(), |s| vec![("SPEC_MODE", s.mode as f64), ("SPEC_FLAGS", s.flags as f64)]);
            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ray"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions { constants: &constants, ..Default::default() },
                cache: None,
            });
            let r = match pollster::block_on(device.pop_error_scope()) {
                Some(e) => Err(format!("shader: {e}")),
                None => Ok(pipeline),
            };
            let _ = tx.send(r);
        });
        self.compiling.push(Compile { generated: self.want.clone(), spec, started, rx: std::sync::Mutex::new(rx) });
    }

    /// Builds the kernel for `spec` (None: the general one) and waits for it.
    fn build_now(&mut self, spec: Option<Spec>) {
        self.start(spec);
        if let Some(i) = self.compiling.iter().position(|c| c.spec == spec) {
            let c = self.compiling.swap_remove(i);
            let r = c.rx.lock().unwrap().recv().unwrap_or_else(|_| Err("shader build failed".into()));
            self.install(c, r);
        }
    }

    /// Takes the finished background builds the scene still needs.
    fn poll(&mut self) {
        let mut i = 0;
        while i < self.compiling.len() {
            let r = self.compiling[i].rx.lock().unwrap().try_recv();
            match r {
                Err(std::sync::mpsc::TryRecvError::Empty) => i += 1,
                r => {
                    let c = self.compiling.swap_remove(i);
                    if c.generated == self.want {
                        self.install(c, r.unwrap_or_else(|_| Err("shader build failed".into())));
                    }
                }
            }
        }
    }

    fn install(&mut self, c: Compile, r: Result<wgpu::ComputePipeline, String>) {
        let ms = c.started.elapsed().as_secs_f32() * 1000.0;
        match (r, c.spec) {
            (Ok(p), spec) => {
                let first = self.pipeline.is_none() && self.special.values().all(Option::is_none);
                match spec {
                    None => self.pipeline = Some(p),
                    Some(s) => _ = self.special.insert(s, Some(p)),
                }
                self.compile_ms = ms;
                self.structures = c.generated.matches("_t(p0").count();
                self.generated = c.generated;
                self.error = None;
                let which = if spec.is_some() { "the kernel for this mode and lighting" } else { "the general kernel" };
                if self.background && first {
                    eprintln!("ray: GPU shader built in {ms:.0} ms ({} structures, {which}); the GPU joins in", self.structures);
                } else if self.background && spec.is_some() && spec == self.spec {
                    eprintln!("ray: GPU kernel for this mode and lighting built in {ms:.0} ms; it takes over");
                }
            }
            (Err(e), Some(s)) => {
                // (the general kernel draws it)
                eprintln!("ray: GPU kernel for this mode and lighting not built ({e}); the general one draws it");
                self.special.insert(s, None);
            }
            (Err(e), None) => {
                self.error = Some(e);
                self.pipeline = None;
                self.special.clear();
                self.generated.clear();
            }
        }
    }

    /// Whether the kernel built for the last frame's mode and flags draws it
    /// (else the general one does).
    #[cfg(test)]
    pub fn special_ready(&self) -> bool {
        self.spec.is_some_and(|s| self.special.get(&s).is_some_and(Option::is_some))
    }

    /// Whether the GPU is waiting for a shader built in the background: it
    /// has no kernel yet that draws the scene.
    pub fn compiling(&self) -> bool {
        self.spec.is_some_and(|s| self.kernel(s).is_none()) && self.compiling.iter().any(|c| c.generated == self.want)
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
        if self.kernel(params.spec).is_none() || self.scene_buf.is_none() {
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
            pass.set_pipeline(self.kernel(params.spec)?);
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
