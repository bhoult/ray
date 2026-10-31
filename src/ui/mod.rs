//! The interactive editor and viewer. The scene document is edited in place
//! (outliner and inspector on the left), rendered on a background thread
//! (settings on the right), with the machine's load at the bottom.
//!
//! In the view: left click selects, left drag on a movable object picks it up
//! (it drops with physics), left drag elsewhere looks around, right drag pans,
//! the wheel moves forward and back, WASD flies.

pub(crate) mod actions;
pub mod doc;
mod emitter;
mod config;
mod game;
mod record;
mod help;
mod inspector;
mod library;
mod materials;
mod panel;
mod perf;
pub mod physics;
mod previews;
mod control;
mod remote;
mod render_thread;
pub mod schema;
mod style;

use crate::camera::Camera;
use crate::math::{Mat3, Vec3};
use crate::render::{FrameStats, ObjCost, View, Waves, pick_ray};
use crate::scene::{Scene, TerrainCache};
use crate::settings::{Mode, Settings};
use actions::Act;
use doc::{Doc, Selection};
use eframe::egui;
use materials::Library;
use perf::Perf;
use physics::Body;
use render_thread::{Job, RenderThread};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime};

/// When the viewer started, for the startup times it logs.
static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();

/// Milliseconds since the viewer started.
pub(crate) fn since_start() -> u128 {
    STARTED.get().map_or(0, |t| t.elapsed().as_millis())
}

pub fn run(path: PathBuf) -> Result<(), String> {
    STARTED.get_or_init(Instant::now);
    let (config, config_error) = config::Config::load();
    // the renderer may use a share of the cores (90% unless the preferences
    // say otherwise); the rest keeps the UI and the compositor smooth
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    let _ = rayon::ThreadPoolBuilder::new()
        .num_threads(((cores as f32 * config.cpu_share.clamp(0.1, 1.0)).round() as usize).max(1))
        .thread_name(|i| format!("trace {i}"))
        .build_global();
    // (the cycle counter's rate, measured over 20 ms off the interface's
    // thread: the heat map's and the inspector's costs ask for it)
    std::thread::spawn(crate::render::ticks_per_us);
    record::probe();
    // the window on Vulkan (probing GL as well took 70 ms at start); where
    // that cannot open a window, again with GL
    match launch(&path, wgpu::Backends::PRIMARY, &config, config_error.clone()) {
        Err(e) if !e.contains("ray:") && wgpu::Backends::from_env().is_none() => {
            eprintln!("ray: {e}; trying again with OpenGL as well");
            launch(&path, wgpu::Backends::PRIMARY | wgpu::Backends::GL, &config, config_error)
        }
        r => r,
    }
}

/// Opens the window on `backends` (the scene loaded afresh for it).
fn launch(path: &std::path::Path, backends: wgpu::Backends, config: &config::Config, config_error: Option<String>) -> Result<(), String> {
    let config = config.clone();
    let (mut v, moment) = crate::scene::read_doc(path).map_err(|e| format!("ray: {}: {e}", path.display()))?;
    crate::scene::json::migrate(&mut v);
    // (a screenshot: the file it came from is the one edited)
    let shot = moment.as_ref().map(|m| (path.to_path_buf(), m.clone()));
    let path = moment.as_ref().map_or_else(|| path.to_path_buf(), |m| still_source(path, m));
    let mut cache = TerrainCache::default();
    let scene = Scene::from_value(&v, &mut cache).map_err(|e| format!("ray: {}: {e}", path.display()))?;
    let settings = crate::initial_settings(&scene, None).map_err(|e| format!("ray: {e}"))?;
    // (the app id names the window to the desktop, which finds ray.desktop by
    // it: tools/install-desktop.sh; without one GNOME calls it "Unknown")
    let mut viewport = egui::ViewportBuilder::default().with_title("ray").with_app_id("ray").with_inner_size([1700.0, 1000.0]);
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Wgpu,
        // frame times should show the renderer's cost, not the display's refresh
        vsync: false,
        wgpu_options: window_device(backends),
        ..Default::default()
    };
    eframe::run_native(
        "ray",
        options,
        Box::new(move |cc| {
            eprintln!("ray: window up {} ms after start", since_start());
            let mut app = App::new(cc.egui_ctx.clone(), path, v, scene, cache, settings, config);
            if let Some((png, m)) = &shot {
                app.take_moment(png, m);
            }
            if let Some(e) = &config_error {
                app.say(format!("preferences: {e}"));
            }
            app.config_error = config_error;
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| e.to_string())
}

/// Why a command is greyed out (its menu entry says so on hover).
fn why_not(act: Act) -> &'static str {
    match act {
        Act::Undo => "Nothing to undo.",
        Act::Redo => "Nothing to redo.",
        Act::Duplicate | Act::Delete | Act::Frame => "Select an object first.",
        Act::QuickLoad => "Nothing quick saved yet (F5).",
        _ => "",
    }
}

/// How long the menu bar (or game mode's view) says where a recording went.
const VIDEO_NOTE_SECS: f32 = 20.0;

/// Opens a file or folder with the desktop's choice of program.
fn open_externally(p: &std::path::Path) {
    let _ = std::process::Command::new("xdg-open").arg(p).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}

/// A length as people say it, to three figures: metres down to nanometres.
fn length_text(m: f64) -> String {
    let (v, unit) = match m {
        m if m >= 1.0 => (m, "m"),
        m if m >= 1e-3 => (m * 1e3, "mm"),
        m if m >= 1e-6 => (m * 1e6, "µm"),
        m => (m * 1e9, "nm"),
    };
    let digits = if v >= 100.0 { 0 } else if v >= 10.0 { 1 } else { 2 };
    format!("{v:.digits$} {unit}")
}


/// The window's icon: assets/icon.png, a render of assets/icon.json.
fn icon() -> Option<egui::IconData> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(&include_bytes!("../../assets/icon.png")[..]));
    dec.set_transformations(png::Transformations::normalize_to_color8() | png::Transformations::ALPHA);
    let mut reader = dec.read_info().ok()?;
    let mut rgba = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut rgba).ok()?;
    rgba.truncate(info.buffer_size());
    (info.color_type == png::ColorType::Rgba).then_some(egui::IconData { rgba, width: info.width, height: info.height })
}

/// eframe's device for the window, with wgpu's smaller memory blocks: by
/// default it takes host memory in 128 MB blocks, and pins them
fn window_device(backends: wgpu::Backends) -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuSetup, WgpuSetupCreateNew};
    let mut base = WgpuSetupCreateNew::default();
    // (WGPU_BACKEND, when set, still decides)
    base.instance_descriptor.backends = wgpu::Backends::from_env().unwrap_or(backends);
    let make = base.device_descriptor.clone();
    eframe::egui_wgpu::WgpuConfiguration {
        wgpu_setup: WgpuSetup::CreateNew(WgpuSetupCreateNew {
            device_descriptor: std::sync::Arc::new(move |adapter| wgpu::DeviceDescriptor {
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                ..make(adapter)
            }),
            ..base
        }),
        ..Default::default()
    }
}

/// A mouse drag in the view.
#[derive(Clone, Copy, PartialEq)]
enum Gesture {
    Look,
    /// moves the camera in its plane; metres per point at the grabbed depth
    Pan(f32),
    /// carries `objects[top]`: distance along the ray and the offset from the
    /// grabbed point to its centre of mass
    Carry { top: usize, dist: f32, offset: Vec3 },
    /// slides `objects[top]` along world axis `axis` (the move handles): its
    /// position when grabbed, and where along the axis it was grabbed
    Axis { top: usize, axis: usize, start: Vec3, grab: f32 },
    /// turns `objects[top]` (the rotate rings): its rotation so far, and the
    /// pointer's angle about its centre on the screen last frame
    Turn { top: usize, axis: usize, rot: Mat3, angle: f32 },
    /// resizes `objects[top]` along its own axis `axis` (the scale handles):
    /// where along the axis it was grabbed, from `origin` along `dir`. The
    /// node as grabbed is in `App::size_start`
    Size { top: usize, axis: usize, grab: f32, origin: Vec3, dir: Vec3 },
}

impl Gesture {
    /// The object a handle is dragging.
    fn handle_target(&self) -> Option<usize> {
        match *self {
            Gesture::Axis { top, .. } | Gesture::Turn { top, .. } | Gesture::Size { top, .. } => Some(top),
            _ => None,
        }
    }

    /// The axis of the handle being dragged.
    fn handle_axis(&self) -> Option<usize> {
        match *self {
            Gesture::Axis { axis, .. } | Gesture::Turn { axis, .. } | Gesture::Size { axis, .. } => Some(axis),
            _ => None,
        }
    }
}

/// A handle drag driven by RAY_TEST_DRAG=tool,axis,dx,dy[,shift] (tests):
/// the pointer is grabbed at the handle at frame 30, moved by (dx, dy)
/// points over 20 frames and let go.
#[derive(Clone, Copy)]
struct TestDrag {
    tool: Tool,
    axis: usize,
    delta: egui::Vec2,
    shift: bool,
    /// the pointer now, and whether its button is down
    pos: egui::Pos2,
    down: bool,
    start: egui::Pos2,
    /// the pointer as the gesture last saw it
    prev: egui::Pos2,
}

/// The scene file a screenshot `png` was taken of, as its moment says:
/// saving its scene writes there, never into the picture.
fn still_source(png: &std::path::Path, moment: &serde_json::Value) -> PathBuf {
    moment
        .get("source")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .unwrap_or_else(|| png.with_extension("json"))
}

/// A screenshot in the Open window: its objects and what it shows (None:
/// a picture with no scene in it).
fn still_entry(png: &std::path::Path) -> Option<(usize, String)> {
    let f = std::io::BufReader::new(std::fs::File::open(png).ok()?);
    let t = crate::render::png_text(f, &[crate::scene::STILL_MOMENT]).ok()?.pop()??;
    let m: serde_json::Value = serde_json::from_str(&t).ok()?;
    let n = m.get("objects").and_then(serde_json::Value::as_u64).unwrap_or(0) as usize;
    let from = still_source(png, &m).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut d = format!("screenshot of {from}");
    if let Some(h) = m.pointer("/game/height").and_then(serde_json::Value::as_f64) {
        d += &format!(", the walker {} tall", length_text(h));
    }
    Some((n, d))
}

/// Where a screenshot of scene `stem` goes in `dir`: named by the local
/// time `stamp`, as recordings are; a second one within the same second
/// gets -2, -3...
fn shot_path(dir: &std::path::Path, stem: &str, stamp: &str) -> PathBuf {
    let base = format!("{stem}-{stamp}");
    let mut out = dir.join(format!("{base}.png"));
    for k in 2.. {
        if !out.exists() {
            break;
        }
        out = dir.join(format!("{base}-{k}.png"));
    }
    out
}

/// The first sentence of `text`, at most `max` characters (with … when cut).
fn first_sentence(text: &str, max: usize) -> String {
    let s = text.split_inclusive(". ").next().unwrap_or(text).trim_end();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>().trim_end())
    }
}

/// What a screenshot of the whole window (P with the interface) is asked
/// with, so it is told from the recording's and the tests'.
#[derive(Clone, Copy, Debug)]
struct WindowShot;

/// The camera easing from one pose to another (the control socket's
/// `glide`) over `secs` of the scene's clock from `t0`, smoothly or at an
/// even rate: its position (none: it stays), and yaw, pitch and field of
/// view (radians, radians, degrees).
struct Glide {
    from_pos: [f64; 3],
    to_pos: Option<[f64; 3]>,
    from: [f32; 3],
    to: [f32; 3],
    t0: f64,
    secs: f32,
    linear: bool,
}

/// The Open and Save as window: the scene files in a folder, and the name
/// typed or picked.
/// A screenshot rendering whole (P): the moment as it was when P went down
/// (the file and the picture's text chunks: keyword, text, compressed),
/// written once the view, held still at the screenshot's size, has every
/// sample (`posted`: the job it waits for, once it is posted).
struct Still {
    out: PathBuf,
    text: Vec<(&'static str, String, bool)>,
    posted: Option<u64>,
}

struct FileDialog {
    save: bool,
    dir: PathBuf,
    name: String,
    /// each file: its path, objects, and description
    files: Vec<(PathBuf, usize, String)>,
    /// the folders in `dir`
    dirs: Vec<PathBuf>,
    /// Save as onto an existing file asks again
    confirm: bool,
}

impl FileDialog {
    fn new(save: bool, current: &std::path::Path) -> FileDialog {
        let dir = current.parent().filter(|d| d.is_dir()).map_or_else(|| PathBuf::from("saves"), PathBuf::from);
        let name = current.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let mut d = FileDialog { save, dir, name, files: Vec::new(), dirs: Vec::new(), confirm: false };
        d.list();
        d
    }

    /// Reads the folder: its scenes (with their sizes and descriptions) and folders.
    fn list(&mut self) {
        let entries: Vec<PathBuf> = std::fs::read_dir(&self.dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
        self.dirs = entries.iter().filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).cloned().collect();
        self.dirs.sort();
        self.files = entries
            .into_iter()
            .filter_map(|p| match p.extension().and_then(|x| x.to_str()) {
                Some("json") => {
                    let v: serde_json::Value = std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                    let n = v.get("objects").and_then(serde_json::Value::as_array).map_or(0, Vec::len);
                    let d = v.get("description").and_then(serde_json::Value::as_str).unwrap_or("").to_string();
                    Some((p, n, d))
                }
                // (screenshots open as the moment they were taken)
                Some("png") if !self.save => still_entry(&p).map(|(n, d)| (p, n, d)),
                _ => None,
            })
            .collect();
        self.files.sort();
        self.confirm = false;
    }

    /// Into another folder.
    fn go(&mut self, dir: PathBuf) {
        self.dir = dir;
        self.list();
    }

    /// The typed name as a path: in the folder unless it has one, with .json.
    fn target(&self) -> PathBuf {
        let mut p = PathBuf::from(self.name.trim());
        if p.extension().is_none() {
            p.set_extension("json");
        }
        if p.components().count() == 1 { self.dir.join(p) } else { p }
    }
}

/// What the selected object's handles do (the view's toolbar, or T).
#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum Tool {
    #[default]
    Move,
    Rotate,
    Scale,
}

impl Tool {
    const ALL: [Tool; 3] = [Tool::Move, Tool::Rotate, Tool::Scale];

    fn label(self) -> &'static str {
        match self {
            Tool::Move => "Move",
            Tool::Rotate => "Rotate",
            Tool::Scale => "Scale",
        }
    }

    fn tip(self) -> &'static str {
        match self {
            Tool::Move => "Drag an x / y / z arrow to slide the object along that axis (Ctrl: on a 10 cm grid). T cycles the tools.",
            Tool::Rotate => "Drag a ring to turn the object freely, like a trackball; hold Shift to turn it only about that ring's axis. T cycles the tools.",
            Tool::Scale => "Drag a handle to resize the object evenly; hold Shift to stretch it only along that axis (its own axes: boxes, ellipsoids, clouds, cylinders, cones; a sphere becomes an ellipsoid; other shapes scale evenly). T cycles the tools.",
        }
    }
}

/// The selected object's handles: where it is, each axis handle's ends on
/// the screen (world axes to move, the object's own to scale), and the
/// rotate rings as screen polylines.
struct Handles {
    top: usize,
    origin: Vec3,
    dirs: [Vec3; 3],
    arrows: [(egui::Pos2, egui::Pos2); 3],
    rings: [Vec<egui::Pos2>; 3],
}

const AXES: [Vec3; 3] = [Vec3::new(1.0, 0.0, 0.0), Vec3::Y, Vec3::new(0.0, 0.0, 1.0)];
const AXIS_NAMES: [&str; 3] = ["x", "y", "z"];
const AXIS_COLORS: [egui::Color32; 3] =
    [egui::Color32::from_rgb(0xe0, 0x48, 0x48), egui::Color32::from_rgb(0x4c, 0xc0, 0x50), egui::Color32::from_rgb(0x4a, 0x84, 0xf0)];

/// Where along the line `a + e * s` (unit `e`) comes closest to the ray
/// `o + d * t`; None when they are nearly parallel.
fn closest_on_axis(a: Vec3, e: Vec3, o: Vec3, d: Vec3) -> Option<f32> {
    let w = a - o;
    let b = e.dot(d);
    let denom = 1.0 - b * b;
    if denom < 1e-4 {
        return None;
    }
    Some((b * d.dot(w) - e.dot(w)) / denom)
}

/// A cloud node at `p`: a volume cloud, the default cumulus.
pub(crate) fn cloud_node(p: [f64; 3]) -> serde_json::Value {
    serde_json::json!({ "name": "cloud", "type": "cloud", "position": p })
}

/// A rotation by `a` radians about the unit axis `e`.
fn turn(e: Vec3, a: f32) -> Mat3 {
    let (s, c) = a.sin_cos();
    let t = 1.0 - c;
    Mat3 {
        m: [
            Vec3::new(t * e.x * e.x + c, t * e.x * e.y - s * e.z, t * e.x * e.z + s * e.y),
            Vec3::new(t * e.x * e.y + s * e.z, t * e.y * e.y + c, t * e.y * e.z - s * e.x),
            Vec3::new(t * e.x * e.z - s * e.y, t * e.y * e.z + s * e.x, t * e.z * e.z + c),
        ],
    }
}

/// Stretches a shape node along its own axis `axis` by `f`, where its kind
/// has a size along each axis: boxes, ellipsoids, clouds (`stretch`),
/// cylinders and cones (their radius for x and z); a sphere becomes an
/// ellipsoid. False for the rest.
fn stretch(n: &mut serde_json::Map<String, serde_json::Value>, axis: usize, f: f64) -> bool {
    use serde_json::Value;
    let round = |v: f64| Value::from((v * 1000.0).round() / 1000.0);
    let num = |n: &serde_json::Map<String, Value>, k: &str, d: f64| n.get(k).and_then(Value::as_f64).unwrap_or(d);
    let vec = |n: &serde_json::Map<String, Value>, k: &str, d: [f64; 3]| n.get(k).and_then(doc::as_vec3).unwrap_or(d);
    let sized = |n: &mut serde_json::Map<String, Value>, k: &str, mut v: [f64; 3]| {
        v[axis] *= f;
        n.insert(k.into(), Value::Array(v.iter().map(|&c| round(c)).collect()));
    };
    match n.get("type").and_then(Value::as_str).unwrap_or("") {
        "box" => {
            let v = vec(n, "size", [1.0; 3]);
            sized(n, "size", v);
        }
        "ellipsoid" => {
            let v = vec(n, "radii", [1.0, 0.5, 0.75]);
            sized(n, "radii", v);
        }
        "cloud" => {
            let v = vec(n, "stretch", [1.0; 3]);
            sized(n, "stretch", v);
        }
        "sphere" => {
            let r = num(n, "radius", 1.0);
            n.remove("radius");
            n.insert("type".into(), "ellipsoid".into());
            sized(n, "radii", [r; 3]);
        }
        // NURBS: the control points themselves (a revolve's heights, or its radii all round)
        "revolve" => {
            let mut prof = n.get("profile").cloned().unwrap_or_else(|| serde_json::to_value(crate::scene::json::vase()).unwrap_or_default());
            let k = if axis == 1 { 1 } else { 0 };
            for p in prof.as_array_mut().into_iter().flatten() {
                if let Some(c) = p.get(k).and_then(Value::as_f64) {
                    p[k] = round(c * f);
                }
            }
            n.insert("profile".into(), prof);
        }
        "nurbs" => {
            let mut pts = n.get("points").cloned().unwrap_or_else(|| serde_json::to_value(crate::scene::json::sheet()).unwrap_or_default());
            for p in pts.as_array_mut().into_iter().flatten().flat_map(|row| row.as_array_mut().into_iter().flatten()) {
                if let Some(c) = p.get(axis).and_then(Value::as_f64) {
                    p[axis] = round(c * f);
                }
            }
            n.insert("points".into(), pts);
        }
        "cylinder" | "cone" => {
            if axis == 1 {
                let h = num(n, "height", 1.0);
                n.insert("height".into(), round(h * f));
            } else {
                let r = num(n, "radius", 0.5);
                n.insert("radius".into(), round(r * f));
                if let Some(t) = n.get("top_radius").and_then(Value::as_f64) {
                    n.insert("top_radius".into(), round(t * f));
                }
            }
        }
        _ => return false,
    }
    true
}

