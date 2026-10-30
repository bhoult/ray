//! Surfaces, materials, lighting and the integrators, eight rays at a time.
//!
//! - `direct`: Whitted-style and noise free. Sun with (soft) shadows, sky
//!   ambient with AO, point lights; mirror reflection and refraction chains
//!   up to `bounces`, plus one cheap extra reflection ray at glass and water
//!   (fraktal256's trace2). fraktal256's look.
//! - `path`: Monte Carlo path tracing. Diffuse, glossy, metal and dielectric
//!   lobes chosen per lane, the sun and point lights sampled directly,
//!   Russian roulette; emissive surfaces light the scene through bounces.
//! - `debug`: normals, albedo, depth and the march cost of the primary ray.

use super::Ctx;
use super::atmos;
use super::trace::{self, BIG, Hit, Ray, grain8};
use super::pattern;
use super::vm::{Regs, eval, ticks};
use crate::math::Vec3;
use crate::noise::noised8;
use crate::scene::terrain::Terrain;
use crate::scene::{Shape, tex};
use crate::settings::Mode;
use crate::simd::{F8, I8, M8, V8};

/// Water absorption per metre (exp2 exponents), fraktal256's.
const WATER_ABSORB: Vec3 = Vec3::new(1.0, 0.40, 0.28);
const WAVE_SEED: i32 = 0x1656_67b1;
/// Water's index of refraction.
const WATER_IOR: f32 = 1.33;
/// Ambient strength (fraktal256's c_ambk).
pub(crate) const AMBIENT: f32 = 2.0;

/// What a ray hit: position, outward normal and material, per lane.
pub struct Surf {
    pub p: V8,
    pub n: V8,
    pub albedo: V8,
    pub emission: V8,
    pub rough: F8,
    pub metal: F8,
    pub trans: F8,
    pub ior: F8,
    /// absorption inside (transmissive), exp2 exponents per metre
    pub absorb: V8,
    /// light scattered out of the medium inside (water's deep colour)
    pub scatter: V8,
    /// baked occlusion (terrain cavities), 1 elsewhere
    pub ao: F8,
    pub terrain: M8,
    /// the node of the object's tree that was hit (SDF objects; 0 otherwise)
    pub node: I8,
    /// sunlight scattered through it (clouds), 0..1
    pub transl: F8,
    /// the pixel's footprint there, in metres (fractals seen from a
    /// millimetre away, a floor under a micrometre-tall walker): secondary
    /// rays start that close and AO looks that small; 1 on terrain
    pub foot: F8,
    /// the point's offset from the frame's anchor (Ray.od): the origin of
    /// the rays it sends, finer than `p` near the camera
    pub dp: V8,
    /// lanes on anchored objects (sdf::Program::anchored): f32's floor there
    /// is the offset's, not the world position's
    pub fine: M8,
}

pub struct Sample {
    pub col: V8,
    pub steps: I8,
    /// the object the primary ray hit (-1: sky)
    pub obj: I8,
}

/// Eight independent random streams (counter-based: a hashed counter).
pub struct Rng8 {
    s: I8,
}

#[inline(always)]
pub fn lowbias(x: I8) -> I8 {
    let x = x ^ x.shr::<16>();
    let x = x.mul_lo(I8::splat(0x7feb_352d));
    let x = x ^ x.shr::<15>();
    let x = x.mul_lo(I8::splat(0x846c_a68bu32 as i32));
    x ^ x.shr::<16>()
}

#[inline(always)]
pub fn unit(h: I8) -> F8 {
    (h.shr::<9>() | 0x3F80_0000).cast_f8() - 1.0
}

impl Rng8 {
    pub fn new(pixel: I8, sample: u32) -> Rng8 {
        let s = pixel.mul_lo(I8::splat(0x9E37_79B1u32 as i32)) + (sample.wrapping_mul(0x85EB_CA77) as i32);
        Rng8 { s: lowbias(s) }
    }
    /// Uniform in [0, 1).
    #[inline(always)]
    pub fn next(&mut self) -> F8 {
        self.s = self.s + 0x9E37_79B9u32 as i32;
        unit(lowbias(self.s))
    }
}

/// Smoothstep of `(x - a) * k` (fraktal's SMOOTH).
#[inline(always)]
fn smooth(x: F8, a: f32, k: f32) -> F8 {
    let t = ((x - a) * k).saturate();
    t * t * (F8::splat(3.0) - t - t)
}

#[inline(always)]
fn schlick(f0: V8, c: F8) -> V8 {
    let m = (F8::one() - c).saturate();
    let m2 = m * m;
    let k = m2 * m2 * m;
    V8::new(
        (F8::one() - f0.x).mul_add(k, f0.x),
        (F8::one() - f0.y).mul_add(k, f0.y),
        (F8::one() - f0.z).mul_add(k, f0.z),
    )
}

#[inline(always)]
fn schlick1(f0: F8, c: F8) -> F8 {
    let m = (F8::one() - c).saturate();
    let m2 = m * m;
    (F8::one() - f0).mul_add(m2 * m2 * m, f0)
}

/// Refraction of `d` through `nf` (facing against `d`) with ratio `eta`, and
/// the lanes that reflect totally instead.
#[inline(always)]
fn refract(d: V8, nf: V8, eta: F8) -> (V8, M8) {
    let cosi = -d.dot(nf);
    let k = F8::one() - eta * eta * (F8::one() - cosi * cosi);
    let tir = k.lt(F8::zero());
    let s = eta.mul_add(cosi, -k.max(F8::zero()).sqrt());
    (d * eta + nf * s, tir)
}

/// An orthonormal basis around unit `n` (Duff et al., branchless).
#[inline(always)]
fn basis(n: V8) -> (V8, V8) {
    let sign = F8::one().copysign(n.z);
    let a = -F8::one() / (sign + n.z);
    let b = n.x * n.y * a;
    (V8::new(F8::one() + sign * n.x * n.x * a, sign * b, -sign * n.x), V8::new(b, sign + n.y * n.y * a, -n.y))
}

/// Cosine-weighted direction about `n`.
#[inline(always)]
fn cosine_dir(n: V8, u1: F8, u2: F8) -> V8 {
    let r = u1.sqrt();
    let phi = u2 * core::f32::consts::TAU;
    let (t1, t2) = basis(n);
    let z = (F8::one() - u1).max(F8::zero()).sqrt();
    t1 * (r * phi.cos()) + t2 * (r * phi.sin()) + n * z
}

#[inline(always)]
fn gather3(a: &[Vec<f32>; 3], m: I8) -> V8 {
    V8::new(F8::gather(&a[0], m), F8::gather(&a[1], m), F8::gather(&a[2], m))
}

