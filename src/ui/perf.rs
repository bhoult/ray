//! CPU and GPU load over time and now, split into this program (ray) and
//! every other process: CPU from /proc, GPU from NVML's per-process samples
//! when an NVIDIA driver is present (the kernel's gpu_busy_percent, total
//! only, otherwise); plus the render time per frame and each core's load.
//!
//! The charts follow one rule set: one axis per chart (CPU %, GPU %, frame
//! time and cores each get their own), the same colour for the same thing in
//! every chart (ray blue, other processes orange: a validated categorical
//! pair), 2 px lines, a legend with the current values, a hover crosshair.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Seconds of history kept and shown.
const WINDOW: f64 = 60.0;
const PERIOD: f32 = 0.25;

const RAY: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
const OTHER: Color32 = Color32::from_rgb(0xd9, 0x59, 0x26);
const CORES: Color32 = Color32::from_rgb(0x19, 0x9e, 0x70);
const FRAME: Color32 = Color32::from_rgb(0x90, 0x85, 0xe9);

#[derive(Clone, Copy, Default)]
pub struct Sample {
    pub t: f64,
    /// CPU: this process and all the others, percent of the whole machine
    pub ray: f32,
    pub other: f32,
    /// GPU: this process and all the others, percent utilisation
    pub ray_gpu: Option<f32>,
    pub other_gpu: Option<f32>,
    /// GPU memory: this process (GiB), and used / total (GiB)
    pub ray_mem: Option<f32>,
    pub gpu_mem: Option<(f32, f32)>,
    /// render thread: trace + post (ms)
    pub frame_ms: f32,
}

#[derive(Clone, Copy, Default)]
struct CpuTimes {
    busy: u64,
    total: u64,
}

/// What the sampler thread publishes.
#[derive(Clone, Default)]
struct Data {
    history: VecDeque<Sample>,
    cores: Vec<f32>,
    now: Sample,
    gpu_name: String,
}

struct Shared {
    data: Mutex<Data>,
    /// the latest render time (f32 bits), folded into the next sample
    frame_ms: AtomicU32,
    stop: AtomicBool,
}

/// The load monitor: a thread samples every quarter second (the NVML
/// per-process queries can take a while, so never on the UI thread); the UI
/// draws from a snapshot.
pub struct Perf {
    shared: Arc<Shared>,
    history: VecDeque<Sample>,
    cores: Vec<f32>,
    pub now: Sample,
    gpu_name: String,
}

impl Drop for Perf {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

struct Sampler {
    start: Instant,
    prev: Vec<CpuTimes>,
    prev_proc: u64,
    last: Instant,
    gpu: Option<Gpu>,
    ncpu: usize,
}

impl Sampler {
    fn sample(&mut self, sh: &Shared) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f32().max(1e-3);
        self.last = now;
        let stat = read_stat();
        let pct = |a: CpuTimes, b: CpuTimes| {
            let total = b.total.saturating_sub(a.total);
            if total == 0 { 0.0 } else { b.busy.saturating_sub(a.busy) as f32 / total as f32 * 100.0 }
        };
        let all = if !stat.is_empty() && !self.prev.is_empty() { pct(self.prev[0], stat[0]) } else { 0.0 };
        let cores: Vec<f32> = (0..self.ncpu)
            .map(|i| match (self.prev.get(i + 1), stat.get(i + 1)) {
                (Some(a), Some(b)) => pct(*a, *b),
                _ => 0.0,
            })
            .collect();
        self.prev = stat;
        let proc = read_proc();
        // clock ticks are 1/100 s on Linux
        let ray = (proc.saturating_sub(self.prev_proc)) as f32 / 100.0 / dt / self.ncpu as f32 * 100.0;
        self.prev_proc = proc;
        let g = match &self.gpu {
            Some(g) => g.sample(),
            None => GpuSample::default(),
        };
        let ray = ray.min(100.0);
        let s = Sample {
            t: (now - self.start).as_secs_f64(),
            ray,
            other: (all - ray).max(0.0),
            ray_gpu: g.mine,
            other_gpu: g.total.map(|t| (t - g.mine.unwrap_or(0.0)).max(0.0)),
            ray_mem: g.mine_mem,
            gpu_mem: g.mem,
            frame_ms: f32::from_bits(sh.frame_ms.load(Ordering::Relaxed)),
        };
        let mut d = sh.data.lock().unwrap();
        d.history.push_back(s);
        while d.history.front().is_some_and(|h| s.t - h.t > WINDOW) {
            d.history.pop_front();
        }
        d.cores = cores;
        d.now = s;
    }
}

