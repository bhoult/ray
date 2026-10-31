//! The CPU renderer: a tile pass over the frame on all cores (16 x 16 tiles,
//! eight pixels of a row at a time, as in fraktal256), progressive
//! accumulation while nothing changes, then the post passes (bloom, edge AA,
//! tone mapping, the cost heat map, the selection outline) into 8-bit pixels.
//!
//! Every frame also measures what it cost: cycles per pixel (the heat map) and
//! cycles and evaluations per object (the cost table).

pub mod atmos;
pub mod nurbs;
pub mod pattern;
mod post;
pub mod shade;
pub mod trace;
pub mod vm;
pub mod volume;

use crate::camera::Camera;
use crate::gpu::{self, Gpu};
use crate::math::Vec3;
use crate::pool::Shared;
use crate::scene::{Env, Scene};
use crate::settings::{Device, Mode, Settings};
use crate::simd::{F8, I8, M8, V8};
use rayon::prelude::*;
use shade::{Rng8, lowbias, unit};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
pub use vm::ObjCost;

pub const TILE: usize = 16;
/// The object id of a pixel where the sky was seen through a cloud (-1:
/// clear sky).
pub const CLOUDED_SKY: i32 = -2;

/// Rows of a tile per CPU job: small, so that one costly tile (a fractal
/// seen through glass, say) is shared by several threads instead of holding
/// up the frame on one.
const JOB_ROWS: usize = 2;

/// What every shader of a frame reads.
pub struct Ctx<'a> {
    pub scene: &'a Scene,
    pub env: &'a Env,
    pub s: &'a Settings,
    pub cam: &'a Camera,
    /// hit threshold per metre: the pixel footprint times `detail`
    pub cone: f32,
    /// count cycles per object
    pub profile: bool,
    pub waves: &'a Waves,
    /// per object, the log of the detail its fractals resolve this frame
    /// (fractal_detail; none: their own iteration counts)
    pub detail: &'a [f32],
    /// where the headlight is this frame, from the camera (shade::lamp)
    pub lamp: Vec3,
    /// the frame's anchor, the camera's position: rays carry their offset
    /// from it (Ray.od), surfaces theirs (Surf.dp)
    pub anchor: Vec3,
    /// per object, its anchor table this frame (anchors; empty: none)
    pub tables: &'a [Vec<f32>],
    /// the cost view: per object, its price on the colour ramp (0 cheapest,
    /// 1 dearest; empty outside that view)
    pub prices: &'a [f32],
}

impl Ctx<'_> {
    /// Readies `regs` to evaluate object `i`: its detail and anchor table.
    #[inline(always)]
    pub fn prepare(&self, regs: &mut vm::Regs, i: usize) {
        regs.ln_thr = self.lod(i);
        // SAFETY: the frame's tables outlive this Ctx and so the evaluations
        // made with it; every object's evaluation comes through here first
        unsafe { regs.set_table(self.tables.get(i).map_or(&[], Vec::as_slice)) };
    }

    /// Regs.ln_thr for evaluating object `i`.
    #[inline(always)]
    pub fn lod(&self, i: usize) -> f32 {
        self.detail.get(i).copied().unwrap_or(f32::INFINITY)
    }
}

/// The detail each object's fractals resolve this frame (the log of a
/// threshold in world metres; infinite for the rest): two pixels at the
/// camera's distance from its nearest surface. One value for every ray,
/// normal, shadow and AO sample of the frame: iterations that changed along
/// a ray made walls of the tunnels the camera was in (a finer level's hole
/// is solid at the coarser ones).
fn fractal_detail(scene: &Scene, cam: &Camera, s: &Settings, cone: f32, tables: &[Vec<f32>]) -> Vec<f32> {
    let mut out = vec![f32::INFINITY; scene.objects.len()];
    if !s.fractal_detail {
        return out;
    }
    let mut regs = vm::Regs::new();
    // (at the camera, `fine` from the anchor: exact where it has a table)
    regs.dp = V8::splat(cam.fine);
    for (i, o) in scene.objects.iter().enumerate() {
        let crate::scene::Shape::Sdf(prog) = &o.shape else { continue };
        if !prog.lod {
            continue;
        }
        let (lo, hi, p) = (o.aabb.min, o.aabb.max, cam.pos);
        let fine = trace::grain((p - o.org).length());
        let inside = p.x >= lo.x && p.y >= lo.y && p.z >= lo.z && p.x <= hi.x && p.y <= hi.y && p.z <= hi.z;
        let d = if inside {
            // (its finest: the camera may be in a tunnel only that has)
            regs.ln_thr = -1e30;
            // SAFETY: `tables` outlives this function's evaluations
            unsafe { regs.set_table(tables.get(i).map_or(&[], Vec::as_slice)) };
            vm::eval(prog, V8::splat(p - o.org), &mut regs, false).0.lane(0).abs()
        } else {
            (lo - p).max(p - hi).max(Vec3::ZERO).length()
        };
        let fine = if prog.anchored && tables.get(i).is_some_and(|t| !t.is_empty()) { 1.0e-12 } else { fine };
        out[i] = (d * cone).max(fine).ln();
    }
    out
}

/// What each object's shape costs: nanoseconds per evaluation of its
/// distance at eight points in its box (around the camera for unbounded
/// ones), at the frame's fractal detail and anchoring, the best of three
/// runs (Mode::Cost paints them). NURBS are measured exactly, as rays pay.
pub fn prices(ctx: &Ctx) -> Vec<f32> {
    const RUNS: usize = 3;
    const EVALS: usize = 24;
    let mut regs = vm::Regs::new();
    let cam = ctx.cam.pos;
    ctx.scene
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let (lo, hi) = (o.aabb.min, o.aabb.max);
            let finite = lo.is_finite() && hi.is_finite() && (hi - lo).max_comp() < 1e6;
            let (lo, hi) = if finite { (lo, hi) } else { (cam - Vec3::splat(20.0), cam + Vec3::splat(20.0)) };
            // eight points per evaluation, spread through the box
            let at = |k: usize| {
                let u = |c: u32| F8::from_array(std::array::from_fn(|l| ((k * 8 + l) as u32).wrapping_mul(0x9e3779b9).wrapping_add(c).wrapping_mul(0x85ebca6b) as f32 / u32::MAX as f32));
                let (x, y, z) = (u(1), u(2), u(3));
                V8::new(x.mul_add(F8::splat(hi.x - lo.x), F8::splat(lo.x)), y.mul_add(F8::splat(hi.y - lo.y), F8::splat(lo.y)), z.mul_add(F8::splat(hi.z - lo.z), F8::splat(lo.z)))
            };
            let pts: Vec<V8> = (0..EVALS).map(at).collect();
            let mut best = f64::MAX;
            for _ in 0..RUNS {
                let t0 = std::time::Instant::now();
                for p in &pts {
                    let d = trace::object_dist(ctx, &mut regs, i, *p, *p - V8::splat(ctx.anchor), F8::splat(trace::BIG), M8::none_set(), false);
                    std::hint::black_box(d);
                }
                best = best.min(t0.elapsed().as_nanos() as f64 / EVALS as f64);
            }
            best as f32
        })
        .collect()
}

/// Where the cost view's ramp starts: the cheapest objects a dark blue, not
/// the near-black of empty space.
pub const PRICE_RAMP_START: f32 = 0.15;