/// The angle of `p` about `centre` on the screen, counterclockwise.
fn screen_angle(centre: egui::Pos2, p: egui::Pos2) -> f32 {
    (centre.y - p.y).atan2(p.x - centre.x)
}

/// Distance from `p` to the segment `a`-`b` (screen points).
fn segment_distance(p: egui::Pos2, a: egui::Pos2, b: egui::Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

pub struct App {
    path: PathBuf,
    doc: Doc,
    scene: Arc<Scene>,
    compiled: u64,
    /// the emitters' copies (not in the document), and their version compiled
    emitters: emitter::Emitters,
    compiled_emit: u64,
    scene_id: u64,
    cache: TerrainCache,
    error: Option<String>,
    mtime: Option<SystemTime>,
    last_check: Instant,
    changed_on_disk: bool,
    message: Option<(String, Instant)>,
    settings: Settings,
    cam: Camera,
    speed: f32,
    sel: Selection,
    library: Library,
    lib_name: String,
    /// the material library window, its filter and the entry being renamed
    show_library: bool,
    /// the modifier library window
    show_modifiers: bool,
    /// a widget had the keyboard last frame (its Esc is not ours)
    had_focus: bool,
    /// the Environment window (sun, sky, time of day, fog)
    show_env: bool,
    /// the headlight's strength when L last turned it off
    headlight_was: f32,
    lib_filter: String,
    lib_edit: Option<library::LibEdit>,
    previews: previews::Previews,
    render: RenderThread,
    last_job: Option<Job>,
    texture: Option<egui::TextureHandle>,
    /// the GPU's name, error and shader build time, as the render thread last reported
    gpu_state: Option<(String, Option<String>, f32, bool)>,
    /// the last frame shown (for screenshots)
    shown: Option<Arc<egui::ColorImage>>,
    stats: FrameStats,
    costs: Vec<ObjCost>,
    perf: Perf,
    heat: bool,
    profile: bool,
    physics: bool,
    show_scene: bool,
    show_render: bool,
    show_perf: bool,
    help: bool,
    /// Revert with unsaved edits waits for a second press until then
    revert_armed: Option<Instant>,
    /// Quick load was asked for once over unsaved edits
    quickload_armed: Option<Instant>,
    gesture: Option<Gesture>,
    /// what the selected object's handles do
    pub(super) tool: Tool,
    /// the node a scale handle grabbed, as it was
    size_start: Option<serde_json::Value>,
    /// objects placed by hand with the handles (top-level indices): they stay
    /// put, floating or not, until dropped; and how many objects there were
    /// (indices shift when that changes, so the set is cleared)
    placed: (std::collections::HashSet<usize>, usize),
    test_drag: Option<TestDrag>,
    file_dialog: Option<FileDialog>,
    /// the control socket (the MCP server's way in), and screenshots it
    /// asked for that wait for the image to settle
    remote: Option<remote::Remote>,
    shots: Vec<control::PendingShot>,
    /// a screenshot of the whole window to ask egui for (Preferences: with
    /// the interface)
    /// since when, and the frame and time it was last asked at (None: not yet)
    window_shot: Option<(Instant, Option<(u32, Instant)>)>,
    /// a screenshot rendering whole (P), and MCP callers waiting for it
    still: Option<Still>,
    still_waiters: Vec<std::sync::mpsc::Sender<Result<serde_json::Value, String>>>,
    /// how the last one went: its file, or why it failed
    last_still: Option<Result<PathBuf, String>>,
    /// jobs posted to the render thread, and the one the shown image is of
    posted: u64,
    shown_serial: u64,
    world: physics::World,
    /// the scene as opened (Reset scene goes back to it), and whether the
    /// file still holds it (no save since)
    original: serde_json::Value,
    original_on_disk: bool,
    /// the water's clock starts here
    clock: Instant,
    /// a top-level object whose physics changed in the inspector: it moves
    /// again at once
    pub(super) wake: Option<usize>,
    /// the inspector's work, kept while its node is the same but for where it
    /// is (a body moving under the physics changes the document every frame):
    /// the object's mass without a `mass` of its own, and the node with its
    /// defaults filled in
    natural_mass: Option<inspector::MassCache>,
    defaults_cache: Option<inspector::DefaultsCache>,
    /// when the right button started charging a throw
    charge: Option<Instant>,
    /// when the view or the scene last changed (fast-while-moving)
    last_change: Instant,
    last_pose: [f32; 9],
    last_version: u64,
    last: Instant,
    frame_ms: f32,
    view_rect: egui::Rect,
    render_size: (usize, usize),
    /// the view's size in pixels (whatever the render scale)
    view_px: (f32, f32),
    /// game mode: the walker (None: editing), and the eye it last put the
    /// camera at (a camera moved otherwise, by a quick load or a remote,
    /// starts the walker there anew)
    game: Option<game::Game>,
    game_eye: [f64; 3],
    /// game mode: Shift tapped while moving keeps it running until it stops;
    /// and whether Shift was down last frame (egui has no key event for it)
    run_on: bool,
    shift_was: bool,
    /// keys and mouse buttons held from outside (the control socket's
    /// `hold`): each name, until when (the scene's clock), and whether it
    /// went down yet
    held: Vec<(String, f64, bool)>,
    /// the camera easing to a pose (the control socket's `glide`), and the
    /// glides queued after it
    glides: std::collections::VecDeque<Glide>,
    /// game mode's gravity gun: charging since (the left button held);
    /// armed once the button has been up (the click that started game mode
    /// does not fire it); when it last fired (its flash); whether something
    /// it can move is under the crosshair, in reach
    /// (the scene's clock)
    gun: Option<f64>,
    gun_armed: bool,
    gun_shot: Option<f64>,
    gun_target: bool,
    /// game mode holds the pointer (the desktop lets go when the window
    /// loses focus; it is taken again when it comes back)
    game_grab: bool,
    /// a Tab (true: Shift+Tab) taken from egui's input (raw_input_hook), to
    /// carry out this frame
    tab: Option<bool>,
    /// the preferences (config.rs: for every scene), their window, and what
    /// went wrong reading or writing them
    config: config::Config,
    show_prefs: bool,
    config_error: Option<String>,
    /// the preferences as last written (a change anywhere is saved); when
    /// "Back to the defaults" asked to be clicked again
    config_saved: config::Config,
    prefs_reset_armed: Option<Instant>,
    /// tests: the microphone for recordings, whatever the preferences say
    test_mic: Option<bool>,
    /// recording the window (F10); a stopped recording finishing its file;
    /// the window's size in pixels; the window's title as last set
    recorder: Option<record::Recorder>,
    /// a steady recording's clock (the scene moves a frame of the video at
    /// a time); after one, how far the scene's clock is behind the wall's
    steady: Option<record::Steady>,
    clock_behind: f64,
    record_done: Option<std::thread::JoinHandle<Result<String, String>>>,
    window_px: [usize; 2],
    title: String,
    /// the last finished recording (absolute path) and when it was saved:
    /// the menu bar (or game mode's view) says so for a while, with buttons
    /// to play it and open its folder
    last_video: Option<(PathBuf, Instant)>,
    /// why the last recording failed (until the next starts)
    record_error: Option<String>,
    /// the cost view: per object, nanoseconds per evaluation of its shape
    prices: Vec<f32>,
    rng: u64,
    /// RAY_TEST_SHOT=path RAY_TEST_FRAMES=n: capture the window after n frames and quit (tests)
    test_shot: Option<(PathBuf, u32)>,
    /// the heap at the last test log line
    #[cfg(feature = "profile")]
    heap: crate::prof::Heap,
    frames: u32,
}

/// The most pixels a frame may have (about 12 MP, e.g. 4620 x 2600; memory
/// and the texture stay reasonable) and the longest side (the GPU's texture limit).
const MAX_PIXELS: f32 = 12.0e6;
const MAX_SIDE: f32 = 8192.0;

/// The render size for a view of `w` x `h` pixels: shrunk evenly (never per
/// axis, which would squash the image) to fit the limits.
fn render_size(w: f32, h: f32) -> (usize, usize) {
    let (w, h) = (w.max(8.0), h.max(8.0));
    let k = (MAX_PIXELS / (w * h)).sqrt().min(MAX_SIDE / w).min(MAX_SIDE / h).min(1.0);
    (((w * k) as usize).max(8), ((h * k) as usize).max(8))
}

/// A steady recording's frame size for `w` x `h` asked for: within the
/// render's limits (render_size), and even (the encoder's 4:2:0 wants it).
fn video_size(w: f32, h: f32) -> [usize; 2] {
    let (w, h) = render_size(w.max(16.0), h.max(16.0));
    [w & !1, h & !1]
}

fn mtime(p: &std::path::Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl App {
    fn new(
        ctx: egui::Context,
        path: PathBuf,
        v: serde_json::Value,
        scene: Scene,
        cache: TerrainCache,
        settings: Settings,
        config: config::Config,
    ) -> App {
        style::apply(&ctx);
        let problems = (!scene.problems.is_empty()).then(|| scene.problems.join("; "));
        let cam = crate::start_camera(&scene);
        let speed = scene.camera.speed;
        let mut app = App {
            mtime: mtime(&path),
            path,
            original: v.clone(),
            original_on_disk: true,
            clock: Instant::now(),
            wake: None,
            natural_mass: None,
            defaults_cache: None,
            doc: Doc::new(v),
            scene: Arc::new(scene),
            compiled: 1,
            emitters: emitter::Emitters::default(),
            compiled_emit: 0,
            scene_id: 1,
            cache,
            error: problems,
            last_check: Instant::now(),
            changed_on_disk: false,
            message: None,
            settings,
            cam,
            speed,
            sel: Selection::None,
            library: Library::load(),
            lib_name: String::new(),
            show_library: false,
            show_modifiers: std::env::var_os("RAY_TEST_MODIFIERS").is_some(),
            had_focus: false,
            show_env: false,
            headlight_was: 1.0,
            lib_filter: String::new(),
            lib_edit: None,
            previews: previews::Previews::new(ctx.clone()),
            render: RenderThread::start(ctx.clone()),
            gpu_state: None,
            last_job: None,
            texture: None,
            shown: None,
            stats: FrameStats::default(),
            costs: Vec::new(),
            perf: Perf::new(),
            heat: false,
            profile: true,
            physics: true,
            show_scene: true,
            show_render: true,
            show_perf: true,
            help: false,
            revert_armed: None,
            quickload_armed: None,
            gesture: None,
            tool: Tool::default(),
            size_start: None,
            placed: Default::default(),
            test_drag: None,
            file_dialog: None,
            remote: if std::env::var_os("RAY_NO_SOCKET").is_some() { None } else { remote::Remote::start(ctx.clone()) },
            shots: Vec::new(),
            window_shot: None,
            still: None,
            still_waiters: Vec::new(),
            last_still: None,
            posted: 0,
            shown_serial: 0,
            world: physics::World::default(),
            charge: None,
            last_change: Instant::now(),
            last_pose: [0.0; 9],
            last_version: 0,
            last: Instant::now(),
            frame_ms: 0.0,
            view_rect: egui::Rect::NOTHING,
            render_size: (0, 0),
            view_px: (0.0, 0.0),
            game: None,
            game_eye: [0.0; 3],
            run_on: false,
            shift_was: false,
            held: Vec::new(),
            glides: Default::default(),
            gun: None,
            gun_armed: false,
            gun_shot: None,
            gun_target: false,
            game_grab: false,
            tab: None,
            config_saved: config.clone(),
            prefs_reset_armed: None,
            config,
            show_prefs: false,
            config_error: None,
            test_mic: None,
            recorder: None,
            steady: None,
            clock_behind: 0.0,
            record_done: None,
            window_px: [0, 0],
            title: String::new(),
            last_video: None,
            record_error: None,
            prices: Vec::new(),
            rng: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7),
            #[cfg(feature = "profile")]
            heap: crate::prof::heap(),
            test_shot: std::env::var_os("RAY_TEST_SHOT").map(|p| {
                let n = std::env::var("RAY_TEST_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
                (PathBuf::from(p), n)
            }),
            frames: 0,
        };
        // RAY_TEST_SELECT=n selects object n, n/a/b its part a's part b,
        // =env the environment (tests)
        match std::env::var("RAY_TEST_SELECT").ok().as_deref() {
            Some("env") => app.show_env = true,
            Some(i) => {
                let mut it = i.split('/').map(str::parse::<usize>);
                if let Some(Ok(top)) = it.next() {
                    app.sel = Selection::Node { top, sub: it.flatten().collect() };
                }
            }
            None => {}
        }
        // RAY_TEST_PREFS=1: the Preferences window shows (tests)
        app.show_prefs = std::env::var_os("RAY_TEST_PREFS").is_some();
        // RAY_TEST_OPEN=1: the Open window shows (tests)
        if std::env::var_os("RAY_TEST_OPEN").is_some() {
            app.file_dialog = Some(FileDialog::new(false, &app.path));
        }
        if let Ok(v) = std::env::var("RAY_TEST_DRAG") {
            let f: Vec<&str> = v.split(',').collect();
            let num = |i: usize| f.get(i).and_then(|v| v.trim().parse::<f32>().ok()).unwrap_or(0.0);
            let tool = match f.first().map(|t| t.trim()) {
                Some("rotate") => Tool::Rotate,
                Some("scale") => Tool::Scale,
                _ => Tool::Move,
            };
            app.tool = tool;
            let p = egui::Pos2::ZERO;
            app.test_drag = Some(TestDrag { tool, axis: num(1) as usize, delta: egui::vec2(num(2), num(3)), shift: num(4) != 0.0, pos: p, down: false, start: p, prev: p });
        }
        // RAY_TEST_LIBRARY=1 opens the material library (tests)
        app.show_library = std::env::var_os("RAY_TEST_LIBRARY").is_some();
        // RAY_TEST_HELP=1 opens the help, =controls or =materials on that tab (tests)
        if let Ok(v) = std::env::var("RAY_TEST_HELP") {
            app.help = true;
            match v.as_str() {
                "controls" => help::show_tab(&ctx, help::Tab::Controls),
                "materials" => help::show_tab(&ctx, help::Tab::Materials),
                _ => {}
            }
        }
        if std::env::var_os("RAY_TEST_CHARGE").is_some() {
            app.charge = Instant::now().checked_sub(std::time::Duration::from_millis(700));
        }
        app.heat = std::env::var_os("RAY_TEST_HEAT").is_some();
        // RAY_TEST_DROP=n: object n falls from where it is (physics tests)
        if let Ok(i) = std::env::var("RAY_TEST_DROP")
            && let Ok(top) = i.parse::<usize>()
            && let Some(b) = physics::body_of(&app.doc.v, top)
        {
            app.world.bodies.push(b);
        }
        app
    }

    fn say(&mut self, m: impl Into<String>) {
        self.message = Some((m.into(), Instant::now()));
    }

    /// Recompiles when the document changed.
    fn compile(&mut self) {
        if self.doc.version == self.compiled && self.emitters.version == self.compiled_emit {
            return;
        }
        self.compiled = self.doc.version;
        self.compiled_emit = self.emitters.version;
        // the emitters' copies after the document's objects, known by their ids
        let scene = if self.emitters.particles.is_empty() {
            Scene::from_value(&self.doc.v, &mut self.cache)
        } else {
            let mut v = self.doc.v.clone();
            let n = self.doc.objects().len();
            if let Some(objs) = v.get_mut("objects").and_then(serde_json::Value::as_array_mut) {
                objs.extend(self.emitters.nodes());
            }
            let ids: Vec<usize> = self.emitters.particles.iter().map(|p| p.id).collect();
            Scene::from_value(&v, &mut self.cache).map(|mut s| {
                for o in &mut s.objects {
                    if o.doc_index >= n {
                        o.doc_index = ids.get(o.doc_index - n).copied().unwrap_or(usize::MAX);
                        // without physics they pass through everything
                        o.ghost |= self.emitters.find(o.doc_index).is_some_and(|p| !p.physics);
                    }
                }
                for c in &mut s.volumes {
                    if c.doc_index >= n {
                        c.doc_index = ids.get(c.doc_index - n).copied().unwrap_or(usize::MAX);
                    }
                }
                s
            })
        };
        match scene {
            Ok(s) => self.adopt(s),
            Err(e) => self.error = Some(e),
        }
    }

    /// A newly compiled scene becomes the one drawn.
    fn adopt(&mut self, s: Scene) {
        // objects that would not compile are left out, and said so
        self.error = (!s.problems.is_empty()).then(|| format!("{} (left out until fixed; Ctrl+Z undoes the edit)", s.problems.join("; ")));
        self.scene = Arc::new(s);
        self.scene_id += 1;
    }

    /// Reloads the file (discarding edits).
    fn reload(&mut self, reset_camera: bool) {
        self.mtime = mtime(&self.path);
        self.changed_on_disk = false;
        match crate::scene::read_doc(&self.path).map(|(v, _)| v) {
            Ok(mut v) => {
                crate::scene::json::migrate(&mut v);
                self.original = v.clone();
                self.original_on_disk = true;
                self.doc = Doc::new(v);
                self.compiled = 0;
                self.emitters.clear();
                self.world.bodies.clear();
                self.world.ripples.clear();
                self.placed.0.clear();
                self.gesture = None;
                if let Selection::Node { top, sub } = &self.sel
                    && self.doc.node(*top, sub).is_none()
                {
                    self.sel = Selection::None;
                }
                self.compile();
                if self.error.is_none() {
                    if reset_camera {
                        self.cam = crate::start_camera(&self.scene);
                        self.speed = self.scene.camera.speed;
                    }
                    self.say(format!("loaded {}", self.path.display()));
                }
            }
            Err(e) => self.error = Some(format!("{}: {e}", self.path.display())),
        }
    }

    /// The camera (exactly where it is: close up a millimetre matters) and
    /// every render setting into the document `v`, as a file should open.
    fn view_into(&self, v: &mut serde_json::Value) {
        let c = self.cam;
        let x = |v: f32| v as f64;
        v["camera"] = serde_json::json!({
            "position": c.exact(),
            "yaw": c.yaw.to_degrees() as f64,
            "pitch": c.pitch.to_degrees() as f64,
            "fov": x(c.fov),
            "speed": x(self.speed),
        });
        if let Ok(s) = serde_json::to_value(self.settings) {
            v["render"] = s;
        }
    }

    /// The view and the settings into the document (what saving writes).
    fn store_view(&mut self) {
        let mut v = self.doc.v.clone();
        self.view_into(&mut v);
        if v != self.doc.v {
            self.doc.v = v;
            self.doc.touch();
        }
    }

    /// Saves the scene (saying how it went; the error for the MCP too).
    fn save(&mut self) -> Result<(), String> {
        self.save_as(self.path.clone())
    }

    /// Saves to `path` and goes on editing that file.
    fn save_as(&mut self, path: PathBuf) -> Result<(), String> {
        if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("png")) {
            let e = "save failed: a scene saves as .json (a screenshot stays the picture it is)".to_string();
            self.say(e.clone());
            return Err(e);
        }
        self.store_view();
        match self.doc.save(&path) {
            Ok(()) => {
                self.path = path;
                self.mtime = mtime(&self.path);
                self.changed_on_disk = false;
                self.original_on_disk = false;
                self.say(format!("saved {}", self.path.display()));
                Ok(())
            }
            Err(e) => {
                let e = format!("save failed: {e}");
                self.say(e.clone());
                Err(e)
            }
        }
    }

    /// The quick save slot (F5 / F9): the preferences say where.
    pub(super) fn quick_save_path(&self) -> PathBuf {
        config::expand(&self.config.quick_save)
    }

    /// Keeps everything as it is now in the quick save slot: the scene with
    /// its unsaved edits, the camera and every render setting.
    fn quick_save(&mut self) -> Result<(), String> {
        let mut v = self.doc.v.clone();
        self.view_into(&mut v);
        let path = self.quick_save_path();
        let r = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .map_err(|e| e.to_string())
            .and_then(|_| serde_json::to_string_pretty(&v).map_err(|e| e.to_string()))
            .and_then(|t| std::fs::write(&path, t + "\n").map_err(|e| e.to_string()));
        match r {
            Ok(()) => {
                // (the file being edited: no reload of what was just written)
                if self.path == path {
                    self.mtime = mtime(&self.path);
                }
                self.say(format!("quick saved to {}: the scene, the view and every setting (F9 brings them back)", path.display()));
                Ok(())
            }
            Err(e) => {
                let e = format!("quick save failed: {e}");
                self.say(e.clone());
                Err(e)
            }
        }
    }

    /// Back to the quick save (asking twice over unsaved edits).
    fn quick_load(&mut self) {
        let _ = self.quick_load_as(false);
    }

    /// Back to the quick save; with unsaved edits it asks twice unless
    /// `force` (a remote that means it). The edits are only given up once the
    /// quick save has loaded.
    pub(super) fn quick_load_as(&mut self, force: bool) -> Result<(), String> {
        let path = self.quick_save_path();
        if !path.exists() {
            self.say("nothing quick saved yet (F5 keeps the scene, the view and the settings)");
            return Err(format!("nothing quick saved yet ({})", path.display()));
        }
        let armed = force || self.quickload_armed.is_some_and(|t| t.elapsed().as_secs_f32() < 4.0);
        if self.doc.dirty && !armed {
            self.quickload_armed = Some(Instant::now());
            self.say("unsaved edits: Quick load again to discard them");
            return Err("unsaved edits: quick load again to discard them".into());
        }
        self.quickload_armed = None;
        match self.open(path.clone()) {
            Ok(()) => {
                self.say(format!("quick save loaded ({}; saving writes there)", path.display()));
                Ok(())
            }
            Err(e) => {
                self.say(format!("quick load failed: {e}"));
                Err(format!("quick load failed: {e}"))
            }
        }
    }

    /// Opens another scene file: its document, start view and render settings
    /// (the device and resolution scale stay unless the file sets them), with
    /// the physics reset. A file that does not load leaves the current scene as it is.
    /// A screenshot opens as the moment it was taken (take_moment).
    pub(super) fn open(&mut self, path: PathBuf) -> Result<(), String> {
        let (mut v, moment) = crate::scene::read_doc(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        crate::scene::json::migrate(&mut v);
        let scene = Scene::from_value(&v, &mut self.cache).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut s = crate::initial_settings(&scene, None)?;
        let set = |k: &str| scene.render.as_ref().is_some_and(|r| r.get(k).is_some());
        if !set("device") {
            s.device = self.settings.device;
        }
        if !set("scale") {
            s.scale = self.settings.scale;
        }
        self.settings = s;
        self.path = moment.as_ref().map_or_else(|| path.clone(), |m| still_source(&path, m));
        self.mtime = mtime(&self.path);
        self.changed_on_disk = false;
        self.original = v.clone();
        self.original_on_disk = true;
        self.doc = Doc::new(v);
        self.compiled = 0;
        self.emitters.clear();
        self.world.bodies.clear();
        self.world.ripples.clear();
        self.placed.0.clear();
        self.gesture = None;
        self.charge = None;
        self.sel = Selection::None;
        // (compiled above already: the document is that file, no emitters)
        self.adopt(scene);
        self.compiled = self.doc.version;
        self.compiled_emit = self.emitters.version;
        self.cam = crate::start_camera(&self.scene);
        self.speed = self.scene.camera.speed;
        self.say(format!("opened {}", self.path.display()));
        if let Some(m) = &moment {
            self.take_moment(&path, m);
        }
        Ok(())
    }

    /// A screenshot `png` just opened (its scene, view and settings in):
    /// the file it was taken of is the one edited, with unsaved edits if its
    /// scene is not that file's as it is now; the walker back as it stood
    /// (game mode exactly as it was: entered anew, it would guess its height
    /// from the ground), or out of game mode.
    fn take_moment(&mut self, png: &std::path::Path, moment: &serde_json::Value) {
        let on_disk = crate::scene::read_doc(&self.path).ok().map(|(mut v, _)| {
            crate::scene::json::migrate(&mut v);
            v
        });
        let unsaved = !on_disk.is_some_and(|v| crate::scene::same_scene(&v, &self.doc.v));
        self.original_on_disk = !unsaved;
        self.doc.dirty = unsaved;
        self.game = None;
        let walker = moment.get("game").and_then(|g| Some((serde_json::from_value::<[f64; 3]>(g.get("feet")?.clone()).ok()?, g.get("height")?.as_f64()?)));
        if let Some((feet, height)) = walker {
            let cam = self.view_cam();
            let g = crate::render::view_query(&self.scene, &cam, &self.settings, |c, regs| game::Game::stand(&mut game::Probe::new(c, regs), feet, height));
            self.gun = None;
            self.gun_armed = false;
            self.game_eye = self.cam.exact();
            self.game = Some(g);
        }
        self.say(format!("opened {}: a screenshot of {} (saving writes there)", png.display(), self.path.display()));
    }

    /// The Open / Save as window.
    fn file_window(&mut self, ctx: &egui::Context) {
        // (the quick save slot is named as such in the list)
        let slot = std::fs::canonicalize(self.quick_save_path()).ok();
        let shots = config::expand(&self.config.screenshots);
        let Some(d) = &mut self.file_dialog else { return };
        let mut open = true;
        let (mut go, mut pick, mut cancel, mut into) = (false, None, false, None);
        let title = if d.save { "Save scene as" } else { "Open scene" };
        egui::Window::new(title).open(&mut open).collapsible(false).resizable(true).default_width(520.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let up = d.dir.parent().filter(|p| !p.as_os_str().is_empty()).map(PathBuf::from).or_else(|| std::fs::canonicalize(&d.dir).ok().and_then(|c| c.parent().map(PathBuf::from)));
                if ui.add_enabled(up.is_some(), egui::Button::new("↑")).on_hover_text("The folder this one is in.").clicked() {
                    into = up;
                }
                if ui.button("saves/").on_hover_text("The scenes that come with ray, and the quick save.").clicked() {
                    into = Some(PathBuf::from("saves"));
                }
                if !d.save && ui.button("screenshots/").on_hover_text("Your screenshots (the folder in Preferences): open one to go back to the moment it shows.").clicked() {
                    let _ = std::fs::create_dir_all(&shots);
                    into = Some(shots.clone());
                }
                if ui.button("📁").on_hover_text("Open this folder on the desktop.").clicked() {
                    open_externally(&std::fs::canonicalize(&d.dir).unwrap_or(d.dir.clone()));
                }
                style::note(ui, format!("Folder: {}", d.dir.display()));
            });
            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for p in &d.dirs {
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if ui.selectable_label(false, format!("📁 {name}/")).on_hover_text("Open this folder.").clicked() {
                        into = Some(p.clone());
                    }
                }
                for (p, n, desc) in &d.files {
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    let tag = if slot.is_some() && std::fs::canonicalize(p).ok() == slot { "   (quick save, F9)" } else { "" };
                    let r = ui.selectable_label(d.name == name, format!("{name}   ({n} objects){tag}"));
                    let r = if desc.is_empty() { r } else { r.on_hover_text(desc.as_str()) };
                    // (the first sentence: the whole of a long one left room
                    // for three scenes; all of it on hover)
                    if !desc.is_empty() {
                        style::note(ui, first_sentence(desc, 110));
                    }
                    ui.add_space(2.0);
                    if r.clicked() {
                        d.name = name;
                        d.confirm = false;
                    }
                    if r.double_clicked() {
                        pick = Some(p.clone());
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("File");
                let r = ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(320.0).hint_text("name.json, or a path"));
                if r.changed() {
                    d.confirm = false;
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    go = true;
                }
            });
            let exists = d.target().exists();
            if d.save && d.confirm {
                ui.colored_label(style::WARN, format!("{} exists: Save again to replace it.", d.target().display()));
            }
            if !d.save && self.doc.dirty {
                ui.colored_label(style::WARN, "Unsaved edits to this scene will be lost.");
            }
            ui.horizontal(|ui| {
                let label = if d.save { if d.confirm { "Replace" } else { "Save" } } else { "Open" };
                if ui.add_enabled(!d.name.trim().is_empty() && (d.save || exists), egui::Button::new(egui::RichText::new(label).strong())).clicked() {
                    go = true;
                }
                if ui.button("Cancel").on_hover_text("Close this window (Esc).").clicked() {
                    cancel = true;
                }
            });
        });
        if let Some(dir) = into {
            d.go(dir);
        }
        let target = d.target();
        let (save, confirm) = (d.save, d.confirm);
        if !open || cancel {
            self.file_dialog = None;
            return;
        }
        if let Some(p) = pick.filter(|_| !save) {
            go = true;
            self.file_dialog.as_mut().unwrap().name = p.to_string_lossy().into_owned();
        }
        if !go {
            return;
        }
        if save {
            // replacing another file takes a second click
            if target.exists() && target != self.path && !confirm {
                self.file_dialog.as_mut().unwrap().confirm = true;
                return;
            }
            // (a failed save keeps the window open, to try another name)
            if self.save_as(target).is_ok() {
                self.file_dialog = None;
            }
        } else {
            let target = self.file_dialog.as_ref().map(FileDialog::target).unwrap_or(target);
            match self.open(target) {
                Ok(()) => self.file_dialog = None,
                Err(e) => self.say(format!("open failed: {e}")),
            }
        }
    }

    /// P / F12: the 3D view as a PNG, or (Preferences) the whole window as
    /// drawn, which egui hands over a frame later (window_shot).
    fn screenshot(&mut self) {
        if self.config.screenshot_interface {
            self.window_shot = Some((Instant::now(), None));
            return;
        }
        // (while recording: the image as it is; a steady recording holds the
        // view for its own frames)
        if self.recorder.is_some() {
            let Some(img) = &self.shown else { return };
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            let size = img.size;
            return self.save_shot(size, &rgba);
        }
        if let Some(st) = &self.steady {
            let wants = st.wants(self.settings.max_samples);
            return self.say(format!("a screenshot is still rendering ({} of {wants} samples)", self.stats.samples.min(wants)));
        }
        // rendered whole, as a steady recording's frame is: at the
        // preferences' size (the view's at full resolution unless set), the
        // scene's clock held, until every pixel has its samples
        let [w, h] = self.config.screenshot_size;
        let (w, h) = if w > 0 && h > 0 { render_size(w as f32, h as f32) } else { render_size(self.view_px.0, self.view_px.1) };
        let (out, text) = self.shot_moment([w, h]);
        let mut st = record::Steady::new(self.config.video_fps, self.config.screenshot_samples, [w, h], Some(1), self.now());
        // (no more than 90% busy, as render_video: the window stays smooth)
        st.load = 0.9;
        self.say(format!("screenshot: rendering {w} x {h}, {} samples a pixel", st.wants(self.settings.max_samples)));
        self.steady = Some(st);
        self.still = Some(Still { out, text, posted: None });
    }

    /// The screenshot rendering whole, once its view has every sample: its
    /// pixels written with the moment it was taken at. Meanwhile how far it
    /// has got (and the scene holds still: true).
    fn still_frame(&mut self) -> bool {
        let (Some(still), Some(st)) = (&self.still, &self.steady) else { return false };
        let wants = st.wants(self.settings.max_samples);
        let drawn = still.posted == Some(self.shown_serial) && (self.stats.samples >= wants || !self.stats.traced);
        if !drawn {
            self.say(format!("screenshot: {} of {wants} samples", if still.posted == Some(self.shown_serial) { self.stats.samples.min(wants) } else { 0 }));
            return true;
        }
        let Some(still) = self.still.take() else { return false };
        self.end_steady();
        let done = match &self.shown {
            Some(img) => {
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                let size = img.size;
                self.write_shot(&still.out, size, &rgba, &still.text)
            }
            None => Err("no image".into()),
        };
        self.last_still = Some(done);
        false
    }

    /// A screenshot's pixels into the screenshots folder, named after the
    /// scene, with the moment in the picture: the scene as a quick save keeps
    /// it (unsaved edits, the camera exactly, every setting), the file it is
    /// of and the walker. Opened (File > Open, `ray` with the picture), it
    /// all comes back.
    fn save_shot(&mut self, size: [usize; 2], rgba: &[u8]) {
        let (out, text) = self.shot_moment(size);
        let _ = self.write_shot(&out, size, rgba, &text);
    }

    /// Where a screenshot of `size` taken now goes, and the moment it keeps:
    /// its text chunks (keyword, text, compressed).
    fn shot_moment(&self, size: [usize; 2]) -> (PathBuf, Vec<(&'static str, String, bool)>) {
        let stem = self.path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "ray".into());
        let dir = config::expand(&self.config.screenshots);
        let _ = std::fs::create_dir_all(&dir);
        let stamp = record::stamp();
        let out = shot_path(&dir, &stem, &stamp);
        let mut scene = self.doc.v.clone();
        self.view_into(&mut scene);
        let version = env!("CARGO_PKG_VERSION");
        let moment = serde_json::json!({
            "source": std::path::absolute(&self.path).unwrap_or_else(|_| self.path.clone()),
            "taken": stamp,
            "size": size,
            "objects": self.doc.objects().len(),
            "game": self.game.as_ref().map(|g| serde_json::json!({"feet": g.feet, "height": g.height})),
            "version": version,
        });
        // (for people and picture viewers: where it was taken from)
        let (c, p) = (self.cam, self.cam.exact());
        let mut about = format!(
            "ray: {} from ({:.6}, {:.6}, {:.6}) m, yaw {:.2}°, pitch {:.2}°, field of view {:.1}°",
            self.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            p[0],
            p[1],
            p[2],
            c.yaw.to_degrees(),
            c.pitch.to_degrees(),
            c.fov
        );
        if let Some(g) = &self.game {
            about += &format!("; the walker {} tall", length_text(g.height));
        }
        let scene = serde_json::to_string(&scene).unwrap_or_default();
        let text = vec![(crate::scene::STILL_SCENE, scene, true), (crate::scene::STILL_MOMENT, moment.to_string(), true), ("Description", about, false), ("Software", format!("ray {version}"), false)];
        (out, text)
    }

    /// A screenshot's pixels into `out` with its text chunks; said on screen.
    fn write_shot(&mut self, out: &std::path::Path, size: [usize; 2], rgba: &[u8], text: &[(&'static str, String, bool)]) -> Result<PathBuf, String> {
        let text: Vec<(&str, &str, bool)> = text.iter().map(|(k, t, z)| (*k, t.as_str(), *z)).collect();
        match crate::render::write_png_text(out, size[0], size[1], rgba, &text) {
            Ok(()) => {
                self.say(format!("saved {}", out.display()));
                Ok(out.to_path_buf())
            }
            Err(e) => {
                self.say(format!("screenshot failed: {e}"));
                Err(e)
            }
        }
    }

    /// A screenshot of the whole window: asked for (the frame being drawn is
    /// captured), then saved when egui hands it over, a frame later.
    fn window_shot(&mut self, ctx: &egui::Context) {
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, user_data, .. } if user_data.data.as_ref().is_some_and(|d| d.downcast_ref::<WindowShot>().is_some()) => Some(image.clone()),
                _ => None,
            })
        });
        // (asked again, it may come twice: the first is the one)
        if let Some(image) = shot
            && self.window_shot.take().is_some()
        {
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            self.save_shot(image.size, &rgba);
        }
        // asked again until it comes, every third frame and no more than
        // three times a second: a compositor drops a frame now and then, and
        // the request with it (an idle window draws a frame a second: asked
        // again only every 20 frames, with 5 s to come, it was given up on)
        if let Some((since, asked)) = self.window_shot {
            if since.elapsed().as_secs_f32() > 20.0 {
                self.window_shot = None;
                self.say("screenshot failed: the window was not captured");
            } else {
                if asked.is_none_or(|(f, t)| self.frames >= f + 3 && t.elapsed().as_secs_f32() >= 0.3) {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(WindowShot)));
                    self.window_shot = Some((since, Some((self.frames, Instant::now()))));
                }
                ctx.request_repaint();
            }
        }
    }

    /// An emitter is sending copies out.
    fn emitting(&self) -> bool {
        self.doc.objects().iter().any(|o| {
            o.get("type").and_then(serde_json::Value::as_str) == Some("emitter")
                && o.get("disabled").and_then(serde_json::Value::as_bool) != Some(true)
                && o.get("rate").and_then(serde_json::Value::as_f64).unwrap_or(2.0) > 0.0
        })
    }

    /// The compiled object of `objects[top]`.
    fn object_of(&self, top: usize) -> Option<u32> {
        self.scene.objects.iter().position(|o| o.doc_index == top).map(|i| i as u32)
    }

    /// The compiled objects of the selection, first and one past the last: a
    /// group's members (they are compiled one after another), or the member
    /// a part of which is selected.
    fn selected_objects(&self) -> Option<(u32, u32)> {
        let Selection::Node { top, sub } = &self.sel else { return None };
        // one path leads on from the other
        let along = |a: &[usize]| a.iter().zip(sub).all(|(x, y)| x == y);
        let mut it = self.scene.objects.iter().enumerate().filter(|(_, o)| o.doc_index == *top && along(&o.path)).map(|(i, _)| i as u32);
        let first = it.next()?;
        Some((first, it.next_back().unwrap_or(first) + 1))
    }

    /// The camera set up for the view (in points) and the ray through `pos`.
    fn view_ray(&self, pos: egui::Pos2) -> (Camera, Vec3) {
        let r = self.view_rect;
        let mut c = self.cam;
        c.setup(r.width().max(1.0) as u32, r.height().max(1.0) as u32);
        (c, c.ray(pos.x - r.left(), pos.y - r.top()))
    }

    /// Whether the view is moving, for "fast while moving" (fewer pixels).
    fn moving(&self) -> bool {
        // (a steady recording draws every frame whole)
        self.settings.fast_motion && self.steady.is_none() && self.last_change.elapsed().as_secs_f32() < 0.15
    }

    /// The scene's clock, seconds (waves, ripples, stars, held keys, the
    /// gravity gun's charge): the wall clock's, except that a steady
    /// recording moves it a frame of the video at a time (and it goes on
    /// from there after).
    fn now(&self) -> f64 {
        self.steady.as_ref().map_or(self.clock.elapsed().as_secs_f64() - self.clock_behind, |s| s.t)
    }

    /// The camera, easing toward a glide's pose: its position too, except
    /// in game mode (the walker moves it; it turns).
    fn glide_step(&mut self) {
        let now = self.now();
        // (a glide whose time is over gives way to the next, which starts
        // where and when it ended)
        while self.glides.len() > 1 && now >= self.glides[0].t0 + self.glides[0].secs as f64 {
            self.glides.pop_front();
        }
        let Some(g) = self.glides.front() else { return };
        let k = ((now - g.t0) as f32 / g.secs.max(1e-3)).clamp(0.0, 1.0);
        let e = if g.linear { k } else { k * k * (3.0 - 2.0 * k) };
        if self.game.is_none()
            && let Some(to) = g.to_pos
        {
            let p = std::array::from_fn(|i| g.from_pos[i] + (to[i] - g.from_pos[i]) * e as f64);
            self.cam.set_exact(p);
        }
        self.cam.yaw = g.from[0] + (g.to[0] - g.from[0]) * e;
        self.cam.pitch = (g.from[1] + (g.to[1] - g.from[1]) * e).clamp(-1.55, 1.55);
        self.cam.fov = g.from[2] + (g.to[2] - g.from[2]) * e;
        if k >= 1.0 {
            self.glides.pop_front();
        }
    }

    /// Mouse look: radians per point (narrower views turn slower), and -1 when
    /// the preferences turn up and down the other way.
    fn look_gain(&self) -> (f32, f32) {
        (0.0025 * self.config.look_speed.clamp(0.05, 10.0) * (self.cam.fov / 60.0), if self.config.invert_look { -1.0 } else { 1.0 })
    }

    /// The size the view renders at: a steady recording's frames' (whatever
    /// the window's), else the view's (fewer pixels while moving, with
    /// "fast while moving").
    fn job_size(&self) -> (usize, usize) {
        if let Some(st) = &self.steady {
            return (st.size[0], st.size[1]);
        }
        let (w, h) = self.render_size;
        if !self.moving() {
            return (w, h);
        }
        let k = self.settings.motion_scale.clamp(0.1, 1.0);
        (((w as f32 * k) as usize).max(8), ((h as f32 * k) as usize).max(8))
    }

    /// The camera set up as the render sees it (its pixels: the fractals'
    /// detail follows them; fewer while moving, as post_job renders them),
    /// for what must collide with what is drawn.
    fn view_cam(&self) -> Camera {
        let mut c = self.cam;
        let (w, h) = self.job_size();
        if w > 0 {
            c.setup(w as u32, h as u32);
        } else {
            c.setup(self.view_rect.width().max(1.0) as u32, self.view_rect.height().max(1.0) as u32);
        }
        c
    }

    /// Into game mode: a walker whose eye is where the camera is.
    fn enter_game(&mut self) {
        let cam = self.view_cam();
        let eye = cam.exact();
        let g = crate::render::view_query(&self.scene, &cam, &self.settings, |c, regs| game::Game::enter(&mut game::Probe::new(c, regs), eye));
        if self.game.is_none() {
            self.gun = None;
            self.gun_armed = false;
        }
        self.cam.set_exact(g.eye());
        // (as the camera holds it: compared with it next frame)
        self.game_eye = self.cam.exact();
        self.game = Some(g);
        self.gesture = None;
    }

    /// A frame of game mode: the pointer held, the mouse looks, the keys
    /// walk; the camera is the walker's eye. Then the crosshair and how tall
    /// it is.
    fn game_frame(&mut self, ctx: &egui::Context, ui: &egui::Ui, dt: f32) {
        let focused = ctx.input(|i| i.focused);
        if focused && !self.game_grab {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
            self.game_grab = true;
        } else if !focused {
            self.game_grab = false;
        }
        if self.cam.exact() != self.game_eye {
            self.enter_game();
        }
        // (the mouse only while the window has the pointer; keys come only
        // to a focused window anyway)
        if focused {
            let raw = ctx.input(|i| i.events.iter().fold(egui::Vec2::ZERO, |a, e| if let egui::Event::MouseMoved(d) = e { a + *d } else { a }));
            let (k, flip) = self.look_gain();
            self.cam.look(-raw.x * k, -raw.y * k * flip);
        }
        // the gravity gun: the left button charges it, letting go fires
        let lmb = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        if !lmb {
            self.gun_armed = true;
            if let Some(t0) = self.gun.take() {
                self.gun_fire(((self.now() - t0) as f32 / physics::GUN_CHARGE).min(1.0));
            }
        } else if self.gun_armed && self.gun.is_none() {
            self.gun = Some(self.now());
        }
        self.gun_target = self.gun_aim().is_some_and(|(_, movable)| movable);
        // Shift held runs; tapped while moving, it runs on until the keys
        // stop
        let (shift, moving) = ctx.input(|i| (i.modifiers.shift, [egui::Key::W, egui::Key::A, egui::Key::S, egui::Key::D].iter().any(|k| i.key_down(*k))));
        if !moving {
            self.run_on = false;
        } else if shift && !self.shift_was {
            self.run_on = true;
        }
        self.shift_was = shift;
        let run = shift || self.run_on;
        let input = ctx.input(|i| {
            let k = |key: egui::Key| if i.key_down(key) && !i.modifiers.command { 1.0 } else { 0.0 };
            game::Input {
                forward: k(egui::Key::W) - k(egui::Key::S),
                right: k(egui::Key::D) - k(egui::Key::A),
                run,
                jet: i.pointer.button_down(egui::PointerButton::Secondary),
                // (as it goes down, not its repeats: held in the water it
                // kicked thirty times a second)
                jump: i.events.iter().any(|e| matches!(e, egui::Event::Key { key: egui::Key::Space, pressed: true, repeat: false, .. })),
                grow: k(egui::Key::R) - k(egui::Key::F),
                fast: i.modifiers.shift,
            }
        });
        let cam = self.view_cam();
        let look = (self.cam.yaw, self.cam.pitch);
        let gravity = self.scene.gravity as f64;
        // the water's surface under the walker: its waves and ripples as the
        // frame draws them, so it rides them. A walker smaller than a person
        // rides them as much smaller (the water is drawn as a flat surface
        // with the waves in its shading: a small walker riding full-size
        // waves had its eye dip under it, the view flickering underwater)
        let now = self.now() as f32;
        let wave_time = if self.waves_moving() { now * self.scene.wave_speed } else { 0.0 };
        let (wave_height, ripples) = (self.scene.wave_height, &self.world.ripples);
        let ride = self.game.as_ref().map_or(1.0, |g| (g.height / game::HUMAN).min(1.0)) as f32;
        let surface = self.scene.water.map(|(level, _)| {
            move |x: f64, z: f64| {
                let at = |v: f64| crate::simd::F8::splat(v as f32);
                let (y, _, _) = crate::render::shade::water_surface(now, wave_time, ripples, wave_height, at(x), at(z));
                (level + y.to_array()[0] * ride) as f64
            }
        });
        let water = surface.as_ref().map(|f| f as &dyn Fn(f64, f64) -> f64);
        let Some(g) = self.game.as_mut() else { return };
        crate::render::view_query(&self.scene, &cam, &self.settings, |c, regs| g.step(&mut game::Probe::new(c, regs), look, &input, dt as f64, gravity, water));
        self.cam.set_exact(g.eye());
        self.game_eye = self.cam.exact();
        self.game_hud(ui);
    }

    /// What the gravity gun points at, in reach: the hit, and whether it can
    /// move (a body already, or an object that is not locked).
    fn gun_aim(&self) -> Option<(crate::render::Pick, bool)> {
        let reach = self.game.as_ref().map_or(physics::GUN_REACH, |g| physics::GUN_REACH.max(15.0 * g.height as f32));
        let p = self.pick_at(self.view_rect.center()).filter(|p| p.t <= reach)?;
        let o = self.scene.objects.get(p.object as usize)?;
        let movable = self.world.find(o.doc_index).is_some() || (o.doc_index < emitter::BASE && !o.locked);
        Some((p, movable))
    }

    /// The gravity gun fires, `charge` 0..1: what is under the crosshair, in
    /// reach, is sent off along the look (harder the longer it was held).
    fn gun_fire(&mut self, charge: f32) {
        self.gun_shot = Some(self.now());
        let Some((p, movable)) = self.gun_aim() else { return };
        if !movable {
            self.say("fixed in place: the gravity gun moves only what can move (not locked)");
            return;
        }
        let top = self.scene.objects[p.object as usize].doc_index;
        let i = match self.world.find(top) {
            Some(i) => i,
            None => match Self::body_for(&self.doc, &self.scene, p.object) {
                Some(b) => {
                    self.world.bodies.push(b);
                    self.world.bodies.len() - 1
                }
                None => return,
            },
        };
        // (one undo step back to where it was)
        self.doc.checkpoint(true);
        self.placed.0.remove(&top);
        let (_, d) = self.view_ray(self.view_rect.center());
        let b = &mut self.world.bodies[i];
        let speed = physics::punt_speed(b.mass, charge);
        b.punt(p.point, d, speed);
        let m = b.mass;
        self.say(format!("gravity gun: {m:.1} kg sent off at {speed:.0} m/s"));
    }

    /// The gravity gun on the crosshair: while it charges, a ring filling
    /// round it (amber to white-hot, its strength beside it); when it fires,
    /// a beam from below the view to the crosshair, fading.
    fn gun_sight(&self, p: &egui::Painter, r: egui::Rect, c: egui::Pos2) {
        if let Some(t0) = self.gun {
            let k = ((self.now() - t0) as f32 / physics::GUN_CHARGE).min(1.0);
            let ring = |a0: f32, a1: f32, w: f32, col: egui::Color32| {
                let n = ((a1 - a0) * 40.0).ceil().max(2.0) as usize;
                let pts: Vec<egui::Pos2> = (0..=n).map(|i| {
                    let a = a0 + (a1 - a0) * i as f32 / n as f32 - std::f32::consts::FRAC_PI_2;
                    c + egui::vec2(a.cos(), a.sin()) * 20.0
                }).collect();
                p.add(egui::Shape::line(pts, egui::Stroke::new(w, col)));
            };
            ring(0.0, std::f32::consts::TAU, 4.0, egui::Color32::from_black_alpha(110));
            let hot = egui::Color32::from_rgb(255, (150.0 + 105.0 * k) as u8, (40.0 + 200.0 * k * k) as u8);
            ring(0.0, std::f32::consts::TAU * k.max(0.02), 3.0, hot);
            let strength = 0.1 + 0.9 * k;
            p.text(c + egui::vec2(28.0, 0.0), egui::Align2::LEFT_CENTER, format!("{:.0}%", strength * 100.0), egui::FontId::proportional(12.0), hot);
        }
        if let Some(t) = self.gun_shot {
            let age = (self.now() - t) as f32;
            if age < 0.2 {
                let a = (255.0 * (1.0 - age / 0.2)) as u8;
                let from = egui::pos2(c.x + r.width() * 0.12, r.bottom());
                p.line_segment([from, c], egui::Stroke::new(6.0, egui::Color32::from_rgba_unmultiplied(255, 150, 40, a / 3)));
                p.line_segment([from, c], egui::Stroke::new(2.0, egui::Color32::from_rgba_unmultiplied(255, 230, 180, a)));
                p.circle_filled(c, 6.0 + 30.0 * age, egui::Color32::from_rgba_unmultiplied(255, 180, 60, a / 2));
            }
        }
    }

    /// Game mode's rocket gauge (bottom right): it fills as the rocket builds
    /// up, from amber to red, with a tick where its push holds the walker up
    /// at this lean under this gravity: above it, it climbs.
    fn rocket_meter(&self, p: &egui::Painter, r: egui::Rect, g: &game::Game) {
        let (w, h) = (16.0, 160.0);
        let bar = egui::Rect::from_min_size(egui::pos2(r.right() - 30.0 - w, r.bottom() - 40.0 - h), egui::vec2(w, h));
        let back = bar.expand2(egui::vec2(10.0, 24.0));
        p.rect_filled(back, egui::CornerRadius::same(6), egui::Color32::from_black_alpha(if g.power > 0.0 { 150 } else { 90 }));
        p.rect_stroke(bar, egui::CornerRadius::same(3), egui::Stroke::new(1.0, egui::Color32::from_white_alpha(120)), egui::StrokeKind::Outside);
        let k = g.power.clamp(0.0, 1.0) as f32;
        if k > 0.0 {
            let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k) as u8;
            let col = egui::Color32::from_rgb(255, mix(196, 70), mix(64, 24));
            let fill = egui::Rect::from_min_max(egui::pos2(bar.left(), bar.bottom() - bar.height() * k), bar.max);
            p.rect_filled(fill, egui::CornerRadius::same(3), col);
        }
        if g.lift <= 1.0 {
            let y = bar.bottom() - bar.height() * g.lift as f32;
            p.line_segment([egui::pos2(bar.left() - 5.0, y), egui::pos2(bar.right() + 5.0, y)], egui::Stroke::new(2.0, egui::Color32::WHITE));
            p.text(egui::pos2(bar.left() - 8.0, y), egui::Align2::RIGHT_CENTER, "lift", egui::FontId::proportional(11.0), egui::Color32::from_white_alpha(200));
        }
        let font = egui::FontId::proportional(12.0);
        p.text(egui::pos2(bar.center().x, bar.bottom() + 6.0), egui::Align2::CENTER_TOP, format!("{:.0}%", k * 100.0), font.clone(), egui::Color32::from_white_alpha(230));
        p.text(egui::pos2(bar.center().x, bar.top() - 6.0), egui::Align2::CENTER_BOTTOM, "rocket", font, egui::Color32::from_white_alpha(200));
    }

    /// Game mode's crosshair, the walker's height and, for a while, the keys.
    fn game_hud(&self, ui: &egui::Ui) {
        let Some(g) = &self.game else { return };
        let p = ui.painter();
        let r = self.view_rect;
        let c = r.center();
        // (orange over something the gravity gun can move)
        let lit = if self.gun_target { egui::Color32::from_rgb(255, 160, 40) } else { egui::Color32::from_white_alpha(220) };
        for (w, col) in [(3.0, egui::Color32::from_black_alpha(140)), (1.5, lit)] {
            p.line_segment([c - egui::vec2(9.0, 0.0), c + egui::vec2(9.0, 0.0)], egui::Stroke::new(w, col));
            p.line_segment([c - egui::vec2(0.0, 9.0), c + egui::vec2(0.0, 9.0)], egui::Stroke::new(w, col));
        }
        self.gun_sight(p, r, c);
        let mut text = format!("{} tall", length_text(g.height));
        if g.jetting {
            let deg = g.lean.to_degrees().round();
            let lean = match deg {
                d if d > 0.0 => format!(", leaning {d}° forward"),
                d if d < 0.0 => format!(", leaning {}° back", -d),
                _ => String::new(),
            };
            text.push_str(&format!("   ▲ rocket{lean}"));
        } else if g.diving {
            text.push_str("   ≈ diving (look and W A S D to swim; Space swims up; come up for air)");
        } else if g.wet > 0.3 && !g.grounded {
            text.push_str("   ≈ swimming (Space kicks up, looking down dives; swim into rock to climb out)");
        }
        if self.run_on {
            text.push_str("   » running");
        }
        // (two lines, the way back first: one ran under the rocket gauge and
        // lost its end in a narrower window)
        if g.since.elapsed().as_secs_f32() < 10.0 {
            text = format!(
                "Tab or Esc: back to editing · F1 these keys · F10 record\nW A S D walk or swim · Shift run (tap: keeps running) · Space jump (with W A S D: leap; in water: kick up, looking down: dive) · R / F grow / shrink · O get out (stuck)\nleft button: gravity gun (hold to charge, let go to fire) · right button: rocket (look down to lean)\n{text}"
            );
        }
        self.rocket_meter(p, r, g);
        let at = egui::pos2(r.left() + 14.0, r.bottom() - 14.0);
        let font = egui::FontId::proportional(15.0);
        p.text(at + egui::vec2(1.0, 1.0), egui::Align2::LEFT_BOTTOM, &text, font.clone(), egui::Color32::from_black_alpha(180));
        p.text(at, egui::Align2::LEFT_BOTTOM, &text, font.clone(), egui::Color32::from_white_alpha(230));
        // what the menu bar would say (a recording just finished, a quick
        // save, F9's "again to discard"), at the top; not while recording:
        // it would be in the video
        if self.recorder.is_none() {
            let mut notes = Vec::new();
            if let Some((path, saved)) = &self.last_video
                && saved.elapsed().as_secs_f32() < VIDEO_NOTE_SECS
            {
                notes.push((format!("🎬 saved {}", path.display()), egui::Color32::from_rgb(255, 210, 120)));
            }
            if let Some((m, at)) = &self.message
                && at.elapsed().as_secs_f32() < 4.0
            {
                notes.push((m.clone(), egui::Color32::from_white_alpha(230)));
            }
            let mut at = egui::pos2(r.left() + 14.0, r.top() + 14.0);
            for (note, col) in notes {
                p.text(at + egui::vec2(1.0, 1.0), egui::Align2::LEFT_TOP, &note, font.clone(), egui::Color32::from_black_alpha(180));
                p.text(at, egui::Align2::LEFT_TOP, &note, font.clone(), col);
                at.y += 22.0;
            }
        }
    }

    fn pick_at(&self, pos: egui::Pos2) -> Option<crate::render::Pick> {
        let (c, d) = self.view_ray(pos);
        pick_ray(&self.scene, &c, &self.settings, c.pos, d)
    }

    /// Selects what is under `pos`: the hit node of the hit object, or a
    /// cloud in front of it.
    fn select_at(&mut self, pos: egui::Pos2) {
        let (c, d) = self.view_ray(pos);
        let cloud = crate::render::volume::pick(&self.scene, c.pos, d);
        match self.pick_at(pos) {
            Some(p) if cloud.is_none_or(|(t, _)| p.t < t) => {
                let o = &self.scene.objects[p.object as usize];
                self.sel = match self.emitters.find(o.doc_index) {
                    // an emitted copy: what its emitter sends out
                    Some(c) if self.doc.node(c.emitter, &[0]).is_some() => Selection::Node { top: c.emitter, sub: vec![0] },
                    Some(c) if c.emitter < self.doc.objects().len() => Selection::Node { top: c.emitter, sub: vec![] },
                    Some(_) => Selection::None,
                    // (a group's member: its path in the group, then the part hit)
                    None => Selection::Node { top: o.doc_index, sub: o.path.iter().copied().chain(o.node_path(p.node as usize)).collect() },
                };
            }
            _ => self.sel = cloud.map_or(Selection::None, |(_, top)| Selection::Node { top, sub: vec![] }),
        }
    }

    /// A body for compiled object `o` (None when it cannot move).
    fn body_for(doc: &Doc, scene: &Scene, o: u32) -> Option<Body> {
        let ob = scene.objects.get(o as usize)?;
        if ob.locked {
            return None;
        }
        let mut b = physics::body_of(&doc.v, ob.doc_index)?;
        b.obj = Some(o);
        Some(b)
    }

    /// Starts carrying the object under `pos`, if it can move.
    fn carry(&mut self, pos: egui::Pos2) -> bool {
        let Some(p) = self.pick_at(pos) else { return false };
        let top = self.scene.objects[p.object as usize].doc_index;
        if top >= emitter::BASE {
            return false;
        }
        self.placed.0.remove(&top);
        // a body already in flight is caught again
        let i = match self.world.find(top) {
            Some(i) => i,
            None => match Self::body_for(&self.doc, &self.scene, p.object) {
                Some(b) => {
                    self.world.bodies.push(b);
                    self.world.bodies.len() - 1
                }
                None => return false,
            },
        };
        let body = &mut self.world.bodies[i];
        body.held = true;
        let offset = body.x - p.point;
        self.doc.checkpoint(true);
        self.sel = Selection::Node { top, sub: vec![] };
        self.gesture = Some(Gesture::Carry { top, dist: p.t, offset });
        self.charge = None;
        true
    }

    /// Writes the bodies' placements into the document.
    fn place_all(&mut self) {
        if self.world.bodies.is_empty() {
            return;
        }
        let mut doc_moved = false;
        for b in &self.world.bodies {
            let (pos, rot) = b.placement();
            if b.top >= emitter::BASE {
                // an emitted copy: not in the document
                self.emitters.place(b.top, pos, rot);
                continue;
            }
            if let Some(n) = self.doc.node_mut(b.top, &[]).and_then(|n| n.as_object_mut()) {
                n.insert("position".into(), doc::vec3(pos.map(|v| v as f64)));
                n.insert("rotation".into(), doc::vec3(rot.map(|v| v as f64)));
                doc_moved = true;
            }
        }
        if doc_moved {
            self.doc.touch();
        }
    }

    /// Whether stars show and twinkle.
    fn twinkling(&self) -> bool {
        let env = &self.scene.env;
        self.settings.twinkle && (env.sky == crate::scene::Sky::Stars || env.night > 0.0)
    }

    /// Whether the water's waves are moving.
    fn waves_moving(&self) -> bool {
        self.settings.animate_water && self.scene.water.is_some() && self.scene.wave_height > 0.0 && self.scene.wave_speed > 0.0
    }

    /// Sets `top` moving (again), from where it is now and with its current
    /// settings, keeping its speed if it was moving.
    fn refresh_body(&mut self, top: usize) {
        self.placed.0.remove(&top);
        let old = self.world.find(top).map(|i| self.world.bodies.swap_remove(i));
        if self.doc.node(top, &[]).and_then(|n| n.get("locked")).and_then(serde_json::Value::as_bool) == Some(true) {
            return;
        }
        if let Some(mut b) = physics::body_of(&self.doc.v, top) {
            if let Some(o) = old {
                (b.v, b.w) = (o.v, o.w);
            }
            b.obj = self.object_of(top);
            self.world.bodies.push(b);
        }
    }

    /// Sets every object that can move falling (Drop all).
    fn drop_all(&mut self) {
        self.placed.0.clear();
        for o in 0..self.scene.objects.len() {
            let ob = &self.scene.objects[o];
            if ob.locked || self.world.find(ob.doc_index).is_some() {
                continue;
            }
            if let Some(b) = Self::body_for(&self.doc, &self.scene, o as u32) {
                self.world.bodies.push(b);
            }
        }
        self.say(format!("{} objects falling", self.world.bodies.len()));
    }

    fn physics_step(&mut self, dt: f32) {
        let now = self.now() as f32;
        self.world.time = now;
        self.world.ripples.retain(|r| r.alive(now));
        let moving = self.waves_moving();
        self.world.waves_moving = moving;
        self.world.wave_time = if moving { now * self.scene.wave_speed } else { 0.0 };
        if let Some(top) = self.wake.take() {
            self.refresh_body(top);
        }
        let n = self.doc.objects().len();
        if self.placed.1 != n {
            self.placed = (Default::default(), n);
        }
        // what a handle is dragging stays out of the physics
        let hold = self.gesture.and_then(|g| g.handle_target());
        if let Some(i) = hold.and_then(|t| self.world.find(t)) {
            self.world.bodies.swap_remove(i);
        }
        // moving waves carry everything floating in them along, except what
        // was placed by hand
        if moving && let Some((level, _)) = self.scene.water {
            let reach = 0.25 * self.scene.wave_height + 0.2;
            for o in 0..self.scene.objects.len() {
                let ob = &self.scene.objects[o];
                if ob.locked
                    || ob.aabb.min.y > level + reach
                    || ob.aabb.max.y < level - reach
                    || self.world.find(ob.doc_index).is_some()
                    || self.placed.0.contains(&ob.doc_index)
                    || hold == Some(ob.doc_index)
                {
                    continue;
                }
                let b = if ob.doc_index >= emitter::BASE {
                    self.emitters.wake(&self.doc.v, ob.doc_index).map(|mut b| {
                        b.obj = Some(o as u32);
                        b
                    })
                } else {
                    Self::body_for(&self.doc, &self.scene, o as u32)
                };
                if let Some(b) = b {
                    self.world.bodies.push(b);
                }
            }
        }
        // copies whose body is gone (their object did not compile) rest
        let world = &self.world;
        self.emitters.orphans(|id| world.find(id).is_some());
        if self.world.bodies.is_empty() {
            return;
        }
        // the compiled indices change when the scene recompiles
        for i in 0..self.world.bodies.len() {
            let top = self.world.bodies[i].top;
            self.world.bodies[i].obj = self.object_of(top);
        }
        self.world.bodies.retain(|b| b.obj.is_some());
        let App { world, doc, scene, settings, placed, emitters, .. } = self;
        let scene = scene.clone();
        // a hit sets an object moving (a placed one too, which is then dropped)
        world.step(&scene, settings, dt, &mut |o| {
            let top = scene.objects.get(o as usize)?.doc_index;
            if hold == Some(top) {
                return None;
            }
            if top >= emitter::BASE {
                let mut b = emitters.wake(&doc.v, top)?;
                b.obj = Some(o);
                return Some(b);
            }
            placed.0.remove(&top);
            Self::body_for(doc, &scene, o)
        });
        self.place_all();
        for b in self.world.bodies.iter().filter(|b| b.asleep() && b.top >= emitter::BASE) {
            self.emitters.rest(b.top);
        }
        self.world.bodies.retain(|b| !b.asleep());
    }

    /// The carried body, if any.
    fn carried(&mut self) -> Option<&mut Body> {
        let Some(Gesture::Carry { top, .. }) = self.gesture else { return None };
        let i = self.world.find(top)?;
        Some(&mut self.world.bodies[i])
    }

    fn view_input(&mut self, ctx: &egui::Context, resp: &egui::Response, dt: f32) {
        use egui::PointerButton as B;
        let (raw, pointer, scroll, latest, through) = ctx.input(|i| {
            let mut raw = egui::Vec2::ZERO;
            for e in &i.events {
                if let egui::Event::MouseMoved(d) = e {
                    raw += *d;
                }
            }
            // (egui turns the wheel sideways while Shift is down: Shift + wheel
            // is the plain step that goes through walls)
            let w = i.raw_scroll_delta;
            let through = i.modifiers.shift;
            (raw, i.pointer.delta(), if through { w.x + w.y } else { w.y }, i.pointer.latest_pos(), through)
        });
        let motion = if raw != egui::Vec2::ZERO { raw } else { pointer };

        let handles = self.handles();
        let tool = self.tool;
        let on_handle = |p: egui::Pos2| handles.as_ref().and_then(|h| Self::handle_at(h, tool, p));
        if resp.clicked_by(B::Primary)
            && let Some(p) = resp.interact_pointer_pos()
            && on_handle(p).is_none()
        {
            self.select_at(p);
        }
        if self.gesture.is_none() {
            let origin = ctx.input(|i| i.pointer.press_origin());
            // a handle: move, turn or resize the object
            if resp.drag_started_by(B::Primary)
                && let Some(o) = origin
                && let Some(axis) = on_handle(o)
                && let Some(h) = &handles
            {
                self.grab_handle(h, axis, o);
            }
            if self.gesture.is_none() && resp.drag_started_by(B::Primary) {
                let carried = origin.is_some_and(|o| self.carry(o));
                if !carried {
                    self.gesture = Some(Gesture::Look);
                }
            } else if self.gesture.is_none() && resp.drag_started_by(B::Secondary) {
                let depth = origin.and_then(|o| self.pick_at(o)).map_or(10.0, |p| p.t);
                let per_point = depth * 2.0 * (self.cam.fov.to_radians() * 0.5).tan() / self.view_rect.height().max(1.0);
                self.gesture = Some(Gesture::Pan(per_point));
            } else if self.gesture.is_none() && resp.drag_started_by(B::Middle) {
                self.gesture = Some(Gesture::Look);
            }
            if matches!(self.gesture, Some(Gesture::Look | Gesture::Pan(_))) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::Locked));
                ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(false));
            }
        }
        let (mut primary, secondary, any_down) =
            ctx.input(|i| (i.pointer.button_down(B::Primary), i.pointer.button_down(B::Secondary), i.pointer.any_down()));
        let mut shift = ctx.input(|i| i.modifiers.shift);
        let (mut latest, mut motion) = (latest, motion);
        if let Some(t) = &mut self.test_drag
            && self.gesture.is_some_and(|g| g.handle_target().is_some())
        {
            motion = t.pos - t.prev;
            t.prev = t.pos;
            (latest, primary, shift) = (Some(t.pos), t.down, t.shift);
        }
        match self.gesture {
            Some(Gesture::Look) => {
                let (k, flip) = self.look_gain();
                self.cam.look(-motion.x * k, -motion.y * k * flip);
            }
            Some(Gesture::Pan(per_point)) => {
                let (c, _) = self.view_ray(self.view_rect.center());
                self.cam.translate(c.u * (motion.y * per_point) - c.r * (motion.x * per_point));
            }
            Some(Gesture::Carry { top, dist, offset }) => {
                let mut dist = dist;
                if scroll != 0.0 {
                    dist = (dist * 1.12f32.powf(scroll.signum())).clamp(0.3, 500.0);
                }
                // up / down arrows: farther / nearer
                let (up, down) = ctx.input(|i| (i.key_down(egui::Key::ArrowUp), i.key_down(egui::Key::ArrowDown)));
                let push = up as i32 as f32 - down as i32 as f32;
                if push != 0.0 {
                    dist = (dist + push * dist.max(1.0) * 1.2 * dt).clamp(0.3, 500.0);
                }
                self.gesture = Some(Gesture::Carry { top, dist, offset });
                // the right button charges a throw; letting go of it throws
                if secondary && self.charge.is_none() {
                    self.charge = Some(Instant::now());
                }
                let pos = latest.unwrap_or(self.view_rect.center());
                let (c, d) = self.view_ray(pos);
                if let Some(b) = self.carried() {
                    b.hold_at(c.pos + d * dist + offset, dt);
                }
                if !secondary && let Some(t0) = self.charge.take() {
                    let charge = (t0.elapsed().as_secs_f32() / physics::CHARGE_TIME).min(1.0);
                    if let Some(b) = self.carried() {
                        b.throw(d, charge);
                        let (m, v) = (b.mass, physics::throw_speed(b.mass, charge));
                        self.say(format!("threw {m:.1} kg at {v:.1} m/s"));
                    }
                    self.gesture = None;
                }
            }
            Some(Gesture::Axis { top, axis, start, grab }) => {
                let pos = latest.unwrap_or(self.view_rect.center());
                let (c, d) = self.view_ray(pos);
                if let Some(s) = closest_on_axis(start, AXES[axis], c.pos, d) {
                    let mut p = start + AXES[axis] * (s - grab);
                    // Ctrl: on a 10 cm grid
                    if ctx.input(|i| i.modifiers.command) {
                        p = Vec3::new((p.x * 10.0).round() / 10.0, (p.y * 10.0).round() / 10.0, (p.z * 10.0).round() / 10.0);
                    }
                    if let Some(n) = self.doc.node_mut(top, &[]).and_then(|n| n.as_object_mut()) {
                        n.insert("position".into(), doc::vec3([p.x, p.y, p.z].map(|v| (v as f64 * 1000.0).round() / 1000.0)));
                    }
                    self.doc.touch();
                }
            }
            Some(Gesture::Turn { top, axis, mut rot, angle }) => {
                let pos = latest.unwrap_or(self.view_rect.center());
                let (c, _) = self.view_ray(pos);
                let centre = self.to_screen(self.position_of(top));
                let now = centre.map_or(angle, |o| screen_angle(o, pos));
                if shift {
                    // Shift: about the grabbed ring's axis, as the pointer goes round the centre
                    let da = (now - angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    let toward = AXES[axis].dot(c.f) < 0.0;
                    rot = turn(AXES[axis], if toward { da } else { -da }).mul(rot);
                } else {
                    // a trackball: across turns it about the view's up, up and down about its right
                    let k = 0.01;
                    rot = turn(c.u, motion.x * k).mul(turn(c.r, motion.y * k)).mul(rot);
                }
                if let Some(n) = self.doc.node_mut(top, &[]).and_then(|n| n.as_object_mut()) {
                    n.insert("rotation".into(), doc::vec3(physics::euler_deg(rot).map(|v| (v as f64 * 100.0).round() / 100.0)));
                }
                self.doc.touch();
                self.gesture = Some(Gesture::Turn { top, axis, rot, angle: now });
            }
            Some(Gesture::Size { top, axis, grab, origin, dir }) => {
                let pos = latest.unwrap_or(self.view_rect.center());
                let (c, d) = self.view_ray(pos);
                if let Some(at) = closest_on_axis(origin, dir, c.pos, d)
                    && let Some(mut node) = self.size_start.clone()
                    && let Some(n) = node.as_object_mut()
                {
                    let f = (at / grab).clamp(0.05, 20.0) as f64;
                    // Shift: along the grabbed axis only, where the shape has sizes per axis
                    let one_axis = shift && stretch(n, axis, f);
                    if !one_axis {
                        let s0 = n.get("scale").and_then(serde_json::Value::as_f64).unwrap_or(1.0);
                        n.insert("scale".into(), ((s0 * f * 1000.0).round() / 1000.0).into());
                    }
                    if shift && !one_axis {
                        let ty = n.get("type").and_then(serde_json::Value::as_str).unwrap_or("this shape").to_string();
                        self.say(format!("a {ty} scales evenly only (boxes, ellipsoids, clouds, cylinders, cones and spheres stretch)"));
                    }
                    if let Some(slot) = self.doc.node_mut(top, &[]) {
                        *slot = node;
                    }
                    self.doc.touch();
                }
            }
            None => {}
        }
        // letting go of a handle leaves the object where it is, floating or not,
        // until it is dropped
        if let Some(top) = self.gesture.and_then(|g| g.handle_target())
            && !primary
        {
            self.placed.0.insert(top);
            self.gesture = None;
            self.size_start = None;
        }
        // letting go of the left button drops what is carried
        if let Some(Gesture::Carry { top, .. }) = self.gesture
            && !primary
        {
            if let Some(i) = self.world.find(top) {
                if self.physics {
                    self.world.bodies[i].held = false;
                } else {
                    // without physics on drop it stays where it is let go
                    self.world.bodies.swap_remove(i);
                    self.placed.0.insert(top);
                }
            }
            self.charge = None;
            self.gesture = None;
        }
        if matches!(self.gesture, Some(Gesture::Look | Gesture::Pan(_))) && !any_down {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
            self.gesture = None;
        }
        // the wheel moves toward (or away from) the point under the cursor
        if resp.hovered() && scroll != 0.0 && !matches!(self.gesture, Some(Gesture::Carry { .. })) {
            let pos = latest.unwrap_or(self.view_rect.center());
            let (c, d) = self.view_ray(pos);
            let notches = (scroll / 50.0).clamp(-4.0, 4.0);
            // each notch takes 15% off the distance to the point under the
            // cursor (or puts it back): finer the nearer, never through it.
            // With Shift (or nothing under the cursor) a step by the fly
            // speed, through whatever is there
            let hit = if through { None } else { pick_ray(&self.scene, &c, &self.settings, c.pos, d) };
            let step = match hit {
                Some(p) => p.t * (1.0 - 0.85f32.powf(notches)),
                None => self.speed * 0.3 * notches,
            };
            self.cam.translate(d * step);
        }
    }

    /// Where world point `p` is on the screen (None behind the camera).
    fn to_screen(&self, p: Vec3) -> Option<egui::Pos2> {
        let r = self.view_rect;
        let mut c = self.cam;
        c.setup(r.width().max(1.0) as u32, r.height().max(1.0) as u32);
        let d = p - c.pos;
        let z = d.dot(c.f);
        if z < 0.05 {
            return None;
        }
        let (col, row) = ((d.dot(c.r) / z - c.bx) / c.kx, (d.dot(c.u) / z - c.by) / c.ky);
        Some(egui::pos2(r.left() + col, r.top() + row))
    }

    /// The selected top-level object's move handles: arrows along x, y and z
    /// from its position, about 80 points long however far away it is.
    /// Terrain, water and planes have none.
    fn handles(&self) -> Option<Handles> {
        let Selection::Node { top, sub } = &self.sel else { return None };
        if !sub.is_empty() {
            return None;
        }
        let node = self.doc.node(*top, &[])?;
        if matches!(node.get("type").and_then(serde_json::Value::as_str), Some("terrain" | "water" | "plane")) {
            return None;
        }
        let origin = self.position_of(*top);
        let depth = (origin - self.cam.pos).dot(self.view_ray(self.view_rect.center()).0.f);
        if depth < 0.1 {
            return None;
        }
        let per_point = 2.0 * (self.cam.fov.to_radians() * 0.5).tan() / self.view_rect.height().max(1.0);
        let len = 80.0 * per_point * depth;
        let a = self.to_screen(origin)?;
        // scale handles follow the object's own axes (its sizes are along them)
        let rot = self.rotation_of(*top);
        let dirs = if self.tool == Tool::Scale { AXES.map(|e| rot.apply(e)) } else { AXES };
        let arrows = dirs.map(|e| (a, self.to_screen(origin + e * len).unwrap_or(a)));
        let rings = if self.tool == Tool::Rotate {
            std::array::from_fn(|i| {
                let (u, v) = (AXES[(i + 1) % 3], AXES[(i + 2) % 3]);
                (0..=64)
                    .filter_map(|k| {
                        let t = k as f32 * std::f32::consts::TAU / 64.0;
                        self.to_screen(origin + (u * t.cos() + v * t.sin()) * len)
                    })
                    .collect()
            })
        } else {
            Default::default()
        };
        Some(Handles { top: *top, origin, dirs, arrows, rings })
    }

    /// Starts dragging handle `axis` of `h`'s object, grabbed at screen point `o`.
    fn grab_handle(&mut self, h: &Handles, axis: usize, o: egui::Pos2) {
        let (c, d) = self.view_ray(o);
        let top = h.top;
        let gesture = match self.tool {
            Tool::Move => closest_on_axis(h.origin, AXES[axis], c.pos, d).map(|grab| Gesture::Axis { top, axis, start: h.origin, grab }),
            Tool::Rotate => Some(Gesture::Turn { top, axis, rot: self.rotation_of(top), angle: screen_angle(h.arrows[0].0, o) }),
            Tool::Scale => closest_on_axis(h.origin, h.dirs[axis], c.pos, d)
                .filter(|&g| g > 1e-3)
                .map(|grab| Gesture::Size { top, axis, grab, origin: h.origin, dir: h.dirs[axis] }),
        };
        if let Some(g) = gesture {
            // it stops moving on its own while it is placed by hand
            if let Some(i) = self.world.find(top) {
                self.world.bodies.swap_remove(i);
            }
            self.doc.checkpoint(true);
            self.size_start = self.doc.node(top, &[]).cloned();
            self.gesture = Some(g);
        }
    }

    /// Drives RAY_TEST_DRAG: grabs at frame 30, moves over 20 frames, lets go,
    /// and logs the object as the document has it and as compiled.
    fn test_drag_step(&mut self) {
        let Some(mut t) = self.test_drag else { return };
        let f = self.frames;
        match f {
            30 => {
                let Some(h) = self.handles() else { return eprintln!("test: drag: no handles") };
                let ring = &h.rings[t.axis.min(2)];
                let at = if t.tool == Tool::Rotate { ring.get(ring.len() / 8).copied() } else { Some(h.arrows[t.axis.min(2)].1) };
                let Some(at) = at else { return };
                (t.start, t.pos, t.prev, t.down) = (at, at, at, true);
                self.test_drag = Some(t);
                self.grab_handle(&h, t.axis.min(2), at);
                eprintln!("test: drag {} axis {} at {at:?}, gesture {}", t.tool.label(), t.axis, self.gesture.is_some());
            }
            31..=50 => {
                t.pos = t.start + t.delta * ((f - 30) as f32 / 20.0);
                self.test_drag = Some(t);
            }
            51 => {
                t.down = false;
                self.test_drag = Some(t);
            }
            55 | 90 | 150 => {}
            _ => return,
        }
        if matches!(f, 40 | 55 | 90 | 150) {
            let Selection::Node { top, .. } = self.sel.clone() else { return };
            let node = self.doc.node(top, &[]).cloned().unwrap_or_default();
            let keep = ["type", "position", "rotation", "scale", "size", "radius", "radii", "height", "stretch", "profile"];
            let shown: serde_json::Map<_, _> = node.as_object().into_iter().flatten().filter(|(k, _)| keep.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect();
            let compiled = self.scene.objects.iter().find(|o| o.doc_index == top).map(|o| o.aabb.center());
            eprintln!(
                "test: drag frame {f}: node {}, compiled centre {compiled:?}, gesture {}, body {}, placed {}",
                serde_json::Value::Object(shown),
                self.gesture.is_some(),
                self.world.find(top).is_some(),
                self.placed.0.contains(&top)
            );
        }
    }

    /// A top-level object's position in the document.
    fn position_of(&self, top: usize) -> Vec3 {
        let p = self.doc.node(top, &[]).and_then(|n| n.get("position")).and_then(doc::as_vec3).unwrap_or([0.0; 3]);
        Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32)
    }

    /// A top-level object's rotation in the document.
    fn rotation_of(&self, top: usize) -> Mat3 {
        self.doc.node(top, &[]).and_then(|n| n.get("rotation")).and_then(doc::as_vec3).map_or(Mat3::IDENTITY, |r| Mat3::euler_deg(r.map(|v| v as f32)))
    }

    /// The handle (axis) under screen point `p`, if any.
    fn handle_at(h: &Handles, tool: Tool, p: egui::Pos2) -> Option<usize> {
        let dist = |i: usize| match tool {
            Tool::Rotate => h.rings[i].windows(2).map(|w| segment_distance(p, w[0], w[1])).fold(f32::INFINITY, f32::min),
            _ if (h.arrows[i].1 - h.arrows[i].0).length() > 6.0 => segment_distance(p, h.arrows[i].0, h.arrows[i].1),
            _ => f32::INFINITY,
        };
        (0..3).map(|i| (i, dist(i))).filter(|&(_, d)| d < 9.0).min_by(|a, b| a.1.total_cmp(&b.1)).map(|(i, _)| i)
    }

    /// Draws the selected object's handles for the tool (the grabbed or
    /// hovered one brighter) and the tool bar, and while one is dragged the
    /// position, rotation or size.
    fn draw_handles(&mut self, ui: &egui::Ui) {
        let Some(h) = self.handles() else { return };
        self.tool_bar(ui);
        let active = match self.gesture {
            Some(g) if g.handle_target().is_some() => g.handle_axis(),
            None => ui.ctx().pointer_hover_pos().and_then(|p| Self::handle_at(&h, self.tool, p)),
            _ => None,
        };
        let painter = ui.painter_at(self.view_rect);
        let shadow = |w: f32| egui::Stroke::new(w + 2.0, egui::Color32::from_black_alpha(140));
        for i in 0..3 {
            let hot = active == Some(i);
            let c = if hot { AXIS_COLORS[i].gamma_multiply(1.4) } else { AXIS_COLORS[i] };
            let width = if hot { 4.0 } else { 2.5 };
            if self.tool == Tool::Rotate {
                let ring = &h.rings[i];
                if ring.len() < 2 {
                    continue;
                }
                painter.add(egui::Shape::line(ring.clone(), shadow(width)));
                painter.add(egui::Shape::line(ring.clone(), egui::Stroke::new(width, c)));
                painter.text(ring[ring.len() / 8] + egui::vec2(8.0, -8.0), egui::Align2::CENTER_CENTER, AXIS_NAMES[i], egui::FontId::proportional(14.0), c);
                continue;
            }
            let (a, b) = h.arrows[i];
            let v = b - a;
            if v.length() < 6.0 {
                continue;
            }
            painter.line_segment([a, b], shadow(width));
            painter.line_segment([a, b], egui::Stroke::new(width, c));
            let dir = v.normalized();
            let edge = egui::Stroke::new(1.0, egui::Color32::from_black_alpha(160));
            let tip = if self.tool == Tool::Scale {
                // a cube at the end
                painter.rect(egui::Rect::from_center_size(b, egui::vec2(11.0, 11.0)), 1.0, c, edge, egui::StrokeKind::Middle);
                b + dir * 6.0
            } else {
                let side = egui::vec2(-dir.y, dir.x);
                let tip = b + dir * 12.0;
                painter.add(egui::Shape::convex_polygon(vec![tip, b + side * 6.0, b - side * 6.0], c, edge));
                tip
            };
            painter.text(tip + dir * 9.0, egui::Align2::CENTER_CENTER, AXIS_NAMES[i], egui::FontId::proportional(14.0), c);
        }
        painter.circle(h.arrows[0].0, 4.0, egui::Color32::WHITE, egui::Stroke::new(1.0, egui::Color32::BLACK));
        let Some(g) = self.gesture.filter(|g| g.handle_target().is_some()) else { return };
        let node = self.doc.node(h.top, &[]);
        let get = |k: &str| node.and_then(|n| n.get(k));
        let text = match g {
            Gesture::Turn { .. } => {
                let r = get("rotation").and_then(doc::as_vec3).unwrap_or([0.0; 3]);
                format!("rotation x {:.1}°   y {:.1}°   z {:.1}°", r[0], r[1], r[2])
            }
            Gesture::Size { .. } => {
                let scale = get("scale").and_then(serde_json::Value::as_f64).unwrap_or(1.0);
                let size = ["size", "radii", "stretch"].iter().find_map(|k| get(k).and_then(doc::as_vec3));
                match size {
                    Some(v) => format!("scale {scale:.3}   size {:.3} × {:.3} × {:.3}", v[0], v[1], v[2]),
                    None => format!("scale {scale:.3}"),
                }
            }
            _ => {
                let o = h.origin;
                format!("x {:.2}   y {:.2}   z {:.2}", o.x, o.y, o.z)
            }
        };
        painter.text(h.arrows[0].0 + egui::vec2(12.0, 14.0), egui::Align2::LEFT_TOP, text, egui::FontId::monospace(13.0), egui::Color32::WHITE);
    }

    /// A scene that does not compile shows the last one that did: say so over
    /// the view, where it is seen (edits seem to do nothing until it is fixed).
    fn error_banner(&self, ui: &egui::Ui) {
        let Some(e) = &self.error else { return };
        let at = self.view_rect.center_top() + egui::vec2(0.0, 48.0);
        egui::Area::new(egui::Id::new("scene error")).fixed_pos(at).pivot(egui::Align2::CENTER_TOP).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).fill(egui::Color32::from_rgb(0x5a, 0x16, 0x16)).show(ui, |ui| {
                ui.set_max_width(self.view_rect.width() * 0.8);
                ui.colored_label(egui::Color32::WHITE, format!("⛔ The scene does not compile, so this is the last one that did: {e}"));
                ui.colored_label(egui::Color32::from_gray(200), "Fix the value, or undo (Ctrl+Z).");
            });
        });
    }

    /// Move / Rotate / Scale buttons in the view's corner, while an object
    /// with handles is selected.
    fn tool_bar(&mut self, ui: &egui::Ui) {
        let at = self.view_rect.left_top() + egui::vec2(10.0, 10.0);
        egui::Area::new(egui::Id::new("handle tools")).fixed_pos(at).order(egui::Order::Foreground).show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    for t in Tool::ALL {
                        if ui.selectable_label(self.tool == t, t.label()).on_hover_text(t.tip()).clicked() {
                            self.tool = t;
                        }
                    }
                });
            });
        });
    }

    /// The throw gauge at the pointer: a ring that fills (and brightens) as the
    /// right button is held, with the speed it will leave at.
    fn charge_gauge(&mut self, ui: &egui::Ui) {
        let Some(t0) = self.charge else { return };
        let pos = ui.ctx().input(|i| i.pointer.latest_pos()).unwrap_or(self.view_rect.center());
        let charge = (t0.elapsed().as_secs_f32() / physics::CHARGE_TIME).min(1.0);
        let mass = self.carried().map_or(1.0, |b| b.mass);
        let p = ui.painter();
        let r = 26.0;
        p.circle_filled(pos, r + 6.0, egui::Color32::from_black_alpha(120));
        p.circle_stroke(pos, r, egui::Stroke::new(4.0, egui::Color32::from_white_alpha(50)));
        let steps = 48;
        let end = charge * std::f32::consts::TAU;
        let pts: Vec<egui::Pos2> = (0..=steps)
            .map(|k| {
                let a = -std::f32::consts::FRAC_PI_2 + end * k as f32 / steps as f32;
                pos + egui::vec2(a.cos(), a.sin()) * r
            })
            .collect();
        let c = crate::render::ramp_rgb(0.45 + 0.55 * charge);
        p.add(egui::Shape::line(pts, egui::Stroke::new(4.0, egui::Color32::from_rgb(c[0], c[1], c[2]))));
        p.text(
            pos + egui::vec2(0.0, r + 14.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%  {:.1} m/s", charge * 100.0, physics::throw_speed(mass, charge)),
            egui::FontId::proportional(13.0),
            egui::Color32::WHITE,
        );
    }

    fn keys(&mut self, ctx: &egui::Context, dt: f32) {
        let typing = ctx.wants_keyboard_input();
        if typing {
            return;
        }
        if self.game.is_some() {
            // (the walker's keys are read by game_frame; of the commands only
            // these make sense with the interface gone)
            for act in actions::pressed(ctx) {
                if matches!(act, Act::Game | Act::Headlight | Act::Screenshot | Act::QuickSave | Act::QuickLoad | Act::Record | Act::GetOut) {
                    self.run_act(act);
                } else if act == Act::Help
                    && let Some(g) = self.game.as_mut()
                {
                    // (the help window is hidden here: the keys' hint again)
                    g.since = Instant::now();
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.game = None;
            }
            return;
        }
        let (mut mv, mut look, mut fast, mut slow) = (Vec3::ZERO, (0.0f32, 0.0f32), false, false);
        ctx.input(|i| {
            let k = |key: egui::Key| if i.key_down(key) && !i.modifiers.command { 1.0 } else { 0.0 };
            mv.z = k(egui::Key::W) - k(egui::Key::S);
            mv.x = k(egui::Key::D) - k(egui::Key::A);
            mv.y = k(egui::Key::R) + k(egui::Key::Space) - k(egui::Key::F) - k(egui::Key::C);
            look.0 = k(egui::Key::ArrowLeft) + k(egui::Key::Q) - k(egui::Key::ArrowRight) - k(egui::Key::E);
            look.1 = k(egui::Key::ArrowUp) - k(egui::Key::ArrowDown);
            fast = i.modifiers.shift;
            slow = i.modifiers.alt;
        });
        let speed = self.speed * if fast { 4.0 } else { 1.0 } * if slow { 0.25 } else { 1.0 };
        if mv != Vec3::ZERO {
            // slow down toward what is ahead, never more than 3 times its
            // distance a second: a wall (or a fractal's crevice) is approached
            // ever more finely and never passed through
            let dir = self.cam.fly_dir(mv).normalize();
            // (the camera as the view sets it up: unset, its pixel angle is 0,
            // and fractals were picked at their finest with a threshold of
            // nothing, missed, and flown through)
            let ahead = pick_ray(&self.scene, &self.view_cam(), &self.settings, self.cam.pos, dir).map_or(f32::INFINITY, |p| p.t);
            let step = (speed * dt).min(ahead * (3.0 * dt).min(0.5));
            self.cam.translate(dir * step);
        }
        if matches!(self.gesture, Some(Gesture::Carry { .. })) {
            // carrying: up / down move it (view_input); left tips it forward, right
            // turns it clockwise as seen from the camera; Shift reverses
            let (left, right, shift) = ctx.input(|i| (i.key_down(egui::Key::ArrowLeft), i.key_down(egui::Key::ArrowRight), i.modifiers.shift));
            let (c, _) = self.view_ray(self.view_rect.center());
            let angle = std::f32::consts::FRAC_PI_2 * dt * if shift { -1.0 } else { 1.0 };
            if let Some(b) = self.carried() {
                if left {
                    b.rotate(-c.r, angle);
                }
                if right {
                    b.rotate(c.f, angle);
                }
            }
        } else if look != (0.0, 0.0) {
            let turn = 1.6 * dt;
            self.cam.look(look.0 * turn, look.1 * turn);
        }
        for act in actions::pressed(ctx) {
            self.run_act(act);
        }
        let pressed = |key: egui::Key| ctx.input(|i| i.key_pressed(key) && !i.modifiers.command);
        // Esc closes what is open, the help first, then the file window; else it
        // clears the selection (not when it just left a text field or a menu)
        let busy = self.had_focus || egui::Popup::is_any_open(ctx);
        if pressed(egui::Key::Escape) && !busy {
            if self.help {
                self.help = false;
            } else if self.file_dialog.is_some() {
                self.file_dialog = None;
            } else if self.show_prefs {
                self.show_prefs = false;
            } else if self.show_env {
                self.show_env = false;
            } else if self.show_library || self.show_modifiers {
                self.show_library = false;
                self.show_modifiers = false;
            } else {
                self.sel = Selection::None;
            }
        }
        if pressed(egui::Key::OpenBracket) {
            self.speed = (self.speed / 1.5).max(0.01);
            self.say(format!("fly speed {:.3} m/s", self.speed));
        }
        if pressed(egui::Key::CloseBracket) {
            self.speed = (self.speed * 1.5).min(5000.0);
            self.say(format!("fly speed {:.3} m/s", self.speed));
        }
        let modes = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7];
        for (key, mode) in modes.into_iter().zip(Mode::ALL) {
            if pressed(key) {
                self.settings.mode = mode;
                self.say(format!("mode: {}", mode.label()));
            }
        }
    }

    /// Carries out a command (menus, keys and buttons all come here).
    fn run_act(&mut self, act: Act) {
        if !self.can(act) {
            return;
        }
        match act {
            Act::Open => self.file_dialog = Some(FileDialog::new(false, &self.path)),
            // (how it went is said on screen)
            Act::Save => {
                let _ = self.save();
            }
            Act::QuickSave => {
                let _ = self.quick_save();
            }
            Act::QuickLoad => self.quick_load(),
            Act::SaveAs => self.file_dialog = Some(FileDialog::new(true, &self.path)),
            Act::Revert => {
                let armed = self.revert_armed.is_some_and(|t| t.elapsed().as_secs_f32() < 4.0);
                if self.doc.dirty && !armed {
                    self.revert_armed = Some(Instant::now());
                    self.say("unsaved edits: Revert again to discard them and load the file");
                } else {
                    self.revert_armed = None;
                    self.reload(false);
                }
            }
            Act::Screenshot => self.screenshot(),
            Act::Undo => self.undo(),
            Act::Redo => self.redo(),
            Act::Duplicate => self.duplicate(),
            Act::Delete => self.delete(),
            Act::DropAll => self.drop_all(),
            Act::Reset => self.reset_scene(),
            Act::Frame => self.frame_selected(),
            Act::StartView => self.cam = crate::start_camera(&self.scene),
            Act::GetOut => self.get_out(),
            Act::Panels => {
                let on = !(self.show_scene || self.show_render || self.show_perf);
                self.show_scene = on;
                self.show_render = on;
                self.show_perf = on;
            }
            Act::Heat => self.heat = !self.heat,
            Act::Preferences => self.show_prefs = !self.show_prefs,
            Act::Game => {
                if self.game.take().is_none() {
                    self.enter_game();
                }
            }
            Act::Record => {
                if self.recorder.is_some() {
                    self.stop_recording();
                } else {
                    // (steady: of the view, at the preferences' size)
                    let steady = self.config.video_steady.then(|| {
                        let [w, h] = self.config.video_size;
                        let size = if w > 0 && h > 0 { video_size(w as f32, h as f32) } else { video_size(self.view_px.0, self.view_px.1) };
                        record::Steady::new(self.config.video_fps, self.config.video_steady_samples, size, None, self.now())
                    });
                    if let Err(e) = self.start_recording(steady, self.config.video_quality) {
                        self.say(format!("cannot record: {e}"));
                    }
                }
            }
            Act::Device => {
                self.settings.device = self.settings.device.next();
                self.say(format!("rendering on: {}", self.settings.device.label()));
            }
            Act::Tool => {
                let i = Tool::ALL.iter().position(|&t| t == self.tool).unwrap_or(0);
                self.tool = Tool::ALL[(i + 1) % Tool::ALL.len()];
                self.say(format!("handles: {}", self.tool.label().to_lowercase()));
            }
            Act::Help => self.help = !self.help,
            Act::Library => self.show_library = !self.show_library,
            Act::Modifiers => self.show_modifiers = !self.show_modifiers,
            Act::Headlight => {
                // off keeps the strength it had, for the next time on
                if self.settings.headlight > 0.0 {
                    self.headlight_was = self.settings.headlight;
                    self.settings.headlight = 0.0;
                } else {
                    self.settings.headlight = self.headlight_was;
                }
                self.say(if self.settings.headlight > 0.0 { "headlight on" } else { "headlight off" });
            }
        }
    }

    /// Whether a command can be carried out now.
    /// A command's name as things stand (a switch says what it will do).
    fn act_name(&self, act: Act) -> String {
        match act {
            Act::Panels if !(self.show_scene || self.show_render || self.show_perf) => "Show panels".into(),
            Act::Record => match &self.recorder {
                Some(r) => format!("Stop recording ({})", record::clock(r.started.elapsed().as_secs_f32())),
                None => actions::get(act).name.into(),
            },
            _ => actions::get(act).name.into(),
        }
    }

    /// Whether a command that switches something on and off has it on.
    fn act_on(&self, act: Act) -> bool {
        match act {
            Act::Heat => self.heat,
            Act::Headlight => self.settings.headlight > 0.0,
            Act::Library => self.show_library,
            Act::Modifiers => self.show_modifiers,
            Act::Help => self.help,
            Act::Preferences => self.show_prefs,
            Act::Record => self.recorder.is_some(),
            _ => false,
        }
    }

    fn can(&self, act: Act) -> bool {
        let node = matches!(self.sel, Selection::Node { .. });
        match act {
            Act::Undo => self.doc.can_undo(),
            Act::Redo => self.doc.can_redo(),
            Act::Duplicate | Act::Delete | Act::Frame => node,
            Act::QuickLoad => self.quick_save_path().exists(),
            _ => true,
        }
    }

    /// A menu entry for a command: its name, keys and description; it
    /// closes the menu.
    fn item(&mut self, ui: &mut egui::Ui, act: Act) {
        let a = actions::get(act);
        let keys = actions::keys_text(ui.ctx(), a);
        let button = egui::Button::new(self.act_name(act)).shortcut_text(keys).selected(self.act_on(act));
        if ui.add_enabled(self.can(act), button).on_hover_text(a.tip).on_disabled_hover_text(why_not(act)).clicked() {
            self.run_act(act);
            // a Revert that only asked stays open for the second click
            if !(act == Act::Revert && self.revert_armed.is_some()) {
                ui.close();
            }
        }
    }

    /// The emitters: send out what they owe, age and fly the copies; new
    /// copies are compiled at once, so their bodies find their objects.
    fn emit_step(&mut self, dt: f32) {
        // (in step with the physics, which takes a twentieth of a second at most a frame)
        let ch = self.emitters.step(&self.doc.v, dt.min(0.05), self.scene.gravity);
        for id in ch.gone {
            if let Some(i) = self.world.find(id) {
                self.world.bodies.swap_remove(i);
            }
        }
        if ch.spawned {
            self.compile();
        }
        self.world.bodies.extend(ch.bodies);
    }

    /// Puts the scene back as it was opened: every edit and everything the
    /// physics moved (one undo step), the start view.
    fn reset_scene(&mut self) {
        self.doc.checkpoint(true);
        self.doc.v = self.original.clone();
        self.doc.touch();
        self.doc.dirty = !self.original_on_disk;
        self.emitters.clear();
        self.world.bodies.clear();
        self.world.ripples.clear();
        self.placed.0.clear();
        self.gesture = None;
        self.charge = None;
        self.sel = Selection::None;
        // the start view of the scene as it was opened
        self.compile();
        self.cam = crate::start_camera(&self.scene);
        self.speed = self.scene.camera.speed;
        self.say("scene reset (Ctrl+Z brings the changes back)");
    }

    /// The document changed under the physics (undo, redo, a deletion, a
    /// reload): its bodies go (the emitters' copies keep theirs, from where
    /// they are), and a selection of what is no longer there.
    fn lost_bodies(&mut self) {
        self.world.bodies.clear();
        let copies = self.emitters.bodies_again(&self.doc.v);
        self.world.bodies.extend(copies);
        if let Selection::Node { top, sub } = &self.sel
            && self.doc.node(*top, sub).is_none()
        {
            self.sel = Selection::None;
        }
    }

    fn undo(&mut self) {
        if self.doc.undo() {
            self.lost_bodies();
            self.placed.0.clear();
            self.say("undone");
        }
    }

    fn redo(&mut self) {
        if self.doc.redo() {
            self.lost_bodies();
            self.placed.0.clear();
            self.say("redone");
        }
    }

    fn delete(&mut self) {
        let sel = self.sel.clone();
        let next = self.doc.delete(&sel);
        if next != sel {
            self.sel = next;
            self.lost_bodies();
        }
    }

    fn duplicate(&mut self) {
        let sel = self.sel.clone();
        self.sel = self.doc.duplicate(&sel, [0.6, 0.0, 0.6]);
    }

    /// Points the camera at the selected object from a distance that fits it.
    fn frame_selected(&mut self) {
        let Selection::Node { top, .. } = self.sel else { return };
        let objects = self.selected_objects().map_or(&[][..], |(a, b)| &self.scene.objects[a as usize..b as usize]);
        let clouds = self.scene.volumes.iter().filter(|v| v.doc_index == top).map(|v| v.aabb);
        let Some(b) = objects.iter().map(|o| o.aabb).chain(clouds).reduce(|a, b| a.union(b)) else { return };
        if !b.is_finite() {
            return;
        }
        let c = b.center();
        let r = b.size().length() * 0.5;
        let dist = r / (self.cam.fov.to_radians() * 0.5).tan() * 1.3;
        let (sy, cy) = self.cam.yaw.sin_cos();
        let (sp, cp) = self.cam.pitch.sin_cos();
        let f = Vec3::new(-sy * cp, sp, -cy * cp);
        self.cam.place(c - f * dist.max(0.5));
    }

    /// Out of whatever the camera is inside, or in game mode the walker is
    /// stuck in: to the nearest place with room around it (a tenth of a
    /// second's flying for the camera; the body for the walker, as tall).
    fn get_out(&mut self) {
        let cam = self.view_cam();
        let eye = cam.exact();
        let (who, moved) = if let Some(g) = self.game.as_mut() {
            let moved = crate::render::view_query(&self.scene, &cam, &self.settings, |c, regs| g.get_out(&mut game::Probe::new(c, regs)));
            self.cam.set_exact(g.eye());
            self.game_eye = self.cam.exact();
            ("the walker", moved)
        } else {
            let room = 0.1 * self.speed as f64;
            let to = crate::render::view_query(&self.scene, &cam, &self.settings, |c, regs| game::open_space(&mut game::Probe::new(c, regs), eye, &[[0.0; 3]], room, (1e4 * room).max(1e3)));
            if let Some(p) = to {
                self.cam.set_exact(p);
            }
            ("the camera", to.map(|p| (0..3).map(|i| (p[i] - eye[i]).powi(2)).sum::<f64>().sqrt()))
        };
        match moved {
            Some(m) if m > 0.0 => self.say(format!("got out: {who} moved {} to the nearest open space", length_text(m))),
            Some(_) => self.say(format!("{who} has room around it already: nothing to get out of")),
            None => self.say(format!("no open space near enough for {who} to get out to")),
        }
    }

    /// A new top-level object of type `ty` in front of the camera, on whatever is there.
    fn add_object(&mut self, ty: &str) {
        if ty == "cloud" {
            return self.add_cloud();
        }
        let Some(info) = schema::shape(ty) else { return };
        let Ok(mut node) = serde_json::from_str::<serde_json::Value>(info.new) else { return };
        let (c, d) = self.view_ray(self.view_rect.center());
        let at = match pick_ray(&self.scene, &c, &self.settings, c.pos, d) {
            // (an emitter's nozzle stands on the surface)
            Some(p) if p.t < 40.0 => p.point + p.normal * if ty == "emitter" { 0.12 } else { 0.02 },
            _ => c.pos + d * 5.0,
        };
        if let Some(o) = node.as_object_mut()
            && !matches!(ty, "terrain" | "water" | "plane")
        {
            o.insert("position".into(), doc::vec3([at.x as f64, at.y as f64, at.z as f64]));
        }
        self.sel = self.doc.add_object(node);
        // compiled now: the physics drops bodies whose object it cannot find
        self.compile();
        // let it settle onto the surface
        if self.physics
            && !matches!(ty, "terrain" | "water" | "plane" | "gyroid")
            && let Selection::Node { top, .. } = self.sel
            && let Some(mut b) = physics::body_of(&self.doc.v, top)
        {
            // start just above the surface
            b.x.y += b.radius + 0.05;
            self.world.bodies.push(b);
        }
    }

    /// The checkerboard floor: flat terrain under the camera.
    fn add_floor(&mut self) {
        let (x, z) = ((self.cam.pos.x as f64).round(), (self.cam.pos.z as f64).round());
        self.sel = self.doc.add_object(serde_json::json!({
            "name": "floor",
            "type": "terrain",
            "noise": "flat",
            "size": 2000,
            "base": 0,
            "position": [x, 0, z],
            "material": { "color": [0.5, 0.48, 0.45], "color2": [0.22, 0.21, 0.2], "texture": "checker", "texture_scale": 1, "roughness": 0.7, "space": "world" }
        }));
    }

    /// A ready-made light: a small glowing sphere that floats, 4 m ahead.
    fn add_light(&mut self) {
        let (c, d) = self.view_ray(self.view_rect.center());
        let at = c.pos + d * 4.0;
        let r = crate::scene::json::LIGHT_RADIUS;
        let p = [at.x, at.y, at.z].map(|v| (v as f64 * 100.0).round() / 100.0);
        self.sel = self.doc.add_object(serde_json::json!({
            "name": "light",
            "type": "sphere",
            "radius": r,
            "position": p,
            "gravity": false,
            "material": { "color": [1.0, 0.85, 0.6], "emission": [800.0, 680.0, 480.0] }
        }));
    }

    /// A volume cloud 35 m ahead and 10 m up (a cumulus about 13 m across).
    pub(super) fn add_cloud(&mut self) {
        let (c, d) = self.view_ray(self.view_rect.center());
        let flat = Vec3::new(d.x, 0.0, d.z).normalize();
        let flat = if flat.is_finite() { flat } else { Vec3::new(0.0, 0.0, -1.0) };
        let at = c.pos + flat * 35.0 + Vec3::new(0.0, 10.0, 0.0);
        let p = [at.x, at.y, at.z].map(|v| (v as f64 * 10.0).round() / 10.0);
        self.sel = self.doc.add_object(cloud_node(p));
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                self.item(ui, Act::Open);
                self.item(ui, Act::Save);
                self.item(ui, Act::SaveAs);
                self.item(ui, Act::QuickSave);
                self.item(ui, Act::QuickLoad);
                self.item(ui, Act::Revert);
                self.item(ui, Act::Preferences);
                ui.checkbox(&mut self.config.auto_reload, "Reload when the file changes")
                    .on_hover_text("Pick up edits made to the JSON in a text editor (only while there are no unsaved edits here).");
                ui.separator();
                self.item(ui, Act::Screenshot);
                ui.checkbox(&mut self.config.screenshot_interface, "Screenshots include the interface")
                    .on_hover_text("Screenshots are the whole window as you see it (panels, menus), not the 3D view alone.");
                if ui.button("Open the screenshots folder").on_hover_text("Where screenshots go (Preferences: a ray folder in your Pictures folder unless set otherwise).").clicked() {
                    let dir = config::expand(&self.config.screenshots);
                    let _ = std::fs::create_dir_all(&dir);
                    open_externally(&std::fs::canonicalize(&dir).unwrap_or(dir));
                    ui.close();
                }
                self.item(ui, Act::Record);
                ui.checkbox(&mut self.config.microphone, "Record the microphone")
                    .on_hover_text("Recordings take in the system's default microphone (Settings > Sound > Input), for narrating. Takes effect at the next recording.");
                let last = self.last_video.as_ref().map(|v| v.0.clone());
                if ui.add_enabled(last.is_some(), egui::Button::new("Play the last recording")).on_hover_text("Open the last video recorded (F10) in the desktop's video player.").clicked()
                    && let Some(p) = last
                {
                    open_externally(&p);
                    ui.close();
                }
                if ui.button("Open the videos folder").on_hover_text("Where recordings go (Preferences: your Videos folder unless set otherwise).").clicked() {
                    let dir = config::expand(&self.config.videos);
                    let _ = std::fs::create_dir_all(&dir);
                    open_externally(&std::fs::canonicalize(&dir).unwrap_or(dir));
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                self.item(ui, Act::Undo);
                self.item(ui, Act::Redo);
                ui.separator();
                self.item(ui, Act::Duplicate);
                self.item(ui, Act::Delete);
                ui.separator();
                self.item(ui, Act::DropAll);
                self.item(ui, Act::Reset);
                ui.checkbox(&mut self.physics, "Physics on drop")
                    .on_hover_text("Dropped and newly added objects fall and settle under gravity. Off: they stay where you let go.");
            });
            ui.menu_button("Add", |ui| self.add_menu(ui));
            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_scene, "Scene panel").on_hover_text("Objects, the inspector and materials (left).");
                ui.checkbox(&mut self.show_render, "Render panel").on_hover_text("Render settings and statistics (right).");
                ui.checkbox(&mut self.show_perf, "Load panel").on_hover_text("CPU, GPU and render time over time (bottom).");
                self.item(ui, Act::Game);
                if ui.add(egui::Button::new("Environment…").selected(self.show_env)).on_hover_text("The scene's sun, sky, fog, colours and gravity, in a window of their own.").clicked() {
                    self.show_env = !self.show_env;
                    ui.close();
                }
                self.item(ui, Act::Panels);
                self.item(ui, Act::Library);
                self.item(ui, Act::Modifiers);
                ui.separator();
                self.item(ui, Act::Frame);
                self.item(ui, Act::StartView);
                self.item(ui, Act::GetOut);
                ui.separator();
                ui.menu_button("Render mode", |ui| {
                    for (k, m) in Mode::ALL.into_iter().enumerate() {
                        if ui.add(egui::Button::selectable(self.settings.mode == m, m.label()).shortcut_text(format!("{}", k + 1))).clicked() {
                            self.settings.mode = m;
                            ui.close();
                        }
                    }
                });
                self.item(ui, Act::Device);
                self.item(ui, Act::Tool);
                self.item(ui, Act::Heat);
                self.item(ui, Act::Headlight);
            });
            ui.menu_button("Help", |ui| {
                for (tab, name, tip) in [
                    (help::Tab::Shapes, "Shapes", "Every shape type and its parameters."),
                    (help::Tab::Materials, "Materials", "Material settings, the patterns and lights."),
                    (help::Tab::Controls, "Controls", "Mouse and keyboard."),
                ] {
                    let keys = actions::keys_text(ui.ctx(), actions::get(Act::Help));
                    let b = egui::Button::new(name).shortcut_text(if tab == help::Tab::Shapes { keys } else { String::new() });
                    if ui.add(b).on_hover_text(tip).clicked() {
                        help::show_tab(ui.ctx(), tab);
                        self.help = true;
                        ui.close();
                    }
                }
            });
            ui.separator();
            // the scene's quick actions
            for (act, icon) in [(Act::DropAll, "⬇"), (Act::Reset, "⟲"), (Act::Game, "▶")] {
                let a = actions::get(act);
                let keys = actions::keys_text(ui.ctx(), a);
                let tip = if keys.is_empty() { a.tip.to_string() } else { format!("{} ({keys})", a.tip) };
                if ui.button(format!("{icon} {}", a.name)).on_hover_text(tip).clicked() {
                    self.run_act(act);
                }
            }
            ui.separator();
            let name = self.path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
            if self.doc.dirty {
                ui.label(egui::RichText::new(format!("{name} ●")).strong()).on_hover_text("Unsaved edits (Ctrl+S saves).");
            } else {
                ui.label(name).on_hover_text("Saved.");
            }
            if self.changed_on_disk {
                ui.colored_label(style::WARN, "⚠ changed on disk")
                    .on_hover_text("The file changed outside ray while you have unsaved edits. File > Revert loads it (losing your edits); Save overwrites it.");
            }
            if let Some(e) = &self.error {
                ui.colored_label(style::ERROR, format!("⛔ {e}"));
            }
            if let Some((m, at)) = &self.message
                && at.elapsed().as_secs_f32() < 4.0
            {
                ui.label(egui::RichText::new(m).color(ui.visuals().weak_text_color()));
            }
            // a finished recording: where it is, for a while
            if let Some((path, at)) = &self.last_video
                && at.elapsed().as_secs_f32() < VIDEO_NOTE_SECS
            {
                let path = path.clone();
                ui.label(egui::RichText::new(format!("🎬 saved {}", path.display())).color(style::WARN));
                if ui.small_button("▶ play").on_hover_text("Open it in the desktop's video player.").clicked() {
                    open_externally(&path);
                }
                if ui.small_button("📁 folder").on_hover_text("Open the folder it is in.").clicked()
                    && let Some(dir) = path.parent()
                {
                    open_externally(dir);
                }
            }
        });
    }

    fn post_job(&mut self) {
        // a steady recording's frame holds still until it is in the video:
        // its view goes out once (a walker in the water settling by a
        // micrometre each window frame restarted the frame's samples, and
        // it never had them all)
        if let Some(st) = &mut self.steady {
            if st.posted_for == Some(st.frames) {
                return;
            }
            st.posted_for = Some(st.frames);
        }
        // anything moving? then (optionally) one sample per frame and fewer pixels
        let pose = self.cam.pose();
        if pose != self.last_pose || self.doc.version != self.last_version || self.gesture.is_some() || !self.world.bodies.is_empty() {
            self.last_change = Instant::now();
            self.last_pose = pose;
            self.last_version = self.doc.version;
        }
        let mut s = self.settings;
        // a steady recording's frame accumulates its samples and stops there
        // (on, it kept the GPU from the window)
        if let Some(st) = &self.steady {
            s.accumulate = true;
            s.max_samples = st.wants(s.max_samples);
            // (in about eight passes: each pass's post and hand-over cost as
            // much as a sample of a simple scene)
            s.spp = s.spp.max(s.max_samples.div_ceil(8)).min(64);
        }
        if self.moving() {
            s.spp = 1;
        }
        let (w, h) = self.job_size();
        // moving water: the waves' clock while they are animated, and the ripples
        let now = self.now() as f32;
        let animate = self.waves_moving();
        let waves = if animate || !self.world.ripples.is_empty() {
            Waves { time: now, wave_time: if animate { now * self.scene.wave_speed } else { 0.0 }, ripples: self.world.ripples.clone() }
        } else {
            Waves::default()
        };
        let job = Job {
            scene: self.scene.clone(),
            scene_id: self.scene_id,
            cam: self.cam,
            s,
            w,
            h,
            waves,
            view: View {
                // (a steady recording's frames without the editor's outline)
                selected: self.selected_objects().filter(|_| self.steady.is_none()),
                heat: self.heat,
                profile: self.profile,
                twinkle: self.twinkling().then(|| self.now() as f32),
            },
            serial: self.posted + 1,
            load: self.steady.as_ref().map_or(1.0, |s| s.load),
        };
        if self.last_job.as_ref().is_none_or(|j| !j.same(&job)) {
            self.posted += 1;
            self.render.post(job.clone());
            self.last_job = Some(job);
        }
        // (the job a screenshot rendering whole waits for: this one)
        if let Some(st) = &mut self.still
            && st.posted.is_none()
        {
            st.posted = Some(self.posted);
        }
    }

    fn test_hook(&mut self, ctx: &egui::Context) {
        let Some((path, n)) = self.test_shot.clone() else { return };
        self.test_drag_step();
        if self.frames.is_multiple_of(5) {
            eprintln!(
                "test: frame {} at {:.1} s, samples {}, gpu share {:.2} ({:.1} ms gpu, {:.1} ms cpu), traced {}, bodies {}, ripples {}",
                self.frames,
                self.perf.now.t,
                self.stats.samples,
                self.stats.gpu_share,
                self.stats.gpu_ms,
                self.stats.cpu_ms,
                self.stats.traced,
                self.world.bodies.len(),
                self.world.ripples.len()
            );
            if let Some(g) = &self.game {
                eprintln!("test: game {} tall, feet {:.4?}, grounded {}, rocket {}, in water {:.2}", length_text(g.height), g.feet, g.grounded, g.jetting, g.wet);
            }
            #[cfg(feature = "profile")]
            eprintln!("test: heap {}", crate::prof::heap_since(&mut self.heap));
        }
        // (not while a screenshot of the window, P, is on its way: this one
        // closes the window, and under a busy GPU it came first)
        if self.window_shot.is_some() {
            ctx.request_repaint();
            return;
        }
        // asked again now and then: a headless compositor sometimes drops the request
        if self.frames >= n && (self.frames - n).is_multiple_of(20) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, user_data, .. } if user_data.data.is_none() => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(img) = shot {
            let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            let _ = crate::render::write_png(&path, img.size[0], img.size[1], &rgba);
            println!(
                "test shot: {} (render {:.1} ms, {}x{}, {} spp, ui {:.1} ms)",
                path.display(),
                self.stats.trace_ms + self.stats.post_ms,
                self.render_size.0,
                self.render_size.1,
                self.stats.samples,
                self.frame_ms
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint();
    }
}

