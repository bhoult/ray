//! Fractal flames in three dimensions: Scott Draves's flame algorithm (1992)
//! with 3D maps, as Apophysis's 3D hack and JWildfire draw them. A flame is
//! an iterated function system: a few affine maps, each bent by nonlinear
//! variations (a sine, an inversion in the unit sphere, a swirl...), applied
//! one after another, chosen at random by their weights (the chaos game). The
//! points the orbit visits pile up on the system's attractor; each map also
//! pulls a colour coordinate toward its own, so every part of the attractor
//! is coloured by the maps that made it.
//!
//! Drawn in 2D, the flame is the log of how often each pixel was hit. Here
//! the hits go into a grid of voxels instead: the log of each voxel's count
//! is its density and the average palette colour of its hits its colour, and
//! the renderers march through the grid as a volume (render/volume.rs, and
//! lib.wgsl's twin), where it glows, scatters light and casts shadows.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// The variations, in the order their ids are: what each does to the point
/// `t` the affine part made (r = |t|).
pub const VARIATIONS: [(&str, &str); 10] = [
    ("linear", "t itself."),
    ("sinusoidal", "The sine of each coordinate: folds space into a box of side 2, with soft edges."),
    ("spherical", "t / r²: inversion in the unit sphere, turning the attractor inside out (bubbles and shells)."),
    ("swirl", "A turn about the y axis by r² radians: whorls that wind tighter farther out."),
    ("horseshoe", "In the xz plane, the angle about y doubled: horseshoe arcs."),
    ("eyefish", "t · 2 / (r + 1): a fisheye that pulls everything into the ball of radius 2."),
    ("cylinder", "(sin x, y, cos x): wraps x around the y axis."),
    ("bent", "Negative x and z doubled, negative y halved: creases along the coordinate planes."),
    ("julia3d", "The map's `power`-th root, like a Julia set's inverse step: petals and spirals (power 2 to 5)."),
    ("blur", "A random point in the unit ball: fog and fuzz around the rest."),
];

/// One map of the system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct XFormDef {
    /// how often it is chosen, relative to the others
    #[serde(default = "one")]
    pub weight: f32,
    /// 0..1: the palette colour the points it makes are pulled toward
    #[serde(default)]
    pub color: f32,
    /// the affine part's 3 x 3 matrix, as rows
    #[serde(default = "identity")]
    pub matrix: [[f32; 3]; 3],
    /// the affine part's translation
    #[serde(default)]
    pub offset: [f32; 3],
    /// variation name -> weight
    #[serde(default = "linear_only")]
    pub variations: BTreeMap<String, f32>,
    /// julia3d's power
    #[serde(default = "two")]
    pub power: f32,
}

fn one() -> f32 {
    1.0
}
fn two() -> f32 {
    2.0
}
fn identity() -> [[f32; 3]; 3] {
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
}
fn linear_only() -> BTreeMap<String, f32> {
    BTreeMap::from([("linear".to_string(), 1.0)])
}

/// A map ready to apply.
#[derive(Clone, Debug)]
struct XForm {
    m: [[f32; 3]; 3],
    o: [f32; 3],
    vars: Vec<(u8, f32)>,
    color: f32,
    /// how far toward `color` each step pulls (0: symmetry maps leave it)
    speed: f32,
    power: f32,
}

impl XForm {
    fn from_def(d: &XFormDef) -> Result<XForm, String> {
        let mut vars = Vec::new();
        for (name, &w) in &d.variations {
            let id = VARIATIONS.iter().position(|v| v.0 == name.as_str()).ok_or_else(|| {
                let names: Vec<&str> = VARIATIONS.iter().map(|v| v.0).collect();
                format!("unknown variation '{name}' ({})", names.join(", "))
            })?;
            if w != 0.0 {
                vars.push((id as u8, w));
            }
        }
        if !d.weight.is_finite() || d.weight < 0.0 {
            return Err("a transform's weight must be 0 or more".into());
        }
        Ok(XForm { m: d.matrix, o: d.offset, vars, color: d.color.clamp(0.0, 1.0), speed: 0.5, power: if d.power.abs() < 1.0 { 1.0 } else { d.power } })
    }

    /// A pure turn about y by `a` radians that keeps the colour (symmetry).
    fn turn(a: f32) -> XForm {
        let (s, c) = a.sin_cos();
        XForm { m: [[c, 0.0, -s], [0.0, 1.0, 0.0], [s, 0.0, c]], o: [0.0; 3], vars: vec![(0, 1.0)], color: 0.0, speed: 0.0, power: 2.0 }
    }

    #[inline]
    fn apply(&self, p: [f32; 3], rng: &mut Rng) -> [f32; 3] {
        let m = &self.m;
        let t = [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + self.o[0],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + self.o[1],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + self.o[2],
        ];
        let mut out = [0.0f32; 3];
        for &(v, w) in &self.vars {
            let q = variation(v, t, self.power, rng);
            out[0] += w * q[0];
            out[1] += w * q[1];
            out[2] += w * q[2];
        }
        out
    }
}