/// Prices on the colour ramp: logarithmic from the cheapest object to the
/// dearest.
fn price_ramp(prices: &[f32]) -> Vec<f32> {
    let lo = prices.iter().copied().filter(|p| *p > 0.0).fold(f32::INFINITY, f32::min).max(0.1);
    let hi = prices.iter().copied().fold(0.0f32, f32::max).max(lo * 1.0001);
    prices.iter().map(|p| ((p.max(lo) / lo).ln() / (hi / lo).ln()).clamp(0.0, 1.0).mul_add(1.0 - PRICE_RAMP_START, PRICE_RAMP_START)).collect()
}

/// Each object's anchor table this frame (sdf::Program::anchors): its
/// anchored ops worked out in f64 at the camera, so that the frame's samples
/// only add their offset from it, which f32 holds finely near the camera.
/// Only for objects the camera is in (or nearly): from outside, the pixels
/// are coarser than f32 anyway.
fn anchors(scene: &Scene, cam: &Camera) -> Vec<Vec<f32>> {
    use crate::scene::sdf::AnchorKind;
    scene
        .objects
        .iter()
        .map(|o| {
            let crate::scene::Shape::Sdf(prog) = &o.shape else { return Vec::new() };
            let (lo, hi, p) = (o.aabb.min, o.aabb.max, cam.pos);
            let m = (hi - lo) * 0.05;
            let (lo, hi) = (lo - m, hi + m);
            let near = p.x >= lo.x && p.y >= lo.y && p.z >= lo.z && p.x <= hi.x && p.y <= hi.y && p.z <= hi.z;
            if prog.anchors.is_empty() || !near {
                return Vec::new();
            }
            let k0 = [p.x as f64 - o.org.x as f64, p.y as f64 - o.org.y as f64, p.z as f64 - o.org.z as f64];
            let mut t = vec![0.0f32; prog.table_len];
            for a in &prog.anchors {
                let k = a.xf.apply(k0);
                let e = &mut t[a.at as usize..];
                match a.kind {
                    AnchorKind::Menger => menger_entry(&a.xf, k, e),
                    // the normal in the object's frame, and the anchor's distance
                    AnchorKind::Slice { n, off } => {
                        for c in 0..3 {
                            e[c] = (0..3).map(|r| a.xf.a[r][c] * n[r] as f64).sum::<f64>() as f32;
                        }
                        e[3] = ((0..3).map(|r| n[r] as f64 * k[r]).sum::<f64>() - off as f64) as f32;
                    }
                }
            }
            t
        })
        .collect()
}

/// A Menger sponge's anchor table entry (vm.rs menger_anchored) for the
/// anchor at `k` in its frame: the map from the object's frame, the box from
/// the anchor, and per level and axis the straight stretch of the fold the
/// anchor is on (the fold minus 1 there, its slope, the offsets it holds
/// between) and the anchor's residue.
fn menger_entry(xf: &crate::scene::sdf::Xf, k: [f64; 3], e: &mut [f32]) {
    for r in 0..3 {
        for c in 0..3 {
            e[r * 3 + c] = xf.a[r][c] as f32;
        }
    }
    for i in 0..3 {
        e[9 + i] = (k[i].abs() - 1.0) as f32;
        e[12 + i] = if k[i] < 0.0 { -1.0 } else { 1.0 };
    }
    for lvl in 0..crate::scene::sdf::MENGER_LEVELS {
        let half_s = 0.5 * 3f64.powi(lvl as i32);
        for i in 0..3 {
            let x = k[i] * half_s;
            let a0 = x - x.floor();
            // |1 - 3 |2a - 1|| - 1 is straight between 0, 1/3, 1/2, 2/3 and 1
            let (lo, hi, g0, slope) = if a0 < 1.0 / 3.0 {
                (0.0, 1.0 / 3.0, 1.0 - 6.0 * a0, -6.0)
            } else if a0 < 0.5 {
                (1.0 / 3.0, 0.5, 6.0 * a0 - 3.0, 6.0)
            } else if a0 < 2.0 / 3.0 {
                (0.5, 2.0 / 3.0, 3.0 - 6.0 * a0, -6.0)
            } else {
                (2.0 / 3.0, 1.0, 6.0 * a0 - 5.0, 6.0)
            };
            let o = 15 + (lvl * 3 + i) * 5;
            e[o..o + 5].copy_from_slice(&[g0 as f32, (slope * half_s) as f32, ((lo - a0) / half_s) as f32, ((hi - a0) / half_s) as f32, a0 as f32]);
        }
    }
}

/// The water's motion at a moment: the clock its ripples are timed by, the
/// clock its waves drift by (0 holds them still), and the ripples spreading
/// from where things moved through the surface.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Waves {
    pub time: f32,
    pub wave_time: f32,
    pub ripples: Vec<Ripple>,
}

/// Still water.
pub const STILL: &Waves = &Waves { time: 0.0, wave_time: 0.0, ripples: Vec::new() };

/// A ring spreading over the water from (`x`, `z`), started at time `t0`,
/// `amp` metres high at first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ripple {
    pub x: f32,
    pub z: f32,
    pub t0: f32,
    pub amp: f32,
}

/// The most ripples on the water at once.
pub const MAX_RIPPLES: usize = 32;
/// A ripple's ring speed (m/s), fading (per second) and wave number (radians per metre).
pub const RIPPLE_SPEED: f32 = 0.9;
pub const RIPPLE_FADE: f32 = 1.3;
pub const RIPPLE_K: f32 = 15.7;
/// How many ring widths from its ring a ripple is left out (its height there
/// is e^-12.25 of its crest's).
pub const RIPPLE_REACH: f32 = 3.5;

impl Ripple {
    /// Its height `age` seconds after it started.
    pub fn height(&self, age: f32) -> f32 {
        self.amp * (-RIPPLE_FADE * age.max(0.0)).exp()
    }

    /// Whether it still shows at `time`.
    pub fn alive(&self, time: f32) -> bool {
        self.height(time - self.t0) > 5e-4
    }
}

/// How a frame is presented (none of this restarts accumulation).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct View {
    /// outline these objects (first, one past the last)
    pub selected: Option<(u32, u32)>,
    /// show the cost heat map instead of the image
    pub heat: bool,
    /// measure each object's cost
    pub profile: bool,
    /// the stars twinkle as at this time (seconds)
    pub twinkle: Option<f32>,
}

/// The frame: accumulated and averaged HDR planes, object ids, cost,
/// and the 8-bit output. Planes are padded to a width that is a multiple of 64
/// and a height that is a multiple of 16 (tiles and bloom octets); only
/// `w` x `h` is shown.
pub struct Frame {
    pub w: usize,
    pub h: usize,
    pub pw: usize,
    pub ph: usize,
    acc: [Vec<f32>; 3],
    hdr: [Vec<f32>; 3],
    /// the object each primary ray hit (-1: sky)
    pub ids: Vec<i32>,
    /// cycles spent on each pixel in the last traced frame
    pub cost: Vec<f32>,
    /// 0xAARRGGBB, `pw` x `ph`
    pub out: Vec<u32>,
    bloom: post::Bloom,
}

impl Frame {
    pub fn new(w: usize, h: usize) -> Frame {
        let (w, h) = (w.max(8), h.max(8));
        let pw = w.next_multiple_of(64);
        let ph = h.next_multiple_of(TILE);
        // eight floats of slack: the composite reads one pixel past the end
        let n = pw * ph + 8;
        let plane = || vec![0.0f32; n];
        Frame {
            w,
            h,
            pw,
            ph,
            acc: [plane(), plane(), plane()],
            hdr: [plane(), plane(), plane()],
            ids: vec![-1; n],
            cost: plane(),
            out: vec![0u32; pw * ph],
            bloom: post::Bloom::new(pw, ph),
        }
    }