/// The surfaces of the lanes in `active` that hit something.
pub fn surface(ctx: &Ctx, regs: &mut Regs, ray: &Ray, hit: &Hit, active: M8, cone: f32) -> Surf {
    let sc = ctx.scene;
    let mut p = V8::at(ray.o, ray.d, hit.t);
    let mut dp = V8::at(ray.od, ray.d, hit.t);
    let mut fine = M8::none_set();
    let mut n = V8::new(F8::zero(), F8::one(), F8::zero());
    let mut mat = I8::zero();
    let mut node = I8::zero();
    // fractals' orbit traps (the orbit pattern colours by them)
    let mut trap = F8::zero();
    let mut ao = F8::one();
    // (the pixel's footprint: close up, secondary rays start that close and AO
    // looks that small; terrain keeps 1, its grit and cavities scale themselves)
    let mut foot = hit.t * cone;
    let mut terrain = M8::none_set();
    // terrain lanes coloured by fraktal's terrain material
    let mut terrain_tex = M8::none_set();
    let mut tcol = V8::zero();
    let live = active & hit.obj.gt(I8::splat(-1));
    let mut todo = live;
    while todo.any() {
        let idx = hit.obj.lane(todo.bits().trailing_zeros() as usize);
        let g = hit.obj.eq(I8::splat(idx)) & todo;
        todo = todo.and_not(g);
        let o = &sc.objects[idx as usize];
        let om = I8::splat(o.material as i32);
        let t0 = if ctx.profile { ticks() } else { 0 };
        match &o.shape {
            Shape::Sphere { c, r } => {
                n = V8::select(g, (p - *c) * (1.0 / r), n);
                mat = I8::select(g, om, mat);
            }
            Shape::Box { c, rot, half } => {
                let q = p - *c;
                let m = &rot.m;
                let l = V8::new(q.dot3(m[0]) * (1.0 / half.x), q.dot3(m[1]) * (1.0 / half.y), q.dot3(m[2]) * (1.0 / half.z));
                let (ax, ay, az) = (l.x.abs(), l.y.abs(), l.z.abs());
                let xm = ax.ge(ay) & ax.ge(az);
                let ym = ay.ge(az).and_not(xm);
                let zm = !(xm | ym);
                let s = |v: F8, on: M8| F8::one().copysign(v).and(on);
                let (sx, sy, sz) = (s(l.x, xm), s(l.y, ym), s(l.z, zm));
                let wn = V8::splat(m[0]) * sx + V8::splat(m[1]) * sy + V8::splat(m[2]) * sz;
                n = V8::select(g, wn, n);
                mat = I8::select(g, om, mat);
            }
            Shape::Plane { n: pn, .. } => {
                n = V8::select(g, V8::splat(*pn), n);
                mat = I8::select(g, om, mat);
            }
            Shape::Nurbs { surf, c, rot, s } => {
                // the hit again, for its normal (an open surface shows both sides)
                let (lo, ld) = trace::to_local(ray, *c, rot);
                let k = 1.0 / s;
                let limit = hit.t.mul_add(F8::splat(k * (1.0 + 1e-4)), F8::splat(1e-6));
                let (_, nl, _) = super::nurbs::hit8(surf, lo * k, ld, limit, g, 1e-5 * k, super::nurbs::Find::Normal);
                let m = &rot.m;
                let mut wn = V8::splat(m[0]) * nl.x + V8::splat(m[1]) * nl.y + V8::splat(m[2]) * nl.z;
                let missed = wn.length_sq().lt(F8::splat(0.5));
                wn = V8::select(missed, -ray.d, wn);
                if !surf.solid {
                    wn = V8::select(wn.dot(ray.d).gt(F8::zero()), -wn, wn);
                }
                n = V8::select(g, wn, n);
                mat = I8::select(g, om, mat);
            }
            Shape::Sdf(prog) => {
                ctx.prepare(regs, idx as usize);
                // (from the object's position, along the ray: see Object.org;
                // anchored, f32 is as fine as the offset from the camera)
                let pr = V8::at(ray.o - o.org, ray.d, hit.t);
                let anchored = prog.anchored && ctx.tables.get(idx as usize).is_some_and(|t| !t.is_empty());
                let hd = V8::at(ray.od, ray.d, hit.t);
                let grain = grain8(if anchored { hd.length() } else { pr.length() });
                let eps = (hit.t * (cone * 0.5)).max(grain * 2.0);
                foot = F8::select(g, (hit.t * cone).max(grain), foot);
                let nn = trace::sdf_normal(prog, regs, pr, hd, eps);
                regs.dp = hd;
                let (d, tag) = eval(prog, pr, regs, true);
                trap = F8::select(g, regs.trap(), trap);
                // onto the surface: hits stop within the pixel footprint of it
                p = V8::select(g, nn.mul_add(-d, p), p);
                dp = V8::select(g, nn.mul_add(-d, hd), dp);
                if anchored {
                    fine |= g;
                }
                n = V8::select(g, nn, n);
                mat = I8::select(g, tag & 0xffff, mat);
                node = I8::select(g, tag.shr::<16>(), node);
            }
            Shape::Terrain(tr) => {
                let (nn, a) = tr.normal8(p.x, p.z);
                ao = F8::select(g, a, ao);
                terrain |= g;
                foot = F8::select(g, F8::one(), foot);
                mat = I8::select(g, om, mat);
                if sc.materials[o.material as usize].tex == tex::TERRAIN {
                    let (col, nd) = terrain_material(tr, p, hit.t, nn, cone);
                    n = V8::select(g, nd, n);
                    tcol = V8::select(g, col, tcol);
                    terrain_tex |= g;
                } else {
                    n = V8::select(g, nn, n);
                }
            }
        }
        if ctx.profile {
            regs.charge(idx as u32, ticks() - t0, 5);
        }
    }

    // material parameters: one splat when the octet shares a material
    let first = live.bits().trailing_zeros().min(7) as usize;
    let m0 = mat.lane(first);
    let ms = &sc.mats;
    let uniform = (mat.eq(I8::splat(m0)) | !live).all();
    let m = &sc.materials[m0 as usize];
    let g3 = |a: &[Vec<f32>; 3], v: Vec3| if uniform { V8::splat(v) } else { gather3(a, mat) };
    let g1 = |a: &Vec<f32>, v: f32| if uniform { F8::splat(v) } else { F8::gather(a, mat) };
    let mut albedo = g3(&ms.color, m.color);
    let mut emission = g3(&ms.emission, m.emission);
    let mut rough = g1(&ms.roughness, m.roughness);
    let mut metal = g1(&ms.metallic, m.metallic);
    let trans = g1(&ms.transmission, m.transmission);
    let ior = g1(&ms.ior, m.ior);
    let transl = g1(&ms.translucency, m.translucency);
    let absorb = g3(&ms.absorb, m.absorb);
    let texk = if uniform { I8::splat(m.tex) } else { I8::gather(&ms.tex, mat) };
    let albedo2 = g3(&ms.color2, m.color2);

    // procedural patterns, in each object's own frame unless the material
    // says world: the colour's, and the relief's (by default the same one)
    let usable = |k: I8| k.gt(I8::splat(tex::NONE)) & k.ne_i(tex::TERRAIN) & k.ne_i(tex::WATER);
    let bump = g1(&ms.bump, m.bump);
    let bumpk = if uniform { I8::splat(m.bump_tex) } else { I8::gather(&ms.bump_tex, mat) };
    let colour = live.and_not(terrain_tex) & usable(texk);
    // (an orbit trap has no slope to bump by)
    let bumped = live.and_not(terrain_tex) & usable(bumpk) & bumpk.ne_i(tex::ORBIT) & bump.ne(F8::zero());
    let pat = colour | bumped;
    if pat.any() {
        let emission2 = g3(&ms.emission2, m.emission2);
        let rough2 = g1(&ms.roughness2, m.roughness2);
        let metal2 = g1(&ms.metallic2, m.metallic2);
        let tscale = g1(&ms.tex_scale, m.tex_scale);
        let bscale = g1(&ms.bump_scale, m.bump_scale);
        let warp = g1(&ms.warp, m.warp);
        let sharp = g1(&ms.sharpness, m.sharpness);
        let world = if uniform { I8::splat(m.world as i32) } else { I8::gather(&ms.world, mat) }.eq(I8::splat(1));
        let mut todo = pat;
        while todo.any() {
            let idx = hit.obj.lane(todo.bits().trailing_zeros() as usize);
            let g = hit.obj.eq(I8::splat(idx)) & todo;
            todo = todo.and_not(g);
            let o = &sc.objects[idx as usize];
            let lp = pattern::mat_apply(&o.local_m, p - o.local_t);
            let ln = pattern::mat_apply(&o.local_m, n).normalize();
            let at = V8::select(world, p, lp);
            let q = at * tscale;
            let qn = V8::select(world, n, ln);
            let mut kinds = g & colour;
            while kinds.any() {
                let k = texk.lane(kinds.bits().trailing_zeros() as usize);
                let km = texk.eq(I8::splat(k)) & kinds;
                kinds = kinds.and_not(km);
                let (t, shade) = if k == tex::ORBIT { (orbit(trap, tscale, sharp), F8::one()) } else { pattern::eval(k, q, qn, warp, sharp) };
                albedo = V8::select(km, V8::lerp(albedo, albedo2, t) * shade, albedo);
                rough = F8::select(km, F8::lerp(rough, rough2, t), rough);
                metal = F8::select(km, F8::lerp(metal, metal2, t), metal);
                emission = V8::select(km, V8::lerp(emission, emission2, t), emission);
            }
            let qb = at * bscale;
            let mut kinds = g & bumped;
            while kinds.any() {
                let k = bumpk.lane(kinds.bits().trailing_zeros() as usize);
                let km = bumpk.eq(I8::splat(k)) & kinds;
                kinds = kinds.and_not(km);
                let gq = pattern::gradient(k, qb, qn, warp, sharp);
                // back to world directions: q = scale * m (p - t), so the
                // world gradient is scale * m^T gq (the pattern scale cancels
                // out of the relief, so fine and coarse patterns bump alike)
                let gw = V8::select(world, gq, pattern::mat_apply_t(&o.local_m, gq));
                let tangential = gw - n * gw.dot(n);
                let nb = tangential.mul_add(bump * -0.05, n).normalize();
                n = V8::select(km, nb, n);
            }
        }
    }
    let water = live & texk.eq(I8::splat(tex::WATER));
    if water.any() {
        n = V8::select(water, wave_normal(ctx, p, hit.t, n), n);
    }
    albedo = V8::select(terrain_tex, tcol, albedo);
    let scatter = V8::select(water, albedo2 * ctx.env.daylight, V8::zero());
    let absorb = V8::select(water, V8::splat(WATER_ABSORB), absorb);

    Surf { p, n, albedo, emission, rough, metal, trans, ior, absorb, scatter, ao, terrain, node, transl, foot, dp, fine }
}

