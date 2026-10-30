//! Post-processing (fraktal256's bloom and composite): a thresholded bloom at
//! 1/4 and 1/8 resolution (separable 9-tap blurs), then the composite: edge
//! anti-aliasing on the HDR image, bilinear bloom upsampling, exposure and
//! vignette, ACES, gamma and dither into the 8-bit output. The frame's
//! padded size (width a multiple of 64, height of 16) is what the bloom's
//! octet downsamples need.

use super::Frame;
use crate::pool::Shared;
use crate::settings::Settings;
use crate::simd::{F8, I8};
use rayon::prelude::*;

/// Zero floats on both sides of every bloom row (the blurs and the upsample
/// read past the row ends and must see black).
const PAD: usize = 8;
/// Bloom threshold: only light above this (HDR luminance) blooms.
const THRESHOLD: f32 = 3.5;
/// 9-tap gaussian, centre first.
const G: [f32; 5] = [0.2270, 0.1945, 0.1216, 0.0540, 0.0162];
/// Weights of the two bloom levels in the composite.
const BLOOM_WEIGHT: [f32; 2] = [0.10, 0.16];
const VIGNETTE: f32 = 0.55;
const AA_THRESHOLD: f32 = 0.18;
/// fraktal256's base exposure.
const EXPOSURE: f32 = 0.5;

/// One bloom level: three colour planes (r, g, b) of `w` x `h` values with
/// `PAD` zero floats around every row.
struct Level {
    w: usize,
    h: usize,
    stride: usize,
    c: [Vec<f32>; 3],
    tmp: [Vec<f32>; 3],
}

impl Level {
    fn new(w: usize, h: usize) -> Level {
        let stride = w + 2 * PAD;
        let plane = || vec![0.0f32; stride * h];
        Level { w, h, stride, c: [plane(), plane(), plane()], tmp: [plane(), plane(), plane()] }
    }
}

pub(crate) struct Bloom {
    l0: Level,
    l1: Level,
}

impl Bloom {
    pub fn new(w: usize, h: usize) -> Bloom {
        Bloom { l0: Level::new(w / 4, h / 4), l1: Level::new(w / 8, h / 8) }
    }
}

/// Bloom, then the composite into `frame.out`. `edge_aa` is off once enough
/// samples have accumulated.
/// What twinkling stars need: the camera (set up for the frame), the sky, the
/// stars' size and the time.
pub(crate) struct Twinkle<'a> {
    pub cam: &'a crate::camera::Camera,
    pub env: &'a crate::scene::Env,
    pub cone: f32,
    pub time: f32,
}

pub(crate) fn run(s: &Settings, frame: &mut Frame, edge_aa: bool, tw: Option<&Twinkle>) {
    let (w, h) = (frame.pw, frame.ph);
    let Frame { hdr, out, bloom, ids, .. } = frame;
    let hdr = [&hdr[0][..], &hdr[1][..], &hdr[2][..]];
    if s.bloom {
        downsample4(&hdr, w, &mut bloom.l0);
        blur(&mut bloom.l0);
        downsample2(&mut bloom.l0, &mut bloom.l1);
        blur(&mut bloom.l1);
    }
    composite(s, &hdr, bloom, w, h, out, edge_aa, ids, tw);
}

/// The debug views: the HDR values straight to 8 bits (`gamma`: square root).
pub(crate) fn plain(frame: &mut Frame, gamma: bool) {
    let w = frame.pw;
    let Frame { hdr, out, .. } = frame;
    out.par_chunks_mut(w).enumerate().for_each(|(row, dst)| {
        for (x, d) in dst.iter_mut().enumerate() {
            let i = row * w + x;
            let f = |v: f32| {
                let v = v.clamp(0.0, 1.0);
                ((if gamma { v.sqrt() } else { v }) * 255.0 + 0.5) as u32
            };
            *d = 0xff00_0000 | f(hdr[0][i]) << 16 | f(hdr[1][i]) << 8 | f(hdr[2][i]);
        }
    });
}

