//! Marching through volume clouds (scene/volume.rs), eight rays at a time:
//! the light the droplets scatter toward the ray's origin, and how much of
//! what lies behind gets through. Sunlight (or moonlight) reaching each step
//! is found by a short march toward the light through the cloud itself;
//! single scattering with a forward and a back lobe (the silver lining when
//! looking toward the sun), a lift for light scattered many times over, and
//! the sky's fill light.

use super::Ctx;
use super::shade::Rng8;
use super::trace::{Ray, slab};
use crate::math::Vec3;
use crate::noise::perlin3;
use crate::scene::flame::FlameGrid;
use crate::scene::volume::{FlameVol, Volume};
use crate::simd::{F8, I8, M8, V8};

/// Steps through a cloud along a view ray.
const VIEW_STEPS: u32 = 40;
/// Steps through a cloud along a shadow ray.
const SHADOW_STEPS: u32 = 14;
/// The march toward the light from each step: where each sample is and the
/// length it stands for, in the cloud's metres (each twice the last).
const LIGHT_AT: [f32; 4] = [0.3, 1.2, 3.0, 6.6];
const LIGHT_LEN: [f32; 4] = [0.6, 1.2, 2.4, 4.8];
/// The sky's fill light inside a cloud, as a multiple of the scene's ambient.
const FILL: f32 = 1.2;

/// Local coordinates of world points.
#[inline(always)]
fn local(v: &Volume, p: V8) -> V8 {
    let q = p - V8::splat(v.pos);
    let m = &v.to_local.m;
    V8::new(q.dot3(m[0]), q.dot3(m[1]), q.dot3(m[2]))
}

/// Distance (local metres) to the surface of the puffs' smooth union.
#[inline(always)]
fn envelope(v: &Volume, q: V8) -> F8 {
    let k = v.blend.max(1e-4);
    let mut d = F8::splat(1e9);
    for b in &v.puffs {
        let e = (q - V8::splat(Vec3::new(b[0], b[1], b[2]))).length() - b[3];
        // polynomial smooth minimum
        let h = ((e - d) * (0.5 / k) + 0.5).clamp(0.0, 1.0);
        d = (d - e).mul_add(h, e) - h * (F8::one() - h) * k;
    }
    d
}

#[inline(always)]
fn smoothstep01(x: F8) -> F8 {
    let x = x.clamp(0.0, 1.0);
    x * x * (F8::splat(3.0) - x - x)
}

/// How far (local metres) turbulence moves the domain's surface at most, per
/// unit of `turbulence`.
pub const BILLOW: f32 = 2.4;

/// Density, 0..1, at local points `q` (`d_env` their envelope distance).
/// Turbulence noise (`octaves` of it) pushes the domain's surface in and
/// out, into billows, and thins what is left near it; the density fades in
/// over `softness` inside and above the flat base.
#[inline(always)]
fn density(v: &Volume, q: V8, d_env: F8, octaves: u32) -> F8 {
    let n = cloud_noise(q * v.frequency, octaves, v.detail, I8::splat(v.seed as i32));
    let d = d_env - n * (v.turbulence * BILLOW);
    let shape = smoothstep01(-d * (1.0 / v.softness)) * smoothstep01((q.y - (v.base - 0.8)) * (1.0 / 1.4));
    // the outer part thins where the noise is low
    let dens = shape * (F8::one() - (F8::splat(0.5) - n).clamp(0.0, 1.0) * v.turbulence.min(1.0) * (F8::one() - shape));
    if v.wisps <= 0.0 {
        return dens;
    }
    // wisps: a finer noise carves the inside into filaments, more of it as
    // `wisps` grows, so the cloud's inner structure shows
    let m = cloud_noise(q * (v.frequency * WISP_SCALE), octaves, v.detail, I8::splat((v.seed ^ WISP_SEED) as i32)) + 0.5;
    let keep = smoothstep01((m - v.wisps * 0.7) * (1.0 / (0.1 + 0.25 * (1.0 - v.wisps))));
    // (eased in, so the first half of the range already shows)
    let k = v.wisps.sqrt();
    dens * keep.mul_add(F8::splat(k), F8::splat(1.0 - k))
}

