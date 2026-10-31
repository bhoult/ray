//! Heightfield terrain: the height map is built once at load (on all cores)
//! from noise, then sampled bilinearly eight points at a time while
//! marching. From fraktal256-rs world/terrain.rs, generalised from its fixed
//! 1024 x 1024 map of 0.5 m cells to any size and resolution, with Perlin,
//! ridged and billow noise beside fraktal's eroded value-noise fBm.

use super::json::TerrainDef;
use crate::math::{Aabb, Vec3};
use crate::noise::{noised8, perlin2};
use crate::simd::{F8, I8, V8};
use rayon::prelude::*;

/// Cavity AO box blur radius in cells.
const BLUR_R: i32 = 8;

pub struct Terrain {
    /// samples per side
    pub n: usize,
    /// metres per cell
    pub cell: f32,
    pub inv_cell: f32,
    /// world x / z of sample (0, 0)
    pub origin_x: f32,
    pub origin_z: f32,
    /// heights in metres (world y), row-major (z rows, x columns), with slack after
    pub h: Vec<f32>,
    /// packed normals: x in the low 16 bits, z in the high 16, scaled by 32767
    pub nrm: Vec<i32>,
    /// cavity AO: 0.3 in a pit .. 1.0 on a summit
    pub ao: Vec<f32>,
    pub min_h: f32,
    pub max_h: f32,
    pub relax: f32,
    pub rock: Vec3,
    pub grass: Vec3,
    pub snow: Vec3,
    pub sand: Vec3,
    /// grass thins out above this height, snow starts at `snow_line`, sand
    /// lies below `sand_line` (-inf without water)
    pub grass_top: f32,
    pub snow_line: f32,
    pub sand_line: f32,
}

/// Cell index and fractions of a bilinear fetch at 8 points.
pub struct Cell8 {
    pub idx: I8,
    pub fx: F8,
    pub fz: F8,
}