/// Variation `v` of the point `t` (VARIATIONS).
#[inline]
fn variation(v: u8, t: [f32; 3], power: f32, rng: &mut Rng) -> [f32; 3] {
    let [x, y, z] = t;
    let r2 = x * x + y * y + z * z;
    match v {
        0 => t,
        1 => [x.sin(), y.sin(), z.sin()],
        2 => {
            let k = 1.0 / (r2 + 1e-9);
            [x * k, y * k, z * k]
        }
        3 => {
            let (s, c) = r2.sin_cos();
            [x * c - z * s, y, x * s + z * c]
        }
        4 => {
            let rho = (x * x + z * z).sqrt() + 1e-9;
            [(x - z) * (x + z) / rho, y, 2.0 * x * z / rho]
        }
        5 => {
            let k = 2.0 / (r2.sqrt() + 1.0);
            [x * k, y * k, z * k]
        }
        6 => [x.sin(), y, x.cos()],
        7 => [if x < 0.0 { 2.0 * x } else { x }, if y < 0.0 { 0.5 * y } else { y }, if z < 0.0 { 2.0 * z } else { z }],
        8 => {
            // (JWildfire's julia3D)
            let n = power.abs();
            let zz = z / n;
            let rxy2 = x * x + y * y;
            let r = (rxy2 + zz * zz).powf((1.0 / power - 1.0) * 0.5);
            let r2d = r * rxy2.sqrt();
            let k = (rng.f() * n).floor().min(n - 1.0);
            let a = (y.atan2(x) + std::f32::consts::TAU * k) / power;
            let (s, c) = a.sin_cos();
            [r2d * c, r2d * s, r * zz]
        }
        _ => {
            // a uniform point in the unit ball: a direction, and a radius by its cube root
            let (u, a) = (rng.f() * 2.0 - 1.0, rng.f() * std::f32::consts::TAU);
            let s = (1.0 - u * u).max(0.0).sqrt();
            let rad = rng.f().cbrt();
            [rad * s * a.cos(), rad * u, rad * s * a.sin()]
        }
    }
}

/// A small fast generator (xorshift64*), seeded through splitmix64.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut z = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        Rng((z ^ (z >> 31)) | 1)
    }
    #[inline]
    pub fn u32(&mut self) -> u32 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }
    /// 0 <= f < 1
    #[inline]
    pub fn f(&mut self) -> f32 {
        (self.u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }
}

/// The palettes, by name: colour stops from one end of the colour
/// coordinate to the other.
pub const PALETTES: &[(&str, &[[f32; 3]])] = &[
    ("fire", &[[0.25, 0.02, 0.0], [0.85, 0.12, 0.01], [1.0, 0.45, 0.04], [1.0, 0.78, 0.3], [1.0, 0.97, 0.85]]),
    ("ice", &[[0.04, 0.08, 0.35], [0.1, 0.35, 0.85], [0.3, 0.75, 1.0], [0.75, 0.95, 1.0], [1.0, 1.0, 1.0]]),
    ("nebula", &[[0.25, 0.05, 0.55], [0.75, 0.1, 0.65], [1.0, 0.35, 0.35], [1.0, 0.7, 0.3], [0.6, 0.9, 1.0]]),
    ("aurora", &[[0.0, 0.35, 0.3], [0.05, 0.8, 0.45], [0.4, 1.0, 0.6], [0.45, 0.4, 1.0], [0.85, 0.3, 0.9]]),
    ("sunset", &[[0.3, 0.05, 0.4], [0.8, 0.15, 0.35], [1.0, 0.4, 0.15], [1.0, 0.7, 0.2], [1.0, 0.9, 0.6]]),
    ("gold", &[[0.3, 0.15, 0.04], [0.7, 0.42, 0.1], [1.0, 0.72, 0.25], [1.0, 0.88, 0.55], [1.0, 0.97, 0.88]]),
    ("rainbow", &[[1.0, 0.15, 0.15], [1.0, 0.75, 0.1], [0.3, 1.0, 0.3], [0.2, 0.7, 1.0], [0.7, 0.3, 1.0]]),
    ("white", &[[1.0, 1.0, 1.0], [1.0, 1.0, 1.0]]),
];