/// The wisps' noise: this much finer than the billows', on another seed.
const WISP_SCALE: f32 = 2.7;
const WISP_SEED: u32 = 0x9e37_79b9;

/// How strong each finer octave of the clouds' noise is, relative to the
/// last (Blender's Noise Texture roughness): with fBm's 0.5 the fine
/// octaves hardly showed, so `detail` seemed to do nothing.
pub const ROUGHNESS: f32 = 0.7;

/// The clouds' noise at `p`: `octaves` of 3D Perlin noise (each half the
/// size of the last, rotated like fbm3's) scaled to the range of
/// `full` octaves (the light's march takes fewer of them).
#[inline(always)]
fn cloud_noise(p: V8, octaves: u32, full: u32, seed: I8) -> F8 {
    let (mut sum, mut amp, mut q) = (F8::zero(), 1.0f32, p);
    for _ in 0..octaves {
        sum = perlin3(q, seed).mul_add(F8::splat(amp), sum);
        q = V8::new(q.x * 1.6 + q.z * 1.2, q.y * 2.0, q.z * 1.6 - q.x * 1.2);
        amp *= ROUGHNESS;
    }
    let norm = (1.0 - ROUGHNESS.powi(full as i32)) / (1.0 - ROUGHNESS);
    sum * (0.94 / norm)
}

/// Whether points with envelope distance `d_env` may have density (the
/// noise can push the surface out by up to half its reach).
#[inline(always)]
fn near(v: &Volume, d_env: F8) -> M8 {
    d_env.lt(F8::splat(v.turbulence * BILLOW * 0.5))
}

/// The light the clouds are lit by: direction and colour.
fn lit_by(env: &crate::scene::Env) -> (Vec3, Vec3) {
    if env.glow_col.max_comp() > env.sun_col.max_comp() { (env.glow_dir, env.glow_col) } else { (env.sun_dir, env.sun_col) }
}

/// Henyey-Greenstein phase for cosine `c`, as a multiple of the isotropic one.
#[inline(always)]
pub fn hg(g: f32, c: F8) -> F8 {
    let x = (c * (-2.0 * g)).mul_add(F8::one(), F8::splat(1.0 + g * g));
    F8::splat(1.0 - g * g) / (x * x.sqrt())
}

