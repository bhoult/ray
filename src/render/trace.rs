//! Ray queries on eight rays at once: nearest hit, shadow visibility and the
//! scene's distance at a point (for AO). Objects are found through the BVH
//! (the eight rays traverse it as a packet) plus the unbounded list; each is
//! then intersected exactly (sphere, box, plane), sphere-traced inside its
//! bounds (SDF programs, as fraktal256 marched its structures inside their
//! bounding spheres), height-marched (terrain, fraktal256's terrain march) or
//! solved on its patches (NURBS, render/nurbs.rs).

use super::Ctx;
use super::vm::{Regs, eval, ticks};
use crate::math::Vec3;
use super::nurbs::Find;
use crate::scene::nurbs::Nurbs;
use crate::scene::terrain::Terrain;
use crate::scene::{Scene, Shape};
use crate::simd::{F8, I8, M8, V8};

/// "No hit" distance.
pub const BIG: f32 = 1.0e9;
/// f32's resolution relative to a coordinate's size: four of its steps
/// (2^-21: half a micrometre a metre out).
pub const REL: f32 = 1.0 / 2_097_152.0;

/// The finest a march resolves `m` metres from where its positions are
/// measured from (Object.org; hit points: the world's origin): a few f32
/// steps there. Relative, not a fixed length: a fixed 4 µm floor rounded off
/// a sponge's micrometre holes and drew arcs where it took over from the
/// pixel footprint, though the floats resolved far finer. (The constant only
/// keeps it above zero: an anchored walker is nanometres tall.)
#[inline(always)]
pub fn grain(m: f32) -> f32 {
    m.mul_add(REL, 1.0e-12)
}

/// `grain` for eight lanes.
#[inline(always)]
pub fn grain8(m: F8) -> F8 {
    m.mul_add(F8::splat(REL), F8::splat(1.0e-12))
}

/// Eight rays, with the reciprocal direction for slab tests and the object
/// each lane travels inside (-1: none), which flips that object's distance;
/// and the origin's offset from the frame's anchor (Ctx::anchor), which near
/// the camera is finer than the origin itself (anchored fractals use it);
/// and how near a hit on an exact shape may be (about an f32 step of the
/// origin: nearer is the surface the ray left), worked out once per ray.
#[derive(Clone, Copy)]
pub struct Ray {
    pub o: V8,
    pub od: V8,
    pub d: V8,
    pub inv: V8,
    pub inside: I8,
    pub near: F8,
}

impl Ray {
    #[inline(always)]
    pub fn new(o: V8, od: V8, d: V8, inside: I8) -> Ray {
        let inv = |c: F8| F8::one() / c.abs().max(F8::splat(1e-12)).copysign(c);
        Ray { o, od, d, inv: V8::new(inv(d.x), inv(d.y), inv(d.z)), inside, near: grain8(o.length()) * 0.25 }
    }

    /// This ray in an object's own frame (origin `o`, direction `d`): its
    /// near limit and the rest stay.
    #[inline(always)]
    pub fn local(&self, o: V8, d: V8) -> Ray {
        let inv = |c: F8| F8::one() / c.abs().max(F8::splat(1e-12)).copysign(c);
        Ray { o, d, inv: V8::new(inv(d.x), inv(d.y), inv(d.z)), ..*self }
    }
}

/// The nearest hits: distance (the query's limit on a miss), object (-1 on a
/// miss) and the march iterations spent (the cost view).
pub struct Hit {
    pub t: F8,
    pub obj: I8,
    pub steps: I8,
}

#[inline(always)]
pub(crate) fn slab(r: &Ray, min: Vec3, max: Vec3) -> (F8, F8) {
    let ax = |o: F8, inv: F8, lo: f32, hi: f32| {
        let t1 = (F8::splat(lo) - o) * inv;
        let t2 = (F8::splat(hi) - o) * inv;
        (t1.min(t2), t1.max(t2))
    };
    let (nx, fx) = ax(r.o.x, r.inv.x, min.x, max.x);
    let (ny, fy) = ax(r.o.y, r.inv.y, min.y, max.y);
    let (nz, fz) = ax(r.o.z, r.inv.z, min.z, max.z);
    (nx.max(ny).max(nz), fx.min(fy).min(fz))
}