    /// The visible pixels as RGBA bytes.
    pub fn rgba(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.w * self.h * 4);
        for row in 0..self.h {
            for &p in &self.out[row * self.pw..][..self.w] {
                v.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8, 255]);
            }
        }
        v
    }

    pub fn write_png(&self, path: &std::path::Path) -> Result<(), String> {
        write_png(path, self.w, self.h, &self.rgba())
    }
}

pub fn write_png(path: &std::path::Path, w: usize, h: usize, rgba: &[u8]) -> Result<(), String> {
    write_png_text(path, w, h, rgba, &[])
}

/// A PNG with text in it: each (keyword, text, compressed) an iTXt chunk,
/// ahead of the pixels (a reader finds them without decoding the image).
pub fn write_png_text(path: &std::path::Path, w: usize, h: usize, rgba: &[u8], text: &[(&str, &str, bool)]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    // fdeflate: some 20 times faster than the default, files a sixth larger
    enc.set_compression(png::Compression::Fast);
    let mut wr = enc.write_header().map_err(|e| e.to_string())?;
    for &(key, t, compressed) in text {
        let mut c = png::text_metadata::ITXtChunk::new(key, t);
        c.compressed = compressed;
        wr.write_text_chunk(&c).map_err(|e| e.to_string())?;
    }
    wr.write_image_data(rgba).map_err(|e| e.to_string())
}

/// The iTXt texts of a PNG under `keys`, in their order (None: not in it);
/// only the chunks ahead of the pixels are read.
pub fn png_text<R: std::io::BufRead + std::io::Seek>(r: R, keys: &[&str]) -> Result<Vec<Option<String>>, String> {
    let reader = png::Decoder::new(r).read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    keys.iter()
        .map(|k| info.utf8_text.iter().find(|c| c.keyword == *k).map(|c| c.get_text().map_err(|e| e.to_string())).transpose())
        .collect()
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    /// time of this frame's trace pass and post pass
    pub trace_ms: f32,
    pub post_ms: f32,
    /// samples per pixel accumulated so far
    pub samples: u32,
    /// march iterations per primary ray
    pub steps: f32,
    /// primary rays per second, millions
    pub mrays: f32,
    /// whether this frame traced (false: converged, only post ran)
    pub traced: bool,
    /// cycles per pixel at the top of the heat map's scale
    pub heat_max: f32,
    /// all cycles the last traced frame spent, over all threads
    pub cycles: u64,
    /// share of the rows the GPU traced, and how long each device took (ms)
    pub gpu_share: f32,
    pub gpu_ms: f32,
    pub cpu_ms: f32,
}

#[derive(Clone, PartialEq)]
struct Key {
    pose: [f32; 9],
    s: Settings,
    env: Env,
    w: usize,
    h: usize,
    scene_id: u64,
    waves: Waves,
    /// the cost heat map or profiling: traced on the CPU alone, timed (an
    /// image the GPU drew has no costs: it is traced again)
    timed: bool,
}

pub struct Renderer {
    pub frame: Option<Frame>,
    key: Option<Key>,
    samples: u32,
    pub stats: FrameStats,
    /// per object (scene order): what the last traced frame spent on it
    pub costs: Vec<ObjCost>,
    /// the cost view: per object, nanoseconds per evaluation of its shape
    /// (prices), measured when the view starts anew
    pub prices: Vec<f32>,
    /// the GPU half, when there is a GPU
    pub gpu: Option<Gpu>,
    /// the share of the bands the GPU gets, balanced so both finish together
    split: f32,
}

impl Default for Renderer {
    fn default() -> Renderer {
        // a GPU usually traces much faster than the CPU: start there and adapt
        Renderer {
            frame: None,
            key: None,
            samples: 0,
            stats: FrameStats::default(),
            costs: Vec::new(),
            prices: Vec::new(),
            gpu: None,
            split: 0.9,
        }
    }
}

/// Bands for the GPU and the CPU: `g` of the `n` to the GPU, spread evenly
/// (every device gets rows from the whole picture).
fn split_bands(n: usize, g: usize) -> (Vec<u32>, Vec<u32>) {
    let (mut gv, mut cv) = (Vec::new(), Vec::new());
    for i in 0..n {
        if (i + 1) * g / n > i * g / n { gv.push(i as u32) } else { cv.push(i as u32) }
    }
    (gv, cv)
}

/// The uniform block of the GPU shader (lib.wgsl's Frame) for this frame.
fn gpu_params(ctx: &Ctx, lay: &gpu::codegen::Layout, tables: &[u32], base: u32, spp: u32, width: usize, rows: usize) -> gpu::FrameParams {
    let c = ctx.cam;
    let e = ctx.env;
    let s = ctx.s;
    let mut w: Vec<u32> = Vec::with_capacity(64);
    let mut v4 = |v: Vec3, x: u32| w.extend([v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), x]);
    v4(c.pos, c.kx.to_bits());
    v4(c.f, c.bx.to_bits());
    v4(c.r, c.ky.to_bits());
    v4(c.u, c.by.to_bits());
    v4(e.sun_dir, e.fog.to_bits());
    v4(e.sun_col, e.clouds.to_bits());
    v4(e.amb_col, e.ambient.to_bits());
    v4(e.sky_zen, ctx.cone.to_bits());
    v4(e.sky_hor, s.max_dist.to_bits());
    v4(e.fog_col, s.ao_radius.to_bits());
    let (wl, wc) = ctx.scene.water.unwrap_or((0.0, Vec3::ZERO));
    v4(wc, wl.to_bits());
    let mode = match s.mode {
        Mode::Direct => 0,
        Mode::PathTrace => 1,
        Mode::Normals => 2,
        Mode::Albedo => 3,
        Mode::Depth => 4,
        Mode::Steps => 5,
        Mode::Cost => 6,
    };
    let flags = s.shadows as u32
        | (s.soft_shadows as u32) << 1
        | (s.ao as u32) << 2
        | (s.lights as u32) << 3
        | (s.light_shadows as u32) << 4
        | (s.fog as u32) << 5
        | (s.clouds as u32) << 6
        | (ctx.scene.water.is_some() as u32) << 7
        | ((e.sky == crate::scene::Sky::Stars) as u32) << 8
        | ((e.sky == crate::scene::Sky::Off) as u32) << 9
        | (!s.fractal_detail as u32) << 10
        | (s.headlight_shadows as u32) << 11;
    w.extend([mode, s.bounces, s.max_steps, flags]);
    w.extend([s.shadow_hardness.to_bits(), spp, base, width as u32]);
    w.extend([lay.n_objects, lay.off_objects, lay.n_nodes, lay.off_nodes]);
    w.extend([lay.off_items, lay.n_unbounded, lay.off_unbounded, lay.off_mats]);
    w.extend([lay.n_lights, lay.off_lights, lay.n_emitters, lay.off_emitters]);
    w.extend([lay.n_terrains, lay.off_terrains, rows as u32, e.daylight.to_bits()]);
    let wv = ctx.waves;
    let ripples = &wv.ripples[..wv.ripples.len().min(MAX_RIPPLES)];
    w.extend([wv.time.to_bits(), wv.wave_time.to_bits(), ripples.len() as u32, ctx.scene.wave_height.to_bits()]);
    for i in 0..MAX_RIPPLES {
        let r = ripples.get(i).map_or([0.0; 4], |r| [r.x, r.z, r.t0, r.amp]);
        w.extend(r.map(f32::to_bits));
    }
    for (v, x) in [(e.glow_dir, e.night.to_bits()), (e.glow_col, e.moon.to_bits()), (e.moon_dir, ctx.s.headlight.to_bits())] {
        w.extend([v.x.to_bits(), v.y.to_bits(), v.z.to_bits(), x]);
    }
    w.extend([lay.n_volumes, lay.off_volumes, 0, 0]);
    // (from the camera: the surfaces' offsets are, Surf.dp)
    let lamp = ctx.lamp;
    w.extend([lamp.x.to_bits(), lamp.y.to_bits(), lamp.z.to_bits(), ctx.s.headlight_reach.max(1e-4).to_bits()]);
    let fine = c.fine;
    w.extend([fine.x.to_bits(), fine.y.to_bits(), fine.z.to_bits(), 0]);
    // each fractal object's detail this frame (lib.wgsl obj_lod), and its
    // anchor table: the header (whether it has one now) and the table
    let mut patches = Vec::new();
    // the cost view: each object's place on the ramp (word 11, the box's
    // spare: lib.wgsl M_COST)
    for (i, t) in ctx.prices.iter().enumerate() {
        patches.push((lay.off_objects + (i * gpu::codegen::OBJ) as u32 + 11, vec![t.to_bits()]));
    }
    for (i, o) in ctx.scene.objects.iter().enumerate() {
        let crate::scene::Shape::Sdf(p) = &o.shape else { continue };
        if p.lod {
            let d = ctx.lod(i);
            patches.push((lay.off_objects + (i * gpu::codegen::OBJ) as u32 + 19, vec![if d.is_finite() { d } else { 3.0e38 }.to_bits()]));
        }
        if let Some(&at) = tables.get(i).filter(|&&at| at != 0) {
            let t = ctx.tables.get(i).map_or(&[][..], Vec::as_slice);
            let mut run = Vec::with_capacity(1 + t.len());
            run.push((!t.is_empty() as u32 as f32).to_bits());
            run.extend(t.iter().map(|v| v.to_bits()));
            patches.push((at, run));
        }
    }
    gpu::FrameParams { words: w, width: width as u32, patches }
}

