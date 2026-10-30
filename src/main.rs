//! ray: real-time ray tracing of mesh-free scenes. A JSON scene of signed
//! distance shapes, fractals, heightfields and analytic primitives, traced on
//! the CPU (AVX2, all cores; the renderer of fraktal256-rs, generalised) in
//! an interactive viewer with a settings panel, or headless to PNG.
//!
//!   ray scene.json                      the viewer
//!   ray scene.json --out shot.png       render headless (see --help)

mod camera;
mod gpu;
mod math;
mod mcp;
mod noise;
mod pool;
#[cfg(feature = "profile")]
mod prof;
mod render;
mod scene;
mod settings;
mod simd;
mod ui;

use camera::Camera;
use render::Renderer;
use scene::Scene;
use settings::{Mode, Settings};
use std::path::PathBuf;
use std::time::Instant;

const HELP: &str = "\
ray: real-time ray tracing of mesh-free scenes

usage:
  ray [SCENE.json]                    open the viewer (default: saves/showcase.json)
  ray mcp                             an MCP server (stdin / stdout) that drives the
                                      running viewer: see the README (MCP)
  ray SCENE.json --out FILE.png [options]
                                      render headless and write a PNG

headless options:
  --size WxH          image size (default 1280x720)
  --frames N          frames to accumulate (default 1)
  --spp N             samples per pixel per frame (default: the scene's, else 1)
  --mode M            direct | path_trace (pt) | normals | albedo | depth | steps | cost
  --bounces N         reflection / GI bounces
  --camera x,y,z,yaw,pitch   camera position and angles (degrees)
  --set JSON          render settings, e.g. '{\"ao\": false, \"max_steps\": 96}'
  --bench             print the time of every frame
  --device D          gpu+cpu sharing (both, the default), gpu or cpu
  --time T            the water's clock (its waves as at T seconds) and the stars' twinkle (RAY_TEST_RIPPLES=
                      x,z,age,height;... adds ripples)
  --drop N[,H]        drop object N from H metres (default 2) above where it is, let
                      the physics settle it, then render (tests the drag-and-drop physics)
";

fn main() {
    if !(std::arch::is_x86_feature_detected!("avx2") && std::arch::is_x86_feature_detected!("fma")) {
        eprintln!("ray needs an x86-64 CPU with AVX2 and FMA (x86-64-v3)");
        std::process::exit(1);
    }
    if let Err(e) = run() {
        eprintln!("ray: {e}");
        std::process::exit(1);
    }
}

struct Args {
    scene: PathBuf,
    out: Option<PathBuf>,
    size: (usize, usize),
    frames: u32,
    spp: Option<u32>,
    mode: Option<Mode>,
    bounces: Option<u32>,
    camera: Option<[f32; 5]>,
    set: Option<serde_json::Value>,
    bench: bool,
    drop: Option<(usize, f32)>,
    time: f32,
    device: Option<settings::Device>,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        scene: PathBuf::from("saves/showcase.json"),
        out: None,
        size: (1280, 720),
        frames: 1,
        spp: None,
        mode: None,
        bounces: None,
        camera: None,
        set: None,
        bench: false,
        drop: None,
        time: 0.0,
        device: None,
    };
    let mut it = std::env::args().skip(1);
    let value = |it: &mut std::iter::Skip<std::env::Args>, name: &str| it.next().ok_or(format!("{name} needs a value"));
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                std::process::exit(0);
            }
            "--out" | "-o" => a.out = Some(PathBuf::from(value(&mut it, "--out")?)),
            "--size" => {
                let v = value(&mut it, "--size")?;
                let (w, h) = v.split_once('x').ok_or("--size takes WxH")?;
                a.size = (w.parse().map_err(|_| "bad width")?, h.parse().map_err(|_| "bad height")?);
            }
            "--frames" => a.frames = value(&mut it, "--frames")?.parse().map_err(|_| "bad --frames")?,
            "--spp" => a.spp = Some(value(&mut it, "--spp")?.parse().map_err(|_| "bad --spp")?),
            "--bounces" => a.bounces = Some(value(&mut it, "--bounces")?.parse().map_err(|_| "bad --bounces")?),
            "--mode" => {
                a.mode = Some(match value(&mut it, "--mode")?.as_str() {
                    "direct" => Mode::Direct,
                    "pt" | "path" | "path_trace" => Mode::PathTrace,
                    "normals" => Mode::Normals,
                    "albedo" => Mode::Albedo,
                    "depth" => Mode::Depth,
                    "steps" => Mode::Steps,
                    "cost" => Mode::Cost,
                    m => return Err(format!("unknown mode '{m}' (direct, path_trace, normals, albedo, depth, steps or cost)")),
                })
            }
            "--camera" => {
                let v: Vec<f32> = value(&mut it, "--camera")?
                    .split(',')
                    .map(|s| s.trim().parse::<f32>().map_err(|_| "bad --camera".to_string()))
                    .collect::<Result<_, _>>()?;
                if v.len() != 5 {
                    return Err("--camera takes x,y,z,yaw,pitch".into());
                }
                a.camera = Some([v[0], v[1], v[2], v[3], v[4]]);
            }
            "--set" => {
                a.set = Some(serde_json::from_str(&value(&mut it, "--set")?).map_err(|e| format!("--set: {e}"))?)
            }
            "--bench" => a.bench = true,
            "--device" => {
                a.device = Some(match value(&mut it, "--device")?.as_str() {
                    "cpu" => settings::Device::Cpu,
                    "gpu" => settings::Device::Gpu,
                    "both" | "gpu+cpu" => settings::Device::Both,
                    d => return Err(format!("unknown device '{d}' (both, gpu, cpu)")),
                })
            }
            "--time" => a.time = value(&mut it, "--time")?.parse().map_err(|_| "bad --time")?,
            "--drop" => {
                let v = value(&mut it, "--drop")?;
                let (n, h) = v.split_once(',').unwrap_or((&v, "2"));
                a.drop = Some((n.parse().map_err(|_| "bad --drop")?, h.parse().map_err(|_| "bad --drop height")?));
            }
            s if s.starts_with('-') => return Err(format!("unknown option {s} (see --help)")),
            s => a.scene = PathBuf::from(s),
        }
    }
    Ok(a)
}