/// Visits the objects the rays may hit before `limit`: the unbounded ones,
/// then the BVH leaves, nearer child first. `visit(object, limit, active)`
/// returns the new limit and active lanes.
#[inline(always)]
fn traverse(sc: &Scene, ray: &Ray, limit: F8, active: M8, mut visit: impl FnMut(u32, F8, M8) -> (F8, M8)) {
    let (mut limit, mut active) = (limit, active);
    for &i in &sc.unbounded {
        (limit, active) = visit(i, limit, active);
        if active.none() {
            return;
        }
    }
    let nodes = &sc.bvh.nodes;
    if nodes.is_empty() {
        return;
    }
    let reach = |tn: F8, tf: F8, limit: F8, active: M8| tn.le(tf) & tf.ge(F8::zero()) & tn.lt(limit) & active;
    let mut stack = [0u32; 64];
    let mut sp = 1;
    while sp > 0 {
        sp -= 1;
        let n = &nodes[stack[sp] as usize];
        let (tn, tf) = slab(ray, n.min, n.max);
        if reach(tn, tf, limit, active).none() {
            continue;
        }
        if n.count > 0 {
            for &item in &sc.bvh.items[n.index as usize..(n.index + n.count) as usize] {
                (limit, active) = visit(item, limit, active);
                if active.none() {
                    return;
                }
            }
        } else {
            let (l, r) = (n.index, n.index + 1);
            let (ln, lf) = { let c = &nodes[l as usize]; slab(ray, c.min, c.max) };
            let (rn, rf) = { let c = &nodes[r as usize]; slab(ray, c.min, c.max) };
            let lm = reach(ln, lf, limit, active);
            let rm = reach(rn, rf, limit, active);
            let near_left = F8::select(lm, ln, F8::splat(BIG)).hmin() <= F8::select(rm, rn, F8::splat(BIG)).hmin();
            let (first, second) = if near_left { ((l, lm), (r, rm)) } else { ((r, rm), (l, lm)) };
            if second.1.any() && sp < 64 {
                stack[sp] = second.0;
                sp += 1;
            }
            if first.1.any() && sp < 64 {
                stack[sp] = first.0;
                sp += 1;
            }
        }
    }
}

/// The nearest hit along the rays before `tmax` for the lanes in `active`
/// (inactive lanes come back with t = -1, obj = -1). `cone` is the hit
/// threshold per metre of distance (the pixel footprint).
pub fn intersect(ctx: &Ctx, regs: &mut Regs, ray: &Ray, tmax: F8, active: M8, cone: f32) -> Hit {
    let mut hit = Hit { t: F8::select(active, tmax, F8::splat(-1.0)), obj: I8::splat(-1), steps: I8::zero() };
    traverse(ctx.scene, ray, hit.t, active, |i, _, act| {
        if ctx.profile {
            let t0 = ticks();
            let n = object(ctx, regs, i, ray, &mut hit, cone);
            regs.charge(i, ticks() - t0, n);
        } else {
            object(ctx, regs, i, ray, &mut hit, cone);
        }
        (hit.t, act)
    });
    hit
}

