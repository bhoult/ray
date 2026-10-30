//! Sky and fog (fraktal256's atm_sky): the horizon-to-zenith gradient, haze
//! below the horizon, noise clouds on a plane, the sun's glow and disk; or a
//! night sky of stars; or none.

use super::Ctx;
use super::shade::{lowbias, unit};
use crate::math::Vec3;
use crate::noise::noised8;
use crate::scene::Sky;
use crate::simd::{F8, I8, M8, V8};

const CLOUD_SEED: i32 = 0x5bd1_e995;
const CLOUD_HEIGHT: f32 = 1000.0;
const CLOUD_FREQ: f32 = 0.0012;

/// The night sky: its glow at the zenith and the horizon, the band where the
/// stars crowd (the plane it lies in, by its normal) and the band's glow.
const NIGHT_ZEN: Vec3 = Vec3::new(0.0012, 0.0016, 0.0040);
const NIGHT_HOR: Vec3 = Vec3::new(0.0035, 0.0042, 0.0075);
const BAND: Vec3 = Vec3::new(0.36, 0.62, 0.70);
const BAND_COL: Vec3 = Vec3::new(0.0040, 0.0040, 0.0060);
/// Cells per unit of direction: at most one star in each (coarse enough that
/// a star's glow never reaches the edge of its cell).
const STAR_GRID: f32 = 45.0;

/// The sky along unit directions `d`. Lanes in `disk` include the sun's disk
/// (the path tracer leaves it out where it samples the sun directly).
pub fn sky(ctx: &Ctx, d: V8, disk: M8) -> V8 {
    let env = &ctx.env;
    match env.sky {
        Sky::Off => return V8::zero(),
        Sky::Stars => return stars(ctx, d),
        Sky::Day => {}
    }
    let sun = |v: V8| v.dot3(env.sun_dir).max(F8::zero());
    let g = d.y.max(F8::zero()).sqrt();
    let col = V8::lerp(V8::splat(env.sky_hor), V8::splat(env.sky_zen), g);
    let haze = (d.y * -8.0).clamp(0.0, 1.0);
    let mut col = V8::lerp(col, V8::splat(env.fog_col), haze);

    // clouds, sampled where the ray climbs; alpha also hides the sun
    let mut alpha = F8::zero();
    if ctx.s.clouds && env.clouds > 0.0 && !d.y.lt(F8::zero()).all() {
        let t = F8::splat(CLOUD_HEIGHT) / d.y.max(F8::splat(0.02));
        let u = t * d.x * CLOUD_FREQ;
        let v = t * d.z * CLOUD_FREQ;
        let seed = I8::splat(CLOUD_SEED);
        let (n1, _, _) = noised8(u, v, seed);
        let (n2, _, _) = noised8(u * 2.3, v * 2.3, seed);
        let density = n2 * 0.5 + n1;
        // fraktal's cover threshold: cover 0.42 -> threshold 0.42; more cover, lower threshold
        let threshold = 1.26 - env.clouds * 2.0;
        alpha = ((density - threshold) * 2.5).clamp(0.0, 0.85);
        alpha *= (d.y * 6.0).clamp(0.0, 1.0);
        let lit = sun(d);
        let lit = (lit * lit) * (lit * lit);
        let cloud = (V8::splat(env.sky_hor * 2.0) + V8::splat(env.sun_col) * lit) * 0.6;
        col = V8::lerp(col, cloud, alpha);
    }

    // the sun's glow and disk (where the sun is, also after it set, while
    // the glow lingers)
    let s = d.dot3(env.glow_dir).max(F8::zero());
    let s2 = s * s;
    let s4 = s2 * s2;
    let s8 = s4 * s4;
    let s16 = s8 * s8;
    let s32 = s16 * s16;
    let s64 = s32 * s32;
    let glow = s8 * 0.3;
    let glow = s64.mul_add(F8::splat(0.8), glow);
    let s1024 = {
        let s128 = s64 * s64;
        let s256 = s128 * s128;
        let s512 = s256 * s256;
        s512 * s512
    };
    let above = d.y.gt(F8::splat(-0.01));
    let glow = (s1024 * (F8::one() - alpha)).and(disk & above).mul_add(F8::splat(12.0), glow);
    let mut col = col + V8::splat(env.glow_col) * glow;
    if env.moon > 0.0 {
        col += moon(env, d, disk & above, alpha);
    }
    if env.night > 0.0 {
        col += stars(ctx, d) * ((F8::one() - alpha) * env.night * (d.y * 20.0 + 0.2).clamp(0.0, 1.0));
    }
    col
}

