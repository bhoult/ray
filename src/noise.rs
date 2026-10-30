//! Noise on eight points at a time: fraktal's 2D value noise with analytic
//! derivatives (`noised8`: terrain erosion, clouds, waves) and Perlin gradient
//! noise in 2D (heightmaps) and 3D (surface displacement).

use crate::simd::{F8, I8, M8, V8};

const K1: i32 = 0x8DA6_B343u32 as i32;
const K2: i32 = 0xD816_3841u32 as i32;
const K3: i32 = 0x2C1B_3C6Du32 as i32;
const K4: i32 = 0xCB1A_B31Fu32 as i32;

/// The integer hash all the noises share: 32 bits from a lattice key.
#[inline(always)]
fn mix(h: I8) -> I8 {
    let h = h ^ h.shr::<15>();
    let h = h.mul_lo(I8::splat(K3));
    h ^ h.shr::<13>()
}

/// 23 hash bits as a float in [0, 1).
#[inline(always)]
fn unit(h: I8) -> F8 {
    (h.shr::<9>() | 0x3F80_0000).cast_f8() - 1.0
}

/// Value noise at (x, y) with `seed`: returns (n in [0, 1), dn/dx, dn/dy).
#[inline(always)]
pub fn noised8(x: F8, y: F8, seed: I8) -> (F8, F8, F8) {
    let fx0 = x.floor();
    let fy0 = y.floor();
    let fx = x - fx0;
    let fy = y - fy0;
    let ix = fx0.to_i8();
    let iy = fy0.to_i8();
    let k1 = I8::splat(K1);
    let k2 = I8::splat(K2);
    let x0 = ix.mul_lo(k1);
    let y0 = iy.mul_lo(k2);
    let x1 = x0 + k1; // (ix + 1) * k1
    let y1 = y0 + k2;
    let hash = |xt: I8, yt: I8| unit(mix(xt ^ yt ^ seed));
    let a = hash(x0, y0);
    let b = hash(x1, y0);
    let c = hash(x0, y1);
    let d = hash(x1, y1);
    let ux = fx * fx * (F8::splat(3.0) - (fx + fx));
    let dux = (F8::one() - fx) * fx * 6.0;
    let uy = fy * fy * (F8::splat(3.0) - (fy + fy));
    let duy = (F8::one() - fy) * fy * 6.0;
    let k1 = b - a;
    let k2 = c - a;
    let k4 = (d - b) - k2;
    let n = k4.mul_add(ux * uy, k2.mul_add(uy, k1.mul_add(ux, a)));
    let dx = k4.mul_add(uy, k1) * dux;
    let dy = k4.mul_add(ux, k2) * duy;
    (n, dx, dy)
}

/// The quintic fade 6t^5 - 15t^4 + 10t^3.
#[inline(always)]
fn fade(t: F8) -> F8 {
    let p = t.mul_add(F8::splat(6.0), F8::splat(-15.0)).mul_add(t, F8::splat(10.0));
    p * t * t * t
}

#[inline(always)]
fn sel(m: M8, a: F8, b: F8) -> F8 {
    F8::select(m, a, b)
}

/// 2D Perlin noise, about [-0.7, 0.7]: eight gradient directions.
#[inline(always)]
pub fn perlin2(x: F8, y: F8, seed: I8) -> F8 {
    let (fx0, fy0) = (x.floor(), y.floor());
    let (fx, fy) = (x - fx0, y - fy0);
    let (x0, y0) = (fx0.to_i8().mul_lo(I8::splat(K1)), fy0.to_i8().mul_lo(I8::splat(K2)));
    let (x1, y1) = (x0 + K1, y0 + K2);
    let grad = |xt: I8, yt: I8, dx: F8, dy: F8| {
        let h = mix(xt ^ yt ^ seed);
        let bit = |b: i32| (h & b).eq(I8::splat(b));
        // (+-1, +-1) diagonals or (+-1, 0) / (0, +-1) axes
        let u = sel(bit(4), dy, dx);
        let v = sel(bit(4), dx, dy);
        let u = sel(bit(1), -u, u);
        let v = sel(bit(2), -v, v).and(bit(8));
        u + v
    };
    let a = grad(x0, y0, fx, fy);
    let b = grad(x1, y0, fx - 1.0, fy);
    let c = grad(x0, y1, fx, fy - 1.0);
    let d = grad(x1, y1, fx - 1.0, fy - 1.0);
    let (u, v) = (fade(fx), fade(fy));
    let top = F8::lerp(a, b, u);
    let bot = F8::lerp(c, d, u);
    F8::lerp(top, bot, v)
}

/// 3D Perlin noise, about [-1, 1] (Perlin's twelve edge gradients).
#[inline(always)]
pub fn perlin3(p: V8, seed: I8) -> F8 {
    let (fx0, fy0, fz0) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (fx, fy, fz) = (p.x - fx0, p.y - fy0, p.z - fz0);
    let x0 = fx0.to_i8().mul_lo(I8::splat(K1));
    let y0 = fy0.to_i8().mul_lo(I8::splat(K2));
    let z0 = fz0.to_i8().mul_lo(I8::splat(K4));
    let grad = |h: I8, x: F8, y: F8, z: F8| {
        let h = mix(h ^ seed) & 15;
        let u = sel(h.lt(I8::splat(8)), x, y);
        let xz = h.eq(I8::splat(12)) | h.eq(I8::splat(14));
        let v = sel(h.lt(I8::splat(4)), y, sel(xz, x, z));
        let bit = |b: i32| (h & b).eq(I8::splat(b));
        sel(bit(1), -u, u) + sel(bit(2), -v, v)
    };
    let mut c = [F8::zero(); 8];
    for (i, ci) in c.iter_mut().enumerate() {
        let (ox, oy, oz) = (i & 1, (i >> 1) & 1, (i >> 2) & 1);
        let h = (if ox == 1 { x0 + K1 } else { x0 }) ^ (if oy == 1 { y0 + K2 } else { y0 }) ^ (if oz == 1 { z0 + K4 } else { z0 });
        *ci = grad(h, fx - ox as f32, fy - oy as f32, fz - oz as f32);
    }
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));
    let x00 = F8::lerp(c[0], c[1], u);
    let x10 = F8::lerp(c[2], c[3], u);
    let x01 = F8::lerp(c[4], c[5], u);
    let x11 = F8::lerp(c[6], c[7], u);
    F8::lerp(F8::lerp(x00, x10, v), F8::lerp(x01, x11, v), w)
}

/// fBm of 3D Perlin noise: `octaves` octaves, lacunarity 2, gain 0.5.
#[inline(always)]
pub fn fbm3(p: V8, octaves: u32, seed: I8) -> F8 {
    let mut sum = F8::zero();
    let mut amp = 0.5;
    let mut q = p;
    for _ in 0..octaves {
        sum = perlin3(q, seed).mul_add(F8::splat(amp), sum);
        // rotate a little between octaves so the lattices do not line up
        q = V8::new(q.x * 1.6 + q.z * 1.2, q.y * 2.0, q.z * 1.6 - q.x * 1.2);
        amp *= 0.5;
    }
    sum
}