/// The orbit pattern: a fractal's orbit trap through the two colours and
/// back, more often the larger the scale; sharpness sets the contrast.
#[inline(always)]
fn orbit(trap: F8, scale: F8, sharp: F8) -> F8 {
    let b = (trap * scale * core::f32::consts::PI).cos().neg_mul_add(F8::splat(0.5), F8::splat(0.5));
    (b - 0.5).mul_add(sharp, F8::splat(0.5)).saturate()
}

/// The water's surface at (x, z): its height above the water level and its
/// slope (dh/dx, dh/dz). fraktal256's two noise octaves and a finer one (the
/// small waves that draw the caustics), `height` times their usual size,
/// drifting different ways with the wave clock `wave_time`; plus the
/// ripples, each a damped ring spreading from where it started. The
/// renderer takes the slope; the physics floats things on the height.
pub fn water_surface(time: f32, wave_time: f32, ripples: &[super::Ripple], height: f32, x: F8, z: F8) -> (F8, F8, F8) {
    let tt = wave_time;
    let seed = I8::splat(WAVE_SEED);
    let (n1, g1x, g1z) = noised8(x * 0.35 + tt * 0.21, z * 0.35 + tt * 0.13, seed);
    let (n2, g2x, g2z) = noised8(x * 1.3 - tt * 0.47, z * 1.3 + tt * 0.36, seed);
    let (n3, g3x, g3z) = noised8(x * 3.7 + tt * 0.9, z * 3.7 - tt * 0.7, seed);
    let k = 0.12 * height;
    let mut sx = g3x.mul_add(F8::splat(0.2), g2x.mul_add(F8::splat(0.45), g1x)) * k;
    let mut sz = g3z.mul_add(F8::splat(0.2), g2z.mul_add(F8::splat(0.45), g1z)) * k;
    // the heights those are the slopes of (an octave's slope is its height
    // times its frequency)
    let mut y = ((n1 - 0.5) * (1.0 / 0.35) + (n2 - 0.5) * (0.45 / 1.3) + (n3 - 0.5) * (0.2 / 3.7)) * k;
    for r in &ripples[..ripples.len().min(super::MAX_RIPPLES)] {
        let age = time - r.t0;
        if age < 0.0 {
            continue;
        }
        let rad = super::RIPPLE_SPEED * age;
        let width = 0.18 + 0.35 * age;
        let (dx, dz) = (x - r.x, z - r.z);
        let dist = dz.mul_add(dz, dx * dx).sqrt().max(F8::splat(1e-4));
        let off = dist - rad;
        let u = off * (1.0 / width);
        // only near its ring: RIPPLE_REACH widths away it is e^-12 high
        if !u.abs().lt(F8::splat(super::RIPPLE_REACH)).any() {
            continue;
        }
        let a = r.height(age) / (1.0 + 2.0 * rad).sqrt();
        let ph = off * super::RIPPLE_K;
        let (sin, cos) = (ph.sin(), ph.cos());
        let env = (u * u * -core::f32::consts::LOG2_E).exp2() * a;
        y = env.mul_add(cos, y);
        let dh = env * (u * (-2.0 / width) * cos - sin * super::RIPPLE_K);
        let k = dh / dist;
        sx = dx.mul_add(k, sx);
        sz = dz.mul_add(k, sz);
    }
    (y, sx, sz)
}

/// The water's slope at (x, z) in this frame.
pub fn wave_slope(ctx: &Ctx, x: F8, z: F8) -> (F8, F8) {
    let w = ctx.waves;
    let (_, sx, sz) = water_surface(w.time, w.wave_time, &w.ripples, ctx.scene.wave_height, x, z);
    (sx, sz)
}

/// Sunlight reaching `p`, `depth` below the water: the direction toward the
/// light inside the water, refracted where it entered through the waves,
/// and the metres it travelled in the water. The entry point comes from the
/// flat surface, then once more from the waves' slope there.
fn under_water(ctx: &Ctx, sun: V8, p: V8, depth: F8) -> (V8, F8) {
    let eta = F8::splat(1.0 / WATER_IOR);
    let (t0, _) = refract(-sun, V8::new(F8::zero(), F8::one(), F8::zero()), eta);
    let len0 = depth / (-t0.y).max(F8::splat(0.05));
    let s0 = t0.mul_add(-len0, p);
    let (sx, sz) = wave_slope(ctx, s0.x, s0.z);
    let wn = V8::new(-sx, F8::one(), -sz).normalize();
    let (t1, _) = refract(-sun, wn, eta);
    let lw = (-t1).normalize();
    (lw, depth / lw.y.max(F8::splat(0.05)))
}

/// The water's normal: its slope, calmer with distance (`t`).
fn wave_normal(ctx: &Ctx, p: V8, t: F8, n: V8) -> V8 {
    let (sx, sz) = wave_slope(ctx, p.x, p.z);
    let fade = F8::one() / (t * 0.02 + 1.0);
    let (nx, nz) = (-(sx * fade), -(sz * fade));
    let ny = (nz.mul_add(nz, nx * nx) + 1.0).rsqrt();
    let w = V8::new(nx * ny, ny, nz * ny);
    // planes facing down get the mirrored waves
    V8::select(n.y.lt(F8::zero()), -w, w)
}

