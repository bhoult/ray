//! Procedural patterns, eight points at a time. A pattern returns a blend
//! factor t in [0, 1] between a material's two sets of values (colour,
//! roughness, metal, glow) and a brightness variation; `relief` turns its
//! gradient into a bumped normal.

use super::shade::{lowbias, unit};
use crate::math::Mat3;
use crate::noise::{fbm3, perlin3};
use crate::scene::tex;
use crate::simd::{F8, I8, V8};

const K1: i32 = 0x8DA6_B343u32 as i32;
const K2: i32 = 0xD816_3841u32 as i32;
const K3: i32 = 0xCB1A_B31Fu32 as i32;

#[inline(always)]
fn smoothstep(a: f32, b: f32, x: F8) -> F8 {
    F8::smoothstep(a, b, x)
}

/// Distances to the nearest and second-nearest Voronoi feature points, and a
/// random number of the nearest cell.
#[inline(always)]
fn voronoi(q: V8) -> (F8, F8, F8) {
    let c = V8::new(q.x.floor(), q.y.floor(), q.z.floor());
    let f = q - c;
    let (ix, iy, iz) = (c.x.to_i8(), c.y.to_i8(), c.z.to_i8());
    let mut f1 = F8::splat(9.0);
    let mut f2 = F8::splat(9.0);
    let mut id = F8::zero();
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let h = lowbias((ix + dx).mul_lo(I8::splat(K1)) ^ (iy + dy).mul_lo(I8::splat(K2)) ^ (iz + dz).mul_lo(I8::splat(K3)));
                let h2 = lowbias(h ^ 0x5bd1_e995);
                let h3 = lowbias(h2 ^ 0x27d4_eb2f);
                let px = unit(h) + (dx as f32) - f.x;
                let py = unit(h2) + (dy as f32) - f.y;
                let pz = unit(h3) + (dz as f32) - f.z;
                let d = pz.mul_add(pz, py.mul_add(py, px * px));
                let nearer = d.lt(f1);
                f2 = F8::select(nearer, f1, f2.min(d));
                id = F8::select(nearer, unit(lowbias(h3)), id);
                f1 = f1.min(d);
            }
        }
    }
    (f1.sqrt(), f2.sqrt(), id)
}

/// Distorts the points by vector noise (for the cellular patterns).
#[inline(always)]
fn warped(q: V8, warp: F8) -> V8 {
    let seed = I8::splat(0x1234_5678);
    let n = V8::new(fbm3(q, 3, seed), fbm3(q + crate::math::Vec3::new(5.2, 1.3, 7.7), 3, seed), fbm3(q + crate::math::Vec3::new(9.1, 4.7, 2.3), 3, seed));
    n.mul_add(warp * 0.8, q)
}