impl Perf {
    pub fn new() -> Perf {
        let shared = Arc::new(Shared { data: Mutex::new(Data::default()), frame_ms: AtomicU32::new(0), stop: AtomicBool::new(false) });
        let sh = shared.clone();
        std::thread::Builder::new()
            .name("load".into())
            .spawn(move || {
                let gpu = Gpu::open();
                sh.data.lock().unwrap().gpu_name = gpu.as_ref().map(|g| g.name()).unwrap_or_else(|| "no GPU counters".into());
                let prev = read_stat();
                let ncpu = prev.len().saturating_sub(1).max(1);
                let mut s = Sampler { start: Instant::now(), prev, prev_proc: read_proc(), last: Instant::now(), gpu, ncpu };
                while !sh.stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_secs_f32(PERIOD));
                    s.sample(&sh);
                }
            })
            .expect("load sampler thread");
        Perf { shared, history: VecDeque::new(), cores: Vec::new(), now: Sample::default(), gpu_name: String::new() }
    }

    pub fn frame(&mut self, ms: f32) {
        self.shared.frame_ms.store(ms.to_bits(), Ordering::Relaxed);
    }

    /// Takes the sampler's latest data for drawing.
    pub fn tick(&mut self) {
        let d = self.shared.data.lock().unwrap();
        if d.now.t != self.now.t || self.gpu_name.is_empty() {
            self.history.clone_from(&d.history);
            self.cores.clone_from(&d.cores);
            self.now = d.now;
            self.gpu_name.clone_from(&d.gpu_name);
        }
    }

    /// The charts, side by side.
    pub fn ui(&self, ui: &mut egui::Ui) {
        // all of it, exactly: the panel takes the height of what is in it
        let h = ui.available_height().max(60.0);
        let w = ui.available_width() - 30.0;
        ui.horizontal(|ui| {
            let cw = (w * 0.27).max(200.0);
            let fw = (w * 0.2).max(150.0);
            let bw = (w - 2.0 * cw - fw).max(120.0);
            self.cpu_chart(ui, Vec2::new(cw, h));
            self.gpu_chart(ui, Vec2::new(cw, h));
            self.frame_chart(ui, Vec2::new(fw, h));
            self.core_bars(ui, Vec2::new(bw, h));
        });
    }

    fn cpu_chart(&self, ui: &mut egui::Ui, size: Vec2) {
        ui.vertical(|ui| {
            ui.set_width(size.x);
            let start = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CPU").strong())
                    .on_hover_text("Share of all CPU cores in use over the last minute: this program against everything else on the machine (100% = every core busy).");
                legend(ui, RAY, "ray", Some(self.now.ray), "This program: the CPU renderer (all cores while it traces), the physics and the UI.");
                legend(ui, OTHER, "other processes", Some(self.now.other), "Every other process on the machine together.");
            });
            // the height left under the header, exactly: more grows the panel every frame
            let used = ui.cursor().top() - start;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(size.x, (size.y - used).max(40.0)), egui::Sense::hover());
            let p = ui.painter_at(rect);
            grid(ui, &p, rect, &[0.0, 25.0, 50.0, 75.0, 100.0], 100.0, "%");
            self.line(&p, rect, OTHER, 100.0, |s| Some(s.other));
            self.line(&p, rect, RAY, 100.0, |s| Some(s.ray));
            if let Some(pos) = resp.hover_pos()
                && let Some(s) = self.at(rect, pos)
            {
                crosshair(&p, rect, pos);
                resp.on_hover_text_at_pointer(format!("{:.1} s ago\nray {:.0}%\nother processes {:.0}%", self.now.t - s.t, s.ray, s.other));
            }
        });
    }

    fn gpu_chart(&self, ui: &mut egui::Ui, size: Vec2) {
        ui.vertical(|ui| {
            ui.set_width(size.x);
            let start = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("GPU").strong()).on_hover_text(format!(
                    "{}: utilisation over the last minute, this program (its GPU renderer and the window) against everything else.",
                    self.gpu_name
                ));
                legend(ui, RAY, "ray", self.now.ray_gpu, "This program's share of the GPU (NVML per-process samples).");
                legend(ui, OTHER, "other processes", self.now.other_gpu, "Every other program's share of the GPU.");
            });
            // the height left under the header, exactly: more grows the panel every frame
            let used = ui.cursor().top() - start;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(size.x, (size.y - used).max(40.0)), egui::Sense::hover());
            let p = ui.painter_at(rect);
            grid(ui, &p, rect, &[0.0, 25.0, 50.0, 75.0, 100.0], 100.0, "%");
            self.line(&p, rect, OTHER, 100.0, |s| s.other_gpu);
            self.line(&p, rect, RAY, 100.0, |s| s.ray_gpu);
            if let Some(pos) = resp.hover_pos()
                && let Some(s) = self.at(rect, pos)
            {
                crosshair(&p, rect, pos);
                let pct = |v: Option<f32>| v.map_or("n/a".into(), |v| format!("{v:.0}%"));
                let mem = match (s.ray_mem, s.gpu_mem) {
                    (Some(r), Some((u, t))) => format!("\nmemory: ray {:.0} MiB, all {u:.1} / {t:.1} GiB", r * 1024.0),
                    (None, Some((u, t))) => format!("\nmemory: {u:.1} / {t:.1} GiB"),
                    _ => String::new(),
                };
                resp.on_hover_text_at_pointer(format!(
                    "{:.1} s ago\nray {}\nother processes {}{mem}",
                    self.now.t - s.t,
                    pct(s.ray_gpu),
                    pct(s.other_gpu)
                ));
            }
        });
    }

    fn frame_chart(&self, ui: &mut egui::Ui, size: Vec2) {
        let max_ms = self.history.iter().map(|s| s.frame_ms).fold(0.0f32, f32::max);
        let top = nice_ceiling(max_ms.max(1.0));
        ui.vertical(|ui| {
            ui.set_width(size.x);
            let start = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Render time").strong()).on_hover_text(
                    "Time the render thread spends per frame (trace + post), over the last minute. Lower is faster; 16.7 ms is 60 frames per second.",
                );
                let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 10.0), egui::Sense::hover());
                ui.painter().line_segment([Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y)], Stroke::new(2.0, FRAME));
                ui.label(egui::RichText::new(format!("{:.1} ms", self.now.frame_ms)).color(ui.visuals().text_color()));
            });
            // the height left under the header, exactly: more grows the panel every frame
            let used = ui.cursor().top() - start;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(size.x, (size.y - used).max(40.0)), egui::Sense::hover());
            let p = ui.painter_at(rect);
            grid(ui, &p, rect, &[0.0, top * 0.5, top], top, "ms");
            self.line(&p, rect, FRAME, top, |s| Some(s.frame_ms));
            if let Some(pos) = resp.hover_pos()
                && let Some(s) = self.at(rect, pos)
            {
                crosshair(&p, rect, pos);
                resp.on_hover_text_at_pointer(format!("{:.1} s ago\n{:.1} ms per frame", self.now.t - s.t, s.frame_ms));
            }
        });
    }

    fn core_bars(&self, ui: &mut egui::Ui, size: Vec2) {
        ui.vertical(|ui| {
            ui.set_width(size.x);
            let start = ui.cursor().top();
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CPU cores").strong())
                    .on_hover_text("Current load of each CPU core, all processes together (Linux cannot split one core by process). The renderer uses the share of the cores set in Preferences (90% by default) while it traces.");
                if let Some((u, t)) = self.now.gpu_mem {
                    let mine = self.now.ray_mem.map_or(String::new(), |r| format!("ray {:.0} MiB, ", r * 1024.0));
                    ui.label(egui::RichText::new(format!("GPU memory: {mine}all {u:.1} / {t:.1} GiB")).color(ui.visuals().text_color()))
                        .on_hover_text("Video memory in use: this program's share, then the whole card.");
                }
            });
            // the height left under the header, exactly: more grows the panel every frame
            let used = ui.cursor().top() - start;
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(size.x, (size.y - used).max(40.0)), egui::Sense::hover());
            let p = ui.painter_at(rect);
            grid(ui, &p, rect, &[0.0, 50.0, 100.0], 100.0, "%");
            let n = self.cores.len().max(1) as f32;
            let gap = 2.0;
            let bw = ((rect.width() - 28.0 - gap * (n - 1.0)) / n).max(1.0);
            let mut hovered = None;
            for (i, &c) in self.cores.iter().enumerate() {
                let x0 = rect.left() + 28.0 + i as f32 * (bw + gap);
                let top = rect.bottom() - rect.height() * (c / 100.0).clamp(0.0, 1.0);
                let r = Rect::from_min_max(Pos2::new(x0, top), Pos2::new(x0 + bw, rect.bottom()));
                p.rect_filled(r, egui::CornerRadius { nw: 2, ne: 2, sw: 0, se: 0 }, CORES);
                if let Some(pos) = resp.hover_pos()
                    && pos.x >= x0
                    && pos.x < x0 + bw + gap
                {
                    hovered = Some((i, c));
                }
            }
            if let Some((i, c)) = hovered {
                resp.on_hover_text_at_pointer(format!("core {i}: {c:.0}%"));
            }
        });
    }

    fn line(&self, p: &egui::Painter, rect: Rect, col: Color32, top: f32, f: impl Fn(&Sample) -> Option<f32>) {
        let plot = plot_rect(rect);
        let mut pts = Vec::new();
        for s in &self.history {
            if let Some(v) = f(s) {
                let x = plot.right() - ((self.now.t - s.t) / WINDOW) as f32 * plot.width();
                let y = plot.bottom() - (v / top).clamp(0.0, 1.0) * plot.height();
                pts.push(Pos2::new(x, y));
            }
        }
        if pts.len() >= 2 {
            p.add(egui::Shape::line(pts, Stroke::new(2.0, col)));
        }
    }

    /// The sample under a hover position.
    fn at(&self, rect: Rect, pos: Pos2) -> Option<Sample> {
        let plot = plot_rect(rect);
        let t = self.now.t - ((plot.right() - pos.x) / plot.width()) as f64 * WINDOW;
        self.history.iter().min_by(|a, b| (a.t - t).abs().total_cmp(&(b.t - t).abs())).copied()
    }
}