/// Adds the GPU's samples for its bands into the frame's accumulation.
fn merge(frame: &mut Frame, words: &[u32], bands: &[u32], fresh: bool, inv_n: f32) {
    let pw = frame.pw;
    let [a0, a1, a2] = &mut frame.acc;
    let [h0, h1, h2] = &mut frame.hdr;
    let acc = [Shared::new(a0), Shared::new(a1), Shared::new(a2)];
    let hdr = [Shared::new(h0), Shared::new(h1), Shared::new(h2)];
    let ids = Shared::new(&mut frame.ids);
    let cost = Shared::new(&mut frame.cost);
    bands.par_iter().enumerate().for_each(|(bi, &band)| {
        for r in 0..TILE {
            let row = band as usize * TILE + r;
            let src = &words[(bi * TILE + r) * pw * 2..][..pw * 2];
            let at = row * pw;
            // SAFETY: each band's rows belong to this job only
            unsafe {
                let (ar, ag, ab) = (acc[0].slice(at, pw), acc[1].slice(at, pw), acc[2].slice(at, pw));
                let (hr, hg, hb) = (hdr[0].slice(at, pw), hdr[1].slice(at, pw), hdr[2].slice(at, pw));
                let (id, co) = (ids.slice(at, pw), cost.slice(at, pw));
                for x in 0..pw {
                    let c = gpu::rgb9e5(src[x * 2]);
                    let (r0, g0, b0) = if fresh { (c[0], c[1], c[2]) } else { (ar[x] + c[0], ag[x] + c[1], ab[x] + c[2]) };
                    ar[x] = r0;
                    ag[x] = g0;
                    ab[x] = b0;
                    hr[x] = r0 * inv_n;
                    hg[x] = g0 * inv_n;
                    hb[x] = b0 * inv_n;
                    id[x] = src[x * 2 + 1] as i32;
                    co[x] = 0.0;
                }
            }
        }
    });
}

