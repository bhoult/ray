//! NURBS surfaces (scene/nurbs.rs) on eight rays at once: the nearest hit by
//! Newton's method on each piece the rays reach, and the distance to the
//! surface by projecting onto the nearby pieces (AO, physics). Rays and
//! points arrive in the surface's own frame.

use super::trace::{BIG, Ray, slab};
use crate::math::Aabb;
use crate::math::Vec3;
use crate::scene::nurbs::{Grid, Nurbs, Patch, Piece};
use crate::simd::{F8, I8, M8, V8};

/// Newton steps on a piece at most (they converge in 3 to 5 when it is near).
const ITER: usize = 10;
/// Gauss-Newton steps toward the nearest point of a piece (distances).
const PROJECT: usize = 1;

/// The patch at (u, v) for every lane: the point and its tangents along u and v.
#[inline(always)]
pub fn eval8(p: &Patch, u: F8, v: F8) -> (V8, V8, V8) {
    #[inline(always)]
    fn bern(t: F8) -> ([F8; 4], [F8; 4]) {
        let s = F8::one() - t;
        let (t2, s2, ts) = (t * t, s * s, t * s);
        ([s2 * s, ts * s * 3.0, ts * t * 3.0, t2 * t], [s2 * -3.0, (s2 - ts * 2.0) * 3.0, (ts * 2.0 - t2) * 3.0, t2 * 3.0])
    }
    let ((bu, du), (bv, dv)) = (bern(u), bern(v));
    let z = F8::zero();
    let (mut h, mut hu, mut hv) = ([z; 4], [z; 4], [z; 4]);
    for i in 0..4 {
        // the row along v at u-index i, and its v derivative
        let (mut r, mut rv) = ([z; 4], [z; 4]);
        for j in 0..4 {
            let c = p[i * 4 + j];
            for k in 0..4 {
                let ck = F8::splat(c[k]);
                r[k] = bv[j].mul_add(ck, r[k]);
                rv[k] = dv[j].mul_add(ck, rv[k]);
            }
        }
        for k in 0..4 {
            h[k] = bu[i].mul_add(r[k], h[k]);
            hu[k] = du[i].mul_add(r[k], hu[k]);
            hv[k] = bu[i].mul_add(rv[k], hv[k]);
        }
    }
    let iw = F8::one() / h[3];
    let s = V8::new(h[0] * iw, h[1] * iw, h[2] * iw);
    let su = V8::new(s.x.neg_mul_add(hu[3], hu[0]) * iw, s.y.neg_mul_add(hu[3], hu[1]) * iw, s.z.neg_mul_add(hu[3], hu[2]) * iw);
    let sv = V8::new(s.x.neg_mul_add(hv[3], hv[0]) * iw, s.y.neg_mul_add(hv[3], hv[1]) * iw, s.z.neg_mul_add(hv[3], hv[2]) * iw);
    (s, su, sv)
}

/// The unit normal from the tangents, turned outward; where they collapse (a
/// pole: a profile end on the axis) the normal a little toward the piece's
/// centre stands in.
#[inline(always)]
fn normal(surf: &Nurbs, p: &Patch, pc: &Piece, u: F8, v: F8, su: V8, sv: V8) -> V8 {
    let mut n = su.cross(sv);
    let span = su.length_sq() + sv.length_sq();
    let tiny = n.length_sq().le(span * span * 1e-6);
    if tiny.any() {
        let (cu, cv) = ((pc.u0 + pc.u1) * 0.5, (pc.v0 + pc.v1) * 0.5);
        let (_, su2, sv2) = eval8(p, (F8::splat(cu) - u).mul_add(F8::splat(0.02), u), (F8::splat(cv) - v).mul_add(F8::splat(0.02), v));
        n = V8::select(tiny, su2.cross(sv2), n);
    }
    n * (n.length_sq().max(F8::splat(1e-30)).rsqrt() * surf.orient)
}

/// A ray as two planes through it: n1.x + d1 = 0 and n2.x + d2 = 0.
struct Planes {
    n1: V8,
    n2: V8,
    d1: F8,
    d2: F8,
}