/// Intersects object `idx`, keeping nearer hits in `hit`; returns the 8-lane
/// evaluations it took.
fn object(ctx: &Ctx, regs: &mut Regs, idx: u32, ray: &Ray, hit: &mut Hit, cone: f32) -> u64 {
    let mut evals = 1;
    let o = &ctx.scene.objects[idx as usize];
    // (hits nearer than about an f32 step of the origin are the surface the
    // ray left; a fixed 10 µm made floors see-through from closer)
    let near = ray.near;
    let (t, m) = match &o.shape {
        Shape::Sphere { c, r } => {
            let oc = ray.o - *c;
            let b = oc.dot(ray.d);
            let h = b * b - (oc.length_sq() - r * r);
            let sq = h.max(F8::zero()).sqrt();
            let (t0, t1) = (-b - sq, sq - b);
            let t = F8::select(t0.gt(near), t0, t1);
            (t, h.gt(F8::zero()) & t.gt(near))
        }
        Shape::Box { c, rot, half } => {
            let (lo, ld) = to_local(ray, *c, rot);
            let local = ray.local(lo, ld);
            let (tn, tf) = slab(&local, -*half, *half);
            let t = F8::select(tn.gt(near), tn, tf);
            (t, tn.le(tf) & t.gt(near))
        }
        Shape::Plane { n, d } => {
            let dn = ray.d.dot3(*n);
            let t = (F8::splat(*d) - ray.o.dot3(*n)) / dn;
            (t, t.gt(near) & dn.abs().gt(F8::splat(1e-9)))
        }
        Shape::Sdf(prog) => {
            let (tn, tf) = slab(ray, o.aabb.min, o.aabb.max);
            let t0 = tn.max(F8::zero());
            let end = tf.min(hit.t);
            let act = t0.lt(end);
            if act.none() {
                return evals;
            }
            let sign = F8::select(ray.inside.eq(I8::splat(idx as i32)), F8::splat(-1.0), F8::one());
            ctx.prepare(regs, idx as usize);
            let fine = prog.anchored && ctx.tables.get(idx as usize).is_some_and(|t| !t.is_empty());
            let (t, steps, n) = march_sdf(prog, o.step, o.org, fine, regs, ray, t0, end, act, sign, cone, ctx.s.max_steps);
            evals = n;
            hit.steps = hit.steps + steps;
            (t, t.lt(F8::splat(BIG * 0.5)))
        }
        Shape::Nurbs { surf, c, rot, s } => {
            let (t, steps) = nurbs_hit(surf, *c, rot, *s, ray, hit.t, M8::all_true(), 1e-5, Find::Nearest);
            hit.steps = hit.steps + steps;
            (t, t.lt(F8::splat(BIG * 0.5)))
        }
        Shape::Terrain(tr) => {
            let (tn, tf) = slab(ray, o.aabb.min, o.aabb.max);
            let t0 = tn.max(F8::zero());
            let end = tf.min(hit.t);
            let act = t0.lt(end);
            if act.none() {
                return evals;
            }
            let (t, steps) = march_terrain(tr, ray, t0, end, act, ctx.s.max_steps + 60);
            evals = steps.to_array().into_iter().max().unwrap_or(0).max(0) as u64;
            hit.steps = hit.steps + steps;
            (t, t.lt(F8::splat(BIG * 0.5)))
        }
    };
    let m = m & t.lt(hit.t);
    hit.t = F8::select(m, t, hit.t);
    hit.obj = I8::select(m, I8::splat(idx as i32), hit.obj);
    evals
}

/// The nearest hit on a NURBS object before `limit` (BIG on a miss), in the
/// world's metres: the rays go into its frame, q = rot (p - c) / s.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
pub fn nurbs_hit(surf: &Nurbs, c: Vec3, rot: &crate::math::Mat3, s: f32, ray: &Ray, limit: F8, active: M8, near: f32, find: Find) -> (F8, I8) {
    let (lo, ld) = to_local(ray, c, rot);
    let k = 1.0 / s;
    let (t, _, steps) = super::nurbs::hit8(surf, lo * k, ld, limit * k, active, near * k, find);
    (F8::select(t.lt(F8::splat(BIG * 0.5)), t * s, F8::splat(BIG)), steps)
}

/// The rays in a box's frame.
#[inline(always)]
pub(crate) fn to_local(ray: &Ray, c: Vec3, rot: &crate::math::Mat3) -> (V8, V8) {
    let q = ray.o - c;
    let m = &rot.m;
    let ap = |v: V8| V8::new(v.dot3(m[0]), v.dot3(m[1]), v.dot3(m[2]));
    (ap(q), ap(ray.d))
}