/// The scene's render settings, then `--set` over them.
pub fn initial_settings(scene: &Scene, set: Option<&serde_json::Value>) -> Result<Settings, String> {
    let mut v = serde_json::to_value(Settings::default()).map_err(|e| e.to_string())?;
    for layer in [scene.render.as_ref(), set].into_iter().flatten() {
        let obj = layer.as_object().ok_or("render settings must be a JSON object")?;
        for (k, x) in obj {
            v[k] = x.clone();
        }
    }
    Settings::from_json(&v)
}

pub fn start_camera(scene: &Scene) -> Camera {
    let c = scene.camera;
    Camera { fine: c.fine, ..Camera::new(c.pos, c.yaw, c.pitch, c.fov) }
}

/// Drops object `n` from `h` metres above its place and simulates until
/// everything rests (what it hits moves too); returns the scene as it ends up.
fn drop_test(path: &std::path::Path, n: usize, h: f32) -> Result<Scene, String> {
    use ui::physics::World;
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut doc = ui::doc::Doc::new(serde_json::from_str(&text).map_err(|e| e.to_string())?);
    let mut cache = scene::TerrainCache::default();
    doc.node(n, &[]).ok_or("--drop: no such object")?;
    let mut body = ui::physics::body_of(&doc.v, n).ok_or("--drop: that object cannot move")?;
    body.x.y += h;
    // RAY_DROP_VEL=x,y,z: a throw instead of a drop
    if let Ok(v) = std::env::var("RAY_DROP_VEL") {
        let c: Vec<f32> = v.split(',').filter_map(|c| c.trim().parse().ok()).collect();
        if c.len() == 3 {
            body.v = math::Vec3::new(c[0], c[1], c[2]);
        }
    }
    let mut world = World::default();
    world.bodies.push(body);
    // RAY_DROP_FPS=n: the frame rate the physics steps at (default 60)
    let frame_dt = 1.0 / std::env::var("RAY_DROP_FPS").ok().and_then(|v| v.parse::<f32>().ok()).unwrap_or(60.0);
    // RAY_DROP_ALL=1: every other movable object falls too (Drop all)
    if std::env::var_os("RAY_DROP_ALL").is_some() {
        let first = scene::Scene::from_value(&doc.v, &mut cache)?;
        for o in &first.objects {
            if !o.locked
                && o.doc_index != n
                && world.find(o.doc_index).is_none()
                && let Some(b) = ui::physics::body_of(&doc.v, o.doc_index)
            {
                world.bodies.push(b);
            }
        }
    }
    let mut ripples = 0;
    let s = Settings::default();
    let t0 = Instant::now();
    let mut frames = 0;
    let mut woke = std::collections::BTreeSet::new();
    let mut scene = Scene::from_value(&doc.v, &mut cache)?;
    while !world.bodies.is_empty() && (frames as f32) * frame_dt < 60.0 {
        for b in &mut world.bodies {
            b.obj = scene.objects.iter().position(|o| o.doc_index == b.top).map(|i| i as u32);
        }
        let mut wake = |o: u32| {
            let ob = scene.objects.get(o as usize)?;
            let b = ui::physics::body_of(&doc.v, ob.doc_index)?;
            woke.insert(ob.doc_index);
            Some(b)
        };
        let before = world.ripples.len();
        world.time = frames as f32 * frame_dt;
        // RAY_DROP_WAVES=1: the waves move
        if std::env::var_os("RAY_DROP_WAVES").is_some() {
            world.waves_moving = true;
            world.wave_time = world.time * scene.wave_speed;
        }
        world.step(&scene, &s, frame_dt, &mut wake);
        ripples += world.ripples.len().saturating_sub(before);
        if let Some(t) = std::env::var("RAY_DROP_TRACE").ok().and_then(|v| v.parse::<usize>().ok())
            && frames % 6 == 0
            && let Some(b) = world.bodies.iter().find(|b| b.top == t)
        {
            let (pos, rot) = b.placement();
            eprintln!(
                "t {:.2}  pos [{:.3} {:.3} {:.3}]  rot [{:.1} {:.1} {:.1}]  v {:.3}  w [{:.2} {:.2} {:.2}]  rest {:.2}",
                frames as f32 * frame_dt, pos[0], pos[1], pos[2], rot[0], rot[1], rot[2], b.v.length(), b.w.x, b.w.y, b.w.z, b.rest
            );
        }
        for b in &world.bodies {
            let (pos, rot) = b.placement();
            let node = doc.node_mut(b.top, &[]).unwrap();
            node["position"] = ui::doc::vec3(pos.map(|v| v as f64));
            node["rotation"] = ui::doc::vec3(rot.map(|v| v as f64));
        }
        for b in world.bodies.iter().filter(|b| b.asleep()) {
            let (pos, rot) = b.placement();
            println!(
                "drop: object {} rests at [{:.3}, {:.3}, {:.3}], rotation [{:.1}, {:.1}, {:.1}] after {:.2} s",
                b.top, pos[0], pos[1], pos[2], rot[0], rot[1], rot[2], frames as f32 * frame_dt
            );
        }
        world.bodies.retain(|b| !b.asleep());
        scene = Scene::from_value(&doc.v, &mut cache)?;
        frames += 1;
    }
    println!(
        "drop: {frames} frames simulated in {:.0} ms wall; objects set moving by the impact: {:?}; ripples made: {ripples}",
        t0.elapsed().as_secs_f32() * 1000.0,
        woke
    );
    Ok(scene)
}