/// Newton's method on (u, v) for the lanes `todo`, from (u, v): where the
/// patch meets the rays, the lanes that converged there, and the iterations
/// each lane took (added to `steps`: the march-steps view's measure).
#[inline(always)]
fn newton(p: &Patch, mut u: F8, mut v: F8, mut todo: M8, r: &Planes, eps: F8, steps: &mut I8) -> (F8, F8, M8) {
    let mut ok = M8::none_set();
    for _ in 0..ITER {
        *steps = *steps - todo.to_i8();
        let (s, su, sv) = eval8(p, u, v);
        let f1 = s.dot(r.n1) + r.d1;
        let f2 = s.dot(r.n2) + r.d2;
        let conv = (f1.abs() + f2.abs()).lt(eps) & todo;
        ok |= conv;
        todo = todo.and_not(conv);
        if todo.none() {
            break;
        }
        let (j11, j12, j21, j22) = (su.dot(r.n1), sv.dot(r.n1), su.dot(r.n2), sv.dot(r.n2));
        let det = j11 * j22 - j12 * j21;
        todo &= det.abs().gt(F8::splat(1e-20));
        let inv = F8::one() / det;
        u = F8::select(todo, u - (j22 * f1 - j12 * f2) * inv, u);
        v = F8::select(todo, v - (j11 * f2 - j21 * f1) * inv, v);
        // wandered off the patch: no root here
        todo &= u.ge(F8::splat(-0.5)) & u.le(F8::splat(1.5)) & v.ge(F8::splat(-0.5)) & v.le(F8::splat(1.5));
    }
    (u, v, ok)
}

/// What a ray query wants: the nearest hit, also its normal, or any hit (shadows).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Find {
    Nearest,
    Normal,
    Any,
}

/// The nearest hit along the rays (unit directions, own frame) between `near`
/// and `limit`, for the lanes in `active`: distance (BIG on a miss) and, for
/// `Find::Normal`, the outward unit normal there; and the Newton iterations
/// each lane took. `Find::Any` stops a lane at its first hit.
pub fn hit8(surf: &Nurbs, o: V8, d: V8, limit: F8, active: M8, near: f32, find: Find) -> (F8, V8, I8) {
    let want_normal = find == Find::Normal;
    // (its own frame: no distance programs, the anchor offset is unused)
    let ray = Ray::new(o, o, d, I8::splat(-1));
    let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
    let xm = ax.gt(ay) & ax.gt(az);
    let z = F8::zero();
    let n1 = V8::select(xm, V8::new(d.y, -d.x, z), V8::new(z, d.z, -d.y)).normalize();
    let n2 = n1.cross(d);
    let plane = Planes { n1, n2, d1: -n1.dot(o), d2: -n2.dot(o) };
    let eps = F8::splat(2e-5 * surf.size.max(1.0));
    let mut best = F8::select(active, limit, F8::splat(-1.0));
    let mut found = F8::splat(BIG);
    let mut nrm = V8::zero();
    let mut steps = I8::zero();

    let reach = |tn: F8, tf: F8, best: F8| tn.le(tf) & tf.ge(F8::splat(near)) & tn.lt(best) & active;
    let nodes = &surf.nodes;
    let mut stack = [0u32; 48];
    let mut sp = 1;
    while sp > 0 {
        sp -= 1;
        let nd = &nodes[stack[sp] as usize];
        let (tn, tf) = slab(&ray, nd.min, nd.max);
        let live = reach(tn, tf, best);
        if live.none() {
            continue;
        }
        if nd.count == 0 {
            if sp + 2 <= stack.len() {
                stack[sp] = nd.index + 1;
                stack[sp + 1] = nd.index;
                sp += 2;
            }
            continue;
        }
        for pc in &surf.pieces[nd.index as usize..(nd.index + nd.count) as usize] {
            let p = &surf.patches[pc.patch as usize];
            let (w, h) = (pc.u1 - pc.u0, pc.v1 - pc.v0);
            // from the centre; a grazing ray may cross the piece twice, close
            // together, so its lanes look for a nearer root from four more starts
            // only the lanes that cross the piece's box before their nearest hit so far
            let (pn, pf) = slab(&ray, pc.aabb.min, pc.aabb.max);
            let mut lanes = live & reach(pn, pf, best);
            if lanes.none() {
                continue;
            }
            for (k, (fu, fv)) in [(0.5, 0.5), (0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)].into_iter().enumerate() {
                if k > 0 && lanes.none() {
                    break;
                }
                let (u, v, ok) = newton(p, F8::splat(pc.u0 + w * fu), F8::splat(pc.v0 + h * fv), lanes, &plane, eps, &mut steps);
                let found_here = if ok.none() {
                    M8::none_set()
                } else {
                    // a root in the piece or near it (a neighbour's, found again, is harmless)
                    let inside = |x: F8, lo: f32, hi: f32, m: f32| x.ge(F8::splat((lo - m).max(-1e-4))) & x.le(F8::splat((hi + m).min(1.0 + 1e-4)));
                    let (s, su, sv) = eval8(p, u, v);
                    let t = (s - o).dot(d);
                    let here = ok & inside(u, pc.u0, pc.u1, w * 0.5) & inside(v, pc.v0, pc.v1, h * 0.5) & t.gt(F8::splat(near));
                    let take = here & t.lt(best);
                    if take.any() {
                        // (a lane that wants any hit is done: nothing reaches below -1)
                        best = F8::select(take, if find == Find::Any { F8::splat(-1.0) } else { t }, best);
                        found = F8::select(take, t, found);
                        if want_normal {
                            nrm = V8::select(take, normal(surf, p, pc, u, v, su, sv), nrm);
                        }
                    }
                    // (whether or not it was nearest so far, so that tracing the
                    // same ray again with a shorter limit finds the same root)
                    let n = su.cross(sv);
                    let c = n.dot(d);
                    here & (c * c).lt(n.length_sq() * 0.15)
                };
                if k == 0 {
                    lanes = found_here;
                }
            }
        }
    }
    (found, nrm, steps)
}