/// The plot area: room on the left for the axis labels.
fn plot_rect(rect: Rect) -> Rect {
    Rect::from_min_max(Pos2::new(rect.left() + 28.0, rect.top() + 4.0), Pos2::new(rect.right() - 2.0, rect.bottom() - 2.0))
}

fn legend(ui: &mut egui::Ui, col: Color32, name: &str, value: Option<f32>, tip: &str) {
    let (r, _) = ui.allocate_exact_size(Vec2::new(14.0, 10.0), egui::Sense::hover());
    ui.painter().line_segment([Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y)], Stroke::new(2.0, col));
    let v = value.map_or("n/a".into(), |v| format!("{v:.0}%"));
    ui.label(egui::RichText::new(format!("{name} {v}")).color(ui.visuals().text_color())).on_hover_text(tip);
}

/// Recessive horizontal grid lines with their labels.
fn grid(ui: &egui::Ui, p: &egui::Painter, rect: Rect, ticks: &[f32], top: f32, unit: &str) {
    let plot = plot_rect(rect);
    let line = ui.visuals().widgets.noninteractive.bg_stroke.color.gamma_multiply(0.6);
    let text = ui.visuals().weak_text_color();
    for &v in ticks {
        let y = plot.bottom() - (v / top) * plot.height();
        p.line_segment([Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)], Stroke::new(1.0, line));
        let label = if v == 0.0 { "0".to_string() } else { format!("{v:.0}{unit}") };
        p.text(Pos2::new(rect.left() + 2.0, y), egui::Align2::LEFT_CENTER, label, egui::FontId::proportional(10.0), text);
    }
}