/// Pattern `kind` at points `q` (in pattern units) on surfaces with normal
/// `n` (same frame): (blend t, brightness).
#[inline]
pub fn eval(kind: i32, q: V8, n: V8, warp: F8, sharp: F8) -> (F8, F8) {
    let one = F8::one();
    let seed = I8::splat(0x3c6e_f372);
    match kind {
        tex::CHECKER => {
            // sampled a quarter cell inside, so faces on cell borders do not flicker
            let p = n.mul_add(F8::splat(-0.25), q);
            let c = p.x.floor() + p.y.floor() + p.z.floor();
            let odd = (c * 0.5).fract().gt(F8::splat(0.25));
            (one.and(odd), one)
        }
        tex::GRID => {
            let line = |c: F8, nc: F8| {
                let f = c.fract();
                F8::select(nc.abs().lt(F8::splat(0.7)), f.min(one - f), one)
            };
            let d = line(q.x, n.x).min(line(q.y, n.y)).min(line(q.z, n.z));
            (one - smoothstep(0.02, 0.035, d * sharp), one)
        }
        tex::NOISE => {
            let p = if warp.hmax() > 0.0 { warped(q, warp) } else { q };
            let f = fbm3(p, 5, seed);
            ((f * (1.6 * 1.0)).mul_add(sharp, F8::splat(0.5)).saturate(), one)
        }
        tex::MARBLE => {
            let w = fbm3(q * 0.5, 5, seed);
            let v = (q.x * 1.5 + q.y * 0.5 + w * (warp * 6.0 + 2.0)).sin();
            let t = (one - v.abs()).max(F8::splat(1e-6)).powf(sharp * 6.0);
            (t, one)
        }
        tex::WOOD => {
            let w = fbm3(q * 0.7, 3, seed);
            let r = len2(q.x, q.z) * 3.0 + w * (warp * 2.0 + 0.3) + q.y * 0.05;
            let f = r.fract();
            let rings = f.max(F8::splat(1e-6)).powf(sharp * 3.0);
            // fine grain along the length
            let grain = perlin3(V8::new(q.x * 30.0, q.y * 1.5, q.z * 30.0), seed) * 0.08 + 1.0;
            (rings, grain)
        }
        tex::CELLS => {
            let p = if warp.hmax() > 0.0 { warped(q, warp) } else { q };
            let (f1, f2, id) = voronoi(p);
            let gap = one - smoothstep(0.0, 0.08, (f2 - f1) * sharp);
            (gap, id * 0.5 + 0.75)
        }
        tex::CRACKS => {
            let p = warped(q, warp + 0.3);
            let (f1, f2, _) = voronoi(p);
            let crack = one - smoothstep(0.0, 0.04, (f2 - f1) * sharp);
            (crack, one)
        }
        tex::SPOTS => {
            let p = if warp.hmax() > 0.0 { warped(q, warp) } else { q };
            let (f1, _, id) = voronoi(p);
            let r = id * 0.15 + 0.2;
            let edge = F8::splat(0.03) / sharp;
            let t = (one - ((f1 - r + edge) / (edge + edge))).saturate();
            (t * t * (F8::splat(3.0) - t - t), one)
        }
        tex::STRIPES => {
            let w = if warp.hmax() > 0.0 { fbm3(q, 3, seed) * (warp * 4.0) } else { F8::zero() };
            let v = (q.y * core::f32::consts::TAU + w).sin();
            ((v * sharp * 2.0 + 0.5).saturate(), one)
        }
        tex::BRICKS => {
            // project along the surface's main axis
            let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
            let top = ay.ge(ax) & ay.ge(az);
            let side = ax.ge(az).and_not(top);
            let u = F8::select(top, q.x, F8::select(side, q.z, q.x));
            let v = F8::select(top, q.z, q.y) * 2.0;
            let row = v.floor();
            let ub = u * 0.5 + row * 0.5;
            let (fu, fv) = (ub.fract(), v.fract());
            let edge = (fu.min(one - fu) * 2.0).min(fv.min(one - fv));
            let mortar = one - smoothstep(0.04, 0.08, edge * sharp);
            let shade = unit(lowbias((ub.floor().to_i8()).mul_lo(I8::splat(K1)) ^ row.to_i8().mul_lo(I8::splat(K2)))) * 0.3 + 0.85;
            (mortar, shade)
        }
        tex::HEXAGONS => {
            let (u, v) = plane_uv(q, n);
            // two offset lattices of centres; the nearer one's cell wins
            let (rx, ry) = (1.0f32, 1.732_050_8f32);
            let cell = |u: F8, v: F8| (u - (u * (1.0 / rx)).floor() * rx - rx * 0.5, v - (v * (1.0 / ry)).floor() * ry - ry * 0.5);
            let (ax, ay) = cell(u, v);
            let (bx, by) = cell(u - rx * 0.5, v - ry * 0.5);
            let near_a = ax.mul_add(ax, ay * ay).lt(bx.mul_add(bx, by * by));
            let (gx, gy) = (F8::select(near_a, ax, bx), F8::select(near_a, ay, by));
            let hex = gx.abs().max(gx.abs().mul_add(F8::splat(0.5), gy.abs() * 0.866_025_4));
            let grout = one - smoothstep(0.03, 0.06, (F8::splat(0.5) - hex) * sharp);
            let (cx, cy) = (((u - gx) * 2.0).round(), ((v - gy) * (1.0 / 0.866_025_4)).round());
            let shade = unit(lowbias(cx.to_i8().mul_lo(I8::splat(K1)) ^ cy.to_i8().mul_lo(I8::splat(K2)))) * 0.3 + 0.85;
            (grout, shade)
        }
        tex::WAVES => {
            let mut w = (q.x * core::f32::consts::PI).sin() * 0.3 + (q.z * (core::f32::consts::PI * 1.3)).sin() * 0.2;
            if warp.hmax() > 0.0 {
                w = fbm3(q, 3, seed).mul_add(warp * 2.0, w);
            }
            let v = ((q.y + w) * core::f32::consts::TAU).sin();
            ((v * sharp).mul_add(F8::splat(0.5), F8::splat(0.5)).saturate(), one)
        }
        tex::RIPPLES => {
            let mut r = len2(q.x, q.z);
            if warp.hmax() > 0.0 {
                r = fbm3(q, 3, seed).mul_add(warp * 0.5, r);
            }
            let v = (r * core::f32::consts::TAU).sin();
            ((v * sharp).mul_add(F8::splat(0.5), F8::splat(0.5)).saturate(), one)
        }
        tex::SCALES => {
            let (u, v) = plane_uv(q, n);
            let row = v.floor();
            let uu = row.mul_add(F8::splat(0.5), u);
            // each scale is a disc hanging from the middle of its cell's lower edge
            let (fu, fv) = (uu.fract() - 0.5, v.fract());
            let d = fu.mul_add(fu, fv * fv).sqrt();
            let rim = smoothstep(0.45, 0.72, d * sharp);
            let shade = unit(lowbias(uu.floor().to_i8().mul_lo(I8::splat(K1)) ^ row.to_i8().mul_lo(I8::splat(K2)))) * 0.25 + 0.88;
            (rim, shade)
        }
        tex::WEAVE => {
            let (u, v) = plane_uv(q, n);
            let (u, v) = (u * 2.0, v * 2.0);
            let along_u = ((u.floor() + v.floor()) * 0.5).fract().lt(F8::splat(0.25));
            let across = F8::select(along_u, v.fract(), u.fract());
            let along = F8::select(along_u, u.fract(), v.fract());
            let thread = (across * core::f32::consts::PI).sin();
            // the gaps, and a little of the second colour on the strands one way
            let gap = (one - smoothstep(0.0, 0.35, thread * sharp)).max(F8::select(along_u, F8::zero(), F8::splat(0.3)));
            let shade = thread.mul_add(F8::splat(0.4), F8::splat(0.6)) * (along * core::f32::consts::PI).sin().mul_add(F8::splat(0.2), F8::splat(0.8));
            (gap, shade)
        }
        tex::WINDOWS => {
            // a facade: a window in each cell (1 wide, 1.4 tall: storeys), a
            // share of them lit, more on some floors than others; roofs plain
            let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
            let top = ay.ge(ax) & ay.ge(az);
            let side = ax.ge(az).and_not(top);
            let u = F8::select(side, q.z, q.x);
            let v = q.y * (1.0 / 1.4);
            // the face's own coordinate, a quarter cell inside (steady on cell borders)
            let inner = n.mul_add(F8::splat(-0.25), q);
            let w = F8::select(side, inner.x, inner.z);
            let (cu, cv) = (u.floor(), v.floor());
            let (fu, fv) = (u - cu, v - cv);
            let inside = (F8::splat(0.36) - (fu - 0.5).abs()).min(fv - 0.2).min(F8::splat(0.86) - fv);
            // a mullion down the middle; lit from the ceiling, brighter up top
            let mullion = smoothstep(0.0, 0.02, ((fu - 0.5).abs() - 0.012) * sharp);
            let win = (smoothstep(0.0, 0.04, inside * sharp) * mullion).and(!top);
            let up = ((fv - 0.2) * (1.0 / 0.66)).saturate().mul_add(F8::splat(0.35), F8::splat(0.65));
            let (iu, iv, iw) = (cu.to_i8().mul_lo(I8::splat(K1)), cv.to_i8().mul_lo(I8::splat(K2)), w.floor().to_i8().mul_lo(I8::splat(K3)));
            let floor = unit(lowbias(iv ^ iw ^ I8::splat(0x2545_f491)));
            let h = lowbias(iu ^ iv ^ iw);
            let lit = unit(h).lt(floor.mul_add(F8::splat(0.6), F8::splat(0.15)));
            let glow = unit(lowbias(h ^ 0x1b87_3593)).mul_add(F8::splat(0.45), F8::splat(0.55));
            let t = win * F8::select(lit, glow * up, F8::zero());
            let shade = F8::select(lit, one, win.neg_mul_add(F8::splat(0.7), one));
            (t, shade)
        }
        _ => (F8::zero(), one),
    }
}