/// A box whose diagonal is under this fraction of its distance stands in
/// for what it holds in coarse queries: the surface touches the box's faces,
/// so it lies within about a tenth of that distance (AO cannot tell).
pub const FAR: f32 = 0.5;

/// Samples a side of a surface's distance grid.
pub const GRID: usize = 48;

/// The surface's distance grid (built the first time: GRID^3 exact signed
/// distances over its box and a tenth of its size round it, in parallel).
pub fn grid(surf: &Nurbs) -> &Grid {
    surf.grid.get_or_init(|| {
        use rayon::prelude::*;
        let n = GRID;
        let m = Vec3::splat(surf.size * 0.1);
        let lo = surf.aabb.min - m;
        let cell = (surf.aabb.max + m - lo) * (1.0 / (n - 1) as f32);
        let mut values = vec![0.0f32; n * n * n];
        // a row of n along x, eight at a time
        values.par_chunks_mut(n).enumerate().for_each(|(row, out)| {
            let (y, z) = ((row % n) as f32, (row / n) as f32);
            for x0 in (0..n).step_by(8) {
                let xs = F8::from_array(std::array::from_fn(|l| (x0 + l).min(n - 1) as f32));
                let p = V8::new(xs * cell.x + lo.x, F8::splat(lo.y + y * cell.y), F8::splat(lo.z + z * cell.z));
                let d = dist8(surf, p, 4.0 * surf.size, M8::all_true(), false).to_array();
                for l in 0..8 {
                    if x0 + l < n {
                        out[x0 + l] = d[l];
                    }
                }
            }
        });
        // (trilinear weights times the distances to the corners, each at most
        // the true distance's change: at most half the diagonal, at the
        // cell's centre; a tenth more for the projection's own error)
        Grid { n, lo, cell, slack: cell.length() * 0.55, values }
    })
}

/// The grid's trilinear distance at one point (clamped into the grid).
fn grid_at(g: &Grid, p: Vec3) -> f32 {
    let top = (g.n - 1) as f32 - 1e-4;
    let f = |v: f32, lo: f32, c: f32| ((v - lo) / c).clamp(0.0, top);
    let (fx, fy, fz) = (f(p.x, g.lo.x, g.cell.x), f(p.y, g.lo.y, g.cell.y), f(p.z, g.lo.z, g.cell.z));
    let (ix, iy, iz) = (fx as usize, fy as usize, fz as usize);
    let (tx, ty, tz) = (fx - ix as f32, fy - iy as f32, fz - iz as f32);
    let at = |x: usize, y: usize, z: usize| g.values[(z * g.n + y) * g.n + x];
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let row = |y: usize, z: usize| lerp(at(ix, y, z), at(ix + 1, y, z), tx);
    let plane = |z: usize| lerp(row(iy, z), row(iy + 1, z), ty);
    lerp(plane(iz), plane(iz + 1), tz)
}