fn crosshair(p: &egui::Painter, rect: Rect, pos: Pos2) {
    let plot = plot_rect(rect);
    let x = pos.x.clamp(plot.left(), plot.right());
    p.line_segment([Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())], Stroke::new(1.0, Color32::from_white_alpha(90)));
}

/// 1, 2 or 5 times a power of ten, at least `v`.
fn nice_ceiling(v: f32) -> f32 {
    let e = 10f32.powf(v.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if m * e >= v {
            return m * e;
        }
    }
    10.0 * e
}

/// /proc/stat: the whole machine, then every core.
fn read_stat() -> Vec<CpuTimes> {
    let Ok(text) = std::fs::read_to_string("/proc/stat") else { return Vec::new() };
    text.lines()
        .filter(|l| l.starts_with("cpu"))
        .map(|l| {
            let v: Vec<u64> = l.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
            let get = |i: usize| v.get(i).copied().unwrap_or(0);
            // user nice system idle iowait irq softirq steal
            let idle = get(3) + get(4);
            let busy = get(0) + get(1) + get(2) + get(5) + get(6) + get(7);
            CpuTimes { busy, total: busy + idle }
        })
        .collect()
}

/// This process's user + system CPU time, in clock ticks.
fn read_proc() -> u64 {
    let Ok(text) = std::fs::read_to_string("/proc/self/stat") else { return 0 };
    let Some(rest) = text.rsplit_once(')').map(|r| r.1) else { return 0 };
    let f: Vec<&str> = rest.split_whitespace().collect();
    // after the command: state is field 3, utime 14, stime 15
    let get = |i: usize| f.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0);
    get(11) + get(12)
}