/// fraktal256's terrain material: rock with strata, grass where flat and low,
/// snow high up, sand at the shore; detail normals from noise.
fn terrain_material(tr: &Terrain, p: V8, rt: F8, n: V8, cone: f32) -> (V8, V8) {
    let seed = I8::splat(0x27d4_eb2f);
    let (n1, g1x, g1z) = noised8(p.x * 0.7, p.z * 0.7, seed);
    let fade = (F8::one() - rt * 0.01).max(F8::zero()) * 0.35;
    let nx = g1x.neg_mul_add(fade, n.x);
    let nz = g1z.neg_mul_add(fade, n.z);
    let (n2, _, _) = noised8(p.x * 2.9, p.z * 2.9, seed);
    let (macro_n, g3x, g3z) = noised8(p.x * 0.06, p.z * 0.06, seed);
    let macro_n = macro_n - 0.5;
    let fade = (F8::one() - rt * 0.04).max(F8::zero()) * 0.08;
    let nx = g3x.neg_mul_add(fade, nx);
    let nz = g3z.neg_mul_add(fade, nz);
    let n = V8::new(nx, n.y, nz).normalize();

    let strata = (p.y * 1.3 + n1 * 3.0).sin();
    let bright = (strata * 0.12 + 0.9) + n2 * 0.3;
    let bright = macro_n.mul_add(F8::splat(0.5), bright);
    let rock = V8::splat(tr.rock) * bright;
    let warm = macro_n * 0.6;
    let mut col = V8::new(warm.mul_add(rock.x, rock.x), rock.y, warm.neg_mul_add(rock.z, rock.z));
    let flat = smooth(n.y, 0.78, 8.0);
    let high = smooth(n1 * 10.0 + p.y, tr.grass_top, 0.1);
    let patch = smooth(n1 * 0.3 + macro_n, -0.15, 6.0);
    let grass_w = flat * (F8::one() - high) * patch;
    let grass = V8::splat(tr.grass) * (n2 * 0.5 + 0.75);
    col = V8::lerp(col, grass, grass_w);
    let snow_w = smooth(n1 * 8.0 + p.y, tr.snow_line, 0.15) * smooth(n.y, 0.6, 5.0);
    col = V8::lerp(col, V8::splat(tr.snow), snow_w);
    let sand_w = F8::one() - smooth(p.y, tr.sand_line, 1.0);
    col = V8::lerp(col, V8::splat(tr.sand), sand_w);
    let (grit, n) = terrain_grit(p, rt * cone, n);
    (col * grit, n)
}

/// Close-up detail for terrain, so near ground is not a smooth smear of the
/// heightmap: grit in the colour at 25, 7 and 2.5 cm, and bumps in the
/// shading at 25 cm. The first is noise on the three axis planes weighted by
/// the normal (triplanar: not stretched down slopes), the others 3D noise.
/// Each layer fades out as it shrinks below a couple of pixels (`foot`:
/// metres a pixel covers there), so it never shimmers. The brightness
/// factor, and the bumped normal.
fn terrain_grit(p: V8, foot: F8, n: V8) -> (F8, V8) {
    let fade = |f: f32| (F8::splat(1.5) - foot * (3.0 * f)).clamp(0.0, 1.0);
    let f1 = fade(4.0);
    if !f1.gt(F8::zero()).any() {
        return (F8::one(), n);
    }
    let seed = I8::splat(0x6d2b_79f5);
    let f = 4.0;
    let (wx, wy, wz) = (n.x * n.x, n.y * n.y, n.z * n.z);
    // each plane's noise and its gradient along the plane's two axes
    let (vx, gxz, gxy) = noised8(p.z * f, p.y * f, seed);
    let (vy, gyx, gyz) = noised8(p.x * f, p.z * f, seed);
    let (vz, gzx, gzy) = noised8(p.x * f, p.y * f, seed);
    let v = vx * wx + vy * wy + vz * wz;
    let g = V8::new(gyx * wy + gzx * wz, gxy * wx + gzy * wz, gxz * wx + gyz * wy);
    let n = (n - g * (f1 * 0.12)).normalize();
    let mut grit = ((v - 0.5) * 0.7).mul_add(f1, F8::one());
    let (f2, f3) = (fade(14.0), fade(40.0));
    if f2.gt(F8::zero()).any() {
        grit = (crate::noise::perlin3(p * 14.0, seed) * 0.3).mul_add(f2, grit);
    }
    if f3.gt(F8::zero()).any() {
        grit = (crate::noise::perlin3(p * 40.0, seed ^ I8::splat(0x55)) * 0.25).mul_add(f3, grit);
    }
    (grit.max(F8::splat(0.2)), n)
}

/// The surface's normal on the side the ray `d` comes from. Glass and water
/// flip it when the ray is inside. An opaque surface keeps its own, tilted
/// just enough to face the ray: smoothed and bumped normals (terrain) can
/// face away from a grazing ray, and flipped they would point into the
/// ground, which then takes no light at all.
#[inline(always)]
fn facing(d: V8, sf: &Surf) -> V8 {
    let dn = d.dot(sf.n);
    let bent = d.mul_add(-(dn + 0.02), sf.n).normalize();
    let away = V8::select(sf.trans.gt(F8::zero()), -sf.n, bent);
    V8::select(dn.lt(F8::zero()), sf.n, away)
}

/// Sunlight a translucent material (clouds) lets through, seen along `d`:
/// some everywhere, much more looking toward the sun, most at the rims.
/// Not shadowed: it stands for light scattered inside the object, which
/// the object's own surface would block.
fn through(ctx: &Ctx, sf: &Surf, d: V8, n: V8) -> V8 {
    if !sf.transl.gt(F8::zero()).any() {
        return V8::zero();
    }
    let toward = d.dot3(ctx.env.sun_dir).max(F8::zero());
    let t2 = toward * toward;
    let rim = F8::one() - n.dot(-d).abs().min(F8::one());
    let amount = (t2 * t2 * (rim + 0.6)).mul_add(F8::splat(1.5), F8::splat(0.12));
    V8::splat(ctx.env.sun_col) * (amount * sf.transl)
}

/// How far to start secondary rays off the surface.
#[inline(always)]
fn offset(sf: &Surf, t: F8) -> F8 {
    // a millimetre, or four pixel footprints when that is less (close up),
    // past what f32 resolves of the world position the ray starts from (on
    // anchored objects the offset from the camera is what counts: Surf.foot)
    let world = F8::select(sf.fine, F8::zero(), grain8(sf.p.length()));
    let o = t.mul_add(F8::splat(2.0e-5), (sf.foot * 4.0).max(world).min(F8::splat(1.0e-3)));
    F8::select(sf.terrain, o.max(F8::splat(0.02)), o)
}