/// The signed distance from `p` (own frame) to a surface that is a leaf of a
/// distance program (in a boolean, under a modifier). Outside its grid, the
/// distance to its box (a bound from below); in the grid but away from the
/// surface, the grid's distance (the right sign, within a third of the true
/// distance there); near it, exact, from the pieces within the grid's estimate
/// (searching the whole surface at every step of a march, as when deep
/// inside a round solid every piece is about as far, took seconds a frame;
/// and a search within too small a radius called such a point outside).
pub fn sdf8(surf: &Nurbs, p: V8) -> F8 {
    let g = grid(surf);
    let boxd = |lo: Vec3, hi: Vec3| {
        V8::new(
            (F8::splat(lo.x) - p.x).max(p.x - hi.x).max(F8::zero()),
            (F8::splat(lo.y) - p.y).max(p.y - hi.y).max(F8::zero()),
            (F8::splat(lo.z) - p.z).max(p.z - hi.z).max(F8::zero()),
        )
        .length()
    };
    let hi = g.lo + g.cell * (g.n - 1) as f32;
    let inside = boxd(g.lo, hi).le(F8::zero());
    let mut d = boxd(surf.aabb.min, surf.aabb.max);
    if inside.none() {
        return d;
    }
    let (xs, ys, zs) = (p.x.to_array(), p.y.to_array(), p.z.to_array());
    let v = F8::from_array(std::array::from_fn(|l| grid_at(g, Vec3::new(xs[l], ys[l], zs[l]))));
    // (away from it, the grid's own value: unbiased, so the shading's AO and
    // soft shadows, which read distances, are not darkened as by a value
    // shortened by the slack; the march stays short of the surface by
    // stepping at most two thirds of it, Compiler's step for NURBS leaves:
    // the true distance is at least |v| less the slack, two thirds of it)
    let slack = F8::splat(g.slack);
    let far = inside & v.abs().gt(slack * 3.0);
    d = F8::select(far, v, d);
    let near = inside.and_not(far);
    if near.none() {
        return d;
    }
    // (the surface is within the grid's estimate and its slack: the nearest
    // piece is found, so the sign is right)
    let reach = F8::select(near, v.abs() + slack * 1.05, F8::zero()).hmax();
    F8::select(near, dist8(surf, p, reach, near, false), d)
}