/// One reading of the GPU.
#[derive(Default)]
struct GpuSample {
    /// utilisation: whole card, this process
    total: Option<f32>,
    mine: Option<f32>,
    /// memory (GiB): used / total, this process
    mem: Option<(f32, f32)>,
    mine_mem: Option<f32>,
}

/// GPU counters: NVML (loaded at run time) or the kernel's busy percentage.
enum Gpu {
    Nvml(Nvml),
    Sysfs(std::path::PathBuf),
}

impl Gpu {
    fn open() -> Option<Gpu> {
        if let Some(n) = Nvml::open() {
            return Some(Gpu::Nvml(n));
        }
        let dir = std::fs::read_dir("/sys/class/drm").ok()?;
        for e in dir.flatten() {
            let p = e.path().join("device/gpu_busy_percent");
            if p.exists() {
                return Some(Gpu::Sysfs(p));
            }
        }
        None
    }

    fn name(&self) -> String {
        match self {
            Gpu::Nvml(n) => n.name.clone(),
            Gpu::Sysfs(_) => "GPU".into(),
        }
    }

    fn sample(&self) -> GpuSample {
        match self {
            Gpu::Nvml(n) => n.sample(),
            Gpu::Sysfs(p) => GpuSample { total: std::fs::read_to_string(p).ok().and_then(|s| s.trim().parse().ok()), ..Default::default() },
        }
    }
}

type Device = *mut core::ffi::c_void;

/// nvmlProcessUtilizationSample_t
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcUtil {
    pid: u32,
    timestamp: u64,
    sm: u32,
    mem: u32,
    enc: u32,
    dec: u32,
}

/// nvmlProcessInfo_t (v2 / v3)
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcInfo {
    pid: u32,
    used: u64,
    gpu_instance: u32,
    compute_instance: u32,
}

const NVML_ERROR_NOT_FOUND: i32 = 6;
const NVML_ERROR_INSUFFICIENT_SIZE: i32 = 7;

/// The few NVML calls used, resolved from libnvidia-ml.so.1.
struct Nvml {
    _lib: libloading::Library,
    util: unsafe extern "C" fn(Device, *mut [u32; 2]) -> i32,
    mem: unsafe extern "C" fn(Device, *mut [u64; 3]) -> i32,
    proc_util: Option<unsafe extern "C" fn(Device, *mut ProcUtil, *mut u32, u64) -> i32>,
    procs: Vec<unsafe extern "C" fn(Device, *mut u32, *mut ProcInfo) -> i32>,
    dev: Device,
    name: String,
    last_seen: std::cell::Cell<u64>,
}