/// Light arriving at the surfaces (lanes `lm`, facing normal `n`) from the
/// sun, the point lights and glowing objects, to be multiplied by albedo; and
/// the sun's visibility. Path tracing (`pt`) jitters the sun over its disk with
/// hard shadows; otherwise the sun's shadows are soft by estimate. A glowing
/// object is sampled at a random point of its disk either way.
#[allow(clippy::too_many_arguments)]
fn direct_light(ctx: &Ctx, regs: &mut Regs, sf: &Surf, n: V8, t: F8, lm: M8, rng: &mut Rng8, pt: bool) -> (V8, F8) {
    let env = ctx.env;
    let s = ctx.s;
    let off = offset(sf, t);
    let (o, od) = (n.mul_add(off, sf.p), n.mul_add(off, sf.dp));
    let soft = !pt && s.soft_shadows;
    let sun = if pt {
        let j = V8::new(rng.next() - 0.5, rng.next() - 0.5, rng.next() - 0.5) * 0.02;
        (V8::splat(env.sun_dir) + j).normalize()
    } else {
        V8::splat(env.sun_dir)
    };
    // under water the sunlight comes through the waves: refracted (steeper),
    // absorbed over its path in the water, focused or spread by the surface's
    // curvature where it entered (caustics), and shadowed along the refracted
    // ray up to the surface, then along the sun's above it
    let mut ldir = sun;
    let mut tint = V8::one();
    let mut entry = sf.p;
    let mut under = M8::none_set();
    if let Some((level, _)) = ctx.scene.water {
        let depth = F8::splat(level) - sf.p.y;
        under = depth.gt(F8::zero()) & lm;
        if under.any() {
            let (lw, len) = under_water(ctx, sun, sf.p, depth);
            entry = lw.mul_add(len, sf.p);
            let e = 0.05;
            let (ax, _) = wave_slope(ctx, entry.x + e, entry.z);
            let (bx, _) = wave_slope(ctx, entry.x - e, entry.z);
            let (_, az) = wave_slope(ctx, entry.x, entry.z + e);
            let (_, bz) = wave_slope(ctx, entry.x, entry.z - e);
            let curv = (ax - bx + az - bz) * (0.5 / e);
            let spread = (len * (1.0 - 1.0 / WATER_IOR)).mul_add(curv, F8::one()).abs().max(F8::splat(0.25));
            let focus = F8::one() / spread;
            let r0 = (WATER_IOR - 1.0) / (WATER_IOR + 1.0);
            let through = F8::one() - schlick1(F8::splat(r0 * r0), sun.y.max(F8::zero()));
            let absorb = V8::new((len * -WATER_ABSORB.x).exp2(), (len * -WATER_ABSORB.y).exp2(), (len * -WATER_ABSORB.z).exp2());
            tint = V8::select(under, absorb * (focus * through), tint);
            ldir = V8::select(under, lw, sun);
        }
    }
    let ndl = n.dot(ldir).max(F8::zero());
    let m = lm & ndl.gt(F8::zero());
    let mut vis = F8::one();
    if m.any() && s.shadows {
        let reach = F8::select(under, (entry - sf.p).length(), F8::splat(BIG));
        vis = trace::shadow(ctx, regs, o, od, ldir, reach, m, soft, s.shadow_hardness, -1);
        let above = m & under & vis.gt(F8::splat(0.02));
        if above.any() {
            let v = trace::shadow(ctx, regs, entry, entry - V8::splat(ctx.anchor), sun, F8::splat(BIG), above, soft, s.shadow_hardness, -1);
            vis = F8::select(above, vis * v, vis);
        }
    }
    let mut light = V8::splat(env.sun_col).hadamard(tint) * (ndl * vis).and(m);
    if s.lights {
        for l in &ctx.scene.lights {
            let to = V8::splat(l.pos) - sf.p;
            let d2 = to.length_sq();
            let r2 = F8::splat(l.range * l.range);
            let near = d2.lt(r2) & lm;
            if near.none() {
                continue;
            }
            let inv = d2.max(F8::splat(1e-8)).rsqrt();
            let ndl = (to.dot(n) * inv).max(F8::zero());
            let window = F8::one() - d2 / r2;
            let mut weight = window * window / d2.mul_add(F8::splat(0.2), F8::one()) * ndl;
            let lit = near & weight.gt(F8::zero());
            if s.light_shadows && lit.any() {
                let v = trace::shadow(ctx, regs, o, od, to * inv, d2.sqrt(), lit, false, 1.0, -1);
                weight *= v;
            }
            light = V8::splat(l.color) * weight.and(lit) + light;
        }
        // glowing objects: sphere lights (radiance * (r / d)^2 * cos), seen
        // toward a random point of the light's disk, so that shadows soften
        // with its size as samples accumulate; faded out toward its range
        for e in &ctx.scene.emitters {
            let c = V8::splat(e.pos) - sf.p;
            let (t1, t2) = basis(c * c.length_sq().max(F8::splat(1e-12)).rsqrt());
            let rr = rng.next().sqrt() * e.radius;
            let phi = rng.next() * core::f32::consts::TAU;
            let to = c + t1 * (rr * phi.cos()) + t2 * (rr * phi.sin());
            let d2 = to.length_sq();
            let r2 = e.range * e.range;
            let near = d2.lt(F8::splat(r2)) & lm;
            if near.none() {
                continue;
            }
            let inv = d2.max(F8::splat(1e-8)).rsqrt();
            let ndl = (to.dot(n) * inv).max(F8::zero());
            let solid = F8::splat(e.radius * e.radius) / d2.max(F8::splat(e.radius * e.radius));
            let fade = d2.neg_mul_add(F8::splat(1.0 / r2), F8::one()).max(F8::zero());
            let mut weight = ndl * solid * fade * fade;
            let lit = near & weight.gt(F8::zero());
            if lit.none() {
                continue;
            }
            if s.light_shadows {
                // its own object never blocks it
                let v = trace::shadow(ctx, regs, o, od, to * inv, d2.sqrt(), lit, false, 1.0, e.obj as i32);
                weight *= v;
            }
            light = V8::splat(e.color) * weight.and(lit) + light;
        }
    }
    (light, vis)
}

/// Ambient occlusion from five samples of the scene's distance along `n`.
fn occlusion(ctx: &Ctx, regs: &mut Regs, sf: &Surf, n: V8, lm: M8) -> F8 {
    // close up the radius follows the scale in view (a thousand pixel
    // footprints), so creases a millimetre wide darken as at a metre
    let r = F8::splat(ctx.s.ao_radius.max(0.01)).min(sf.foot * 1000.0).max(F8::splat(1e-12));

    let skip_terrain = sf.terrain & lm;
    let mut occ = F8::zero();
    for i in 1..=5 {
        let h = r * (i as f32 / 5.0);
        // nothing farther than h occludes this sample
        let d = trace::scene_dist(ctx, regs, n.mul_add(h, sf.p), n.mul_add(h, sf.dp), h.hmax(), skip_terrain);
        occ = (h - d).max(F8::zero()).mul_add(F8::splat(0.2) / h, occ);
    }
    (F8::one() - occ * 1.2).saturate()
}

/// The sun's GGX highlight.
fn sun_spec(ctx: &Ctx, n: V8, v: V8, rough: F8, f0: V8, vis: F8) -> V8 {
    let l = ctx.env.sun_dir;
    let h = (v + l).normalize();
    let nh = n.dot(h).max(F8::zero());
    let ndl = n.dot3(l).max(F8::zero());
    let vh = v.dot(h).max(F8::zero());
    let a = rough.max(F8::splat(0.2));
    let a2 = (a * a) * (a * a);
    let dd = (nh * nh).mul_add(a2 - 1.0, F8::one());
    let d = a2 / (dd * dd * core::f32::consts::PI);
    schlick(f0, vh).hadamard(V8::splat(ctx.env.sun_col)) * (d * ndl * vis * 0.25)
}

/// A path's state across bounces.
struct Path {
    ray: Ray,
    thr: V8,
    active: M8,
    /// the medium the ray travels in: absorption, in-scatter, and whether it
    /// is in one at all (no fog inside)
    absorb: V8,
    scatter: V8,
    medium: M8,
    /// lanes whose path has bounced off a diffuse surface (path tracing):
    /// what they gather from then on is capped (see clamp_indirect)
    diffused: M8,
}