/// Level 0: the 4 x 4 average of the HDR image, soft-thresholded.
fn downsample4(hdr: &[&[f32]; 3], w: usize, l: &mut Level) {
    let (stride, lw) = (l.stride, l.w);
    let [c0, c1, c2] = &mut l.c;
    let out = [Shared::new(c0), Shared::new(c1), Shared::new(c2)];
    let perm = I8::from_array([0, 4, 1, 5, 2, 6, 3, 7]);
    (0..l.h).into_par_iter().for_each(|row| {
        // SAFETY: each job writes only its own row of the planes.
        let dst: [&mut [f32]; 3] = unsafe {
            [
                out[0].slice(row * stride + PAD, lw),
                out[1].slice(row * stride + PAD, lw),
                out[2].slice(row * stride + PAD, lw),
            ]
        };
        for oct in 0..lw / 8 {
            let mut avg = [F8::zero(); 3];
            for (ch, a) in avg.iter_mut().enumerate() {
                let base = row * 4 * w + oct * 32;
                let sum = |k: usize| {
                    let at = |y: usize| F8::load(&hdr[ch][base + y * w + k * 8..]);
                    at(0) + at(1) + at(2) + at(3)
                };
                let h1 = sum(0).hadd(sum(1));
                let h2 = sum(2).hadd(sum(3));
                *a = h1.hadd(h2).permute(perm) * 0.0625;
            }
            let lum = avg[0].max(avg[1]).max(avg[2]);
            let excess = (lum - THRESHOLD).max(F8::zero());
            let scale = excess / lum.max(F8::splat(0.001));
            for ch in 0..3 {
                (avg[ch] * scale).store(&mut dst[ch][oct * 8..]);
            }
        }
    });
}

/// 2 x 2 average of level 0 into level 1.
fn downsample2(l0: &mut Level, l1: &mut Level) {
    let (s0, s1, w1) = (l0.stride, l1.stride, l1.w);
    let perm = I8::from_array([0, 1, 4, 5, 2, 3, 6, 7]);
    let src = &l0.c;
    let [c0, c1, c2] = &mut l1.c;
    let out = [Shared::new(c0), Shared::new(c1), Shared::new(c2)];
    (0..l1.h).into_par_iter().for_each(|row| {
        for ch in 0..3 {
            // SAFETY: each job writes only its own row.
            let dst = unsafe { out[ch].slice(row * s1 + PAD, w1) };
            let top = (2 * row) * s0 + PAD;
            for oct in 0..w1 / 8 {
                let i = top + oct * 16;
                let a = F8::load(&src[ch][i..]) + F8::load(&src[ch][i + s0..]);
                let b = F8::load(&src[ch][i + 8..]) + F8::load(&src[ch][i + 8 + s0..]);
                (a.hadd(b).permute(perm) * 0.25).store(&mut dst[oct * 8..]);
            }
        }
    });
}

/// Separable 9-tap gaussian of all three planes (rows, then columns with the
/// edge rows repeated).
fn blur(l: &mut Level) {
    let (stride, w, h) = (l.stride, l.w, l.h);
    {
        let src = &l.c;
        let [t0, t1, t2] = &mut l.tmp;
        let out = [Shared::new(t0), Shared::new(t1), Shared::new(t2)];
        (0..h).into_par_iter().for_each(|row| {
            for ch in 0..3 {
                // SAFETY: each job writes only its own row.
                let dst = unsafe { out[ch].slice(row * stride + PAD, w) };
                let at = row * stride + PAD;
                for oct in 0..w / 8 {
                    let i = at + oct * 8;
                    let s = &src[ch];
                    let mut acc = F8::load(&s[i..]) * G[0];
                    for k in 1..5 {
                        // PAD >= 4: the loads stay inside the padded row
                        let side = F8::load(&s[i - k..]) + F8::load(&s[i + k..]);
                        acc = side.mul_add(F8::splat(G[k]), acc);
                    }
                    acc.store(&mut dst[oct * 8..]);
                }
            }
        });
    }
    {
        let src = &l.tmp;
        let [c0, c1, c2] = &mut l.c;
        let out = [Shared::new(c0), Shared::new(c1), Shared::new(c2)];
        (0..h).into_par_iter().for_each(|row| {
            for ch in 0..3 {
                // SAFETY: each job writes only its own row.
                let dst = unsafe { out[ch].slice(row * stride + PAD, w) };
                let at = |y: isize| y.clamp(0, h as isize - 1) as usize * stride + PAD;
                for oct in 0..w / 8 {
                    let s = &src[ch];
                    let o = oct * 8;
                    let mut acc = F8::load(&s[at(row as isize) + o..]) * G[0];
                    for k in 1..5 {
                        let up = F8::load(&s[at(row as isize - k as isize) + o..]);
                        let down = F8::load(&s[at(row as isize + k as isize) + o..]);
                        acc = (up + down).mul_add(F8::splat(G[k]), acc);
                    }
                    acc.store(&mut dst[o..]);
                }
            }
        });
    }
}