impl Terrain {
    pub fn build(d: &TerrainDef, center: Vec3, water: Option<f32>) -> Result<Terrain, String> {
        let kind = match d.noise.as_str() {
            "eroded" => 0,
            "perlin" => 1,
            "ridged" => 2,
            "billow" => 3,
            "flat" => 4,
            other => return Err(format!("unknown terrain noise '{other}' (eroded, perlin, ridged, billow, flat)")),
        };
        // flat ground (a floor) needs no detail
        let flat = kind == 4;
        let n = if flat { 16 } else { d.resolution.clamp(16, 8192) as usize };
        let n = n.next_multiple_of(8);
        if !(d.size > 0.0) {
            return Err("terrain size must be positive".into());
        }
        let cell = d.size / (n - 1) as f32;
        let origin_x = center.x - d.size * 0.5;
        let origin_z = center.z - d.size * 0.5;
        let slack = n + 64;
        let mut h = vec![0.0f32; n * n + slack];
        let seed = I8::splat((d.seed.wrapping_mul(0x9E37_79B1) | 1) as i32);
        let (sx, sz) = (d.seed as f32 * 37.17, d.seed as f32 * 19.31);
        h[..n * n].par_chunks_mut(n).enumerate().for_each(|(row, out)| {
            let z = F8::splat(origin_z + row as f32 * cell);
            for (o, cells) in out.chunks_exact_mut(8).enumerate() {
                let x = (I8::splat(o as i32 * 8) + I8::iota()).to_f8().mul_add(F8::splat(cell), F8::splat(origin_x));
                let f = if flat { F8::zero() } else { fbm8(kind, x, z, d, seed, sx, sz) };
                let mut v = f.mul_add(F8::splat(d.height), F8::splat(d.base + center.y));
                if d.rim > 0.0 {
                    let (dx, dz) = (x - center.x, z - center.z);
                    let r = (dz.mul_add(dz, dx * dx).sqrt() - d.rim).max(F8::zero());
                    if d.rim_height > 0.0 {
                        // a ring of mountains: sin² up and down again across
                        // its width (the slopes stay ones a heightfield
                        // shades smoothly; the bowl's steepened to a wall
                        // that rippled as the view moved)
                        let k = (r * (1.0 / d.rim_width.max(1.0))).clamp(0.0, 1.0).to_array().map(|k| (k * std::f32::consts::PI).sin().powi(2));
                        v = F8::from_array(k).mul_add(F8::splat(d.rim_height), v);
                    } else {
                        v = (r * r).mul_add(F8::splat(0.03), v);
                    }
                }
                if d.island > 0.0 {
                    // full height to a third of the radius, then easing down
                    // to below the sea floor at it
                    let (dx, dz) = (x - center.x, z - center.z);
                    let k = ((dz.mul_add(dz, dx * dx).sqrt() - F8::splat(d.island / 3.0)) * F8::splat(1.5 / d.island)).clamp(0.0, 1.0);
                    let k = k * k * (F8::splat(3.0) - k - k);
                    v = v - k * F8::splat(d.height + d.base.max(0.0) + 8.0);
                }
                if let Some(cap) = d.cap {
                    v = v.min(F8::splat(cap + center.y));
                }
                v.store(cells);
            }
        });
        let (mut min_h, mut max_h) = (f32::INFINITY, f32::NEG_INFINITY);
        for &v in &h[..n * n] {
            min_h = min_h.min(v);
            max_h = max_h.max(v);
        }
        // normals: central differences, edges clamped
        let mut nrm = vec![0i32; n * n + slack];
        let hs = &h;
        nrm[..n * n].par_chunks_mut(n).enumerate().for_each(|(row, out)| {
            let last = n - 1;
            let (up, dn) = (row.saturating_sub(1), (row + 1).min(last));
            for (x, o) in out.iter_mut().enumerate() {
                let (l, r) = (x.saturating_sub(1), (x + 1).min(last));
                let nx = -(hs[row * n + r] - hs[row * n + l]) / ((r - l) as f32 * cell);
                let nz = -(hs[dn * n + x] - hs[up * n + x]) / ((dn - up) as f32 * cell);
                let inv = 1.0 / (nx * nx + nz * nz + 1.0).sqrt();
                let q = |v: f32| (v * inv * 32767.0).round() as i32;
                *o = (q(nx) & 0xFFFF) | (q(nz) << 16);
            }
        });
        // cavity AO: height above a 17 x 17 box blur
        let mut tmp = vec![0.0f32; n * n];
        tmp.par_chunks_mut(n).enumerate().for_each(|(row, dst)| {
            let src = &hs[row * n..][..n];
            let last = n as i32 - 1;
            let at = |i: i32| src[i.clamp(0, last) as usize];
            let mut sum: f32 = (-BLUR_R..=BLUR_R).map(at).sum();
            for (x, d) in dst.iter_mut().enumerate() {
                *d = sum / 17.0;
                sum += at(x as i32 + BLUR_R + 1) - at(x as i32 - BLUR_R);
            }
        });
        let mut ao = vec![0.0f32; n * n + slack];
        let tmp = &tmp;
        // the cavity scale follows fraktal's 0.12 per metre at 0.5 m cells
        let k = 0.12;
        ao[..n * n].par_chunks_mut(n).enumerate().for_each(|(row, dst)| {
            let last = n as i32 - 1;
            for (x, d) in dst.iter_mut().enumerate() {
                let mut sum = 0.0;
                for y in -BLUR_R..=BLUR_R {
                    sum += tmp[(row as i32 + y).clamp(0, last) as usize * n + x];
                }
                *d = ((hs[row * n + x] - sum / 17.0) * k + 0.7).clamp(0.3, 1.0);
            }
        });
        let range = (max_h - min_h).max(1e-3);
        let ground = water.unwrap_or(min_h + 0.05 * range);
        Ok(Terrain {
            n,
            cell,
            inv_cell: 1.0 / cell,
            origin_x,
            origin_z,
            h,
            nrm,
            ao,
            min_h,
            max_h,
            relax: d.relax.clamp(0.05, 1.0),
            rock: Vec3::from_array(d.rock),
            grass: Vec3::from_array(d.grass),
            snow: Vec3::from_array(d.snow),
            sand: Vec3::from_array(d.sand),
            grass_top: ground + 0.11 * range,
            snow_line: d.snow_line.unwrap_or(ground + 0.3 * range),
            sand_line: d.sand_line.unwrap_or(water.map_or(f32::NEG_INFINITY, |w| w + 0.6)),
        })
    }

    pub fn aabb(&self) -> Aabb {
        let size = self.cell * (self.n - 1) as f32;
        Aabb::new(
            Vec3::new(self.origin_x, self.min_h - 0.01, self.origin_z),
            Vec3::new(self.origin_x + size, self.max_h + 0.01, self.origin_z + size),
        )
    }