/// TSC ticks per microsecond, measured once.
pub fn ticks_per_us() -> f64 {
    use std::sync::OnceLock;
    static T: OnceLock<f64> = OnceLock::new();
    *T.get_or_init(|| {
        let (t0, c0) = (Instant::now(), vm::ticks());
        std::thread::sleep(std::time::Duration::from_millis(20));
        let (t1, c1) = (Instant::now(), vm::ticks());
        (c1 - c0) as f64 / ((t1 - t0).as_secs_f64() * 1e6)
    })
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer::default()
    }

    /// Renders `w` x `h` pixels of `scene` from `cam`. `scene_id` changes when
    /// the scene does (a reload, an edit); moving water (`waves`) restarts the
    /// accumulation every frame.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &Scene,
        cam: &Camera,
        s: &Settings,
        w: usize,
        h: usize,
        scene_id: u64,
        view: &View,
        waves: &Waves,
    ) -> &Frame {
        let (w, h) = (w.max(8), h.max(8));
        if self.frame.as_ref().is_none_or(|f| f.w != w || f.h != h) {
            self.frame = Some(Frame::new(w, h));
            self.key = None;
        }
        let mut cam = *cam;
        cam.setup(w as u32, h as u32);
        let key = Key { pose: cam.pose(), s: s.trace_key(), env: scene.env, w, h, scene_id, waves: waves.clone(), timed: view.heat || view.profile };
        let debug = !matches!(s.mode, Mode::Direct | Mode::PathTrace);
        let accumulate = s.accumulate && !debug;
        let fresh = !accumulate || self.key.as_ref() != Some(&key);
        // not accumulating, a frame of an unchanged view is the same frame
        // again (the same jitter and seeds): done once it is traced (the
        // debug views ran CPU and GPU flat out redrawing it)
        let done = !accumulate && self.key.as_ref() == Some(&key) && self.samples > 0;
        if fresh && !done {
            self.samples = 0;
        }
        self.key = Some(key);
        let frame = self.frame.as_mut().unwrap();
        let spp = s.spp.clamp(1, 64);
        let converged = done || (accumulate && self.samples >= s.max_samples.max(1));
        let t0 = Instant::now();
        let mut steps = 0u64;
        let mut cycles = self.stats.cycles;
        let (mut gpu_share, mut gpu_ms, mut cpu_ms) = (self.stats.gpu_share, self.stats.gpu_ms, self.stats.cpu_ms);
        let mut cpu_pixels = (frame.pw * frame.ph) as f32;
        if !converged {
            let cone = cam.pixel_angle * s.detail.max(0.05);
            let tables = anchors(scene, &cam);
            let detail = fractal_detail(scene, &cam, s, cone, &tables);
            let mut ctx = Ctx { scene, env: &scene.env, s, cam: &cam, cone, profile: view.profile, waves, detail: &detail, lamp: Vec3::ZERO, anchor: cam.pos, tables: &tables, prices: &[] };
            ctx.lamp = shade::lamp(&ctx);
            // the cost view: the prices, measured as the view starts anew
            let ramp_at;
            if s.mode == Mode::Cost {
                if self.samples == 0 || self.prices.len() != scene.objects.len() {
                    self.prices = prices(&ctx);
                }
                ramp_at = price_ramp(&self.prices);
                ctx.prices = &ramp_at;
            }
            let base = self.samples;
            let total = base + spp;
            let inv_n = 1.0 / total as f32;
            // who traces which bands (the heat map measures CPU cycles: CPU only)
            let n_bands = frame.ph / TILE;
            let gpu_ok = match (&mut self.gpu, s.device) {
                (Some(g), Device::Gpu | Device::Both) => !view.heat && g.prepare(scene, scene_id),
                _ => false,
            };
            let gpu = if gpu_ok { self.gpu.as_ref() } else { None };
            let n_gpu = match (gpu.is_some(), s.device) {
                (false, _) | (_, Device::Cpu) => 0,
                (true, Device::Gpu) => n_bands,
                // each keeps at least one band, so each keeps being measured
                (true, Device::Both) => ((self.split * n_bands as f32).round() as usize).clamp(1, n_bands.saturating_sub(1).max(1)),
            };
            let (gpu_bands, cpu_bands) = split_bands(n_bands, n_gpu);
            // object costs are CPU cycles: a frame the CPU takes no part in
            // keeps the last ones
            if !cpu_bands.is_empty() {
                self.costs.clear();
            }
            self.costs.resize(scene.objects.len(), ObjCost::default());
            let pending = match gpu {
                Some(g) if !gpu_bands.is_empty() => {
                    let params = gpu_params(&ctx, &g.layout_info, &g.tables, base, spp, frame.pw, gpu_bands.len() * TILE);
                    g_dispatch(&mut self.gpu, &params, &gpu_bands)
                }
                _ => None,
            };
            let gpu = self.gpu.as_ref();
            // the CPU traces its bands while the GPU traces its own
            let (st, cy, c_ms, g_done) = std::thread::scope(|sc| {
                let waiter = match (&pending, gpu) {
                    (Some(p), Some(g)) => Some(sc.spawn(move || g.wait(p))),
                    _ => None,
                };
                let tc = Instant::now();
                let (st, cy) = if cpu_bands.is_empty() {
                    (0, 0)
                } else {
                    trace_pass(&ctx, frame, base, spp, fresh, inv_n, &mut self.costs, &cpu_bands)
                };
                let c_ms = tc.elapsed().as_secs_f32() * 1000.0;
                (st, cy, c_ms, waiter.map(|h| h.join().unwrap()))
            });
            if let (Some(p), Some(g), Some(done)) = (&pending, gpu, g_done) {
                g.read(p, |words| merge(frame, words, &gpu_bands, fresh, inv_n));
                gpu_ms = (done - p.submitted).as_secs_f32() * 1000.0;
                // balance: both should finish together. Steer on finish times
                // rather than rows per millisecond, because the GPU has a fixed
                // cost per frame (submit, copy back) that rates would charge to
                // its rows, starving it on small frames. The share moves in
                // log-odds, a part of the gap each frame: the times swing when
                // other programs are busy
                if s.device == Device::Both && !cpu_bands.is_empty() && gpu_ms > 0.0 && c_ms > 0.0 {
                    let gap = (c_ms - gpu_ms) / (c_ms + gpu_ms);
                    let odds = self.split / (1.0 - self.split) * (0.6 * gap).exp();
                    self.split = (odds / (1.0 + odds)).clamp(0.001, 0.999);
                }
            } else {
                gpu_ms = 0.0;
            }
            gpu_share = gpu_bands.len() as f32 / n_bands.max(1) as f32;
            cpu_ms = c_ms;
            cpu_pixels = (cpu_bands.len() * TILE * frame.pw).max(1) as f32;
            steps = st;
            cycles = cy;
            self.samples = total;
        }
        let trace_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let t1 = Instant::now();
        let pixels = (frame.pw * frame.ph) as f32;
        let heat_max = (cycles as f32 / cpu_pixels * 3.0).max(1.0);
        if debug {
            post::plain(frame, s.mode == Mode::Albedo);
        } else {
            let tw = view.twinkle.filter(|_| s.twinkle).map(|time| post::Twinkle { cam: &cam, env: &scene.env, cone: cam.pixel_angle * s.detail.max(0.05), time });
            post::run(s, frame, s.edge_aa && self.samples < 4, tw.as_ref());
        }
        if view.heat {
            post::heat(frame, heat_max);
        }
        if let Some((a, b)) = view.selected {
            post::outline(frame, a as i32, b as i32);
        }
        let post_ms = t1.elapsed().as_secs_f32() * 1000.0;
        self.stats = FrameStats {
            trace_ms,
            post_ms,
            samples: self.samples,
            steps: if converged { self.stats.steps } else { steps as f32 / cpu_pixels },
            mrays: if converged { 0.0 } else { pixels * spp as f32 / (trace_ms * 1000.0).max(1e-3) },
            traced: !converged,
            heat_max,
            cycles,
            gpu_share,
            gpu_ms,
            cpu_ms,
        };
        frame
    }
}

fn g_dispatch(gpu: &mut Option<Gpu>, params: &gpu::FrameParams, bands: &[u32]) -> Option<gpu::Pending> {
    gpu.as_mut()?.dispatch(params, bands)
}

thread_local! {
    /// Each tracing thread's registers, kept from job to job and frame to
    /// frame (made afresh for every job, they were most of the frame's
    /// allocations). A job drains its costs before it ends.
    static REGS: std::cell::RefCell<Box<vm::Regs>> = std::cell::RefCell::new(vm::Regs::new());
}