/// The moon's angular radius (0.8 degrees: three times the real one, so it
/// reads at a normal field of view), as 1 - cos.
const MOON_R: f32 = 9.75e-5;

/// The moon: a disk with darker seas and a soft edge, and a faint halo.
fn moon(env: &crate::scene::Env, d: V8, disk: M8, alpha: F8) -> V8 {
    let m = d.dot3(env.moon_dir);
    let off = F8::one() - m;
    let inside = ((F8::one() - off * (1.0 / MOON_R)) * 6.0).clamp(0.0, 1.0);
    let mut out = V8::zero();
    if inside.and(disk).gt(F8::zero()).any() {
        let seed = I8::splat(0x2c1b_3c6d);
        let (n, _, _) = noised8((d.x - env.moon_dir.x) * 900.0, (d.y - env.moon_dir.y) * 900.0 + (d.z - env.moon_dir.z) * 900.0, seed);
        let face = F8::splat(1.0) - n * 0.3;
        out = V8::new(F8::splat(4.0), F8::splat(3.9), F8::splat(3.6)) * (face * inside).and(disk);
    }
    let h = m.max(F8::zero());
    let h4 = (h * h) * (h * h);
    let h16 = h4 * h4 * (h4 * h4);
    let halo = h16 * h16 * 0.02;
    (out + V8::new(halo * 0.8, halo * 0.9, halo)) * ((F8::one() - alpha) * env.moon)
}

/// The second layer of stars sees the sky turned by this (orthonormal rows),
/// so that the two grids' cells never line up into rows of stars.
const TURN: [Vec3; 3] = [Vec3::new(0.36, 0.48, -0.80), Vec3::new(-0.80, 0.60, 0.0), Vec3::new(0.48, 0.64, 0.60)];

/// The night sky: its glow, the band, and two layers of stars on grids of
/// different sizes, one turned against the other.
fn stars(ctx: &Ctx, d: V8) -> V8 {
    let up = d.y.max(F8::zero()).sqrt();
    let b = d.dot3(BAND);
    let band = (b * b * -40.0).exp2();
    let col = V8::lerp(V8::splat(NIGHT_HOR), V8::splat(NIGHT_ZEN), up) + V8::splat(BAND_COL) * band;
    let turned = V8::new(d.dot3(TURN[0]), d.dot3(TURN[1]), d.dot3(TURN[2]));
    col + star_layer(ctx.cone, d, STAR_GRID, 0, band, None) + star_layer(ctx.cone, turned, STAR_GRID * 0.83, 0x5f35_6495, band, None)
}

/// How much brighter or dimmer than in the traced image the stars along `d`
/// are at time `t` (seconds): each star swells and fades a little at its
/// own pace. The post pass adds this to the sky it shows, so the traced image
/// can keep accumulating while they twinkle.
pub fn twinkle(env: &crate::scene::Env, cone: f32, d: V8, t: f32) -> V8 {
    let weight = match env.sky {
        Sky::Stars => F8::one(),
        Sky::Day if env.night > 0.0 => (d.y * 20.0 + 0.2).clamp(0.0, 1.0) * env.night,
        _ => return V8::zero(),
    };
    let b = d.dot3(BAND);
    let band = (b * b * -40.0).exp2();
    let turned = V8::new(d.dot3(TURN[0]), d.dot3(TURN[1]), d.dot3(TURN[2]));
    (star_layer(cone, d, STAR_GRID, 0, band, Some(t)) + star_layer(cone, turned, STAR_GRID * 0.83, 0x5f35_6495, band, Some(t))) * weight
}