/// The surface's main plane: (u, v) of `q` seen along the axis nearest the
/// normal (top: x and z; sides: z or x, and y).
#[inline(always)]
fn plane_uv(q: V8, n: V8) -> (F8, F8) {
    let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
    let top = ay.ge(ax) & ay.ge(az);
    let side = ax.ge(az).and_not(top);
    (F8::select(top, q.x, F8::select(side, q.z, q.x)), F8::select(top, q.z, q.y))
}

#[inline(always)]
fn len2(a: F8, b: F8) -> F8 {
    b.mul_add(b, a * a).sqrt()
}

/// `m` times eight vectors.
#[inline(always)]
pub fn mat_apply(m: &Mat3, v: V8) -> V8 {
    V8::new(v.dot3(m.m[0]), v.dot3(m.m[1]), v.dot3(m.m[2]))
}

/// The transpose of `m` times eight vectors.
#[inline(always)]
pub fn mat_apply_t(m: &Mat3, v: V8) -> V8 {
    let r = &m.m;
    V8::new(
        v.z.mul_add(F8::splat(r[2].x), v.y.mul_add(F8::splat(r[1].x), v.x * r[0].x)),
        v.z.mul_add(F8::splat(r[2].y), v.y.mul_add(F8::splat(r[1].y), v.x * r[0].y)),
        v.z.mul_add(F8::splat(r[2].z), v.y.mul_add(F8::splat(r[1].z), v.x * r[0].z)),
    )
}

/// The pattern's gradient in pattern units (central differences), for bump.
#[inline]
pub fn gradient(kind: i32, q: V8, n: V8, warp: F8, sharp: F8) -> V8 {
    let e = 0.01;
    let at = |dx: f32, dy: f32, dz: f32| eval(kind, q + crate::math::Vec3::new(dx, dy, dz), n, warp, sharp).0;
    let gx = at(e, 0.0, 0.0) - at(-e, 0.0, 0.0);
    let gy = at(0.0, e, 0.0) - at(0.0, -e, 0.0);
    let gz = at(0.0, 0.0, e) - at(0.0, 0.0, -e);
    V8::new(gx, gy, gz) * (0.5 / e)
}