/// The light the clouds scatter toward the origin of `ray` before `tmax`
/// (lanes `active`), and the fraction of the light behind them that they let
/// through. The march starts at a random offset (`rng`), so that samples
/// accumulate into a smooth image.
pub fn march(ctx: &Ctx, ray: &Ray, tmax: F8, active: M8, rng: &mut Rng8) -> (V8, F8) {
    let sc = ctx.scene;
    let mut light = V8::zero();
    let mut trans = F8::one();
    if sc.volumes.is_empty() || active.none() {
        return (light, trans);
    }
    let env = ctx.env;
    // the light: the sun, or after sunset whichever is brighter of its
    // lingering glow (high clouds still catch it, from below) and the moon
    let (ldir, lcol) = lit_by(env);
    let sun = V8::splat(ldir);
    let c = ray.d.dot3(ldir);
    let phase = hg(0.6, c) * 0.7 + hg(-0.3, c) * 0.3;
    let fill = V8::splat(env.amb_col) * (FILL * super::shade::AMBIENT * env.ambient);
    let jitter = rng.next();
    for v in &sc.volumes {
        if let Some(f) = &v.flame {
            light += flame_march(ctx, v, f, ray, tmax, active, jitter, &mut trans, (ldir, lcol), phase, fill);
            continue;
        }
        let (t0, t1) = slab(ray, v.aabb.min, v.aabb.max);
        let (t0, t1) = (t0.max(F8::zero()), t1.min(tmax));
        let mut lanes = active & t0.lt(t1) & trans.gt(F8::splat(0.01));
        if lanes.none() {
            continue;
        }
        let dt = (t1 - t0) * (1.0 / VIEW_STEPS as f32);
        let mut t = jitter.mul_add(dt, t0);
        let color = V8::splat(v.color);
        let mut here = V8::zero();
        for _ in 0..VIEW_STEPS {
            let p = V8::at(ray.o, ray.d, t);
            let q = local(v, p);
            let de = envelope(v, q);
            let inside = lanes & near(v, de);
            if inside.any() {
                let s = density(v, q, de, v.detail).and(inside) * v.sigma;
                if s.gt(F8::zero()).any() {
                    // the light's way in, through the cloud
                    let mut tau = F8::zero();
                    for k in 0..4 {
                        let ql = local(v, sun.mul_add(F8::splat(LIGHT_AT[k] * v.scale), p));
                        let dl = envelope(v, ql);
                        tau = (density(v, ql, dl, 2) * v.sigma).mul_add(F8::splat(LIGHT_LEN[k] * v.scale), tau);
                    }
                    let direct = (-tau).exp() * phase;
                    // light scattered many times: dimmed far less, in every direction
                    let many = (tau * -0.2).exp() * 0.3;
                    // the sky's light, more near the top
                    let up = smoothstep01((q.y - v.base) * (1.0 / 6.0)).mul_add(F8::splat(0.55), F8::splat(0.45));
                    let src = (V8::splat(lcol) * (direct + many) + fill * up).hadamard(color);
                    let a = F8::one() - (-(s * dt)).exp();
                    here += src * (trans * a);
                    trans *= F8::one() - a;
                }
            }
            t += dt;
            lanes &= t.lt(t1) & trans.gt(F8::splat(0.01));
            if lanes.none() {
                break;
            }
        }
        // the haze between the eye and the cloud dims it
        let (fog, _) = super::atmos::fog(ctx, t0, ray.d);
        light += here * fog;
    }
    (light, trans)
}

/// The nearest cloud along the ray from `o` along unit `d`, by its puffs
/// (not the noise): the distance and the document's object (selection).
pub fn pick(scene: &crate::scene::Scene, o: Vec3, d: Vec3) -> Option<(f32, usize)> {
    let mut best: Option<(f32, usize)> = None;
    for v in &scene.volumes {
        let r = |c: f32| 1.0 / c.abs().max(1e-12).copysign(c);
        let inv = Vec3::new(r(d.x), r(d.y), r(d.z));
        let (a, b) = (v.aabb.min - o, v.aabb.max - o);
        let (lo, hi) = (Vec3::new(a.x * inv.x, a.y * inv.y, a.z * inv.z), Vec3::new(b.x * inv.x, b.y * inv.y, b.z * inv.z));
        let t0 = lo.min(hi).max_comp().max(0.0);
        let t1 = lo.max(hi).x.min(lo.max(hi).y).min(lo.max(hi).z);
        if t0 >= t1 || best.is_some_and(|(t, _)| t0 > t) {
            continue;
        }
        // a flame: the first point dense enough along the ray, in small steps
        if let Some(f) = &v.flame {
            let dt = (t1 - t0) / 200.0;
            let mut t = t0;
            while t < t1 {
                let q = v.to_local.apply(o + d * t - v.pos);
                if f.grid.density_at([q.x, q.y, q.z]) > PICK_DENSITY {
                    if best.is_none_or(|(b, _)| t < b) {
                        best = Some((t, v.doc_index));
                    }
                    break;
                }
                t += dt;
            }
            continue;
        }
        // sphere tracing through the puffs' smooth union, in local metres
        let mut t = t0;
        while t < t1 {
            let q = v.to_local.apply(o + d * t - v.pos);
            let e = envelope(v, V8::splat(q)).lane(0);
            if e < 0.05 {
                if best.is_none_or(|(b, _)| t < b) {
                    best = Some((t, v.doc_index));
                }
                break;
            }
            t += (e * v.min_scale).max(0.05);
        }
    }
    best
}