/// One layer of stars: a cell of a grid over directions holds one star or
/// none (more along the band), placed well inside the cell's patch of sky.
/// Each is a round glow about a pixel and a half across (a smaller one would
/// show as a square pixel, and flicker), the bright ones a little bigger;
/// most are faint.
/// With `twinkle` (a time), only the change twinkling makes then.
fn star_layer(cone: f32, d: V8, grid: f32, seed: i32, band: F8, twinkle: Option<f32>) -> V8 {
    let q = d * grid;
    let (fx, fy, fz) = (q.x.floor(), q.y.floor(), q.z.floor());
    let h = lowbias(
        fx.to_i8().mul_lo(I8::splat(0x8DA6_B343u32 as i32))
            ^ fy.to_i8().mul_lo(I8::splat(0xD816_3841u32 as i32))
            ^ fz.to_i8().mul_lo(I8::splat(0xCB1A_B31Fu32 as i32))
            ^ I8::splat(seed),
    );
    let (h2, h3, h4) = (lowbias(h ^ I8::splat(1)), lowbias(h ^ I8::splat(2)), lowbias(h ^ I8::splat(3)));
    let h5 = lowbias(h ^ I8::splat(4));
    let at = |f: F8, h: I8| unit(h).mul_add(F8::splat(0.3), f + 0.35);
    let sd = V8::new(at(fx, h2), at(fy, h3), at(fz, h4)).normalize();
    let qs = sd * grid;
    let inner = |q: F8, f: F8| {
        let r = q - f;
        r.ge(F8::splat(0.3)) & r.le(F8::splat(0.7))
    };
    let present = unit(h).lt(band.mul_add(F8::splat(0.3), F8::splat(0.25))) & inner(qs.x, fx) & inner(qs.y, fy) & inner(qs.z, fz);
    let u = unit(h5);
    let u2 = u * u;
    let u4 = u2 * u2;
    let bright = (u4 * u4).mul_add(F8::splat(6.0), u2.mul_add(F8::splat(0.15), F8::splat(0.02)));
    let sigma = u4.mul_add(F8::splat(0.6), F8::splat(0.62)) * cone.max(1e-5);
    let glow = ((d - sd).length_sq() / (sigma * sigma) * -core::f32::consts::LOG2_E).exp2();
    let tint = V8::lerp(V8::new(F8::one(), F8::splat(0.92), F8::splat(0.82)), V8::new(F8::splat(0.85), F8::splat(0.92), F8::one()), unit(h2));
    let amount = match twinkle {
        // up to 45% either way, at 0.25 to 1 swell a second
        Some(t) => (unit(lowbias(h ^ I8::splat(5))).mul_add(F8::splat(4.7), F8::splat(1.6)) * t + unit(lowbias(h ^ I8::splat(6))) * core::f32::consts::TAU).sin() * 0.45,
        None => F8::one(),
    };
    tint * (glow * bright * amount).and(present)
}

/// Fog over a segment of `dist` metres along `dir`: the transmittance and the
/// in-scattered light (the fog colour plus a sun-coloured lobe).
pub fn fog(ctx: &Ctx, dist: F8, dir: V8) -> (F8, V8) {
    let env = &ctx.env;
    if !ctx.s.fog || env.fog <= 0.0 {
        return (F8::one(), V8::zero());
    }
    let trans = (dist * -env.fog).exp2();
    let lobe = dir.dot3(env.sun_dir).max(F8::zero());
    let lobe = lobe * lobe;
    let lobe = lobe * lobe;
    let lobe = lobe * lobe * 0.4;
    let haze = V8::splat(env.fog_col) + V8::splat(env.sun_col) * lobe;
    (trans, haze * (F8::one() - trans))
}