impl App {
    /// Keys and buttons held from outside (`hold`), as input this frame:
    /// down as each starts, up as it runs out (as RAY_TEST_HOLD's are).
    fn inject_held(&mut self, raw: &mut egui::RawInput) {
        if self.held.is_empty() {
            return;
        }
        let now = self.now();
        let pos = self.view_rect.center();
        let mut shift = false;
        for (name, until, down) in &mut self.held {
            let up = now >= *until;
            let event = |pressed: bool| match name.as_str() {
                "lmb" | "rmb" => {
                    let button = if name == "lmb" { egui::PointerButton::Primary } else { egui::PointerButton::Secondary };
                    Some(egui::Event::PointerButton { pos, button, pressed, modifiers: raw.modifiers })
                }
                "space" => Some(egui::Event::Key { key: egui::Key::Space, physical_key: None, pressed, repeat: false, modifiers: raw.modifiers }),
                k => egui::Key::from_name(&k.to_uppercase()).map(|key| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: raw.modifiers }),
            };
            if name == "shift" {
                shift |= !up;
            } else if !*down && !up {
                raw.events.extend(event(true));
            } else if *down && up {
                raw.events.extend(event(false));
            }
            *down = true;
        }
        raw.modifiers.shift |= shift;
        self.held.retain(|(_, until, _)| now < *until);
    }

    /// Holds keys (w, a, s, d, r, f, q, e, space, shift...) and mouse buttons
    /// (lmb, rmb) for `seconds`, as if pressed: one already held is held
    /// longer (pressed once).
    pub(super) fn hold(&mut self, keys: &[String], seconds: f32) {
        let until = self.now() + seconds.clamp(0.0, 600.0) as f64;
        for k in keys {
            let k = k.to_lowercase();
            match self.held.iter_mut().find(|h| h.0 == k) {
                Some(h) => h.1 = h.1.max(until),
                None => self.held.push((k, until, false)),
            }
        }
    }
}