/// The distance from `p` (own frame) to the surface, at most `radius`:
/// signed (negative inside) when the surface is solid. `coarse` (AO) takes a
/// small box's distance for the surface inside it, unsigned.
pub fn dist8(surf: &Nurbs, p: V8, radius: f32, active: M8, coarse: bool) -> F8 {
    let mut best = F8::select(active, F8::splat(radius), F8::zero());
    let mut signed = F8::splat(radius);
    let boxd = |b: &Aabb| {
        let o = V8::new(
            (F8::splat(b.min.x) - p.x).max(p.x - b.max.x).max(F8::zero()),
            (F8::splat(b.min.y) - p.y).max(p.y - b.max.y).max(F8::zero()),
            (F8::splat(b.min.z) - p.z).max(p.z - b.max.z).max(F8::zero()),
        );
        o.length()
    };
    // the lanes still to refine inside a box, after the far ones take its distance
    let near = |b: &Aabb, live: M8, best: &mut F8, signed: &mut F8| -> M8 {
        let bd = boxd(b);
        let live = live & bd.lt(*best);
        if !coarse || live.none() {
            return live;
        }
        let far = live & F8::splat(b.size().length()).lt(bd * FAR);
        *best = F8::select(far, bd, *best);
        *signed = F8::select(far, bd, *signed);
        live.and_not(far)
    };
    let nodes = &surf.nodes;
    let node_box = |i: u32| Aabb::new(nodes[i as usize].min, nodes[i as usize].max);
    // (node, lanes)
    let mut stack = [(0u32, 0u32); 48];
    stack[0] = (0, active.bits());
    let mut sp = 1;
    while sp > 0 {
        sp -= 1;
        let (at, bits) = stack[sp];
        let nd = &nodes[at as usize];
        let live = near(&node_box(at), M8::from_bits(bits), &mut best, &mut signed);
        if live.none() {
            continue;
        }
        if nd.count == 0 {
            // the nearer child first: what it finds prunes the other
            let (l, r) = (nd.index, nd.index + 1);
            let near_left = boxd(&node_box(l)).hmin() <= boxd(&node_box(r)).hmin();
            if sp + 2 <= stack.len() {
                stack[sp] = (if near_left { r } else { l }, live.bits());
                stack[sp + 1] = (if near_left { l } else { r }, live.bits());
                sp += 2;
            }
            continue;
        }
        for pc in &surf.pieces[nd.index as usize..(nd.index + nd.count) as usize] {
            let live = near(&pc.aabb, live, &mut best, &mut signed);
            if live.none() {
                continue;
            }
            let pa = &surf.patches[pc.patch as usize];
            let (lo_u, hi_u, lo_v, hi_v) = (F8::splat(pc.u0), F8::splat(pc.u1), F8::splat(pc.v0), F8::splat(pc.v1));
            let (mut u, mut v) = (F8::splat((pc.u0 + pc.u1) * 0.5), F8::splat((pc.v0 + pc.v1) * 0.5));
            // Gauss-Newton on |S(u, v) - p|^2, kept inside the piece: the
            // pieces are flat enough that one step lands within 1e-3 of the size
            for _ in 0..PROJECT {
                let (s, su, sv) = eval8(pa, u, v);
                let r = s - p;
                let (g1, g2) = (su.dot(r), sv.dot(r));
                let (a, b, c) = (su.length_sq(), su.dot(sv), sv.length_sq());
                let det = (a * c - b * b).max(F8::splat(1e-30));
                u = (u - (c * g1 - b * g2) / det).max(lo_u).min(hi_u);
                v = (v - (a * g2 - b * g1) / det).max(lo_v).min(hi_v);
            }
            let (s, su, sv) = eval8(pa, u, v);
            let r = p - s;
            let dd = r.length();
            let nearer = dd.lt(best) & live;
            if nearer.none() {
                continue;
            }
            best = F8::select(nearer, dd, best);
            let sd = if surf.solid {
                let out = r.dot(normal(surf, pa, pc, u, v, su, sv));
                F8::select(out.lt(F8::zero()), -dd, dd)
            } else {
                dd
            };
            signed = F8::select(nearer, sd, signed);
        }
    }
    signed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;
    use crate::scene::json::ShapeDef;

    fn sphere(r: f32) -> Nurbs {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let shape = ShapeDef::Revolve {
            profile: vec![vec![0.0, -r], vec![r, -r, h], vec![r, 0.0], vec![r, r, h], vec![0.0, r]],
            degree: 2,
            knots: Some(vec![0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0]),
            angle: 360.0,
            solid: None,
        };
        Nurbs::from_shape(&shape).unwrap().unwrap()
    }

    #[test]
    fn rays_hit_a_revolved_sphere_where_the_formula_does() {
        let s = sphere(1.5);
        assert!(s.solid);
        let mut misses = 0;
        let mut worst = 0.0f32;
        // camera straight above (on the axis: the pole), beside, and below
        for eye in [Vec3::new(0.0, 6.0, 0.0), Vec3::new(5.0, 0.3, 2.0), Vec3::new(-0.2, -7.0, 0.1)] {
            for k in 0..64 {
                let jitter = |a: f32| ((k as f32 * a).sin() * 43758.5453).fract() - 0.5;
                // rays at the centre (k = 0 hits the pole exactly from above) and spread over the disc
                let target = if k == 0 { Vec3::ZERO } else { Vec3::new(jitter(12.9), jitter(78.2), jitter(37.7)) * 2.6 };
                let dir = (target - eye).normalize();
                let (t, n, _) = hit8(&s, V8::splat(eye), V8::splat(dir), F8::splat(100.0), M8::all_true(), 1e-4, Find::Normal);
                let t = t.lane(0);
                // the formula
                let b = eye.dot(dir);
                let h = b * b - (eye.length_sq() - 1.5 * 1.5);
                if h <= 1e-4 {
                    assert!(t > 1e8 || h > -1e-3, "hit a missed ray: {t}");
                    continue;
                }
                let want = -b - h.sqrt();
                if t > 1e8 {
                    misses += 1;
                    continue;
                }
                worst = worst.max((t - want).abs());
                let hp = eye + dir * t;
                assert!(n.lane(0).dot(hp.normalize()) > 0.999, "normal at {hp:?}: {:?}", n.lane(0));
            }
        }
        assert!(worst < 1e-3, "hit distance off by {worst}");
        assert_eq!(misses, 0, "rays that should hit missed");
    }

    #[test]
    fn distance_to_a_revolved_sphere() {
        let s = sphere(1.0);
        for q in [Vec3::new(0.0, 1.6, 0.0), Vec3::new(0.3, 0.2, -0.1), Vec3::new(1.2, -0.4, 0.7), Vec3::new(0.0, -0.95, 0.0)] {
            let d = dist8(&s, V8::splat(q), 10.0, M8::all_true(), false).lane(0);
            assert!((d - (q.length() - 1.0)).abs() < 1e-3, "at {q:?}: {d}");
        }
    }
    /// A NURBS leaf's distance (sdf8, from the grid away from the surface,
    /// exact near it) agrees with the sphere's own: within the grid's slack,
    /// with the right sign wherever the surface is further than that (deep
    /// inside too: a difference cut from it depends on it), exact close to
    /// the surface, and outside the grid a bound from below.
    #[test]
    fn leaf_distance_follows_a_sphere() {
        let s = sphere(1.0);
        let slack = grid(&s).slack;
        let (mut worst, mut n) = (0.0f32, 0);
        let mut seed = 0x1234_5678u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        for _ in 0..400 {
            // inside the grid (the box and a tenth more round it)
            let q = Vec3::new(rnd(), rnd(), rnd()) * 1.15;
            let truth = q.length() - 1.0;
            let d = sdf8(&s, V8::splat(q)).lane(0);
            worst = worst.max((d - truth).abs());
            if truth.abs() > slack {
                assert_eq!(d < 0.0, truth < 0.0, "the sign at {q:?}: {d} for {truth}");
            }
            if truth.abs() < 0.5 * slack {
                assert!((d - truth).abs() < 2e-3, "near the surface it is exact: at {q:?} {d} for {truth}");
            }
            n += 1;
        }
        assert!(worst <= slack * 1.01, "off by {worst} (slack {slack}) over {n} points");
        // outside the grid: no further than the surface
        for q in [Vec3::new(3.0, 0.2, 0.0), Vec3::new(-1.5, 1.5, 1.5), Vec3::new(0.0, -4.0, 0.0)] {
            let d = sdf8(&s, V8::splat(q)).lane(0);
            assert!(d > 0.0 && d <= q.length() - 1.0 + 1e-4, "outside at {q:?}: {d}");
        }
    }

    /// A torus (major radius 0.55, minor 0.26) as a revolved rational circle.
    fn torus() -> Nurbs {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let profile = (0..9)
            .map(|k| {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (r, w) = if k % 2 == 0 { (0.26, 1.0) } else { (0.26 / h, h) };
                vec![0.55 + r * a.cos(), r * a.sin(), w]
            })
            .collect();
        let shape = ShapeDef::Revolve { profile, degree: 2, knots: Some(vec![0.0, 0.0, 0.0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1.0, 1.0, 1.0]), angle: 360.0, solid: None };
        Nurbs::from_shape(&shape).unwrap().unwrap()
    }

    #[test]
    fn rays_find_a_torus_including_grazing_ones() {
        let s = torus();
        let f = |p: Vec3| ((p.x * p.x + p.z * p.z).sqrt() - 0.55).powi(2) + p.y * p.y - 0.26 * 0.26;
        let (mut misses, mut checked, mut worst) = (0, 0, 0.0f32);
        let mut seed = 1u32;
        let mut rnd = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        for _ in 0..4000 {
            let eye = Vec3::new(rnd() - 0.5, rnd() - 0.5, rnd() - 0.5).normalize() * 3.0;
            let target = Vec3::new(rnd() - 0.5, (rnd() - 0.5) * 0.6, rnd() - 0.5) * 1.8;
            let dir = (target - eye).normalize();
            // the reference: fine steps, then bisection
            let mut want = None;
            let mut t = 0.0;
            while t < 6.0 {
                if f(eye + dir * (t + 0.0005)) < 0.0 {
                    let (mut a, mut b) = (t, t + 0.0005);
                    for _ in 0..40 {
                        let m = (a + b) * 0.5;
                        if f(eye + dir * m) < 0.0 { b = m } else { a = m }
                    }
                    want = Some(a);
                    break;
                }
                t += 0.0005;
            }
            let Some(want) = want else { continue };
            checked += 1;
            let got = hit8(&s, V8::splat(eye), V8::splat(dir), F8::splat(100.0), M8::all_true(), 1e-4, Find::Nearest).0.lane(0);
            if got > 1e8 {
                misses += 1;
            } else if got < want && f(eye + dir * got).abs() < 1e-4 {
                // a graze thinner than the reference's step: a true nearer hit
            } else {
                worst = worst.max((got - want).abs());
            }
        }
        assert!(checked > 1000);
        assert!(worst < 2e-3, "hit distance off by {worst}");
        assert_eq!(misses, 0, "{misses} of {checked} rays that hit the torus missed");
    }

}