/// Sphere-traces a distance program from `t0` to `end`: hit distances (BIG
/// on a miss), the iterations per lane and the 8-lane evaluations.
#[allow(clippy::too_many_arguments)]
#[inline(always)]
fn march_sdf(
    prog: &crate::scene::sdf::Program,
    step: f32,
    org: Vec3,
    anchored: bool,
    regs: &mut Regs,
    ray: &Ray,
    t0: F8,
    end: F8,
    active: M8,
    sign: F8,
    cone: f32,
    max_steps: u32,
) -> (F8, I8, u64) {
    let mut t = t0;
    let mut res = F8::splat(BIG);
    let mut act = active;
    let mut steps = I8::zero();
    let mut evals = 0;
    // (from the object's position: see Object.org; anchored, as fine as the
    // offset from the camera)
    let ro = ray.o - org;
    let g = grain8(if anchored { ray.od.length() } else { ro.length() });
    for _ in 0..max_steps {
        evals += 1;
        let thr = (t * cone * regs.thr_k).max(t.mul_add(F8::splat(REL), g));
        if regs.has_table() {
            regs.dp = V8::at(ray.od, ray.d, t);
        }
        let (d, _) = eval(prog, V8::at(ro, ray.d, t), regs, false);
        let d = d * sign;
        let h = d.lt(thr) & act;
        if h.any() {
            res = F8::select(h, t, res);
            act = act.and_not(h);
        }
        steps = steps - act.to_i8();
        t = F8::select(act, d.mul_add(F8::splat(step), t), t);
        act &= t.lt(end);
        if act.none() {
            break;
        }
    }
    (res, steps, evals)
}

/// fraktal256's relaxed height-field march: the step is `relax * height
/// above ground`, at least a fraction of a cell growing with distance; a
/// crossing is refined with two secant steps inside the bracket.
#[inline(always)]
pub fn march_terrain(tr: &Terrain, ray: &Ray, t0: F8, t_end: F8, active: M8, max_steps: u32) -> (F8, I8) {
    let (o, d) = (ray.o, ray.d);
    let big = F8::splat(BIG);
    let (step0, step1) = (F8::splat(0.05 * tr.cell), F8::splat(0.0025));
    let tiny = F8::splat(1e-20);
    let mut t = t0;
    let mut t_prev = t0;
    let mut h_prev = big;
    let mut active = active;
    let mut result = big;
    let mut steps = I8::zero();
    let (mut b_tp, mut b_hp, mut b_t, mut b_hh) = (F8::zero(), F8::zero(), F8::zero(), F8::zero());
    for _ in 0..max_steps {
        let p = V8::at(o, d, t);
        let ground = tr.height8(p.x, p.z);
        let dh = p.y - ground;
        let cross = dh.lt(F8::zero()) & active;
        if cross.any() {
            let frac = (h_prev / (h_prev - dh).max(tiny)).min(F8::one());
            let hit = frac.mul_add(t - t_prev, t_prev);
            result = F8::select(cross, hit, result);
            b_tp = F8::select(cross, t_prev, b_tp);
            b_hp = F8::select(cross, h_prev, b_hp);
            b_t = F8::select(cross, t, b_t);
            b_hh = F8::select(cross, dh, b_hh);
            active = active.and_not(cross);
        }
        steps = steps - active.to_i8();
        active &= t.lt(t_end);
        if active.none() {
            break;
        }
        t_prev = t;
        h_prev = dh;
        let step = (dh * tr.relax).max(t.mul_add(step1, step0));
        t += step.and(active);
    }
    let hits = result.lt(F8::splat(BIG * 0.5));
    if hits.none() {
        return (result, steps);
    }
    for _ in 0..2 {
        let p = V8::at(o, d, result);
        let dh = p.y - tr.height8(p.x, p.z);
        let below = dh.lt(F8::zero());
        b_t = F8::select(below, result, b_t);
        b_hh = F8::select(below, dh, b_hh);
        b_tp = F8::select(below, b_tp, result);
        b_hp = F8::select(below, b_hp, dh);
        let frac = (b_hp / (b_hp - b_hh).max(tiny)).min(F8::one());
        let refined = frac.mul_add(b_t - b_tp, b_tp);
        result = F8::select(hits, refined, result);
    }
    (result, steps)
}