impl eframe::App for App {
    /// Tab and Shift+Tab are game mode and the panels, so they are taken out
    /// of the input before egui sees them: egui moves the keyboard focus with
    /// them, and with nothing focused it focused the first field, after which
    /// the keys went to that field and Tab did nothing. Left to egui while a
    /// text field is being typed in (except in game mode, which has none).
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        // RAY_TEST_TAB=n,m,...: Tab presses at those frames, through this same
        // path (tests)
        if std::env::var("RAY_TEST_TAB").is_ok_and(|v| v.split(',').any(|f| f.trim().parse() == Ok(self.frames))) {
            raw.events.push(egui::Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
        }
        self.inject_held(raw);
        // RAY_TEST_HOLD=key:from-to,...: keys held over those frames (w, a,
        // s, d, r, f, space, shift; lmb, rmb: the mouse buttons), through
        // this same path (tests)
        if let Ok(spec) = std::env::var("RAY_TEST_HOLD") {
            for part in spec.split(',') {
                let Some((name, range)) = part.split_once(':') else { continue };
                let Some((a, b)) = range.split_once('-') else { continue };
                let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) else { continue };
                let key = match name {
                    "lmb" | "rmb" => {
                        let button = if name == "lmb" { egui::PointerButton::Primary } else { egui::PointerButton::Secondary };
                        for (at, pressed) in [(a, true), (b, false)] {
                            if at == self.frames {
                                let pos = egui::pos2(400.0, 300.0);
                                raw.events.push(egui::Event::PointerButton { pos, button, pressed, modifiers: raw.modifiers });
                            }
                        }
                        continue;
                    }
                    "space" => egui::Key::Space,
                    "shift" => {
                        raw.modifiers.shift = (a..b).contains(&self.frames);
                        continue;
                    }
                    k => match egui::Key::from_name(&k.to_uppercase()) {
                        Some(k) => k,
                        None => continue,
                    },
                };
                for (at, pressed) in [(a, true), (b, false)] {
                    if at == self.frames {
                        raw.events.push(egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: raw.modifiers });
                    }
                }
            }
        }
        if self.game.is_none() && ctx.wants_keyboard_input() {
            return;
        }
        raw.events.retain(|e| match e {
            egui::Event::Key { key: egui::Key::Tab, pressed, repeat, modifiers, .. } if !(modifiers.ctrl || modifiers.alt || modifiers.command) => {
                if *pressed && !*repeat {
                    self.tab = Some(modifiers.shift);
                }
                false
            }
            _ => true,
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let mut dt = (now - self.last).as_secs_f32().min(0.1);
        self.last = now;
        self.frame_ms = if self.frame_ms == 0.0 { dt * 1000.0 } else { self.frame_ms * 0.9 + dt * 100.0 };
        self.frames += 1;

        if self.last_check.elapsed().as_secs_f32() > 0.5 {
            self.last_check = Instant::now();
            let m = mtime(&self.path);
            if m.is_some() && m != self.mtime {
                if self.doc.dirty {
                    self.changed_on_disk = true;
                } else if self.config.auto_reload {
                    self.reload(false);
                }
            }
        }

        if let Some(out) = self.render.take() {
            self.shown = Some(out.image.clone());
            self.shown_serial = out.serial;
            self.gpu_state = out.gpu.clone();
            let image = egui::ImageData::Color(out.image);
            match &mut self.texture {
                Some(t) => t.set(image, egui::TextureOptions::LINEAR),
                None => {
                    eprintln!("ray: first frame shown {} ms after start", since_start());
                    self.texture = Some(ctx.load_texture("frame", image, egui::TextureOptions::LINEAR));
                }
            }
            self.stats = out.stats;
            self.prices = out.prices;
            if out.stats.traced && out.scene_id == self.scene_id {
                self.costs = out.costs;
            }
            if out.stats.traced {
                self.perf.frame(out.stats.trace_ms + out.stats.post_ms);
            }
        }
        self.perf.tick();

        // the preferences: the interface's size, and any change saved
        // (when the button is let go: changing the size under a slider being
        // dragged moved the slider, and the value jumped)
        if (ctx.zoom_factor() - self.config.ui_scale).abs() > 1e-3 && !ctx.input(|i| i.pointer.any_down()) {
            ctx.set_zoom_factor(self.config.ui_scale.clamp(0.5, 3.0));
        }
        // (saved once a drag ends: a slider rewrote the file every frame)
        if self.config != self.config_saved && !ctx.input(|i| i.pointer.any_down()) {
            self.config_saved = self.config.clone();
            // (said too: from a File menu checkbox, Preferences is not open)
            match self.config.save() {
                Ok(()) => self.config_error = None,
                Err(e) => {
                    self.say(format!("preferences not saved: {e}"));
                    self.config_error = Some(e);
                }
            }
        }
        let ppp = ctx.pixels_per_point();
        let screen = ctx.content_rect();
        self.window_px = [(screen.width() * ppp).round() as usize, (screen.height() * ppp).round() as usize];
        // a steady recording: the scene moves 1/fps once this frame is in
        // the video, and holds still meanwhile
        if let Some(step) = self.record_frame(ctx, ppp) {
            dt = if step { 1.0 / self.config.video_fps.clamp(5, 120) as f32 } else { 0.0 };
        }
        self.glide_step();
        self.window_shot(ctx);
        // Tab / Shift+Tab (raw_input_hook)
        if let Some(shift) = self.tab.take() {
            self.run_act(if shift { Act::Panels } else { Act::Game });
        }
        let playing = self.game.is_some();
        if playing {
            // no field keeps the keys while the interface is gone
            if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        } else {
            self.interface(ctx);
        }

        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(egui::Color32::BLACK)).show(ctx, |ui| {
            let rect = ui.max_rect();
            self.view_rect = rect;
            let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
            if let Some(tex) = &self.texture {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                // (a steady recording's frames are of their own shape: whole,
                // not stretched to the view's)
                let size = tex.size_vec2();
                let at = if self.steady.is_some() { egui::Rect::from_center_size(rect.center(), size * (rect.width() / size.x).min(rect.height() / size.y)) } else { rect };
                ui.painter().image(tex.id(), at, uv, egui::Color32::WHITE);
            }
            if playing {
                self.game_frame(ctx, ui, dt);
            } else {
                self.view_input(ctx, &resp, dt);
                self.draw_handles(ui);
                self.error_banner(ui);
                self.charge_gauge(ui);
                if self.heat {
                    self.heat_legend(ui, rect);
                } else if self.settings.mode == Mode::Cost {
                    self.cost_legend(ui, rect);
                }
            }
            let ppp = ctx.pixels_per_point();
            let scale = self.settings.scale.clamp(0.05, 2.0);
            self.render_size = render_size(rect.width() * ppp * scale, rect.height() * ppp * scale);
            self.view_px = (rect.width() * ppp, rect.height() * ppp);
        });
        // out of game mode: the pointer is free again
        if self.game.is_none() && self.game_grab {
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorGrab(egui::CursorGrab::None));
            ctx.send_viewport_cmd(egui::ViewportCommand::CursorVisible(true));
            self.game_grab = false;
        }
        self.keys(ctx, dt);
        self.serve_remote();
        self.emit_step(dt);
        self.physics_step(dt);
        self.compile();
        self.post_job();
        // (a screenshot of the window on its way: the GPU is the window's)
        let shooting = self.window_shot.is_some() || self.test_shot.as_ref().is_some_and(|(_, n)| self.frames >= *n);
        self.render.pause(shooting);
        self.finish_shots();
        self.finish_stills();
        self.test_hook(ctx);
        self.had_focus = ctx.memory(|m| m.focused().is_some());
        // RAY_TEST_GAME=1: into game mode once the view has a size (tests)
        if self.frames == 3 && std::env::var_os("RAY_TEST_GAME").is_some() {
            self.enter_game();
        }
        // RAY_TEST_RECORD=n,m: F10 at those frames, without the microphone
        // unless RAY_TEST_MIC=1 (tests)
        if std::env::var("RAY_TEST_RECORD").is_ok_and(|v| v.split(',').any(|f| f.trim().parse() == Ok(self.frames))) {
            self.test_mic = Some(std::env::var_os("RAY_TEST_MIC").is_some());
            self.run_act(Act::Record);
        }
        // RAY_TEST_SCREENSHOT=n: P at frame n (tests)
        if std::env::var("RAY_TEST_SCREENSHOT").is_ok_and(|v| v.trim().parse() == Ok(self.frames)) {
            self.run_act(Act::Screenshot);
        }
        // (a steady recording is of the view)
        if self.recorder.is_some() {
            if self.steady.is_none() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(record::Tag)));
            }
            ctx.request_repaint();
        }

        // keep drawing while something moves; an accumulating image wakes the
        // window itself (the render thread asks for a frame with each image),
        // so it is not redrawn at the display's rate for nothing meanwhile
        let water_moves = self.waves_moving() || (self.scene.water.is_some() && !self.world.ripples.is_empty());
        if self.gesture.is_some() || self.game.is_some() || self.recorder.is_some() || !self.glides.is_empty() || !self.world.bodies.is_empty() || water_moves {
            ctx.request_repaint();
        } else if self.twinkling() {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        } else if self.emitting() {
            // (emissions keep pace with the frames: they must keep coming)
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
    }

}