/// The fraction of light the clouds let through between `o` and `o + d *
/// tmax` (shadow rays: a coarser march, no light).
pub fn transmittance(ctx: &Ctx, o: V8, d: V8, tmax: F8, active: M8) -> F8 {
    let sc = ctx.scene;
    let mut trans = F8::one();
    if sc.volumes.is_empty() || active.none() {
        return trans;
    }
    let ray = Ray::new(o, o - V8::splat(ctx.anchor), d, I8::splat(-1));
    // a fixed offset per point, so the steps do not line up into bands
    let jitter = super::shade::unit(super::shade::lowbias((o.x * 37.0 + o.z * 91.0 + o.y * 13.0).to_i8()));
    for v in &sc.volumes {
        if v.flame.is_some() && v.sigma <= 0.0 {
            continue;
        }
        let (t0, t1) = slab(&ray, v.aabb.min, v.aabb.max);
        let (t0, t1) = (t0.max(F8::zero()), t1.min(tmax));
        let mut lanes = active & t0.lt(t1);
        if lanes.none() {
            continue;
        }
        if let Some(f) = &v.flame {
            let dt = (t1 - t0) * (1.0 / FLAME_SHADOW_STEPS as f32);
            let mut t = jitter.mul_add(dt, t0);
            let mut tau = F8::zero();
            for _ in 0..FLAME_SHADOW_STEPS {
                let (dens, _) = flame_sample(&f.grid, local(v, V8::at(o, d, t)), lanes, false);
                tau = (dens * v.sigma).mul_add(dt, tau);
                t += dt;
                lanes &= t.lt(t1);
                if lanes.none() {
                    break;
                }
            }
            trans *= (-tau).exp();
            continue;
        }
        let dt = (t1 - t0) * (1.0 / SHADOW_STEPS as f32);
        let mut t = jitter.mul_add(dt, t0);
        let mut tau = F8::zero();
        for _ in 0..SHADOW_STEPS {
            let q = local(v, V8::at(o, d, t));
            let de = envelope(v, q);
            let inside = lanes & near(v, de);
            if inside.any() {
                tau = (density(v, q, de, 2).and(inside) * v.sigma).mul_add(dt, tau);
            }
            t += dt;
            lanes &= t.lt(t1);
            if lanes.none() {
                break;
            }
        }
        trans *= (-tau).exp();
    }
    trans
}

// ---- fractal flames (scene/flame.rs): a grid of density and colour

/// Steps through a flame along a view ray, and along a shadow ray.
const FLAME_STEPS: u32 = 64;
const FLAME_SHADOW_STEPS: u32 = 16;
/// The march toward the light from each step: where each sample is and the
/// length it stands for, as fractions of the flame's longest side.
const FLAME_LIGHT_AT: [f32; 4] = [0.02, 0.06, 0.14, 0.3];
const FLAME_LIGHT_LEN: [f32; 4] = [0.04, 0.04, 0.12, 0.2];
/// The density a click must find to pick a flame.
const PICK_DENSITY: f32 = 0.08;