/// Planes the jobs write, each only its own pixels.
struct Planes<'a> {
    acc: [Shared<'a, f32>; 3],
    hdr: [Shared<'a, f32>; 3],
    ids: Shared<'a, i32>,
    cost: Shared<'a, f32>,
    pw: usize,
}

/// Traces `spp` samples per pixel into the accumulation (`fresh`: replace
/// it), and the averages into the HDR planes. Returns the march iterations
/// and the cycles spent; adds each object's cost to `costs`.
#[allow(clippy::too_many_arguments)]
fn trace_pass(ctx: &Ctx, frame: &mut Frame, base: u32, spp: u32, fresh: bool, inv_n: f32, costs: &mut [ObjCost], bands: &[u32]) -> (u64, u64) {
    let tiles_x = frame.pw / TILE;
    let per_tile = TILE / JOB_ROWS;
    let jobs: Vec<usize> = bands.iter().flat_map(|&b| b as usize * tiles_x * per_tile..(b as usize + 1) * tiles_x * per_tile).collect();
    let [a0, a1, a2] = &mut frame.acc;
    let [h0, h1, h2] = &mut frame.hdr;
    let planes = Planes {
        acc: [Shared::new(a0), Shared::new(a1), Shared::new(a2)],
        hdr: [Shared::new(h0), Shared::new(h1), Shared::new(h2)],
        ids: Shared::new(&mut frame.ids),
        cost: Shared::new(&mut frame.cost),
        pw: frame.pw,
    };
    let steps = AtomicU64::new(0);
    let cycles = AtomicU64::new(0);
    let sink = Mutex::new(costs);
    jobs.into_par_iter().for_each(|job| {
        REGS.with_borrow_mut(|regs| {
            let (n, c) = render_rows(ctx, regs, &planes, job, tiles_x, base, spp, fresh, inv_n);
            steps.fetch_add(n, Ordering::Relaxed);
            cycles.fetch_add(c, Ordering::Relaxed);
            if ctx.profile {
                regs.drain_costs(&mut sink.lock().unwrap());
            }
        })
    });
    (steps.into_inner(), cycles.into_inner())
}

/// Traces job `job`: `JOB_ROWS` rows of one tile (the tiles in order, each
/// top to bottom).
#[allow(clippy::too_many_arguments)]
fn render_rows(
    ctx: &Ctx,
    regs: &mut vm::Regs,
    p: &Planes,
    job: usize,
    tiles_x: usize,
    base: u32,
    spp: u32,
    fresh: bool,
    inv_n: f32,
) -> (u64, u64) {
    let per_tile = TILE / JOB_ROWS;
    let tile = job / per_tile;
    let x0 = tile % tiles_x * TILE;
    let y0 = tile / tiles_x * TILE + job % per_tile * JOB_ROWS;
    let o = V8::splat(ctx.cam.pos);
    let mut steps = 0u64;
    let mut cycles = 0u64;
    for row in y0..y0 + JOB_ROWS {
        for col in (x0..x0 + TILE).step_by(8) {
            let c0 = vm::ticks();
            let i = row * p.pw + col;
            let pix = I8::splat(i as i32) + I8::iota();
            let mut sum = V8::zero();
            let mut st = I8::zero();
            let mut ids = I8::splat(-1);
            for k in 0..spp {
                let sample = base + k;
                let (jx, jy) = jitter(sample, pix);
                regs.thr_k = thr_jitter(sample, pix);
                let d = primary_rays(ctx.cam, row, col, jx, jy);
                let smp = match ctx.s.mode {
                    Mode::Direct => {
                        let mut rng = Rng8::new(pix, sample);
                        shade::direct(ctx, regs, &mut rng, o, d, ctx.cone)
                    }
                    Mode::PathTrace => {
                        let mut rng = Rng8::new(pix, sample);
                        shade::path(ctx, regs, &mut rng, o, d, ctx.cone)
                    }
                    m => shade::debug(ctx, regs, o, d, ctx.cone, m),
                };
                // NaN and firefly guard
                let clean = |c: F8| F8::select(c.is_nan(), F8::zero(), c).clamp(0.0, 256.0);
                sum += V8::new(clean(smp.col.x), clean(smp.col.y), clean(smp.col.z));
                if k == 0 {
                    st = smp.steps;
                    ids = smp.obj;
                }
            }
            let c = [sum.x, sum.y, sum.z];
            for ch in 0..3 {
                // SAFETY: pixels [i, i + 8) belong to this job only
                unsafe {
                    let acc = p.acc[ch].slice(i, 8);
                    let total = if fresh { c[ch] } else { F8::load(acc) + c[ch] };
                    total.store(acc);
                    (total * inv_n).store(p.hdr[ch].slice(i, 8));
                }
            }
            let spent = vm::ticks() - c0;
            cycles += spent;
            // SAFETY: as above
            unsafe {
                p.ids.slice(i, 8).copy_from_slice(&ids.to_array());
                p.cost.slice(i, 8).fill(spent as f32 / 8.0);
            }
            steps += st.to_array().iter().map(|&v| v.max(0) as u64).sum::<u64>();
        }
    }
    (steps, cycles)
}

/// Sub-pixel offsets in [-0.5, 0.5) for sample `sample` of the pixels `pix`:
/// the R2 sequence, shifted per pixel (Cranley-Patterson). Sample 0 is the
/// pixel centre.
#[inline(always)]
/// A sample's factor on the hit threshold, a quarter to 1.75 (1 for the
/// first sample, so that moving views stay steady): holes and gaps about as
/// wide as a pixel are found by some samples and not others, so they fade in
/// with the distance instead of all vanishing at one (a Menger sponge's holes
/// of one size drew a sharp circle around the camera). A hash of the pixel
/// and the sample, outside the random streams (lib.wgsl thr_jitter).
fn thr_jitter(sample: u32, pix: I8) -> F8 {
    if sample == 0 {
        return F8::one();
    }
    let h = lowbias(pix.mul_lo(I8::splat(0x2c1b_3c6d)) ^ I8::splat((sample.wrapping_mul(0x297a_2d39) ^ 0x68e3_1da4) as i32));
    unit(h).mul_add(F8::splat(1.5), F8::splat(0.25))
}

fn jitter(sample: u32, pix: I8) -> (F8, F8) {
    if sample == 0 {
        return (F8::zero(), F8::zero());
    }
    let k = sample as f32;
    let a = (0.5 + k * 0.754_877_7).fract();
    let b = (0.5 + k * 0.569_840_3).fract();
    let h = lowbias(pix);
    let u = (unit(h) + a).fract() - 0.5;
    let v = (unit(lowbias(h ^ 0x68e3_1da4)) + b).fract() - 0.5;
    (u, v)
}

/// Unit camera rays through pixels (col .. col + 7, row), offset by the
/// jitter.
#[inline(always)]
pub fn primary_rays(cam: &Camera, row: usize, col: usize, jx: F8, jy: F8) -> V8 {
    let v = (jy + row as f32).mul_add(F8::splat(cam.ky), F8::splat(cam.by));
    let px = (F8::iota() + col as f32 + jx).mul_add(F8::splat(cam.kx), F8::splat(cam.bx));
    let d = V8::splat(cam.r) * px + V8::splat(cam.u) * v + cam.f;
    d.normalize()
}

/// The sequential (one hue, dark to light) ramp of the cost views, in sRGB:
/// near zero recedes into the dark, the most expensive pixels are near white.
pub const RAMP: [[f32; 3]; 8] = [
    [0.043, 0.059, 0.098],
    [0.051, 0.212, 0.420],
    [0.094, 0.310, 0.584],
    [0.145, 0.416, 0.749],
    [0.224, 0.529, 0.898],
    [0.427, 0.655, 0.925],
    [0.718, 0.827, 0.965],
    [0.949, 0.973, 1.000],
];

/// The ramp at `x` in [0, 1], eight values at once.
pub fn ramp(x: F8) -> V8 {
    let n = (RAMP.len() - 1) as f32;
    let f = x.saturate() * n;
    let i = f.floor().min(F8::splat(n - 1.0));
    let t = f - i;
    let idx = i.to_i8();
    let mut out = V8::zero();
    for k in 0..RAMP.len() - 1 {
        let m: M8 = idx.eq(I8::splat(k as i32));
        if m.none() {
            continue;
        }
        let (a, b) = (Vec3::from_array(RAMP[k]), Vec3::from_array(RAMP[k + 1]));
        out = V8::select(m, V8::lerp(V8::splat(a), V8::splat(b), t), out);
    }
    out
}

/// The ramp at `x` as 8-bit sRGB (for legends).
pub fn ramp_rgb(x: f32) -> [u8; 3] {
    let c = ramp(F8::splat(x)).lane(0);
    [(c.x * 255.0) as u8, (c.y * 255.0) as u8, (c.z * 255.0) as u8]
}

/// What a click hit.
#[derive(Clone, Copy, Debug)]
pub struct Pick {
    pub object: u32,
    pub node: u32,
    pub point: Vec3,
    pub normal: Vec3,
    pub t: f32,
}

/// The nearest object along a ray (lane 0 of an octet).
pub fn pick_ray(scene: &Scene, cam: &Camera, s: &Settings, o: Vec3, d: Vec3) -> Option<Pick> {
    // (the fractals at the detail the frame shows: at their own iterations
    // the camera deep in a sponge's tunnel was inside solid, the pick hit at
    // once, and the wheel and WASD, which step by its distance, stood still)
    let tables = anchors(scene, cam);
    let detail = fractal_detail(scene, cam, s, cam.pixel_angle * s.detail.max(0.05), &tables);
    let ctx = Ctx { scene, env: &scene.env, s, cam, cone: cam.pixel_angle * 0.5, profile: false, waves: STILL, detail: &detail, lamp: Vec3::ZERO, anchor: cam.pos, tables: &tables, prices: &[] };
    let mut regs = vm::Regs::new();
    // (from the camera: `fine` from the anchor, Camera::fine)
    let od = o - cam.pos + if o == cam.pos { cam.fine } else { Vec3::ZERO };
    let ray = trace::Ray::new(V8::splat(o), V8::splat(od), V8::splat(d), I8::splat(-1));
    let lane0 = M8::from_bits(1);
    let hit = trace::intersect(&ctx, &mut regs, &ray, F8::splat(s.max_dist), lane0, ctx.cone);
    let obj = hit.obj.lane(0);
    if obj < 0 {
        return None;
    }
    let sf = shade::surface(&ctx, &mut regs, &ray, &hit, lane0, ctx.cone);
    Some(Pick { object: obj as u32, node: sf.node.lane(0) as u32, point: sf.p.lane(0), normal: sf.n.lane(0), t: hit.t.lane(0) })
}

/// A Ctx for queries as the view from `cam` (set up for the view) shows the
/// scene: fractals at the frame's detail and anchored at the camera, as
/// pick_ray sees them (game mode's walker collides with what is drawn).
pub fn view_query<R>(scene: &Scene, cam: &Camera, s: &Settings, f: impl FnOnce(&Ctx, &mut vm::Regs) -> R) -> R {
    let tables = anchors(scene, cam);
    let detail = fractal_detail(scene, cam, s, cam.pixel_angle * s.detail.max(0.05), &tables);
    let ctx = Ctx { scene, env: &scene.env, s, cam, cone: cam.pixel_angle * 0.5, profile: false, waves: STILL, detail: &detail, lamp: Vec3::ZERO, anchor: cam.pos, tables: &tables, prices: &[] };
    let mut regs = vm::Regs::new();
    f(&ctx, &mut regs)
}

/// A Ctx for queries outside rendering (physics).
pub fn query<R>(scene: &Scene, s: &Settings, f: impl FnOnce(&Ctx, &mut vm::Regs) -> R) -> R {
    let cam = Camera::default();
    let ctx = Ctx { scene, env: &scene.env, s, cam: &cam, cone: 0.001, profile: false, waves: STILL, detail: &[], lamp: Vec3::ZERO, anchor: cam.pos, tables: &[], prices: &[] };
    let mut regs = vm::Regs::new();
    f(&ctx, &mut regs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::TerrainCache;

    fn scene(v: serde_json::Value) -> Scene {
        Scene::from_value(&v, &mut TerrainCache::default()).unwrap()
    }

    /// A frame of an unchanged view is traced once and then left (the debug
    /// views, and any view without accumulation, used to redraw it flat out);
    /// the heat map asks for a timed CPU trace, so turning it on traces again;
    /// accumulating, it traces until it has its samples.
    #[test]
    fn idle_when_nothing_changes() {
        let sc = scene(serde_json::json!({ "objects": [{ "type": "plane" }, { "type": "sphere", "position": [0, 1, 0] }] }));
        let cam = Camera::new(Vec3::new(0.0, 1.5, 5.0), 0.0, -0.2, 50.0);
        let mut s = Settings { device: Device::Cpu, mode: Mode::Normals, ..Settings::default() };
        let (plain, heat) = (View::default(), View { heat: true, ..View::default() });
        let still = Waves::default();
        let mut r = Renderer::new();
        let traced = |r: &mut Renderer, s: &Settings, v: &View| {
            r.render(&sc, &cam, s, 64, 36, 0, v, &still);
            r.stats.traced
        };
        assert!(traced(&mut r, &s, &plain), "the first frame is traced");
        assert!(!traced(&mut r, &s, &plain), "the same again is not");
        assert!(traced(&mut r, &s, &heat), "the heat map traces again (timed)");
        assert!(!traced(&mut r, &s, &heat));
        s.mode = Mode::Direct;
        s.accumulate = false;
        assert!(traced(&mut r, &s, &plain));
        assert!(!traced(&mut r, &s, &plain), "without accumulation, once");
        s.accumulate = true;
        s.max_samples = 3;
        let n = (0..6).filter(|_| traced(&mut r, &s, &plain)).count();
        assert_eq!(n, 3, "accumulating, until max_samples");
    }

    /// The GPU draws what the CPU does: small scenes of every kind of thing
    /// the renderers handle differently, on both, compared as
    /// tools/parity.sh does (more than 1% of the pixels visibly different
    /// fails). Without a GPU it says so and passes.
    #[test]
    fn the_gpu_draws_what_the_cpu_does() {
        let g = match Gpu::headless() {
            Ok(g) => g,
            Err(e) => {
                eprintln!("no GPU ({e}): the GPU is not compared");
                return;
            }
        };
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let ball = serde_json::json!({ "type": "revolve", "profile": [[0, -0.7], [0.7, -0.7, h], [0.7, 0], [0.7, 0.7, h], [0, 0.7]], "degree": 2, "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1] });
        let floor = serde_json::json!({ "type": "plane", "material": { "color": [0.7, 0.7, 0.7], "color2": [0.3, 0.3, 0.3], "texture": "checker", "space": "world" } });
        let glass = serde_json::json!({ "color": [0.9, 0.97, 1.0], "transmission": 1, "ior": 1.5, "roughness": 0 });
        let cases: Vec<(&str, serde_json::Value, Vec3, f32)> = vec![
            ("shapes", serde_json::json!([floor.clone(),
                { "type": "sphere", "radius": 0.6, "position": [-1.5, 0.6, 0], "material": { "color": [0.8, 0.3, 0.2], "roughness": 0.3 } },
                { "type": "box", "size": [1, 1, 1], "position": [0, 0.5, 0], "material": { "color": [0.6, 0.6, 0.6], "metallic": 1, "roughness": 0.1 } },
                { "type": "torus", "radius": 0.5, "thickness": 0.18, "position": [1.5, 0.5, 0], "rotation": [90, 0, 0] },
                { "type": "union", "smooth": 0.3, "position": [0, 0.4, 1.5], "children": [{ "type": "cylinder", "radius": 0.3, "height": 0.8 }, { "type": "sphere", "radius": 0.35, "position": [0.4, 0.2, 0] }] }
            ]), Vec3::new(0.0, 2.0, 5.0), -0.3),
            ("nurbs alone and in glass", serde_json::json!([floor.clone(),
                { "type": "revolve", "position": [-0.9, 0, 0], "scale": 2 },
                { "type": "revolve", "position": [0.9, 0.7, 0], "profile": ball["profile"], "degree": 2, "knots": ball["knots"], "material": glass }
            ]), Vec3::new(0.0, 1.6, 4.0), -0.25),
            ("nurbs in booleans", serde_json::json!([floor.clone(),
                { "type": "difference", "position": [-1, 0.6, 0], "children": [{ "type": "box", "size": [1.2, 1.2, 1.2] }, { "type": "revolve", "position": [0.4, 0.4, 0.4], "profile": ball["profile"], "degree": 2, "knots": ball["knots"] }] },
                { "type": "twist", "amount": 1.5, "position": [1, 0, 0], "child": { "type": "revolve", "scale": 2 } }
            ]), Vec3::new(0.0, 1.8, 4.5), -0.25),
            ("terrain", serde_json::json!([{ "type": "terrain", "size": 120, "resolution": 128, "height": 12, "base": 2 }, { "type": "water", "level": 3, "waves": 0.3 }]),
                Vec3::new(0.0, 14.0, 40.0), -0.3),
            ("under water", serde_json::json!([{ "type": "water", "waves": 0.3 }, { "type": "box", "size": [2, 2, 2], "position": [0, -3, -6] }]),
                Vec3::new(0.0, -1.0, 0.0), -0.2),
            ("fractal", serde_json::json!([floor, { "type": "menger", "iterations": 4, "scale": 1.2, "position": [0, 1.2, 0] }]),
                Vec3::new(1.5, 2.2, 4.0), -0.3),
        ];
        let mut gpu = Renderer::new();
        gpu.gpu = Some(g);
        let mut cpu = Renderer::new();
        let still = Waves::default();
        // (each scene its own id: the GPU keeps what it uploaded by it)
        for (id, (name, objects, eye, pitch)) in cases.into_iter().enumerate() {
            let sc = scene(serde_json::json!({ "environment": { "preset": "golden" }, "objects": objects }));
            assert!(sc.problems.is_empty(), "{name}: {:?}", sc.problems);
            let cam = Camera::new(eye, 0.0, pitch, 55.0);
            let image = |r: &mut Renderer, device: Device| {
                let s = Settings { device, ..Settings::default() };
                for _ in 0..4 {
                    r.render(&sc, &cam, &s, 96, 54, id as u64 + 1, &View::default(), &still);
                }
                r.frame.as_ref().unwrap().rgba()
            };
            let (a, b) = (image(&mut cpu, Device::Cpu), image(&mut gpu, Device::Gpu));
            let px = a.len() / 4;
            let visible = (0..px).filter(|&i| (0..3).any(|c| (a[i * 4 + c] as i32 - b[i * 4 + c] as i32).abs() > 25)).count();
            assert!(visible * 100 <= px, "{name}: {visible} of {px} pixels differ visibly between CPU and GPU");
        }
    }

    /// The Menger sponge in f64 (vm.rs menger, no trap).
    fn menger64(p: [f64; 3], iters: u32) -> f64 {
        let q = p.map(|c| c.abs() - 1.0);
        let inside = q[0].max(q[1]).max(q[2]).min(0.0);
        let mut d = q.map(|c| c.max(0.0)).iter().map(|c| c * c).sum::<f64>().sqrt() + inside;
        let mut s = 1.0;
        for _ in 0..iters {
            let half_s = 0.5 * s;
            s *= 3.0;
            let f = p.map(|c| {
                let a = c * half_s - (c * half_s).floor();
                (1.0 - (a + a - 1.0).abs() * 3.0).abs()
            });
            let cross = f[0].max(f[1]).min(f[1].max(f[2])).min(f[2].max(f[0]));
            d = d.max((cross - 1.0) / s);
        }
        d
    }

    /// Near the anchor, a sponge a metre and more from its centre comes out
    /// as the f64 sponge does, close to its surface: to a small part of its
    /// finest cell, where plain f32 is off by a good part of one.
    #[test]
    fn anchored_menger_matches_f64() {
        let v = serde_json::json!({ "objects": [
            { "type": "menger", "iterations": 5, "position": [6.6, 1.5, -2.2], "rotation": [10, 73, 5], "scale": 0.75 }
        ]});
        let scene = Scene::from_value(&v, &mut TerrainCache::default()).unwrap();
        let o = &scene.objects[0];
        let crate::scene::Shape::Sdf(prog) = &o.shape else { panic!("not a distance program") };
        assert!(prog.anchored && prog.anchors.len() == 1);
        let xf = prog.anchors[0].xf;
        let scale = 0.75;
        let truth = |k: [f64; 3], n: u32| menger64(xf.apply(k), n) * scale;
        let mut rng = 0x9e3779b97f4a7c15u64;
        let mut unit = move || {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            (rng >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
        };
        let mut regs = vm::Regs::new();
        let (mut worst_a, mut worst_p, mut worst_a16) = (0.0f64, 0.0f64, 0.0f64);
        let mut tried = 0;
        for _ in 0..400 {
            // an anchor in the open, about a metre out from the centre, and a
            // surface point in front of it (sphere-traced in f64)
            let a = [unit() * 0.6, unit() * 0.6, unit() * 0.6];
            let mut cam = Camera::default();
            cam.pos = o.org + Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32);
            let a = (cam.pos - o.org).to_array().map(f64::from);
            if truth(a, 16) <= 1e-6 {
                continue;
            }
            let dir = [unit(), unit(), unit()];
            let len = dir.iter().map(|c| c * c).sum::<f64>().sqrt();
            let dir = dir.map(|c| c / len);
            let mut t = 0.0;
            for _ in 0..400 {
                let d = truth(std::array::from_fn(|c| a[c] + dir[c] * t), 16);
                if d < 1e-12 || t > 1e-4 {
                    break;
                }
                t += d;
            }
            if t > 1e-4 {
                continue;
            }
            tried += 1;
            let tables = anchors(&scene, &cam);
            // eight points within 2 µm of the surface point, as offsets from the anchor
            let off: [[f64; 3]; 8] = std::array::from_fn(|_| std::array::from_fn(|c| dir[c] * t + unit() * 2e-6));
            let lane = |c: usize| F8::from_array(std::array::from_fn(|l| off[l][c] as f32));
            let dp = V8::new(lane(0), lane(1), lane(2));
            let p = V8::splat(cam.pos - o.org) + dp;
            for (n, anchored) in [(12, true), (12, false), (16, true)] {
                // the detail that makes n iterations (sdf.rs Lod)
                regs.ln_thr = (n as f32 - 0.5 - (scale as f32).ln() / 3f32.ln()) * -3f32.ln();
                // SAFETY: `tables` outlives the evaluation below
                unsafe { regs.set_table(if anchored { &tables[0] } else { &[] }) };
                regs.dp = dp;
                let (d, _) = vm::eval(prog, p, &mut regs, false);
                for (l, off) in off.iter().enumerate() {
                    let k = std::array::from_fn(|c| a[c] + off[c] as f32 as f64);
                    let err = (d.lane(l) as f64 - truth(k, n)).abs();
                    let w = match (n, anchored) {
                        (12, true) => &mut worst_a,
                        (12, false) => &mut worst_p,
                        _ => &mut worst_a16,
                    };
                    *w = w.max(err);
                }
            }
            if tried == 24 {
                break;
            }
        }
        assert!(tried >= 8, "found only {tried} surfaces");
        // the finest cells: 12 levels 2.8 µm, 16 levels 35 nm
        let (cell12, cell16) = (1.5 / 3f64.powi(12), 1.5 / 3f64.powi(16));
        eprintln!("near the surface: anchored {worst_a:e} m, plain f32 {worst_p:e} m, anchored at 16 levels {worst_a16:e} m");
        assert!(worst_a < cell12 * 1e-3, "anchored off by {worst_a:e} m");
        assert!(worst_a16 < cell16 * 1e-2, "anchored, 16 levels, off by {worst_a16:e} m");
        assert!(worst_p > worst_a * 10.0, "plain f32 was as good: {worst_p:e} m");
    }
}