/// Visibility 0..1 along the rays up to `tmax` (lanes `active`) through the
/// shadow-casting objects. `soft` uses the SDF penumbra estimate (`k`:
/// hardness) for distance shapes and terrain; analytic shapes are hard.
/// `skip` is an object to ignore (a light's own object), -1 for none.
#[allow(clippy::too_many_arguments)]
pub fn shadow(ctx: &Ctx, regs: &mut Regs, o: V8, od: V8, d: V8, tmax: F8, active: M8, soft: bool, k: f32, skip: i32) -> F8 {
    let ray = Ray::new(o, od, d, I8::splat(-1));
    let mut vis = F8::one();
    let near = ray.near;
    traverse(ctx.scene, &ray, tmax, active, |i, limit, act| {
        let ob = &ctx.scene.objects[i as usize];
        if !ob.shadow || i as i32 == skip {
            return (limit, act);
        }
        let t_start = if ctx.profile { ticks() } else { 0 };
        let mut evals = 1;
        let blocked = match &ob.shape {
            Shape::Sphere { c, r } => {
                let oc = ray.o - *c;
                let b = oc.dot(ray.d);
                let h = b * b - (oc.length_sq() - r * r);
                let sq = h.max(F8::zero()).sqrt();
                let (t0, t1) = (-b - sq, sq - b);
                let t = F8::select(t0.gt(near), t0, t1);
                h.gt(F8::zero()) & t.gt(near) & t.lt(limit)
            }
            Shape::Box { c, rot, half } => {
                let (lo, ld) = to_local(&ray, *c, rot);
                let (tn, tf) = slab(&ray.local(lo, ld), -*half, *half);
                let t = F8::select(tn.gt(near), tn, tf);
                tn.le(tf) & t.gt(near) & t.lt(limit)
            }
            Shape::Plane { n, d: pd } => {
                let t = (F8::splat(*pd) - ray.o.dot3(*n)) / ray.d.dot3(*n);
                t.gt(near) & t.lt(limit)
            }
            Shape::Nurbs { surf, c, rot, s } => nurbs_hit(surf, *c, rot, *s, &ray, limit, act, 1e-4, Find::Any).0.lt(F8::splat(BIG * 0.5)),
            Shape::Sdf(prog) => {
                let (tn, tf) = slab(&ray, ob.aabb.min, ob.aabb.max);
                let ro = ray.o - ob.org;
                ctx.prepare(regs, i as usize);
                let fine = prog.anchored && ctx.tables.get(i as usize).is_some_and(|t| !t.is_empty());
                let g = grain8(if fine { ray.od.length() } else { ro.length() });
                // (from just off the surface: close up its crevices shade it
                // within a millimetre)
                let mut t = tn.max(g * 2.0);
                let end = tf.min(limit);
                let mut a = act & t.lt(end);
                let step = F8::splat(ob.step);
                let max_steps = ctx.s.max_steps.min(160);
                                for _ in 0..max_steps {
                    if a.none() {
                        break;
                    }
                    evals += 1;
                    if regs.has_table() {
                        regs.dp = V8::at(ray.od, ray.d, t);
                    }
                    let (dist, _) = eval(prog, V8::at(ro, ray.d, t), regs, false);
                    if soft {
                        vis = F8::select(a, vis.min(dist * k / t), vis);
                    } else {
                        vis = F8::select(a & dist.lt(t.mul_add(F8::splat(REL), g)), F8::zero(), vis);
                    }
                    // (steps at least a 5000th of the way so far, up to 0.2 mm)
                    t += (dist * step).max(t.min(F8::one()).mul_add(F8::splat(2e-4), g * 2.0));
                    a &= t.lt(end) & vis.gt(F8::splat(0.02));
                }
                M8::none_set()
            }
            Shape::Terrain(tr) => {
                let (tn, tf) = slab(&ray, ob.aabb.min, ob.aabb.max);
                let mut t = tn.max(F8::splat(0.05 * tr.cell));
                let end = tf.min(limit);
                let mut a = act & t.lt(end);
                let kk = if soft { k * 0.4 } else { 1.0e4 };
                // (the penumbra is taken where the steps land: a step of at
                // least 5% of the way so far stepped over the closest
                // approach to a ridge, and a slope lit edge on was banded
                // along its contours, the bands crawling as the view moved;
                // at 1.5% they are gone, at the same cost)
                for _ in 0..96 {
                    if a.none() {
                        break;
                    }
                    evals += 1;
                    let p = V8::at(ray.o, ray.d, t);
                    let dh = p.y - tr.height8(p.x, p.z);
                    vis = F8::select(a, vis.min(dh * kk / t), vis);
                    let step = (dh * 0.6).max(F8::splat(0.6 * tr.cell)).min(F8::splat(16.0 * tr.cell)).max(t * 0.015);
                    t += step;
                    a &= t.lt(end) & vis.gt(F8::splat(0.02));
                }
                M8::none_set()
            }
        };
        vis = F8::select(blocked, F8::zero(), vis);
        if ctx.profile {
            regs.charge(i, ticks() - t_start, evals);
        }
        (limit, act & vis.gt(F8::splat(0.02)))
    });
    let v = vis.saturate();
    // clouds on the way dim it
    let v = (v * v * (F8::splat(3.0) - (v + v))).and(active);
    if ctx.scene.volumes.is_empty() || !v.gt(F8::zero()).any() {
        return v;
    }
    v * super::volume::transmittance(ctx, o, d, tmax, active & v.gt(F8::zero()))
}