/// 256 colours along the stops, as bytes.
fn palette_table(stops: &[[f32; 3]]) -> [[u8; 3]; 256] {
    let mut t = [[0u8; 3]; 256];
    let n = stops.len().max(1);
    for (i, e) in t.iter_mut().enumerate() {
        let x = i as f32 / 255.0 * (n - 1) as f32;
        let k = (x.floor() as usize).min(n.saturating_sub(2));
        let f = if n > 1 { x - k as f32 } else { 0.0 };
        let (a, b) = (stops[k], stops[(k + 1).min(n - 1)]);
        for c in 0..3 {
            e[c] = ((a[c] + (b[c] - a[c]) * f).clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    t
}

/// What a flame node asks for (scene/json.rs ShapeDef::Flame): everything
/// that decides the grid (not where it stands, nor how it glows).
#[derive(Clone, Debug, Serialize)]
pub struct FlameSpec {
    pub preset: String,
    pub seed: u32,
    pub transforms: Option<Vec<XFormDef>>,
    pub final_xform: Option<XFormDef>,
    pub symmetry: u32,
    pub palette: String,
    pub colors: Option<Vec<[f32; 3]>>,
    pub points: f32,
    pub resolution: u32,
    pub gamma: f32,
}

/// The most voxels along the longest side.
pub const MAX_RESOLUTION: u32 = 256;
/// The most points, in millions.
pub const MAX_POINTS: f32 = 400.0;
/// Steps of the orbit thrown away at the start of each run (it has not
/// reached the attractor yet).
const FUSE: usize = 24;
/// Points per run of the orbit (one per job; each run starts afresh).
const RUN: usize = 1 << 16;
/// The share of each side of the box over which the density fades out
/// toward the box's face.
const EDGE_FADE: f32 = 0.08;

/// The grid a flame was drawn into: density and colour per voxel.
pub struct FlameGrid {
    /// voxels along x, y and z, a border of empty ones included (so that
    /// sampling fades to nothing at the box's faces)
    pub n: [u32; 3],
    /// the box's low corner (its border included) in the flame's own
    /// coordinates, centred on the attractor; and the side of a voxel
    pub lo: [f32; 3],
    pub cell: f32,
    /// the box's longest side (the flame's size is set by it)
    pub extent: f32,
    /// per voxel, x fastest: the density 0..1 in the low 16 bits, the
    /// colour as RGB 5:6:5 in the high 16
    pub data: Vec<i32>,
    /// voxels that were hit, and points drawn
    pub filled: usize,
    pub points: u64,
}

impl FlameGrid {
    /// The density 0..1 at `q` (flame coordinates), trilinear (picking).
    pub fn density_at(&self, q: [f32; 3]) -> f32 {
        let g: [f32; 3] = std::array::from_fn(|i| (q[i] - self.lo[i]) / self.cell - 0.5);
        if (0..3).any(|i| !(g[i] >= 0.0 && g[i] <= (self.n[i] - 1) as f32)) {
            return 0.0;
        }
        let i0: [u32; 3] = std::array::from_fn(|i| (g[i].floor() as u32).min(self.n[i] - 2));
        let f: [f32; 3] = std::array::from_fn(|i| g[i] - i0[i] as f32);
        let mut d = 0.0;
        for k in 0..8u32 {
            let (dx, dy, dz) = (k & 1, (k >> 1) & 1, (k >> 2) & 1);
            let w = (if dx == 1 { f[0] } else { 1.0 - f[0] }) * (if dy == 1 { f[1] } else { 1.0 - f[1] }) * (if dz == 1 { f[2] } else { 1.0 - f[2] });
            let idx = (i0[0] + dx) + self.n[0] * ((i0[1] + dy) + self.n[1] * (i0[2] + dz));
            d += w * (self.data[idx as usize] & 0xffff) as f32 * (1.0 / 65535.0);
        }
        d
    }
}

/// The maps, their weights and the final map.
type System = (Vec<XForm>, Vec<f32>, Option<XForm>);

/// The maps of a spec (its own, a preset's, or for preset "random" a random
/// system picked by the seed, with its symmetry's turns added), how much
/// each weighs, and the final map.
fn system(spec: &FlameSpec) -> Result<System, String> {
    let (mut xs, mut weights): (Vec<XForm>, Vec<f32>) = match &spec.transforms {
        Some(t) if !t.is_empty() => (t.iter().map(XForm::from_def).collect::<Result<_, _>>()?, t.iter().map(|x| x.weight).collect()),
        _ if spec.preset == "random" => {
            let xs = random_system(spec.seed);
            let w = random_weights(spec.seed, xs.len());
            (xs, w)
        }
        _ => {
            let p = PRESETS.iter().find(|p| p.0 == spec.preset).ok_or_else(|| {
                let names: Vec<&str> = PRESETS.iter().map(|p| p.0).chain(["random"]).collect();
                format!("unknown flame preset '{}' ({})", spec.preset, names.join(", "))
            })?;
            let defs: Vec<XFormDef> = serde_json::from_str(p.2).map_err(|e| format!("preset {}: {e}", p.0))?;
            (defs.iter().map(XForm::from_def).collect::<Result<_, _>>()?, defs.iter().map(|x| x.weight).collect())
        }
    };
    if xs.iter().all(|x| x.vars.is_empty()) {
        return Err("a flame needs a transform with a variation".into());
    }
    if xs.len() > 200 {
        return Err("a flame takes at most 200 transforms".into());
    }
    if !(weights.iter().sum::<f32>() > 0.0) {
        return Err("a flame's transforms all weigh 0".into());
    }
    // the turns weigh as much as the mean of the maps
    let n = spec.symmetry.clamp(1, 12);
    let mean = weights.iter().sum::<f32>() / weights.len() as f32;
    for k in 1..n {
        xs.push(XForm::turn(std::f32::consts::TAU * k as f32 / n as f32));
        weights.push(mean);
    }
    let fin = spec.final_xform.as_ref().map(XForm::from_def).transpose()?;
    Ok((xs, weights, fin))
}

/// The weights' table: 1024 entries, each a map's index, as many of each as
/// its share of the total weight (flam3's xform_distrib).
fn chooser(xs: &[XForm], weights: &[f32]) -> Vec<u8> {
    let total: f32 = weights.iter().sum();
    let mut t = Vec::with_capacity(1024);
    let mut acc = 0.0;
    let mut k = 0;
    for i in 0..1024 {
        let at = (i as f32 + 0.5) / 1024.0 * total;
        while k + 1 < xs.len() && acc + weights[k] < at {
            acc += weights[k];
            k += 1;
        }
        t.push(k as u8);
    }
    t
}

/// One run of the orbit (number `run` of those from `seed`): `len` points,
/// each (after the final map) handed to `plot` with its colour coordinate.
fn orbit(xs: &[XForm], fin: Option<&XForm>, table: &[u8], seed: u64, run: usize, len: usize, mut plot: impl FnMut([f32; 3], f32)) {
    let mut rng = Rng::new(seed ^ (run as u64).wrapping_mul(0xd1b5_4a32_d192_ed03));
    let mut p = [rng.f() * 2.0 - 1.0, rng.f() * 2.0 - 1.0, rng.f() * 2.0 - 1.0];
    let mut c = rng.f();
    // (steps since the orbit last started: the first FUSE are not drawn)
    let mut warm = 0;
    let mut bad = 0;
    let mut drawn = 0;
    while drawn < len {
        let x = &xs[table[(rng.u32() & 1023) as usize] as usize];
        let q = x.apply(p, &mut rng);
        c += (x.color - c) * x.speed;
        if !(q[0].is_finite() && q[1].is_finite() && q[2].is_finite()) || q[0].abs() + q[1].abs() + q[2].abs() > 1e10 {
            // flown off (a division by nearly 0): start again somewhere
            bad += 1;
            if bad > 64 {
                return;
            }
            p = [rng.f() * 2.0 - 1.0, rng.f() * 2.0 - 1.0, rng.f() * 2.0 - 1.0];
            warm = 0;
            continue;
        }
        p = q;
        warm += 1;
        if warm > FUSE {
            let out = match fin {
                Some(f) => f.apply(p, &mut rng),
                None => p,
            };
            plot(out, c);
            drawn += 1;
        }
    }
}

/// Draws the flame `spec` into a grid on all cores.
pub fn build(spec: &FlameSpec) -> Result<FlameGrid, String> {
    let (xs, weights, fin) = system(spec)?;
    let table = chooser(&xs, &weights);
    let stops: Vec<[f32; 3]> = match &spec.colors {
        Some(c) if !c.is_empty() => c.clone(),
        _ => PALETTES
            .iter()
            .find(|p| p.0 == spec.palette)
            .ok_or_else(|| {
                let names: Vec<&str> = PALETTES.iter().map(|p| p.0).collect();
                format!("unknown palette '{}' ({})", spec.palette, names.join(", "))
            })?
            .1
            .to_vec(),
    };
    let pal = palette_table(&stops);
    let seed = 0x5eed_f1a3_u64;

    // the box: where most of the attractor is (a few stray points do not
    // stretch it), from a first sample of it
    let sample: Vec<[f32; 3]> = (0..16usize)
        .into_par_iter()
        .flat_map_iter(|r| {
            let mut v = Vec::with_capacity(8192);
            orbit(&xs, fin.as_ref(), &table, seed ^ 0xb0c5, r, 8192, |p, _| v.push(p));
            v
        })
        .collect();
    if sample.len() < 1000 {
        return Err("the flame flies off to infinity: no attractor to draw (try other transforms)".into());
    }
    let mut lo = [0.0f32; 3];
    let mut hi = [0.0f32; 3];
    for a in 0..3 {
        let mut c: Vec<f32> = sample.iter().map(|p| p[a]).collect();
        c.sort_unstable_by(f32::total_cmp);
        let at = |f: f32| c[((c.len() - 1) as f32 * f) as usize];
        let (l, h) = (at(0.001), at(0.999));
        // (a margin about as wide as the fade at the faces, so the fade is
        // mostly over the stray points beyond the attractor's bulk)
        let pad = (h - l) * 0.08 + 1e-4;
        (lo[a], hi[a]) = (l - pad, h + pad);
    }
    let extent = (0..3).map(|a| hi[a] - lo[a]).fold(0.0f32, f32::max);
    if !(extent > 1e-6 && extent.is_finite()) {
        return Err("the flame collapses to a point: nothing to draw (try other transforms)".into());
    }
    let res = spec.resolution.clamp(16, MAX_RESOLUTION);
    let cell = extent / res as f32;
    // (an empty voxel on every side)
    let n: [u32; 3] = std::array::from_fn(|a| (((hi[a] - lo[a]) / cell).ceil() as u32).clamp(1, res) + 2);
    let centre: [f32; 3] = std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5);
    let box_lo: [f32; 3] = std::array::from_fn(|a| centre[a] - n[a] as f32 * cell * 0.5);
    let voxels = n[0] as usize * n[1] as usize * n[2] as usize;

    // the chaos game: hits and colour sums per voxel
    let count: Vec<AtomicU32> = (0..voxels).map(|_| AtomicU32::new(0)).collect();
    let rgb: Vec<AtomicU32> = (0..voxels * 3).map(|_| AtomicU32::new(0)).collect();
    let total = (spec.points.clamp(0.01, MAX_POINTS) as f64 * 1e6) as usize;
    let runs = total.div_ceil(RUN);
    let inv = 1.0 / cell;
    (0..runs).into_par_iter().for_each(|r| {
        orbit(&xs, fin.as_ref(), &table, seed, r, RUN, |p, c| {
            let g: [f32; 3] = std::array::from_fn(|a| (p[a] - box_lo[a]) * inv);
            if g[0] >= 1.0 && g[1] >= 1.0 && g[2] >= 1.0 && g[0] < (n[0] - 1) as f32 && g[1] < (n[1] - 1) as f32 && g[2] < (n[2] - 1) as f32 {
                let i = g[0] as usize + n[0] as usize * (g[1] as usize + n[1] as usize * g[2] as usize);
                count[i].fetch_add(1, Ordering::Relaxed);
                let col = pal[(c * 255.0 + 0.5) as usize & 255];
                for k in 0..3 {
                    rgb[i * 3 + k].fetch_add(col[k] as u32, Ordering::Relaxed);
                }
            }
        });
    });

    // density: the log of the count, against a high percentile of the
    // counts (not the most, which a single hot voxel can be), then gamma
    let counts: Vec<u32> = count.iter().map(|c| c.load(Ordering::Relaxed)).collect();
    let mut hit: Vec<u32> = counts.iter().copied().filter(|&c| c > 0).collect();
    let filled = hit.len();
    if filled == 0 {
        return Err("no point of the flame landed in its box".into());
    }
    let k = ((filled - 1) as f64 * 0.995) as usize;
    let top = *hit.select_nth_unstable(k).1 as f32;
    let norm = 1.0 / (1.0 + top).ln();
    let g = 1.0 / spec.gamma.clamp(0.1, 10.0);
    let data: Vec<i32> = counts
        .par_iter()
        .enumerate()
        .map(|(i, &c)| {
            if c == 0 {
                return 0;
            }
            // faded out toward the box's faces, so that the sparse spray the
            // box cuts through ends softly, not in a box's flat sides
            let at = [i % n[0] as usize, i / n[0] as usize % n[1] as usize, i / (n[0] * n[1]) as usize];
            let edge = (0..3).fold(1.0f32, |w, a| {
                let from = at[a].min(n[a] as usize - 1 - at[a]) as f32 / (n[a] as f32 * EDGE_FADE).max(1.0);
                let x = from.min(1.0);
                w * x * x * (3.0 - 2.0 * x)
            });
            let d = ((1.0 + c as f32).ln() * norm).min(1.0).powf(g) * edge;
            let d16 = (d * 65535.0).round() as u32;
            let avg = |k: usize| rgb[i * 3 + k].load(Ordering::Relaxed) as f32 / (c as f32 * 255.0);
            let (r, gg, b) = (avg(0), avg(1), avg(2));
            let c565 = ((r * 31.0).round() as u32) << 11 | ((gg * 63.0).round() as u32) << 5 | (b * 31.0).round() as u32;
            (d16 | c565 << 16) as i32
        })
        .collect();
    Ok(FlameGrid { n, lo: std::array::from_fn(|a| box_lo[a] - centre[a]), cell, extent, data, filled, points: (runs * RUN) as u64 })
}

/// A random system: 2 to 4 maps, each a turn, a squash (so the system
/// contracts, mostly) and an offset, bent by one or two variations. Seeds
/// whose system flies off or collapses are skipped (deterministically).
fn random_system(seed: u32) -> Vec<XForm> {
    for attempt in 0..64u64 {
        let xs = random_try(seed, attempt);
        if plausible(&xs) {
            return xs;
        }
    }
    random_try(seed, 0)
}

fn random_weights(seed: u32, n: usize) -> Vec<f32> {
    let mut rng = Rng::new(seed as u64 ^ 0x77ee);
    (0..n).map(|_| 0.3 + rng.f() * 0.7).collect()
}

/// The variations random flames draw from, and how often (linear most).
const RANDOM_VARS: [(u8, f32); 9] = [(0, 3.0), (1, 1.5), (2, 1.5), (3, 1.0), (4, 0.7), (5, 1.0), (6, 0.4), (7, 0.4), (8, 1.2)];

fn random_try(seed: u32, attempt: u64) -> Vec<XForm> {
    let mut rng = Rng::new((seed as u64) << 8 ^ attempt);
    let n = 2 + (rng.u32() % 3) as usize;
    let total: f32 = RANDOM_VARS.iter().map(|v| v.1).sum();
    let pick = |rng: &mut Rng| {
        let mut x = rng.f() * total;
        for v in RANDOM_VARS {
            if x < v.1 {
                return v.0;
            }
            x -= v.1;
        }
        0
    };
    (0..n)
        .map(|i| {
            // a random turn (axis and angle), then a squash along each axis
            let (u, a) = (rng.f() * 2.0 - 1.0, rng.f() * std::f32::consts::TAU);
            let s = (1.0 - u * u).sqrt();
            let e = [s * a.cos(), u, s * a.sin()];
            let ang = rng.f() * std::f32::consts::TAU;
            let (sn, cs) = ang.sin_cos();
            let t = 1.0 - cs;
            let rot = [
                [t * e[0] * e[0] + cs, t * e[0] * e[1] - sn * e[2], t * e[0] * e[2] + sn * e[1]],
                [t * e[0] * e[1] + sn * e[2], t * e[1] * e[1] + cs, t * e[1] * e[2] - sn * e[0]],
                [t * e[0] * e[2] - sn * e[1], t * e[1] * e[2] + sn * e[0], t * e[2] * e[2] + cs],
            ];
            let sq = [0.3 + rng.f() * 0.6, 0.3 + rng.f() * 0.6, 0.3 + rng.f() * 0.6];
            let m: [[f32; 3]; 3] = std::array::from_fn(|r| std::array::from_fn(|c| rot[r][c] * sq[c]));
            let o = [rng.f() * 1.6 - 0.8, rng.f() * 1.6 - 0.8, rng.f() * 1.6 - 0.8];
            let v1 = pick(&mut rng);
            let mut vars = vec![(v1, 1.0)];
            if rng.f() < 0.5 {
                let v2 = pick(&mut rng);
                if v2 != v1 {
                    let w = 0.2 + rng.f() * 0.6;
                    vars = vec![(v1, 1.0 - w), (v2, w)];
                }
            }
            let power = 2.0 + (rng.u32() % 3) as f32;
            XForm { m, o, vars, color: if n > 1 { i as f32 / (n - 1) as f32 } else { 0.5 }, speed: 0.5, power }
        })
        .collect()
}

/// Whether a system draws something worth drawing: it stays finite, does not
/// shrink to a point or a line, and is neither a speck nor a fog filling its box.
fn plausible(xs: &[XForm]) -> bool {
    let table = chooser(xs, &vec![1.0; xs.len()]);
    let mut pts = Vec::with_capacity(12_000);
    orbit(xs, None, &table, 0xabc, 0, 12_000, |p, _| pts.push(p));
    if pts.len() < 10_000 {
        return false;
    }
    let mut lo = [f32::INFINITY; 3];
    let mut hi = [f32::NEG_INFINITY; 3];
    for p in &pts {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    let ext: [f32; 3] = std::array::from_fn(|a| hi[a] - lo[a]);
    let big = ext.iter().copied().fold(0.0f32, f32::max);
    if !(big > 1e-3 && big < 1e4) || ext.iter().filter(|&&e| e > big * 0.15).count() < 3 {
        return false;
    }
    // occupancy of a coarse grid over the box
    const G: usize = 20;
    let mut cells = vec![false; G * G * G];
    for p in &pts {
        let i: [usize; 3] = std::array::from_fn(|a| (((p[a] - lo[a]) / big * G as f32) as usize).min(G - 1));
        cells[i[0] + G * (i[1] + G * i[2])] = true;
    }
    let filled = cells.iter().filter(|&&c| c).count() as f32 / (G * G * G) as f32;
    (0.012..0.3).contains(&filled)
}

/// The preset flames: name, what it looks like, its maps (JSON, as a node's
/// `transforms`).
pub const PRESETS: &[(&str, &str, &str)] = &[
    (
        "jellyfish",
        "A glassy bell of curved threads trailing tendrils (try palette ice).",
        r#"[
        {"weight": 0.809, "color": 0, "matrix": [[0.483, -0.064, -0.003], [0.057, 0.511, -0.135], [0.015, 0.118, 0.582]], "offset": [-0.699, 0.456, -0.777], "variations": {"cylinder": 1}},
        {"weight": 0.872, "color": 1, "matrix": [[0.207, -0.064, -0.653], [0.029, 0.33, -0.148], [0.274, 0.014, 0.51]], "offset": [0.285, 0.083, 0.201], "variations": {"spherical": 1}}
    ]"#,
    ),
    (
        "wildfire",
        "A tongue of flame curling up out of a smouldering heap (palette fire).",
        r#"[
        {"weight": 0.659, "color": 0, "matrix": [[0.382, 0.022, -0.06], [-0.02, 0.388, 0.048], [0.028, -0.02, 0.872]], "offset": [0.462, 0.636, 0.67], "variations": {"swirl": 0.713, "spherical": 0.287}},
        {"weight": 0.364, "color": 0.333, "matrix": [[0.511, -0.059, -0.641], [-0.197, 0.742, -0.298], [0.508, 0.347, 0.529]], "offset": [-0.662, 0.547, 0.306], "variations": {"swirl": 0.312, "linear": 0.688}},
        {"weight": 0.393, "color": 0.667, "matrix": [[0.448, -0.202, -0.022], [0.129, 0.279, -0.41], [0.177, 0.309, 0.356]], "offset": [0.655, 0.49, 0.035], "variations": {"horseshoe": 1}},
        {"weight": 0.606, "color": 1, "matrix": [[0.767, -0.033, 0.281], [0.136, 0.485, -0.19], [-0.277, 0.148, 0.685]], "offset": [0.07, -0.181, 0.527], "variations": {"linear": 1}}
    ]"#,
    ),
    (
        "seahorse",
        "A curled S of ribbons, like a seahorse's tail (try palette nebula).",
        r#"[
        {"weight": 0.957, "color": 0, "matrix": [[-0.042, -0.675, 0.126], [0.364, -0.183, -0.335], [0.45, 0.084, 0.283]], "offset": [0.405, -0.27, -0.001], "variations": {"linear": 0.741, "spherical": 0.259}},
        {"weight": 0.891, "color": 0.333, "matrix": [[-0.421, 0.358, 0.204], [-0.279, -0.551, 0.121], [0.233, -0.013, 0.514]], "offset": [0.449, 0.796, 0.344], "variations": {"linear": 1}},
        {"weight": 0.755, "color": 0.667, "matrix": [[0.757, 0.056, 0.06], [-0.046, 0.361, -0.122], [-0.181, 0.145, 0.281]], "offset": [-0.309, 0.485, 0.185], "variations": {"linear": 0.524, "julia3d": 0.476}, "power": 2},
        {"weight": 0.852, "color": 1, "matrix": [[-0.516, 0.081, -0.096], [0.122, -0.238, -0.291], [-0.126, -0.563, 0.109]], "offset": [0.156, -0.025, 0.06], "variations": {"cylinder": 1}}
    ]"#,
    ),
    (
        "rosebud",
        "Petals of fine threads furled around a bud (try palette sunset).",
        r#"[
        {"weight": 0.955, "color": 0, "matrix": [[0.048, -0.133, -0.332], [0.442, -0.019, 0.039], [-0.02, -0.746, 0.058]], "offset": [-0.033, -0.15, -0.302], "variations": {"linear": 1}},
        {"weight": 0.448, "color": 0.5, "matrix": [[0.569, -0.229, 0.027], [0.287, 0.385, 0.289], [-0.137, -0.147, 0.716]], "offset": [0.121, 0.262, 0.142], "variations": {"bent": 1}},
        {"weight": 0.477, "color": 1, "matrix": [[0.178, -0.559, -0.21], [0.154, 0.068, 0.742], [-0.203, -0.44, 0.381]], "offset": [-0.517, 0.283, 0.713], "variations": {"swirl": 1}}
    ]"#,
    ),
    (
        "spiral",
        "Ribbons winding up in a helix (try palette aurora).",
        r#"[
        {"weight": 0.31, "color": 0, "matrix": [[-0.442, 0.243, 0.3], [-0.551, 0.014, -0.362], [-0.336, -0.342, 0.199]], "offset": [-0.029, 0.125, 0.462], "variations": {"eyefish": 1}},
        {"weight": 0.674, "color": 0.5, "matrix": [[0.179, 0.174, 0.372], [-0.041, 0.522, -0.127], [-0.716, 0.014, 0.101]], "offset": [0.265, 0.385, 0.003], "variations": {"sinusoidal": 0.665, "linear": 0.335}},
        {"weight": 0.845, "color": 1, "matrix": [[0.611, 0.091, 0.25], [-0.026, 0.647, -0.062], [-0.593, 0.066, 0.26]], "offset": [-0.289, -0.595, -0.686], "variations": {"swirl": 1}}
    ]"#,
    ),
    (
        "nautilus",
        "A coiled shell of lace, spiralling out to a point (try palette gold).",
        r#"[
        {"weight": 0.452, "color": 0, "matrix": [[0.32, 0.158, -0.079], [-0.07, 0.758, -0.095], [0.023, 0.106, 0.8]], "offset": [-0.575, 0.322, -0.454], "variations": {"spherical": 1}},
        {"weight": 0.996, "color": 1, "matrix": [[0.208, 0.741, 0.116], [0.6, -0.228, 0.463], [0.521, -0.034, -0.58]], "offset": [-0.232, 0.781, 0.405], "variations": {"linear": 1}}
    ]"#,
    ),
    (
        "bloom",
        "A billowing bloom of fire, like a firework's burst.",
        r#"[
        {"weight": 0.94, "color": 0, "matrix": [[-0.552, 0.048, -0.454], [0.173, -0.282, -0.303], [-0.485, -0.155, 0.409]], "offset": [-0.656, -0.495, -0.171], "variations": {"linear": 1}},
        {"weight": 0.977, "color": 0.333, "matrix": [[0.669, 0.392, 0.137], [-0.367, 0.69, 0.298], [0.025, -0.348, 0.746]], "offset": [0.147, -0.35, 0.132], "variations": {"linear": 0.359, "spherical": 0.641}},
        {"weight": 0.31, "color": 0.667, "matrix": [[0.509, 0.196, -0.332], [0.378, 0.151, 0.446], [0.193, -0.814, 0.003]], "offset": [0.114, -0.187, -0.482], "variations": {"eyefish": 0.616, "sinusoidal": 0.384}},
        {"weight": 0.862, "color": 1, "matrix": [[0.311, 0.097, 0.48], [-0.07, -0.683, 0.297], [0.254, -0.308, -0.506]], "offset": [0.442, 0.198, 0.57], "variations": {"spherical": 1}}
    ]"#,
    ),
    (
        "dandelion",
        "A ball of fine curved filaments, a seed head (try palette ice).",
        r#"[
        {"weight": 0.61, "color": 0, "matrix": [[0.875, 0.035, 0.006], [-0.039, 0.781, 0.017], [-0.008, -0.026, 0.528]], "offset": [0.517, -0.504, -0.635], "variations": {"eyefish": 0.641, "horseshoe": 0.359}},
        {"weight": 0.664, "color": 1, "matrix": [[-0.111, 0.065, 0.366], [-0.546, 0.448, -0.068], [-0.363, -0.694, -0.01]], "offset": [0.329, 0.496, 0.104], "variations": {"linear": 0.421, "spherical": 0.579}}
    ]"#,
    ),
    (
        "veil",
        "A dome of arching threads over a dense heap (try palette aurora).",
        r#"[
        {"weight": 0.883, "color": 0, "matrix": [[0.47, -0.23, 0.097], [0.088, 0.278, 0.246], [-0.348, -0.24, 0.192]], "offset": [0.775, -0.756, -0.498], "variations": {"linear": 0.791, "horseshoe": 0.209}},
        {"weight": 0.344, "color": 0.333, "matrix": [[-0.018, 0.816, 0.082], [-0.872, 0.016, -0.068], [-0.157, -0.178, 0.369]], "offset": [-0.149, -0.699, 0.113], "variations": {"linear": 0.302, "sinusoidal": 0.698}},
        {"weight": 0.712, "color": 0.667, "matrix": [[0.762, 0.265, 0.074], [0.092, 0.229, -0.289], [-0.38, 0.586, 0.079]], "offset": [-0.287, 0.413, -0.504], "variations": {"spherical": 1}},
        {"weight": 0.49, "color": 1, "matrix": [[-0.008, 0.181, 0.809], [-0.528, 0.262, -0.237], [-0.443, -0.315, 0.267]], "offset": [-0.254, -0.04, 0.241], "variations": {"cylinder": 1}}
    ]"#,
    ),
    (
    "sierpinski",
    "The Sierpinski tetrahedron: four maps that halve toward the corners of a tetrahedron, the plainest flame of all.",
    r#"[
        {"color": 0.0, "matrix": [[0.5,0,0],[0,0.5,0],[0,0,0.5]], "offset": [0.5,0.5,0.5]},
        {"color": 0.33, "matrix": [[0.5,0,0],[0,0.5,0],[0,0,0.5]], "offset": [-0.5,-0.5,0.5]},
        {"color": 0.67, "matrix": [[0.5,0,0],[0,0.5,0],[0,0,0.5]], "offset": [-0.5,0.5,-0.5]},
        {"color": 1.0, "matrix": [[0.5,0,0],[0,0.5,0],[0,0,0.5]], "offset": [0.5,-0.5,-0.5]}
    ]"#,
),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(preset: &str, seed: u32) -> FlameSpec {
        FlameSpec {
            preset: preset.into(),
            seed,
            transforms: None,
            final_xform: None,
            symmetry: 1,
            palette: "fire".into(),
            colors: None,
            points: 1.0,
            resolution: 48,
            gamma: 2.0,
        }
    }

    /// Every preset draws, into a grid with something in it and empty borders.
    #[test]
    fn presets_draw() {
        for p in PRESETS {
            let g = build(&spec(p.0, 0)).unwrap_or_else(|e| panic!("{}: {e}", p.0));
            assert!(g.filled > 100, "{}: {} voxels filled", p.0, g.filled);
            let [nx, ny, _] = g.n;
            // (the first slice of z is the border)
            assert!(g.data[..(nx * ny) as usize].iter().all(|&w| w == 0), "{}: border not empty", p.0);
        }
    }

    /// Random flames are the same for the same seed, and most seeds give
    /// something plausible.
    #[test]
    fn random_flames_repeat() {
        let a = build(&spec("random", 7)).unwrap();
        let b = build(&spec("random", 7)).unwrap();
        assert_eq!(a.n, b.n);
        assert_eq!(a.filled, b.filled);
        let ok = (0..20).filter(|&s| plausible(&random_system(s))).count();
        assert!(ok >= 16, "only {ok} of 20 random seeds plausible");
    }

    /// The Sierpinski tetrahedron has its density at the tetrahedron's corners
    /// and none at its centre's hole.
    #[test]
    fn sierpinski_has_its_holes() {
        let g = build(&FlameSpec { resolution: 64, ..spec("sierpinski", 0) }).unwrap();
        // (the box is centred on the attractor; a corner is at (1, 1, 1) - (0, 0, 0)... scaled to the box)
        let h = g.extent * 0.5 * 0.92;
        assert!(g.density_at([h, h, h]) > 0.05 || g.density_at([h * 0.9, h * 0.9, h * 0.9]) > 0.05);
        assert!(g.density_at([0.0, 0.0, 0.0]) < 0.05);
    }
}