/// The grid's density 0..1 at local points `q` (lanes `lanes`), and with
/// `color` the colour times the density (premultiplied, so that the colour
/// of an empty voxel does not darken its neighbours), both trilinear.
#[inline(always)]
fn flame_sample(g: &FlameGrid, q: V8, lanes: M8, color: bool) -> (F8, V8) {
    let inv = 1.0 / g.cell;
    let at = |c: F8, a: usize| (c - F8::splat(g.lo[a])) * inv - F8::splat(0.5);
    let (gx, gy, gz) = (at(q.x, 0), at(q.y, 1), at(q.z, 2));
    let top = |a: usize| F8::splat((g.n[a] - 1) as f32);
    let inside = lanes & gx.ge(F8::zero()) & gy.ge(F8::zero()) & gz.ge(F8::zero()) & gx.le(top(0)) & gy.le(top(1)) & gz.le(top(2));
    if inside.none() {
        return (F8::zero(), V8::zero());
    }
    let low = |c: F8, a: usize| c.floor().clamp(0.0, (g.n[a] - 2) as f32);
    let (x0, y0, z0) = (low(gx, 0), low(gy, 1), low(gz, 2));
    let (fx, fy, fz) = (gx - x0, gy - y0, gz - z0);
    let (nx, nxy) = (g.n[0] as i32, (g.n[0] * g.n[1]) as i32);
    let base = x0.to_i8() + y0.to_i8().mul_lo(I8::splat(nx)) + z0.to_i8().mul_lo(I8::splat(nxy));
    let mut d = F8::zero();
    let mut dc = V8::zero();
    for k in 0..8 {
        let (dx, dy, dz) = (k & 1, (k >> 1) & 1, (k >> 2) & 1);
        let w = if dx == 1 { fx } else { F8::one() - fx } * if dy == 1 { fy } else { F8::one() - fy } * if dz == 1 { fz } else { F8::one() - fz };
        let word = I8::gather(&g.data, base + (dx + dy * nx + dz * nxy));
        let dk = (word & 0xffff).to_f8() * (1.0 / 65535.0);
        let wd = (w * dk).and(inside);
        d += wd;
        if color {
            let r = (word.shr::<27>() & 31).to_f8() * (1.0 / 31.0);
            let gg = (word.shr::<21>() & 63).to_f8() * (1.0 / 63.0);
            let b = (word.shr::<16>() & 31).to_f8() * (1.0 / 31.0);
            dc += V8::new(r, gg, b) * wd;
        }
    }
    (d, dc)
}

/// A flame between the ray's origin and `tmax`: the light it glows and
/// scatters this way (dimmed by the haze in front of it); `trans` is dimmed
/// by what it stops.
#[allow(clippy::too_many_arguments)]
fn flame_march(ctx: &Ctx, v: &Volume, f: &FlameVol, ray: &Ray, tmax: F8, active: M8, jitter: F8, trans: &mut F8, (ldir, lcol): (Vec3, Vec3), phase: F8, fill: V8) -> V8 {
    let (t0, t1) = slab(ray, v.aabb.min, v.aabb.max);
    let (t0, t1) = (t0.max(F8::zero()), t1.min(tmax));
    let mut lanes = active & t0.lt(t1) & trans.gt(F8::splat(0.01));
    if lanes.none() {
        return V8::zero();
    }
    let dt = (t1 - t0) * (1.0 / FLAME_STEPS as f32);
    let mut t = jitter.mul_add(dt, t0);
    let sun = V8::splat(ldir);
    let mut here = V8::zero();
    for _ in 0..FLAME_STEPS {
        let p = V8::at(ray.o, ray.d, t);
        let (d, dc) = flame_sample(&f.grid, local(v, p), lanes, true);
        if d.gt(F8::zero()).any() {
            let s = d * v.sigma;
            let a = F8::one() - (-(s * dt)).exp();
            // its glow, as much of the step's as gets out of the step
            let tiny = F8::splat(1e-6);
            let w = F8::select(s.gt(tiny), a / s.max(tiny), dt);
            let mut src = dc * (w * f.glow);
            if v.sigma > 0.0 {
                // the light's way in, through the flame
                let mut tau = F8::zero();
                for k in 0..4 {
                    let ql = local(v, sun.mul_add(F8::splat(FLAME_LIGHT_AT[k] * f.size), p));
                    let (dl, _) = flame_sample(&f.grid, ql, lanes, false);
                    tau = (dl * v.sigma).mul_add(F8::splat(FLAME_LIGHT_LEN[k] * f.size), tau);
                }
                let direct = (-tau).exp() * phase;
                let many = (tau * -0.2).exp() * 0.3;
                let albedo = dc * (F8::one() / d.max(tiny));
                src += (V8::splat(lcol) * (direct + many) + fill).hadamard(albedo) * a;
            }
            here += src * *trans;
            *trans *= F8::one() - a;
        }
        t += dt;
        lanes &= t.lt(t1) & trans.gt(F8::splat(0.01));
        if lanes.none() {
            break;
        }
    }
    let (fog, _) = super::atmos::fog(ctx, t0, ray.d);
    here * fog
}