/// The scene's distance at `p`, at most `radius`, from the shadow-casting
/// objects within `radius` (terrain: height above ground along its normal).
pub fn scene_dist(ctx: &Ctx, regs: &mut Regs, p: V8, dp: V8, radius: f32, skip_terrain: M8) -> F8 {
    query(ctx, regs, p, dp, radius, skip_terrain, &[], true).0
}

/// The distance at `p` to the nearest solid object not in `exclude` (at most
/// `radius`), and which object that is (-1: none within `radius`).
/// `skip_terrain`: lanes that leave terrain out (AO on terrain uses its baked cavities).
pub fn dist_query(ctx: &Ctx, regs: &mut Regs, p: V8, radius: f32, skip_terrain: M8, exclude: &[u32]) -> (F8, I8) {
    query(ctx, regs, p, p - V8::splat(ctx.anchor), radius, skip_terrain, exclude, false)
}

/// Object `i`'s distance at `p` (`dp` from the frame's anchor), as query
/// measures it: NURBS roughly from afar when `coarse`, nothing nearer than
/// `best` searched; terrain left out (BIG) on the lanes in `skip_terrain`.
#[allow(clippy::too_many_arguments)]
pub fn object_dist(ctx: &Ctx, regs: &mut Regs, i: usize, p: V8, dp: V8, best: F8, skip_terrain: M8, coarse: bool) -> F8 {
    let ob = &ctx.scene.objects[i];
    match &ob.shape {
        Shape::Sphere { c, r } => (p - *c).length() - *r,
        Shape::Box { c, rot, half } => {
            let q = p - *c;
            let m = &rot.m;
            let l = V8::new(q.dot3(m[0]), q.dot3(m[1]), q.dot3(m[2])).abs() - *half;
            l.max_s(F8::zero()).length() + l.max_comp().min(F8::zero())
        }
        Shape::Plane { n, d } => p.dot3(*n) - *d,
        Shape::Sdf(prog) => {
            ctx.prepare(regs, i);
            regs.dp = dp;
            eval(prog, p - ob.org, regs, false).0
        }
        Shape::Nurbs { surf, c, rot, s } => {
            let q = p - *c;
            let m = &rot.m;
            let k = 1.0 / s;
            let l = V8::new(q.dot3(m[0]) * k, q.dot3(m[1]) * k, q.dot3(m[2]) * k);
            super::nurbs::dist8(surf, l, best.hmax() * k, M8::all_true(), coarse) * *s
        }
        Shape::Terrain(tr) => {
            let (n, _) = tr.normal8(p.x, p.z);
            F8::select(skip_terrain, F8::splat(BIG), (p.y - tr.height8(p.x, p.z)) * n.y)
        }
    }
}