impl Path {
    fn new(ctx: &Ctx, o: V8, d: V8) -> Path {
        let mut p = Path {
            // (the camera's rays: it is `fine` from the anchor, Camera::fine)
            ray: Ray::new(o, o - V8::splat(ctx.anchor) + V8::splat(ctx.cam.fine), d, I8::splat(-1)),
            thr: V8::one(),
            active: M8::all_true(),
            absorb: V8::zero(),
            scatter: V8::zero(),
            medium: M8::none_set(),
            diffused: M8::none_set(),
        };
        // a camera under the water plane starts in the water
        if let Some((level, deep)) = ctx.scene.water
            && ctx.cam.pos.y < level
        {
            p.absorb = V8::splat(WATER_ABSORB);
            p.scatter = V8::splat(deep);
            p.medium = M8::all_true();
        }
        p
    }

    /// After `hit`: the sky for misses (`disk`: with the sun's disk), the
    /// medium or fog over the segment, then the surface and its emission.
    fn advance(&mut self, ctx: &Ctx, regs: &mut Regs, hit: &Hit, col: &mut V8, disk: M8, cone: f32) -> Option<Surf> {
        let miss = self.active & hit.obj.eq(I8::splat(-1));
        if miss.any() {
            let sky = atmos::sky(ctx, self.ray.d, disk);
            // the sky as fill light (seen after a diffuse bounce) follows the ambient level
            let sky = V8::select(disk, sky, sky * ctx.env.ambient);
            // in a medium (under water) out to nowhere: all that the water
            // scatters this way, its own deep colour (black, it drew a hard
            // line under the horizon looking down into a bottomless lake)
            let sky = V8::select(self.medium, self.scatter, sky);
            *col = V8::select(miss, self.thr.hadamard(sky) + *col, *col);
            self.active = self.active.and_not(miss);
        }
        if self.active.none() {
            return None;
        }
        let t = hit.t;
        if self.medium.any() {
            let tr = V8::new((t * -self.absorb.x).exp2(), (t * -self.absorb.y).exp2(), (t * -self.absorb.z).exp2());
            let m = self.medium & self.active;
            let inscatter = self.scatter.hadamard(V8::one() - tr);
            *col = V8::select(m, self.thr.hadamard(inscatter) + *col, *col);
            self.thr = V8::select(m, self.thr.hadamard(tr), self.thr);
        }
        let air = self.active.and_not(self.medium);
        if air.any() {
            let (ft, fin) = atmos::fog(ctx, t, self.ray.d);
            *col = V8::select(air, self.thr.hadamard(fin) + *col, *col);
            self.thr = V8::select(air, self.thr * ft, self.thr);
        }
        let sf = surface(ctx, regs, &self.ray, hit, self.active, cone);
        // glowing objects are sampled as lights after diffuse bounces: count their
        // glow there only when a camera or mirror ray sees it (no double counting)
        let emitter = I8::gather(&ctx.scene.emitter_flags, hit.obj.max(I8::zero())).eq(I8::splat(1));
        let shown = self.active & (disk | !emitter);
        *col = V8::select(shown, self.thr.hadamard(sf.emission) + *col, *col);
        Some(sf)
    }

    /// The clouds between the ray's origin and what it hit: the light they
    /// send this way is added, and what lies behind is dimmed. Returns how
    /// much gets through.
    fn through_clouds(&mut self, ctx: &Ctx, hit: &trace::Hit, col: &mut V8, rng: &mut Rng8) -> F8 {
        if ctx.scene.volumes.is_empty() || self.active.none() {
            return F8::one();
        }
        let (light, through) = super::volume::march(ctx, &self.ray, hit.t, self.active, rng);
        *col += self.thr.hadamard(light);
        self.thr = self.thr * through;
        through
    }

    /// Enters or leaves the medium of a transmissive surface on refraction.
    fn cross(&mut self, lanes: M8, entering: M8, sf: &Surf, obj: I8, inside: &mut I8) {
        let into = lanes & entering;
        let out = lanes.and_not(entering);
        *inside = I8::select(into, obj, *inside);
        *inside = I8::select(out, I8::splat(-1), *inside);
        self.absorb = V8::select(into, sf.absorb, self.absorb);
        self.absorb = V8::select(out, V8::zero(), self.absorb);
        self.scatter = V8::select(into, sf.scatter, self.scatter);
        self.scatter = V8::select(out, V8::zero(), self.scatter);
        self.medium = (self.medium | into).and_not(out);
    }
}

/// The object a camera ray saw, with the sky seen through a cloud marked
/// as such (the post pass twinkles only clear sky).
#[inline(always)]
fn clouded_sky(hit: &trace::Hit, through: F8, obj: I8) -> I8 {
    let clouded = hit.obj.eq(I8::splat(-1)) & through.lt(F8::splat(0.95));
    I8::select(clouded, I8::splat(super::CLOUDED_SKY), obj)
}