/// `v` rows of a bloom level interpolated vertically for output row `row`
/// (`scale` = 4 or 8), into `buf` (three planes of one padded stride).
fn lerp_rows(l: &Level, scale: usize, row: usize, buf: &mut [f32]) {
    let v = (row as f32 + 0.5) / scale as f32 - 0.5;
    let j = v.floor();
    let fy = F8::splat(v - j);
    let a = (j as isize).clamp(0, l.h as isize - 1) as usize;
    let b = (j as isize + 1).clamp(0, l.h as isize - 1) as usize;
    for ch in 0..3 {
        let (ra, rb) = (a * l.stride, b * l.stride);
        for i in (0..l.stride).step_by(8) {
            let p = F8::load(&l.c[ch][ra + i..]);
            let q = F8::load(&l.c[ch][rb + i..]);
            (q - p).mul_add(fy, p).store(&mut buf[ch * l.stride + i..]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn composite(s: &Settings, hdr: &[&[f32]; 3], bloom: &Bloom, w: usize, h: usize, out: &mut [u32], edge_aa: bool, ids: &[i32], tw: Option<&Twinkle>) {
    let (l0, l1) = (&bloom.l0, &bloom.l1);
    let lanes = F8::iota();
    let (i4, j4) = (I8::from_array([0, 0, 1, 1, 1, 1, 2, 2]), I8::from_array([1, 1, 2, 2, 2, 2, 3, 3]));
    let f4 = F8::from_array([0.625, 0.875, 0.125, 0.375, 0.625, 0.875, 0.125, 0.375]);
    let (i8_, j8) = (I8::from_array([0, 0, 0, 0, 1, 1, 1, 1]), I8::from_array([1, 1, 1, 1, 2, 2, 2, 2]));
    let f8_ = F8::from_array([0.5625, 0.6875, 0.8125, 0.9375, 0.0625, 0.1875, 0.3125, 0.4375]);
    let inv_w = 1.0 / w as f32;
    let expo = EXPOSURE * s.exposure;
    let vignette = if s.vignette { VIGNETTE } else { 0.0 };
    // the bloom rows of an output row, in buffers each job keeps for its rows
    // (eight rows at least: rayon would otherwise split down to single rows)
    let bufs = || (vec![0.0f32; 3 * l0.stride], vec![0.0f32; 3 * l1.stride]);
    out.par_chunks_mut(w).enumerate().with_min_len(8).for_each_init(bufs, |(buf0, buf1), (row, dst)| {
        if s.bloom {
            lerp_rows(l0, 4, row, buf0);
            lerp_rows(l1, 8, row, buf1);
        }
        let dy = row as f32 / h as f32 - 0.5;
        let dy2 = F8::splat(dy * dy);
        let (up_row, down_row) = (row.saturating_sub(1), (row + 1).min(h - 1));
        for x in (0..w).step_by(8) {
            let ix = row * w + x;
            let (iu, id) = (up_row * w + x, down_row * w + x);
            let mut col = [F8::zero(); 3];
            if edge_aa {
                // ---- edge anti-aliasing on the HDR image
                let luma = |i: isize| {
                    let ld = |p: &[f32]| load_at(p, i);
                    ld(hdr[0]) * 0.3 + ld(hdr[1]) * 0.6 + ld(hdr[2]) * 0.1
                };
                let l_c = luma(ix as isize);
                let (l_u, l_d) = (luma(iu as isize), luma(id as isize));
                let (l_l, l_r) = (luma(ix as isize - 1), luma(ix as isize + 1));
                let hi = l_c.max(l_u).max(l_d).max(l_l).max(l_r);
                let lo = l_c.min(l_u).min(l_d).min(l_l).min(l_r);
                let contrast = (hi - lo) / hi.max(F8::splat(0.05));
                let weight = ((contrast - AA_THRESHOLD) * 1.5).max(F8::zero()).min(F8::splat(0.5));
                let curve_v = (l_u + l_d - l_c - l_c).abs();
                let curve_h = (l_l + l_r - l_c - l_c).abs();
                let along_x = curve_h.lt(curve_v);
                for ch in 0..3 {
                    let c = load_at(hdr[ch], ix as isize);
                    let ns = load_at(hdr[ch], iu as isize) + load_at(hdr[ch], id as isize);
                    let ew = load_at(hdr[ch], ix as isize - 1) + load_at(hdr[ch], ix as isize + 1);
                    let avg = F8::select(along_x, ns, ew) * 0.5;
                    col[ch] = (avg - c).mul_add(weight, c);
                }
            } else {
                for ch in 0..3 {
                    col[ch] = F8::load(&hdr[ch][ix..]);
                }
            }
            // ---- twinkling stars, where the sky shows
            if let Some(tw) = tw {
                // clear sky only: stars behind a cloud stay as traced
                let sky = I8::from_array(ids[ix..ix + 8].try_into().unwrap()).eq(I8::splat(-1));
                if sky.any() {
                    let d = super::primary_rays(tw.cam, row, x, F8::zero(), F8::zero());
                    let more = super::atmos::twinkle(tw.env, tw.cone, d, tw.time);
                    for (c, m) in col.iter_mut().zip([more.x, more.y, more.z]) {
                        *c = (*c + m.and(sky)).max(F8::zero());
                    }
                }
            }
            if s.bloom {
                // ---- bloom: bilinear upsample of both levels
                for ch in 0..3 {
                    let base0 = ch * l0.stride + (x >> 2) + PAD - 1;
                    let v0 = F8::load(&buf0[base0..]);
                    let (lo0, hi0) = (v0.permute(i4), v0.permute(j4));
                    let s0 = (hi0 - lo0).mul_add(f4, lo0);
                    let base1 = ch * l1.stride + (x >> 3) + PAD - 1;
                    let v1 = F8::load(&buf1[base1..]);
                    let (lo1, hi1) = (v1.permute(i8_), v1.permute(j8));
                    let s1 = (hi1 - lo1).mul_add(f8_, lo1);
                    col[ch] += s1.mul_add(F8::splat(BLOOM_WEIGHT[1]), s0 * BLOOM_WEIGHT[0]);
                }
            }
            // ---- exposure * vignette
            let dx = (lanes + x as f32) * inv_w - 0.5;
            let vig = (F8::one() - dx.mul_add(dx, dy2) * vignette) * expo;
            let [r, g, b] = col.map(|c| c * vig);
            // ---- ACES fit (or clamp), gamma 1/2, dither
            let hash = (I8::splat(ix as i32) + I8::iota()).mul_lo(I8::splat(0x9E37_79B1u32 as i32));
            let row_h = I8::splat(row as i32).mul_lo(I8::splat(0x85EB_CA77u32 as i32));
            let hash = (hash ^ row_h).mul_lo(I8::splat(0x9E37_79B1u32 as i32));
            let noise = (hash.shr::<9>() | 0x3f80_0000).cast_f8() - 1.5;
            let tone = |c: F8| if s.tonemap { aces(c) } else { c.max(F8::zero()).min(F8::one()).sqrt() };
            let pack = |c: F8| tone(c).mul_add(F8::splat(255.0), noise).clamp(0.0, 255.0).round_i8();
            let (ri, gi, bi) = (pack(r), pack(g), pack(b));
            let px = ri.shl::<16>() | gi.shl::<8>() | bi | 0xff00_0000u32 as i32;
            for (d, v) in dst[x..x + 8].iter_mut().zip(px.to_array()) {
                *d = v as u32;
            }
        }
    });
}

/// Eight values of a plane at an index that may be one before the start
/// (the first octet's left neighbour): that lane repeats pixel 0.
#[inline(always)]
fn load_at(p: &[f32], i: isize) -> F8 {
    if i >= 0 {
        F8::load(&p[i as usize..])
    } else {
        let mut a = [0.0f32; 8];
        a[0] = p[0];
        a[1..].copy_from_slice(&p[..7]);
        F8::from_array(a)
    }
}

/// The ACES filmic fit, clamped, then gamma 1/2 by a square root.
#[inline(always)]
fn aces(x: F8) -> F8 {
    let num = (x * 2.51 + 0.03) * x;
    let den = (x * 2.43 + 0.59).mul_add(x, F8::splat(0.14));
    (num / den).max(F8::zero()).min(F8::one()).sqrt()
}

/// The cost heat map: cycles per pixel through the sequential ramp, `max`
/// cycles at the top.
pub(crate) fn heat(frame: &mut Frame, max: f32) {
    let w = frame.pw;
    let Frame { cost, out, .. } = frame;
    let inv = 1.0 / max;
    out.par_chunks_mut(w).enumerate().for_each(|(row, dst)| {
        for x in (0..w).step_by(8) {
            let c = F8::load(&cost[row * w + x..]) * inv;
            let rgb = super::ramp(c);
            let q = |v: F8| (v * 255.0).clamp(0.0, 255.0).round_i8();
            let px = q(rgb.x).shl::<16>() | q(rgb.y).shl::<8>() | q(rgb.z) | 0xff00_0000u32 as i32;
            for (d, v) in dst[x..x + 8].iter_mut().zip(px.to_array()) {
                *d = v as u32;
            }
        }
    });
}

/// Outlines objects `first..end`: their pixels that border other objects or the sky.
pub(crate) fn outline(frame: &mut Frame, first: i32, end: i32) {
    let (w, h) = (frame.pw, frame.ph);
    let Frame { ids, out, .. } = frame;
    const COLOR: u32 = 0xffff_a020;
    out.par_chunks_mut(w).enumerate().for_each(|(row, dst)| {
        let sel = |x: usize, y: usize| (first..end).contains(&ids[y * w + x]);
        for x in 0..w {
            if !sel(x, row) {
                continue;
            }
            let edge = x == 0 || x + 1 == w || row == 0 || row + 1 == h || !sel(x - 1, row) || !sel(x + 1, row) || !sel(x, row - 1) || !sel(x, row + 1);
            if edge {
                dst[x] = COLOR;
            }
        }
    });
}