    #[inline(always)]
    pub fn cell8(&self, x: F8, z: F8) -> Cell8 {
        let lim = F8::splat((self.n - 1) as f32 - 0.001);
        let cx = ((x - self.origin_x) * self.inv_cell).max(F8::zero()).min(lim);
        let cz = ((z - self.origin_z) * self.inv_cell).max(F8::zero()).min(lim);
        let (ix, iz) = (cx.floor(), cz.floor());
        Cell8 { idx: ix.to_i8() + iz.to_i8().mul_lo(I8::splat(self.n as i32)), fx: cx - ix, fz: cz - iz }
    }

    /// The bilinear height at eight world points.
    #[inline(always)]
    pub fn height8(&self, x: F8, z: F8) -> F8 {
        let c = self.cell8(x, z);
        let n = self.n as i32;
        // indices are clamped by cell8; the slack covers the +n + 1 neighbour
        let at = |o: i32| F8::gather_in_bounds(&self.h, c.idx + o);
        let (a, b) = (at(0), at(1));
        let top = (b - a).mul_add(c.fx, a);
        let (a, b) = (at(n), at(n + 1));
        let bot = (b - a).mul_add(c.fx, a);
        (bot - top).mul_add(c.fz, top)
    }

    /// The bilinear unit normal at eight world points, and the cavity AO.
    #[inline(always)]
    pub fn normal8(&self, x: F8, z: F8) -> (V8, F8) {
        let c = self.cell8(x, z);
        let n = self.n as i32;
        let g = |o: i32| I8::gather(&self.nrm, c.idx + o);
        let (n00, n10, n01, n11) = (g(0), g(1), g(n), g(n + 1));
        let lerp2 = |a: I8, b: I8, c2: I8, d: I8| {
            let (a, b, c2, d) = (a.to_f8(), b.to_f8(), c2.to_f8(), d.to_f8());
            let top = (b - a).mul_add(c.fx, a);
            let bot = (d - c2).mul_add(c.fx, c2);
            (bot - top).mul_add(c.fz, top) * (1.0 / 32767.0)
        };
        let lo = |v: I8| v.shl::<16>().sar::<16>();
        let hi = |v: I8| v.sar::<16>();
        let nx = lerp2(lo(n00), lo(n10), lo(n01), lo(n11));
        let nz = lerp2(hi(n00), hi(n10), hi(n01), hi(n11));
        let ny = (F8::one() - nx * nx - nz * nz).max(F8::splat(0.05)).sqrt();
        (V8::new(nx, ny, nz), F8::gather(&self.ao, c.idx))
    }

    /// Height at one point (the camera's ground clamp, statistics).
    pub fn height(&self, x: f32, z: f32) -> f32 {
        self.height8(F8::splat(x), F8::splat(z)).lane(0)
    }
}

/// fBm of the chosen noise, about [-1, 1].
#[inline(always)]
fn fbm8(kind: u32, x: F8, z: F8, d: &TerrainDef, seed: I8, sx: f32, sz: f32) -> F8 {
    let mut px = x * d.frequency + sx;
    let mut pz = z * d.frequency + sz;
    let (mut dx, mut dz) = (F8::zero(), F8::zero());
    let (mut sum, mut amp) = (F8::zero(), F8::one());
    // ridged multifractal: each octave weighted by the previous one
    let mut weight = F8::one();
    for _ in 0..d.octaves.clamp(1, 16) {
        match kind {
            0 => {
                // fraktal's "eroded" fBm: octaves damped by the slope so far
                let (n, gx, gz) = noised8(px, pz, seed);
                let n = n + n - 1.0;
                dx = gx + gx + dx;
                dz = gz + gz + dz;
                let damp = dz.mul_add(dz, dx * dx) + 1.0;
                sum = (n / damp).mul_add(amp, sum);
            }
            1 => sum = (perlin2(px, pz, seed) * 1.4).mul_add(amp, sum),
            2 => {
                let r = F8::one() - (perlin2(px, pz, seed) * 1.4).abs();
                let r = r * r * weight;
                weight = (r * 1.5).clamp(0.0, 1.0);
                sum = (r * 2.0 - 0.8).mul_add(amp, sum);
            }
            _ => sum = ((perlin2(px, pz, seed) * 1.4).abs() * 2.0 - 0.6).mul_add(amp, sum),
        }
        amp *= 0.5;
        // p = M p * 2 with M = (0.8 -0.6; 0.6 0.8), plus an offset in x
        let nx = pz.neg_mul_add(F8::splat(1.2), px * 1.6) + 0.31;
        let nz = pz.mul_add(F8::splat(1.6), px * 1.2);
        px = nx;
        pz = nz;
    }
    sum
}