impl App {
    /// While recording: this frame's screenshot of the window (asked for last
    /// frame) to the recorder, with the pointer where it shows; the time in
    /// the title (outside the window, so not in the video). And a finished
    /// recording's file, once it is written. A steady recording's frame:
    /// whether the scene moves on now (None: not steady).
    fn record_frame(&mut self, ctx: &egui::Context, ppp: f32) -> Option<bool> {
        // the encoder gave up by itself: say why now, not when F10 is pressed
        if self.recorder.as_ref().is_some_and(record::Recorder::failed)
            && let Some(r) = self.recorder.take()
        {
            self.record_done = r.stop();
            self.end_steady();
        }
        let mut step = None;
        // a screenshot rendering whole: the scene holds still meanwhile
        if self.recorder.is_none() && self.still_frame() {
            step = Some(false);
        }
        if let Some(r) = &self.recorder {
            let shot = ctx.input(|i| {
                i.events.iter().find_map(|e| match e {
                    egui::Event::Screenshot { image, user_data, .. } if user_data.data.as_ref().is_some_and(|d| d.downcast_ref::<record::Tag>().is_some()) => Some(image.clone()),
                    _ => None,
                })
            });
            // (hidden while looking around, panning and in game mode)
            let hidden = self.game.is_some() || matches!(self.gesture, Some(Gesture::Look | Gesture::Pan(_)));
            let at = if hidden { None } else { ctx.input(|i| i.pointer.hover_pos()) };
            match &mut self.steady {
                None => {
                    if let Some(image) = shot {
                        r.frame(image, at, ppp);
                    }
                }
                // the view, once the image shown is of the last job, whole
                Some(s) => {
                    let drawn = self.shown_serial == self.posted && (self.stats.samples >= s.wants(self.settings.max_samples) || !self.stats.traced);
                    let image = self.shown.clone().filter(|_| drawn);
                    let stepped = s.tick(image.is_some());
                    if let Some(image) = image {
                        r.frame(image, None, 1.0);
                    }
                    step = Some(stepped);
                }
            }
        }
        // a steady recording of so many frames: ended once it has them
        if self.steady.as_ref().is_some_and(record::Steady::done) {
            self.stop_recording();
        }
        // the title: the scene (● with unsaved edits), or while recording the
        // time and where it goes (outside the window: not in the video)
        let title = match &self.recorder {
            Some(r) => {
                let mic = if r.mic { " + microphone" } else { "" };
                let (time, steady) = match &self.steady {
                    Some(s) => (s.frames as f32 / s.fps as f32, format!(" steady, {} frames", s.frames)),
                    None => (r.started.elapsed().as_secs() as f32, String::new()),
                };
                format!("● REC {}{steady}  → {}{mic}  (F10 stops) — ray", record::clock(time), r.path.display())
            }
            None => {
                let name = self.path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
                format!("{name}{} — ray", if self.doc.dirty { " ●" } else { "" })
            }
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
        if self.record_done.as_ref().is_some_and(|h| h.is_finished()) {
            let done = self.record_done.take().map(|h| h.join().unwrap_or_else(|_| Err("the recorder failed".into())));
            match done {
                Some(Ok(path)) => {
                    let path = std::fs::canonicalize(&path).unwrap_or_else(|_| PathBuf::from(path));
                    self.last_video = Some((path, Instant::now()));
                }
                Some(Err(e)) => {
                    self.say(format!("recording failed: {e}"));
                    self.record_error = Some(e);
                }
                None => {}
            }
        }
        step
    }

    /// Starts a recording: of the window as drawn, or (`steady`) of the view,
    /// a whole frame at a time at the steady recording's size. Its file.
    /// (The title says it records, and where: a note in the window would be
    /// in the video.)
    fn start_recording(&mut self, steady: Option<record::Steady>, quality: u32) -> Result<PathBuf, String> {
        if self.still.take().is_some() {
            self.end_steady();
            self.last_still = Some(Err("a recording started".into()));
        }
        let r = record::Recorder::start(
            &config::expand(&self.config.videos),
            steady.as_ref().map_or(self.window_px, |s| s.size),
            self.test_mic.unwrap_or(self.config.microphone),
            steady.as_ref().map_or(self.config.video_fps, |s| s.fps),
            quality,
            steady.is_some(),
        )?;
        let path = r.path.clone();
        self.recorder = Some(r);
        self.steady = steady;
        self.record_error = None;
        Ok(path)
    }

    /// Ends the recording (its file is finished in the background).
    fn stop_recording(&mut self) {
        if let Some(r) = self.recorder.take() {
            self.say(format!("finishing {}…", r.path.display()));
            self.record_done = r.stop();
            self.end_steady();
        }
    }

    /// A steady recording over: the scene's clock goes on from where it left
    /// it.
    fn end_steady(&mut self) {
        if let Some(s) = self.steady.take() {
            self.clock_behind = self.clock.elapsed().as_secs_f64() - s.t;
        }
    }

    /// The menu, the panels and the windows (all of it but the view; none
    /// of it in game mode).
    fn interface(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu(ui));
        if self.show_perf {
            egui::TopBottomPanel::bottom("perf").resizable(true).default_height(150.0).min_height(90.0).show(ctx, |ui| {
                self.perf.ui(ui);
            });
        }
        if self.show_scene {
            egui::SidePanel::left("scene").resizable(true).default_width(360.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.scene_panel(ui));
            });
        }
        if self.show_render {
            egui::SidePanel::right("render").resizable(true).default_width(310.0).show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.render_panel(ui));
            });
        }
        if self.help {
            self.help_window(ctx);
        }
        self.file_window(ctx);
        self.previews.poll(ctx);
        self.preferences_window(ctx);
        self.library_window(ctx);
        self.modifier_window(ctx);
        if self.show_env {
            let mut open = true;
            // beside the render panel (the view's edge; before the first frame
            // has measured it, the screen's less that panel's width)
            let screen = ctx.content_rect();
            let right = if self.view_rect.width() > 1.0 { self.view_rect.right() } else { screen.right() - 320.0 };
            let at = egui::pos2(right - 8.0, screen.top() + 40.0);
            egui::Window::new("☀ Environment")
                .open(&mut open)
                .pivot(egui::Align2::RIGHT_TOP)
                .default_pos(at)
                .default_width(350.0)
                .default_height(560.0)
                .show(ctx, |ui| egui::ScrollArea::vertical().show(ui, |ui| self.env_inspector(ui)));
            self.show_env = open;
        }
    }
}