#[cfg(test)]
mod dump {
    use super::*;

    /// Prints the systems of the seeds in RAY_FLAME_SEEDS (comma separated)
    /// as `transforms` JSON, three decimals (turning random flames into presets).
    #[test]
    #[ignore]
    fn dump_seeds() {
        let r3 = |x: f32| (x * 1000.0).round() / 1000.0;
        for s in std::env::var("RAY_FLAME_SEEDS").unwrap_or_default().split(',').filter_map(|s| s.trim().parse::<u32>().ok()) {
            let xs = random_system(s);
            let ws = random_weights(s, xs.len());
            let defs: Vec<serde_json::Value> = xs
                .iter()
                .zip(ws)
                .map(|(x, w)| {
                    let mut v = serde_json::json!({
                        "weight": r3(w),
                        "color": r3(x.color),
                        "matrix": x.m.map(|r| r.map(r3)),
                        "offset": x.o.map(r3),
                        "variations": x.vars.iter().map(|&(id, w)| (VARIATIONS[id as usize].0.to_string(), serde_json::json!(r3(w)))).collect::<serde_json::Map<_, _>>(),
                    });
                    if x.vars.iter().any(|v| v.0 == 8) {
                        v["power"] = serde_json::json!(x.power);
                    }
                    v
                })
                .collect();
            println!("SEED {s}: {}", serde_json::to_string(&defs).unwrap());
        }
    }
}