/// Whitted-style: deterministic, noise free.
pub fn direct(ctx: &Ctx, regs: &mut Regs, rng: &mut Rng8, o: V8, d: V8, cone: f32) -> Sample {
    let s = ctx.s;
    let env = ctx.env;
    let mut out = Sample { col: V8::zero(), steps: I8::zero(), obj: I8::splat(-1) };
    let mut path = Path::new(ctx, o, d);
    let mut col = V8::zero();
    for bounce in 0..=s.bounces {
        let hit = trace::intersect(ctx, regs, &path.ray, F8::splat(s.max_dist), path.active, cone);
        if bounce == 0 {
            out.steps = hit.steps;
            out.obj = hit.obj;
        }
        let clear = path.through_clouds(ctx, &hit, &mut col, rng);
        if bounce == 0 {
            out.obj = clouded_sky(&hit, clear, out.obj);
        }
        let Some(sf) = path.advance(ctx, regs, &hit, &mut col, M8::all_true(), cone) else { break };
        let act = path.active;
        let v = -path.ray.d;
        let entering = path.ray.d.dot(sf.n).lt(F8::zero());
        let nf = facing(path.ray.d, &sf);
        let cosi = v.dot(nf).max(F8::zero());
        let f0 = V8::lerp(V8::splat(Vec3::splat(0.04)), sf.albedo, sf.metal);
        if bounce == 0 && s.headlight > 0.0 {
            // (through the fog and whatever the path went through, as all light)
            col = V8::select(act, path.thr.hadamard(headlight(ctx, regs, &sf, nf, act)) + col, col);
        }

        // the diffuse part: sun, lights, sky ambient
        let kd = (F8::one() - sf.metal) * (F8::one() - sf.trans);
        let lit_lanes = act & sf.trans.lt(F8::splat(0.999));
        if lit_lanes.any() {
            let (light, vis) = direct_light(ctx, regs, &sf, nf, hit.t, lit_lanes, rng, false);
            let ao = if s.ao { occlusion(ctx, regs, &sf, nf, lit_lanes) * sf.ao } else { sf.ao };
            let amb = V8::splat(env.amb_col) * ((nf.y * 0.45 + 0.55) * ao * (AMBIENT * env.ambient));
            let diffuse = (light + amb + through(ctx, &sf, path.ray.d, nf)).hadamard(sf.albedo) * kd;
            let spec = sun_spec(ctx, nf, v, sf.rough, f0, vis);
            col = V8::select(lit_lanes, path.thr.hadamard(diffuse + spec) + col, col);
            // rough metal: the sky around the normal stands in for its blurred reflection
            let metal = lit_lanes & sf.metal.gt(F8::splat(0.0));
            if metal.any() {
                let gloss = (F8::one() - sf.rough) * (F8::one() - sf.rough);
                let env_l = atmos::sky(ctx, nf, M8::none_set()) * ((F8::one() - gloss) * sf.metal * ao * 0.6);
                col = V8::select(metal, path.thr.hadamard(env_l.hadamard(sf.albedo)) + col, col);
            }
        }
        if bounce == s.bounces {
            break;
        }

        // continuation: one ray per lane. A rough surface scatters it about
        // the mirror (or refracted) direction, as the path tracer does, so what
        // it reflects or lets through blurs as the samples accumulate.
        let refl_d = path.ray.d.reflect(nf);
        let cdir = cosine_dir(nf, rng.next(), rng.next());
        let r2 = sf.rough * sf.rough;
        let gdir = V8::lerp(refl_d, cdir, r2).normalize();
        let gdir = V8::select(gdir.dot(nf).gt(F8::zero()), gdir, refl_d);
        let off = offset(&sf, hit.t);
        let (above, above_d) = (nf.mul_add(off, sf.p), nf.mul_add(off, sf.dp));
        let (below, below_d) = (nf.mul_add(-off, sf.p), nf.mul_add(-off, sf.dp));
        let mut new_d = gdir;
        let mut new_o = above;
        let mut new_od = above_d;
        let mut w = V8::zero();
        let mut keep = M8::none_set();
        let mut inside = path.ray.inside;
        let tm = act & sf.trans.gt(F8::splat(0.5));
        if tm.any() {
            let eta = F8::select(entering, F8::one() / sf.ior, sf.ior);
            let (rd, tir) = refract(path.ray.d, nf, eta);
            let r0 = (sf.ior - 1.0) / (sf.ior + 1.0);
            let fr = F8::select(tir, F8::one(), schlick1(r0 * r0, cosi));
            let through = tm.and_not(tir);
            if bounce < 2 && through.any() {
                // the reflection, traced once with simple shading
                let rc = simple(ctx, regs, above, above_d, gdir, through, cone);
                col = V8::select(through, path.thr.hadamard(rc) * fr + col, col);
            }
            let frost = V8::lerp(rd, -cdir, r2).normalize();
            new_d = V8::select(through, frost, new_d);
            new_o = V8::select(through, below, new_o);
            new_od = V8::select(through, below_d, new_od);
            let wt = F8::select(tir, F8::one(), (F8::one() - fr) * sf.trans);
            w = V8::select(tm, V8::new(wt, wt, wt), w);
            keep |= tm;
            path.cross(through, entering, &sf, hit.obj, &mut inside);
        }
        let om = act.and_not(tm);
        if om.any() {
            let gloss = (F8::one() - sf.rough) * (F8::one() - sf.rough);
            let ww = schlick(f0, cosi) * gloss;
            let rm = om & ww.max_comp().gt(F8::splat(0.02));
            w = V8::select(rm, ww, w);
            keep |= rm;
        }
        path.thr = path.thr.hadamard(w);
        path.active = keep & path.thr.max_comp().gt(F8::splat(0.003));
        if path.active.none() {
            break;
        }
        path.ray = Ray::new(new_o, new_od, new_d, inside);
    }
    out.col = col;
    out
}

/// One traced reflection with simple shading (sun without shadow, ambient,
/// sky reflections for shiny lanes): fraktal256's trace2.
fn simple(ctx: &Ctx, regs: &mut Regs, o: V8, od: V8, d: V8, lm: M8, cone: f32) -> V8 {
    let env = ctx.env;
    let ray = Ray::new(o, od, d, I8::splat(-1));
    let hit = trace::intersect(ctx, regs, &ray, F8::splat(ctx.s.max_dist), lm, cone);
    let miss = lm & hit.obj.eq(I8::splat(-1));
    let mut col = V8::zero();
    if miss.any() {
        col = V8::select(miss, atmos::sky(ctx, d, M8::all_true()), col);
    }
    let hm = lm.and_not(miss);
    if hm.any() {
        let sf = surface(ctx, regs, &ray, &hit, hm, cone);
        let nf = facing(d, &sf);
        let ndl = nf.dot3(env.sun_dir).max(F8::zero());
        let light = V8::splat(env.sun_col) * ndl + V8::splat(env.amb_col) * ((nf.y * 0.45 + 0.55) * sf.ao * (AMBIENT * env.ambient));
        let kd = (F8::one() - sf.metal) * (F8::one() - sf.trans);
        let shiny = sf.metal.max(sf.trans);
        let refl = atmos::sky(ctx, d.reflect(nf), M8::none_set());
        let c = light.hadamard(sf.albedo) * kd + sf.emission + refl.hadamard(V8::lerp(V8::one(), sf.albedo, sf.metal)) * (shiny * 0.5) + sf.scatter;
        let (ft, fin) = atmos::fog(ctx, hit.t, d);
        col = V8::select(hm, c * ft + fin, col);
    }
    col
}

/// Where the headlight is, from the camera: at it, or with shadows a
/// thirtieth of its reach above it (a lamp at the eye hides every shadow it
/// casts), and no more than half the way to the nearest surface (deep in a
/// fractal's tunnel a millimetre up was inside the walls, and shadowed
/// everything). From the camera, not in world coordinates: a few micrometres
/// up stays exact (Surf.dp).
pub fn lamp(ctx: &Ctx) -> Vec3 {
    let (cam, s) = (ctx.cam, ctx.s);
    if !s.headlight_shadows || s.headlight <= 0.0 {
        return cam.fine;
    }
    let lift = s.headlight_reach.max(1e-4) / 30.0;
    let clear = trace::scene_dist(ctx, &mut Regs::new(), V8::splat(cam.pos), V8::splat(cam.fine), lift * 2.0, M8::none_set()).lane(0);
    cam.fine + cam.u * lift.min(clear * 0.5).max(0.0)
}

/// The headlight on the surfaces the camera rays hit (lanes `lm`), fading
/// out toward its reach: diffuse, and metal's tinted sheen. Its shadows,
/// when it casts them, are hard (a small lamp).
#[inline(always)]
fn headlight(ctx: &Ctx, regs: &mut Regs, sf: &Surf, nf: V8, lm: M8) -> V8 {
    let s = ctx.s;
    let to = V8::splat(ctx.lamp) - sf.dp;
    let d2 = to.length_sq();
    let inv = d2.max(F8::splat(1e-12)).rsqrt();
    let l = to * inv;
    let reach = s.headlight_reach.max(1e-4);
    let fade = d2.neg_mul_add(F8::splat(1.0 / (reach * reach)), F8::one()).max(F8::zero());
    let kd = (F8::one() - sf.metal) * (F8::one() - sf.trans);
    let mut w = sf.metal.mul_add(F8::splat(0.5), kd) * nf.dot(l).max(F8::zero()) * (fade * fade) * (s.headlight * 2.0);
    let lit = lm & w.gt(F8::zero());
    if s.headlight_shadows && lit.any() {
        let dist = d2.sqrt();
        let off = offset(sf, dist);
        w *= trace::shadow(ctx, regs, nf.mul_add(off, sf.p), nf.mul_add(off, sf.dp), l, dist, lit, false, 1.0, -1);
    }
    sf.albedo.hadamard(V8::splat(Vec3::new(1.0, 0.94, 0.85))) * w
}