#[cfg(test)]
mod tests {
    /// Screenshots within one second do not overwrite each other.
    #[test]
    fn screenshots_are_named_apart() {
        let dir = std::env::temp_dir().join(format!("ray-test-{}-shots", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let first = super::shot_path(&dir, "bonsai", "2026-10-29-1200-00");
        assert_eq!(first, dir.join("bonsai-2026-10-29-1200-00.png"));
        std::fs::write(&first, b"x").unwrap();
        let second = super::shot_path(&dir, "bonsai", "2026-10-29-1200-00");
        assert_eq!(second, dir.join("bonsai-2026-10-29-1200-00-2.png"));
        std::fs::write(&second, b"x").unwrap();
        assert_eq!(super::shot_path(&dir, "bonsai", "2026-10-29-1200-00"), dir.join("bonsai-2026-10-29-1200-00-3.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A screenshot carries its moment: the scene (the biggest saved one
    /// here, a megabyte of JSON) comes back exactly, ahead of the pixels;
    /// the Open window lists it; its scene file is the one it was taken of;
    /// a picture with no scene in it does not open.
    #[test]
    fn screenshots_carry_the_scene() {
        let dir = std::env::temp_dir().join(format!("ray-test-{}-stills", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let scene: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("saves/bonsai.json").unwrap()).unwrap();
        let moment = serde_json::json!({"source": "/somewhere/bonsai.json", "objects": 7, "game": {"feet": [1.0, 2.0, 3.0], "height": 0.0042}});
        let (s, m) = (scene.to_string(), moment.to_string());
        let png = dir.join("bonsai-shot.png");
        let rgba: Vec<u8> = (0..4 * 6 * 4).map(|i| i as u8).collect();
        crate::render::write_png_text(&png, 6, 4, &rgba, &[(crate::scene::STILL_SCENE, &s, true), (crate::scene::STILL_MOMENT, &m, true), ("Description", "ray: a test", false)]).unwrap();
        assert!(std::fs::metadata(&png).unwrap().len() < s.len() as u64 / 3, "the scene is compressed");
        let (v, back) = crate::scene::read_doc(&png).unwrap();
        assert_eq!((v, back.clone()), (scene, Some(moment)));
        assert_eq!(super::still_source(&png, back.as_ref().unwrap()), std::path::PathBuf::from("/somewhere/bonsai.json"));
        assert_eq!(super::still_entry(&png), Some((7, "screenshot of bonsai.json, the walker 4.20 mm tall".to_string())));
        // (the pixels as they were)
        let mut r = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&png).unwrap())).read_info().unwrap();
        let mut px = vec![0; r.output_buffer_size().unwrap()];
        r.next_frame(&mut px).unwrap();
        assert_eq!(px, rgba);
        let plain = dir.join("plain.png");
        crate::render::write_png(&plain, 6, 4, &rgba).unwrap();
        assert!(crate::scene::read_doc(&plain).unwrap_err().contains("no scene"));
        assert_eq!(super::still_entry(&plain), None);
        assert!(crate::scene::read_doc(std::path::Path::new("saves/bonsai.json")).unwrap().1.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Open window's descriptions: the first sentence, cut short with
    /// an ellipsis when long.
    #[test]
    fn first_sentences() {
        assert_eq!(super::first_sentence("A box. With a lid.", 110), "A box.");
        assert_eq!(super::first_sentence("No full stop here", 110), "No full stop here");
        let cut = super::first_sentence(&"word ".repeat(40), 20);
        assert!(cut.ends_with('…') && cut.chars().count() <= 21, "{cut}");
    }

    #[test]
    fn icon_decodes() {
        let icon = super::icon().expect("assets/icon.png decodes to RGBA");
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }
}