impl Nvml {
    fn open() -> Option<Nvml> {
        // SAFETY: NVML's documented C API; the symbols are copied out as plain
        // function pointers while the library stays loaded in `_lib`.
        unsafe {
            let lib = libloading::Library::new("libnvidia-ml.so.1").ok()?;
            let init: unsafe extern "C" fn() -> i32 = *lib.get(b"nvmlInit_v2\0").ok()?;
            if init() != 0 {
                return None;
            }
            let handle: unsafe extern "C" fn(u32, *mut Device) -> i32 = *lib.get(b"nvmlDeviceGetHandleByIndex_v2\0").ok()?;
            let mut dev: Device = core::ptr::null_mut();
            if handle(0, &mut dev) != 0 {
                return None;
            }
            let util = *lib.get(b"nvmlDeviceGetUtilizationRates\0").ok()?;
            let mem = *lib.get(b"nvmlDeviceGetMemoryInfo\0").ok()?;
            let proc_util = lib.get(b"nvmlDeviceGetProcessUtilization\0").ok().map(|s| *s);
            let mut procs = Vec::new();
            for name in [&b"nvmlDeviceGetGraphicsRunningProcesses_v3\0"[..], &b"nvmlDeviceGetComputeRunningProcesses_v3\0"[..]] {
                if let Ok(f) = lib.get::<unsafe extern "C" fn(Device, *mut u32, *mut ProcInfo) -> i32>(name) {
                    procs.push(*f);
                }
            }
            let get_name: unsafe extern "C" fn(Device, *mut core::ffi::c_char, u32) -> i32 = *lib.get(b"nvmlDeviceGetName\0").ok()?;
            let mut buf = [0 as core::ffi::c_char; 96];
            let name = if get_name(dev, buf.as_mut_ptr(), buf.len() as u32) == 0 {
                core::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
            } else {
                "NVIDIA GPU".into()
            };
            Some(Nvml { _lib: lib, util, mem, proc_util, procs, dev, name, last_seen: std::cell::Cell::new(0) })
        }
    }

    fn sample(&self) -> GpuSample {
        let mut u = [0u32; 2];
        let mut m = [0u64; 3];
        // SAFETY: valid device handle and out-pointers to the documented struct layouts
        // (nvmlUtilization_t: gpu, memory; nvmlMemory_t: total, free, used)
        let (ru, rm) = unsafe { ((self.util)(self.dev, &mut u), (self.mem)(self.dev, &mut m)) };
        let gib = |b: u64| b as f32 / (1u64 << 30) as f32;
        GpuSample {
            total: (ru == 0).then_some(u[0] as f32),
            mine: self.my_util(),
            mem: (rm == 0).then(|| (gib(m[2]), gib(m[0]))),
            mine_mem: self.my_memory().map(gib),
        }
    }

    /// This process's average SM utilisation since the last reading.
    fn my_util(&self) -> Option<f32> {
        let f = self.proc_util?;
        let pid = std::process::id();
        let mut n = 0u32;
        // SAFETY: a null buffer with a zero count asks for the sample count
        let r = unsafe { f(self.dev, core::ptr::null_mut(), &mut n, self.last_seen.get()) };
        if r == NVML_ERROR_NOT_FOUND || (r == 0 && n == 0) {
            return Some(0.0);
        }
        if r != NVML_ERROR_INSUFFICIENT_SIZE && r != 0 {
            return None;
        }
        let mut buf = vec![ProcUtil::default(); n as usize + 8];
        let mut n = buf.len() as u32;
        // SAFETY: `buf` holds `n` samples
        let r = unsafe { f(self.dev, buf.as_mut_ptr(), &mut n, self.last_seen.get()) };
        if r == NVML_ERROR_NOT_FOUND {
            return Some(0.0);
        }
        if r != 0 {
            return None;
        }
        let got = &buf[..n as usize];
        if let Some(t) = got.iter().map(|s| s.timestamp).max() {
            self.last_seen.set(t);
        }
        let mine: Vec<f32> = got.iter().filter(|s| s.pid == pid).map(|s| s.sm as f32).collect();
        Some(if mine.is_empty() { 0.0 } else { mine.iter().sum::<f32>() / mine.len() as f32 })
    }

    /// This process's video memory (bytes), when the driver reports it.
    fn my_memory(&self) -> Option<u64> {
        let pid = std::process::id();
        let mut total = None;
        for f in &self.procs {
            let mut buf = vec![ProcInfo::default(); 128];
            let mut n = buf.len() as u32;
            // SAFETY: `buf` holds `n` entries
            if unsafe { f(self.dev, &mut n, buf.as_mut_ptr()) } != 0 {
                continue;
            }
            for p in &buf[..n as usize] {
                // u64::MAX: not available (e.g. some display servers)
                if p.pid == pid && p.used != u64::MAX {
                    *total.get_or_insert(0) += p.used;
                }
            }
        }
        total
    }
}