/// dist_query at points also given as their offsets from the frame's anchor
/// (Ray.od): near the camera finer than the points themselves, which
/// anchored fractals use (game mode's walker, micrometres tall).
pub fn dist_query_at(ctx: &Ctx, regs: &mut Regs, p: V8, dp: V8, radius: f32, skip_terrain: M8, exclude: &[u32]) -> (F8, I8) {
    query(ctx, regs, p, dp, radius, skip_terrain, exclude, false)
}

/// dist_query, or AO's distance (`ao`): that one leaves out what casts no
/// shadow (glass, lamps), and lets NURBS surfaces answer roughly from afar;
/// physics collides with everything solid (all but the water's plane, which
/// floats things instead).
/// `dp`: the points' offset from the frame's anchor (Ray.od).
#[allow(clippy::too_many_arguments)]
fn query(ctx: &Ctx, regs: &mut Regs, p: V8, dp: V8, radius: f32, skip_terrain: M8, exclude: &[u32], ao: bool) -> (F8, I8) {
    let coarse = ao;
    let sc = ctx.scene;
    let mut best = F8::splat(radius);
    let mut who = I8::splat(-1);
    let mut visit = |i: u32, regs: &mut Regs| {
        let ob = &sc.objects[i as usize];
        let skip = if ao { !ob.shadow } else { ob.ghost || sc.materials.get(ob.material as usize).is_some_and(|m| m.tex == crate::scene::tex::WATER) };
        if skip || exclude.contains(&i) {
            return;
        }
        if skip_terrain.all() && matches!(ob.shape, Shape::Terrain(_)) {
            return;
        }
        let t0 = if ctx.profile { ticks() } else { 0 };
        let d = object_dist(ctx, regs, i as usize, p, dp, best, skip_terrain, coarse);
        if ctx.profile {
            regs.charge(i, ticks() - t0, 1);
        }
        let nearer = d.lt(best);
        who = I8::select(nearer, I8::splat(i as i32), who);
        best = best.min(d);
    };
    for &i in &sc.unbounded {
        visit(i, regs);
    }
    let nodes = &sc.bvh.nodes;
    if nodes.is_empty() {
        return (best, who);
    }
    let near = |min: Vec3, max: Vec3| {
        let r = radius;
        let inside = |c: F8, lo: f32, hi: f32| c.ge(F8::splat(lo - r)) & c.le(F8::splat(hi + r));
        (inside(p.x, min.x, max.x) & inside(p.y, min.y, max.y) & inside(p.z, min.z, max.z)).any()
    };
    let mut stack = [0u32; 64];
    let mut sp = 1;
    while sp > 0 {
        sp -= 1;
        let n = &nodes[stack[sp] as usize];
        if !near(n.min, n.max) {
            continue;
        }
        if n.count > 0 {
            for &item in &sc.bvh.items[n.index as usize..(n.index + n.count) as usize] {
                visit(item, regs);
            }
        } else if sp + 2 <= 64 {
            stack[sp] = n.index;
            stack[sp + 1] = n.index + 1;
            sp += 2;
        }
    }
    (best, who)
}

/// The unit normal of a distance program at `p` (tetrahedral gradient).
#[inline(always)]
pub fn sdf_normal(prog: &crate::scene::sdf::Program, regs: &mut Regs, p: V8, dp: V8, eps: F8) -> V8 {
    const K: [[f32; 3]; 4] = [[1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, 1.0]];
    let mut g = V8::zero();
    for k in K {
        let tap = |p: V8| V8::new(F8::splat(k[0]).mul_add(eps, p.x), F8::splat(k[1]).mul_add(eps, p.y), F8::splat(k[2]).mul_add(eps, p.z));
        if regs.has_table() {
            regs.dp = tap(dp);
        }
        let (d, _) = eval(prog, tap(p), regs, false);
        g = V8::new(F8::splat(k[0]).mul_add(d, g.x), F8::splat(k[1]).mul_add(d, g.y), F8::splat(k[2]).mul_add(d, g.z));
    }
    g * g.length_sq().max(F8::splat(1.0e-30)).rsqrt()
}