/// Monte Carlo path tracing.
pub fn path(ctx: &Ctx, regs: &mut Regs, rng: &mut Rng8, o: V8, d: V8, cone: f32) -> Sample {
    let s = ctx.s;
    let mut out = Sample { col: V8::zero(), steps: I8::zero(), obj: I8::splat(-1) };
    let mut path = Path::new(ctx, o, d);
    let mut col = V8::zero();
    let mut disk = M8::all_true();
    // what the path had gathered before this bounce, and whether it had
    // bounced diffusely by then
    let mut before = (col, M8::none_set());
    for bounce in 0..=s.bounces {
        clamp_indirect(&mut col, before);
        before = (col, path.diffused);
        let hit = trace::intersect(ctx, regs, &path.ray, F8::splat(s.max_dist), path.active, cone);
        if bounce == 0 {
            out.steps = hit.steps;
            out.obj = hit.obj;
        }
        let clear = path.through_clouds(ctx, &hit, &mut col, rng);
        if bounce == 0 {
            out.obj = clouded_sky(&hit, clear, out.obj);
        }
        let Some(sf) = path.advance(ctx, regs, &hit, &mut col, disk, cone) else { break };
        let act = path.active;
        let entering = path.ray.d.dot(sf.n).lt(F8::zero());
        let nf = facing(path.ray.d, &sf);
        let cosi = (-path.ray.d).dot(nf).max(F8::zero());
        if bounce == 0 && s.headlight > 0.0 {
            // (through the fog and whatever the path went through, as all light)
            col = V8::select(act, path.thr.hadamard(headlight(ctx, regs, &sf, nf, act)) + col, col);
        }

        // pick a lobe per lane: dielectric, metal, specular coat or diffuse
        let tm = act & rng.next().lt(sf.trans);
        let rest = act.and_not(tm);
        let mm = rest & rng.next().lt(sf.metal);
        let dm = rest.and_not(mm);
        let fs = schlick1(F8::splat(0.04), cosi) * (F8::one() - sf.rough);
        let sm = dm & rng.next().lt(fs);
        let diff = dm.and_not(sm);
        path.diffused |= diff;
        if diff.any() {
            let (light, _) = direct_light(ctx, regs, &sf, nf, hit.t, diff, rng, true);
            let light = light + through(ctx, &sf, path.ray.d, nf);
            col = V8::select(diff, path.thr.hadamard(light.hadamard(sf.albedo)) + col, col);
        }
        if bounce == s.bounces {
            break;
        }

        let refl_d = path.ray.d.reflect(nf);
        let off = offset(&sf, hit.t);
        let (above, above_d) = (nf.mul_add(off, sf.p), nf.mul_add(off, sf.dp));
        let (below, below_d) = (nf.mul_add(-off, sf.p), nf.mul_add(-off, sf.dp));
        let cdir = cosine_dir(nf, rng.next(), rng.next());
        let r2 = sf.rough * sf.rough;
        let gdir = V8::lerp(refl_d, cdir, r2).normalize();
        let mut new_d = V8::select(diff, cdir, gdir);
        let mut new_o = above;
        let mut new_od = above_d;
        let mut w = V8::select(diff, sf.albedo, V8::one());
        w = V8::select(mm, schlick(sf.albedo, cosi), w);
        let mut inside = path.ray.inside;
        let mut reflected = sm | mm;
        if tm.any() {
            let eta = F8::select(entering, F8::one() / sf.ior, sf.ior);
            let (rd, tir) = refract(path.ray.d, nf, eta);
            let r0 = (sf.ior - 1.0) / (sf.ior + 1.0);
            let fr = schlick1(r0 * r0, cosi);
            let mirror = tm & (tir | rng.next().lt(fr));
            let through = tm.and_not(mirror);
            let frost = V8::lerp(rd, -cdir, r2).normalize();
            new_d = V8::select(through, frost, new_d);
            new_o = V8::select(through, below, new_o);
            new_od = V8::select(through, below_d, new_od);
            path.cross(through, entering, &sf, hit.obj, &mut inside);
            reflected |= mirror;
        }
        // reflections that end up under the surface are lost
        let lost = reflected & new_d.dot(nf).le(F8::zero());
        disk = tm | (reflected & sf.rough.lt(F8::splat(0.25)));
        path.thr = path.thr.hadamard(w);
        path.active = act.and_not(lost);
        if bounce >= 2 {
            let q = path.thr.max_comp().clamp(0.05, 1.0);
            let survive = rng.next().lt(q);
            path.thr = path.thr * (F8::one() / q);
            path.active &= survive;
        }
        if path.active.none() {
            break;
        }
        path.ray = Ray::new(new_o, new_od, new_d, inside);
    }
    clamp_indirect(&mut col, before);
    out.col = col;
    out
}

/// The brightest light one bounce may add to a path that has already
/// bounced off a diffuse surface. Rare paths that find something very
/// bright that way (the sun through a glass ball: a caustic; a small lamp;
/// a Russian-roulette survivor) are fireflies that more samples barely
/// average out; capped, the image settles. What the camera sees directly,
/// and in mirrors and glass, is not capped.
const MAX_INDIRECT: f32 = 8.0;

/// Caps what a bounce added to `col` since `before.0`, on the lanes that
/// had bounced diffusely before it (`before.1`).
#[inline(always)]
fn clamp_indirect(col: &mut V8, before: (V8, M8)) {
    let (was, lanes) = before;
    if lanes.none() {
        return;
    }
    let added = *col - was;
    let peak = added.max_comp();
    let k = F8::select(lanes & peak.gt(F8::splat(MAX_INDIRECT)), F8::splat(MAX_INDIRECT) / peak.max(F8::splat(1e-6)), F8::one());
    *col = added.mul_add(k, was);
}

/// The debug views of the primary rays.
pub fn debug(ctx: &Ctx, regs: &mut Regs, o: V8, d: V8, cone: f32, mode: Mode) -> Sample {
    let s = ctx.s;
    let ray = Ray::new(o, o - V8::splat(ctx.anchor) + V8::splat(ctx.cam.fine), d, I8::splat(-1));
    let hit = trace::intersect(ctx, regs, &ray, F8::splat(s.max_dist), M8::all_true(), cone);
    let hm = hit.obj.gt(I8::splat(-1));
    let col = match mode {
        Mode::Steps => super::ramp(hit.steps.to_f8() * (2.5 / s.max_steps as f32)),
        Mode::Depth => {
            let v = F8::one() - (hit.t + 1.0).ln() * (1.0 / (s.max_dist + 1.0).ln());
            V8::new(v, v, v).and_mask(hm)
        }
        _ => {
            let sf = surface(ctx, regs, &ray, &hit, hm, cone);
            match mode {
                Mode::Normals => (sf.n * 0.5 + V8::splat(Vec3::splat(0.5))).and_mask(hm),
                // its price on the ramp, shaded by how it faces the camera
                Mode::Cost if ctx.prices.len() == ctx.scene.objects.len() && !ctx.prices.is_empty() => {
                    let t = F8::gather(ctx.prices, I8::select(hm, hit.obj, I8::zero()));
                    let face = sf.n.dot(d).abs().mul_add(F8::splat(0.65), F8::splat(0.35));
                    V8::select(hm, super::ramp(t) * face, atmos::sky(ctx, d, M8::all_true()) * 0.15)
                }
                _ => V8::select(hm, sf.albedo, atmos::sky(ctx, d, M8::all_true()) * 0.5),
            }
        }
    };
    Sample { col, steps: hit.steps, obj: hit.obj }
}

trait AndMask {
    fn and_mask(self, m: M8) -> V8;
}

impl AndMask for V8 {
    fn and_mask(self, m: M8) -> V8 {
        V8::new(self.x.and(m), self.y.and(m), self.z.and(m))
    }
}