fn run() -> Result<(), String> {
    #[cfg(feature = "profile")]
    let prof = prof::start();
    let r = run_args();
    #[cfg(feature = "profile")]
    if let Some(p) = prof {
        p.finish();
    }
    r
}

fn run_args() -> Result<(), String> {
    if std::env::args().nth(1).as_deref() == Some("mcp") {
        return mcp::run();
    }
    let args = parse_args()?;
    if args.out.is_none() {
        return ui::run(args.scene);
    }
    #[cfg(feature = "profile")]
    let mut heap = prof::heap();
    let t = Instant::now();
    let mut scene = Scene::load(&args.scene)?;
    let load_ms = t.elapsed().as_secs_f32() * 1000.0;
    #[cfg(feature = "profile")]
    println!("heap after loading: {}", prof::heap_since(&mut heap));
    if let Some((n, h)) = args.drop {
        scene = drop_test(&args.scene, n, h)?;
    }
    let mut s = initial_settings(&scene, args.set.as_ref())?;
    if let Some(m) = args.mode {
        s.mode = m;
    }
    if let Some(n) = args.spp {
        s.spp = n;
    }
    if let Some(b) = args.bounces {
        s.bounces = b;
    }
    s.max_samples = s.max_samples.max(args.frames * s.spp);
    let mut cam = start_camera(&scene);
    if let Some(c) = args.camera {
        cam.place(math::Vec3::new(c[0], c[1], c[2]));
        cam.yaw = c[3].to_radians();
        cam.pitch = c[4].to_radians();
    }
    let st = &scene.stats;
    println!(
        "scene {}: {} objects ({} analytic, {} nurbs with {} patches, {} sdf with {} ops / {} leaves, {} terrain), bvh {} nodes, {} unbounded; loaded in {:.0} ms",
        args.scene.display(),
        st.objects,
        st.analytic,
        st.nurbs,
        st.patches,
        st.sdf,
        st.sdf_ops,
        st.leaves,
        st.terrains,
        st.bvh_nodes,
        st.unbounded,
        load_ms
    );
    if let Some(d) = args.device {
        s.device = d;
    }
    let (w, h) = args.size;
    let mut r = Renderer::new();
    if s.device != settings::Device::Cpu {
        match gpu::Gpu::headless() {
            Ok(g) => {
                println!("GPU: {}", g.name);
                r.gpu = Some(g);
            }
            Err(e) => println!("GPU unavailable ({e}); CPU only"),
        }
    }
    let mut waves = render::Waves { time: args.time, wave_time: args.time * scene.wave_speed, ripples: Vec::new() };
    if let Ok(v) = std::env::var("RAY_TEST_RIPPLES") {
        for r in v.split(';') {
            let c: Vec<f32> = r.split(',').filter_map(|c| c.trim().parse().ok()).collect();
            if let [x, z, age, amp] = c[..] {
                waves.ripples.push(render::Ripple { x, z, t0: args.time - age, amp });
            }
        }
    }
    let mut times = Vec::new();
    #[cfg(feature = "profile")]
    println!("heap before the first frame: {}", prof::heap_since(&mut heap));
    for f in 0..args.frames.max(1) {
        // --time also sets the stars' twinkle
        let twinkle = (args.time != 0.0).then_some(args.time);
        r.render(&scene, &cam, &s, w, h, 0, &render::View { profile: args.bench, twinkle, ..Default::default() }, &waves);
        let fs = r.stats;
        times.push(fs.trace_ms + fs.post_ms);
        if args.bench {
            println!(
                "frame {f}: trace {:.1} ms (GPU {:.0}% of rows in {:.1} ms, CPU {:.1} ms), post {:.1} ms, {:.1} Mrays/s",
                fs.trace_ms,
                fs.gpu_share * 100.0,
                fs.gpu_ms,
                fs.cpu_ms,
                fs.post_ms,
                fs.mrays
            );
            #[cfg(feature = "profile")]
            println!("  heap: {}", prof::heap_since(&mut heap));
        }
    }
    if let Some(g) = &r.gpu {
        match &g.error {
            Some(e) => println!("GPU error: {e}"),
            None => println!("GPU shader: {} program structures, compiled in {:.0} ms", g.structures, g.compile_ms),
        }
    }
    let frame = r.frame.as_ref().unwrap();
    let out = args.out.unwrap();
    frame.write_png(&out)?;
    times.sort_by(f32::total_cmp);
    println!(
        "{}x{} {:?} on {}, {} spp: median frame {:.1} ms ({} threads, GPU {:.0}% of rows); {:.1} steps/ray; wrote {}",
        w,
        h,
        s.mode,
        s.device.label(),
        r.stats.samples,
        times[times.len() / 2],
        rayon::current_num_threads(),
        r.stats.gpu_share * 100.0,
        r.stats.steps,
        out.display()
    );
    Ok(())
}
