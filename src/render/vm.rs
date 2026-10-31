//! The signed-distance interpreter: runs a compiled program (scene/sdf.rs)
//! on eight points at once. Dispatch costs one branch per op for all eight
//! lanes; the fractal ops are whole loops, so the interpretation overhead
//! stays small next to the maths. The Menger op is fraktal256's menger8.

use crate::math::Vec3;
use crate::noise::fbm3;
use crate::scene::nurbs::Nurbs;
use crate::scene::sdf::{MENGER_LEVELS, MENGER_STRIDE, NO_ANCHOR, NREG, Op, Program, SLICE_STRIDE};
use std::sync::Arc;
use crate::simd::{F8, I8, M8, V8};

/// Per-thread scratch: the register file, reused across evaluations, and the
/// per-object cost counters of the work this thread did.
pub struct Regs {
    p: [V8; NREG],
    d: [F8; NREG],
    m: [I8; NREG],
    /// with the tags: the orbit trap of the nearest leaf (fractals colour by
    /// it; other shapes leave 0)
    o: [F8; NREG],
    /// the log of the threshold (world metres) the fractals of the object
    /// being evaluated resolve detail to (render/mod.rs fractal_detail;
    /// infinite: their own iteration counts)
    pub ln_thr: f32,
    /// this sample's factor on the marches' hit threshold (render/mod.rs
    /// thr_jitter): detail at the edge of what the pixels resolve fades in
    /// over the samples instead of at one distance
    pub thr_k: F8,
    /// the anchor table of the object being evaluated this frame (render/mod.rs
    /// anchors; null: none), set with ln_thr by Ctx::prepare before every
    /// evaluation of an object, from a Ctx that outlives it
    table: (*const f32, usize),
    /// the points' offset from the frame's anchor (the camera) in metres:
    /// small near it, so finer than the points themselves
    pub dp: V8,
    prof: Vec<ObjCost>,
    touched: Vec<u32>,
}

/// What an object cost: CPU cycles and 8-lane evaluations (distance
/// programs, height samples or exact intersections).
#[derive(Clone, Copy, Debug, Default)]
pub struct ObjCost {
    pub cycles: u64,
    pub evals: u64,
}

impl Regs {

    pub fn new() -> Box<Regs> {
        Box::new(Regs {
            p: [V8::zero(); NREG],
            d: [F8::zero(); NREG],
            m: [I8::zero(); NREG],
            o: [F8::zero(); NREG],
            ln_thr: f32::INFINITY,
            thr_k: F8::one(),
            table: (std::ptr::null(), 0),
            dp: V8::zero(),
            prof: Vec::new(),
            touched: Vec::new(),
        })
    }

    /// Whether the object being evaluated has an anchor table (only then do
    /// its evaluations read `dp`).
    #[inline(always)]
    pub fn has_table(&self) -> bool {
        !self.table.0.is_null()
    }

    /// The object's anchor table for the evaluations that follow (empty: none).
    ///
    /// # Safety
    /// `t` must outlive every evaluation made before the next `set_table`
    /// (Ctx::prepare sets it from the frame's tables before each object).
    pub unsafe fn set_table(&mut self, t: &[f32]) {
        self.table = if t.is_empty() { (std::ptr::null(), 0) } else { (t.as_ptr(), t.len()) };
    }

    /// The table entry at `at`, `len` floats, when the object has a table.
    #[inline(always)]
    fn entry(&self, at: u16, len: usize) -> Option<&[f32]> {
        if at == NO_ANCHOR || self.table.0.is_null() {
            return None;
        }
        // SAFETY: set_table's slice outlives the evaluations it is set for
        let t = unsafe { std::slice::from_raw_parts(self.table.0, self.table.1) };
        t.get(at as usize..at as usize + len)
    }

    /// The orbit trap (0..1) at the point the last `eval` with tags saw.
    pub fn trap(&self) -> F8 {
        self.o[0]
    }

    /// Adds work to object `obj`'s account.
    #[inline]
    pub fn charge(&mut self, obj: u32, cycles: u64, evals: u64) {
        let i = obj as usize;
        if i >= self.prof.len() {
            self.prof.resize(i + 1, ObjCost::default());
        }
        let c = &mut self.prof[i];
        if c.cycles == 0 && c.evals == 0 {
            self.touched.push(obj);
        }
        c.cycles += cycles;
        c.evals += evals;
    }

    /// Moves this thread's counters into `into` (indexed by object).
    pub fn drain_costs(&mut self, into: &mut [ObjCost]) {
        for &o in &self.touched {
            let c = std::mem::take(&mut self.prof[o as usize]);
            if let Some(t) = into.get_mut(o as usize) {
                t.cycles += c.cycles;
                t.evals += c.evals;
            }
        }
        self.touched.clear();
    }
}

/// The CPU's time-stamp counter (cycles at the nominal clock).
#[inline(always)]
pub fn ticks() -> u64 {
    // SAFETY: rdtsc has no preconditions on x86-64
    unsafe { core::arch::x86_64::_rdtsc() }
}

/// Cycles one op takes on eight points: measured once per kind of op and
/// iteration count (the fastest of a few short runs, so preemption does not
/// count) and cached. (Keyed on every number, a tumbling body's rotation was
/// a new op each physics frame: a benchmark per frame, and a cache that grew
/// for ever.)
pub fn op_cycles(op: &Op) -> f32 {
    use std::collections::HashMap;
    use std::sync::Mutex;
    type Key = (std::mem::Discriminant<Op>, u32);
    static CACHE: Mutex<Option<HashMap<Key, f32>>> = Mutex::new(None);
    let n = match *op {
        Op::Menger { iters, .. }
        | Op::Mandelbulb { iters, .. }
        | Op::Mandelbox { iters, .. }
        | Op::Julia { iters, .. }
        | Op::Sierpinski { iters, .. }
        | Op::Apollonian { iters, .. }
        | Op::Kifs { iters, .. }
        | Op::Kleinian { iters, .. } => iters,
        // (each formula its own kind)
        Op::Formula { f, iters, .. } => (f as u32) << 8 | iters,
        Op::Displace { octaves, .. } => octaves,
        Op::Relief { kind, .. } => kind as u32,
        _ => 0,
    };
    // (a NURBS leaf is timed with no surface to measure: a projection onto
    // a handful of pieces costs some thousands of cycles)
    if matches!(op, Op::Nurbs { .. }) {
        return 3000.0;
    }
    let key: Key = (std::mem::discriminant(op), n);
    if let Some(&c) = CACHE.lock().unwrap().get_or_insert_with(HashMap::new).get(&key) {
        return c;
    }
    let mut r = Regs::new();
    let mut seed = 0x9E37_79B9u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.4 - 1.2
    };
    let mut pts = Vec::new();
    for _ in 0..16 {
        let mut a = [[0.0f32; 8]; 3];
        for c in a.iter_mut() {
            for v in c.iter_mut() {
                *v = rnd();
            }
        }
        pts.push(V8::new(F8::from_array(a[0]), F8::from_array(a[1]), F8::from_array(a[2])));
    }
    for i in 0..NREG {
        r.p[i] = pts[i % 16];
        r.d[i] = pts[(i + 5) % 16].x;
    }
    let ops = std::slice::from_ref(op);
    let mut best = u64::MAX;
    for _ in 0..5 {
        let t0 = ticks();
        for p in &pts {
            std::hint::black_box(eval_ops(ops, &[], *p, &mut r, false));
        }
        best = best.min(ticks() - t0);
    }
    let c = best as f32 / pts.len() as f32;
    CACHE.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, c);
    c
}

#[inline(always)]
fn v3(a: [f32; 3]) -> Vec3 {
    Vec3::from_array(a)
}

#[inline(always)]
fn len2(x: F8, y: F8) -> F8 {
    y.mul_add(y, x * x).sqrt()
}

/// Distance at `p` from program `prog`; with `mats`, the material of the
/// nearest leaf ends in the returned I8 (else it is garbage).
#[inline(always)]
pub fn eval(prog: &Program, p: V8, r: &mut Regs, mats: bool) -> (F8, I8) {
    eval_ops(&prog.ops, &prog.surfaces, p, r, mats)
}

/// eval of the ops alone (`surfaces`: their Op::Nurbs leaves' surfaces).
#[inline]
fn eval_ops(ops: &[Op], surfaces: &[Arc<Nurbs>], p: V8, r: &mut Regs, mats: bool) -> (F8, I8) {
    r.p[0] = p;
    for op in ops {
        match *op {
            Op::Affine { dst, src, m, t } => {
                let q = r.p[src as usize] - v3(t);
                let row = |a: f32, b: f32, c: f32| q.z.mul_add(F8::splat(c), q.y.mul_add(F8::splat(b), q.x * a));
                r.p[dst as usize] = V8::new(row(m[0], m[1], m[2]), row(m[3], m[4], m[5]), row(m[6], m[7], m[8]));
            }
            Op::Repeat { dst, src, period, limit } => {
                let q = r.p[src as usize];
                let rep = |x: F8, per: f32, lim: f32| {
                    if per == 0.0 {
                        return x;
                    }
                    let mut c = (x * (1.0 / per)).round();
                    if lim >= 0.0 {
                        c = c.clamp(-lim, lim);
                    }
                    c.neg_mul_add(F8::splat(per), x)
                };
                r.p[dst as usize] =
                    V8::new(rep(q.x, period[0], limit[0]), rep(q.y, period[1], limit[1]), rep(q.z, period[2], limit[2]));
            }
            Op::Mirror { dst, src, axes } => {
                let q = r.p[src as usize];
                let f = |x: F8, bit: u8| if axes & bit != 0 { x.abs() } else { x };
                r.p[dst as usize] = V8::new(f(q.x, 1), f(q.y, 2), f(q.z, 4));
            }
            Op::Twist { dst, src, k } => {
                let q = r.p[src as usize];
                let a = q.y * k;
                let (s, c) = (a.sin(), a.cos());
                r.p[dst as usize] = V8::new(c * q.x - s * q.z, q.y, s * q.x + c * q.z);
            }
            Op::Bend { dst, src, k } => {
                let q = r.p[src as usize];
                let a = q.x * k;
                let (s, c) = (a.sin(), a.cos());
                r.p[dst as usize] = V8::new(c * q.x - s * q.y, s * q.x + c * q.y, q.z);
            }
            Op::Sphere { d, p, c, r: rad, mat } => {
                r.d[d as usize] = (r.p[p as usize] - v3(c)).length() - rad;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Box { d, p, b, r: rad, mat } => {
                let q = r.p[p as usize].abs() - v3(b);
                let outside = q.max_s(F8::zero()).length();
                let inside = q.max_comp().min(F8::zero());
                r.d[d as usize] = outside + inside - rad;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Plane { d, p, n, h, mat } => {
                r.d[d as usize] = r.p[p as usize].dot3(v3(n)) - h;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Torus { d, p, big, small, mat } => {
                let q = r.p[p as usize];
                let qx = len2(q.x, q.z) - big;
                r.d[d as usize] = len2(qx, q.y) - small;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Cylinder { d, p, r: rad, h, mat } => {
                let q = r.p[p as usize];
                let dx = len2(q.x, q.z) - rad;
                let dy = q.y.abs() - h;
                let z = F8::zero();
                r.d[d as usize] = dx.max(dy).min(z) + len2(dx.max(z), dy.max(z));
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Cone { d, p, r1, r2, h, mat } => {
                // iq's capped cone: radius r1 at y = -h, r2 at y = +h
                let q = r.p[p as usize];
                let (qx, qy) = (len2(q.x, q.z), q.y);
                let (k2x, k2y) = (r2 - r1, 2.0 * h);
                let rr = F8::select(qy.lt(F8::zero()), F8::splat(r1), F8::splat(r2));
                let cax = qx - qx.min(rr);
                let cay = qy.abs() - h;
                let tt = (((F8::splat(r2) - qx) * k2x + (F8::splat(h) - qy) * k2y) * (1.0 / (k2x * k2x + k2y * k2y))).saturate();
                let cbx = (qx - r2) + tt * k2x;
                let cby = (qy - h) + tt * k2y;
                let s = F8::select(cbx.lt(F8::zero()) & cay.lt(F8::zero()), F8::splat(-1.0), F8::one());
                let dd = cay.mul_add(cay, cax * cax).min(cby.mul_add(cby, cbx * cbx));
                r.d[d as usize] = s * dd.sqrt();
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Capsule { d, p, a, b, r: rad, mat } => {
                let (a, b) = (v3(a), v3(b));
                let pa = r.p[p as usize] - a;
                let ba = b - a;
                let h = (pa.dot3(ba) * (1.0 / ba.length_sq().max(1e-12))).saturate();
                let e = V8::new(h.neg_mul_add(F8::splat(ba.x), pa.x), h.neg_mul_add(F8::splat(ba.y), pa.y), h.neg_mul_add(F8::splat(ba.z), pa.z));
                r.d[d as usize] = e.length() - rad;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Ellipsoid { d, p, r: rad, mat } => {
                let q = r.p[p as usize];
                let s = V8::new(q.x * (1.0 / rad[0]), q.y * (1.0 / rad[1]), q.z * (1.0 / rad[2]));
                let s2 = V8::new(s.x * (1.0 / rad[0]), s.y * (1.0 / rad[1]), s.z * (1.0 / rad[2]));
                let k0 = s.length();
                let k1 = s2.length().max(F8::splat(1e-12));
                r.d[d as usize] = k0 * (k0 - 1.0) / k1;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Nurbs { d, p, surf, mat } => {
                r.d[d as usize] = super::nurbs::sdf8(&surfaces[surf as usize], r.p[p as usize]);
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Octahedron { d, p, s, mat } => {
                let q = r.p[p as usize].abs();
                r.d[d as usize] = (q.x + q.y + q.z - s) * 0.577_350_26;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Gyroid { d, p, f, t, mat } => {
                let q = r.p[p as usize] * f;
                let (sx, cx) = (q.x.sin(), q.x.cos());
                let (sy, cy) = (q.y.sin(), q.y.cos());
                let (sz, cz) = (q.z.sin(), q.z.cos());
                let g = sz.mul_add(cx, sy.mul_add(cz, sx * cy));
                // |grad g| <= sqrt(6): a safe distance bound
                r.d[d as usize] = (g.abs() * (1.0 / f) - t * 0.5) * 0.41;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = F8::zero();
                }
            }
            Op::Menger { d, p, iters, lod, mat, at } => {
                let n = lod.iters(iters, r.ln_thr);
                let (dist, trap) = match r.entry(at, MENGER_STRIDE) {
                    Some(t) => menger_anchored(r.p[p as usize], r.dp, t, n, iters),
                    None => menger(r.p[p as usize], n, iters),
                };
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Mandelbulb { d, p, power, iters, lod, mat } => {
                let (dist, trap) = mandelbulb(r.p[p as usize], power, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Mandelbox { d, p, scale, min_r, iters, lod, mat } => {
                let (dist, trap) = mandelbox(r.p[p as usize], scale, min_r, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Julia { d, p, c, iters, lod, mat } => {
                let (dist, trap) = julia(r.p[p as usize], c, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Sierpinski { d, p, iters, lod, mat } => {
                let (dist, trap) = sierpinski(r.p[p as usize], lod.iters(iters, r.ln_thr), iters);
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Apollonian { d, p, fold, iters, lod, mat } => {
                let (dist, trap) = apollonian(r.p[p as usize], fold, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Kifs { d, p, s, off, rot, iters, lod, mat } => {
                let (dist, trap) = kifs(r.p[p as usize], s, off, rot, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Kleinian { d, p, cbox, size, iters, lod, mat } => {
                let (dist, trap) = kleinian(r.p[p as usize], cbox, size, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Formula { d, p, f, k, iters, lod, mat } => {
                let (dist, trap) = (crate::formulas::get(f).cpu)(r.p[p as usize], &k, lod.iters(iters, r.ln_thr));
                r.d[d as usize] = dist;
                if mats {
                    r.m[d as usize] = I8::splat(mat as i32);
                    r.o[d as usize] = trap;
                }
            }
            Op::Union { a, b } => {
                let (da, db) = (r.d[a as usize], r.d[b as usize]);
                if mats {
                    r.m[a as usize] = I8::select(db.lt(da), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(db.lt(da), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = da.min(db);
            }
            Op::Intersect { a, b } => {
                let (da, db) = (r.d[a as usize], r.d[b as usize]);
                if mats {
                    r.m[a as usize] = I8::select(db.gt(da), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(db.gt(da), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = da.max(db);
            }
            Op::Subtract { a, b } => {
                let (da, nb) = (r.d[a as usize], -r.d[b as usize]);
                if mats {
                    r.m[a as usize] = I8::select(nb.gt(da), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(nb.gt(da), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = da.max(nb);
            }
            Op::SmoothUnion { a, b, k } => {
                let (da, db) = (r.d[a as usize], r.d[b as usize]);
                let h = ((db - da) * (0.5 / k) + 0.5).saturate();
                if mats {
                    r.m[a as usize] = I8::select(h.lt(F8::splat(0.5)), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(h.lt(F8::splat(0.5)), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = F8::lerp(db, da, h) - h * (F8::one() - h) * k;
            }
            Op::SmoothIntersect { a, b, k } => {
                let (da, db) = (r.d[a as usize], r.d[b as usize]);
                let h = ((db - da) * (-0.5 / k) + 0.5).saturate();
                if mats {
                    r.m[a as usize] = I8::select(h.lt(F8::splat(0.5)), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(h.lt(F8::splat(0.5)), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = F8::lerp(db, da, h) + h * (F8::one() - h) * k;
            }
            Op::SmoothSubtract { a, b, k } => {
                let (da, db) = (r.d[a as usize], r.d[b as usize]);
                let h = ((db + da) * (-0.5 / k) + 0.5).saturate();
                if mats {
                    r.m[a as usize] = I8::select(h.gt(F8::splat(0.5)), r.m[b as usize], r.m[a as usize]);
                    r.o[a as usize] = F8::select(h.gt(F8::splat(0.5)), r.o[b as usize], r.o[a as usize]);
                }
                r.d[a as usize] = F8::lerp(da, -db, h) + h * (F8::one() - h) * k;
            }
            Op::Mul { d, s } => r.d[d as usize] *= s,
            Op::Round { d, r: rad } => r.d[d as usize] -= rad,
            Op::Shell { d, t } => r.d[d as usize] = r.d[d as usize].abs() - t,
            Op::Slice { d, p, n, off, half, at } => {
                // anchored: the anchor's distance plus the offset's (t[0..3]:
                // the normal in the object's frame)
                let s = match r.entry(at, SLICE_STRIDE) {
                    Some(t) => r.dp.dot3(Vec3::new(t[0], t[1], t[2])) + t[3],
                    None => r.p[p as usize].dot3(v3(n)) - off,
                };
                let s = if half > 0.0 { s.abs() - half } else { s };
                r.d[d as usize] = r.d[d as usize].max(s);
            }
            Op::Relief { d, p, kind, freq, depth, warp, sharp } => {
                let q = r.p[p as usize];
                let n = q * q.length_sq().max(F8::splat(1e-12)).rsqrt();
                let (t, _) = super::pattern::eval(kind, q * freq, n, F8::splat(warp), F8::splat(sharp));
                r.d[d as usize] = t.neg_mul_add(F8::splat(depth), r.d[d as usize]);
            }
            Op::Displace { d, p, amp, freq, octaves } => {
                let n = fbm3(r.p[p as usize] * freq, octaves, I8::splat(0x2545_F491));
                r.d[d as usize] = n.mul_add(F8::splat(amp), r.d[d as usize]);
            }
        }
    }
    (r.d[0], r.m[0])
}

/// Menger iterations: (s / 2, 1 / (3 s)) per level.
const MENGER: [(f32, f32); MENGER_LEVELS] = [
    (0.5, 1.0 / 3.0),
    (1.5, 1.0 / 9.0),
    (4.5, 1.0 / 27.0),
    (13.5, 1.0 / 81.0),
    (40.5, 1.0 / 243.0),
    (121.5, 1.0 / 729.0),
    (364.5, 1.0 / 2187.0),
    (1093.5, 1.0 / 6561.0),
    (3280.5, 1.0 / 19683.0),
    (9841.5, 1.0 / 59049.0),
    (29524.5, 1.0 / 177147.0),
    (88573.5, 1.0 / 531441.0),
    (265720.5, 1.0 / 1594323.0),
    (797161.5, 1.0 / 4782969.0),
    (2391484.5, 1.0 / 14348907.0),
    (7174453.5, 1.0 / 43046721.0),
    (21523360.5, 1.0 / 129140163.0),
    (64570081.5, 1.0 / 387420489.0),
    (193710244.5, 1.0 / 1162261467.0),
    (581130733.5, 1.0 / 3486784401.0),
];

/// Distance to the Menger sponge filling [-1, 1]^3 (fraktal256's menger8),
/// and its orbit trap: how fine the hole that shapes the point (0 the outer
/// box, 1 the finest level).
#[inline(always)]
fn menger(p: V8, iters: u32, norm: u32) -> (F8, F8) {
    let q = V8::new(p.x.abs() - 1.0, p.y.abs() - 1.0, p.z.abs() - 1.0);
    let inside = q.max_comp().min(F8::zero());
    let mut d = q.max_s(F8::zero()).length() + inside;
    let mut level = F8::zero();
    // (12 at most: past that f32 a metre from the centre is coarser than the
    // holes; menger_anchored goes further)
    for (i, &(half_s, inv3s)) in MENGER[..(iters as usize).min(12)].iter().enumerate() {
        // |1 - 3 |mod(c s, 2) - 1||
        let fold = |c: F8| {
            let a = c * half_s;
            let a = a - a.floor();
            (F8::one() - (a + a - 1.0).abs() * 3.0).abs()
        };
        let (a, b, c) = (fold(p.x), fold(p.y), fold(p.z));
        let cross = a.max(b).min(b.max(c)).min(c.max(a));
        let hole = (cross - 1.0) * inv3s;
        level = F8::select(hole.gt(d), F8::splat((i + 1) as f32), level);
        d = d.max(hole);
    }
    // (the trap counts the levels as set: deeper ones close up go past 1)
    (d, level * (1.0 / norm.max(1) as f32))
}

/// `menger` from the frame's anchor: `t` is the sponge's anchor table entry
/// (render/mod.rs anchors) and `dp` the points' offset from the anchor in the
/// object's frame. A level's fold near a wall is a small number, so the
/// anchor's fold (from f64) plus its slope times the offset is exact while a
/// point stays on the anchor's straight stretch of the fold; past it the
/// fold comes from the anchor's residue plus the offset, as fine as plain
/// f32 there (far from the anchor, where the pixels are coarse anyway).
/// Everything is the fold minus 1, the hole's edge, so it stays small.
#[inline(always)]
fn menger_anchored(p: V8, dp: V8, t: &[f32], iters: u32, norm: u32) -> (F8, F8) {
    let dc = [0, 1, 2].map(|r| dp.dot3(Vec3::new(t[r * 3], t[r * 3 + 1], t[r * 3 + 2])));
    let pc = [p.x, p.y, p.z];
    // the box: |x| - 1 from the anchor's, on the anchor's side of the centre
    let q = [0, 1, 2].map(|i| {
        let (e, sg) = (F8::splat(t[9 + i]), F8::splat(t[12 + i]));
        let same = dc[i].mul_add(sg, e + 1.0).gt(F8::zero());
        F8::select(same, dc[i].mul_add(sg, e), pc[i].abs() - 1.0)
    });
    let inside = q[0].max(q[1]).max(q[2]).min(F8::zero());
    let mut d = V8::new(q[0], q[1], q[2]).max_s(F8::zero()).length() + inside;
    let mut level = F8::zero();
    for (k, &(half_s, inv3s)) in MENGER[..(iters as usize).min(MENGER_LEVELS)].iter().enumerate() {
        let g = [0, 1, 2].map(|i| {
            let e = &t[15 + (k * 3 + i) * 5..][..5];
            let (c, on) = (dc[i], dc[i].gt(F8::splat(e[2])) & dc[i].lt(F8::splat(e[3])));
            let near = c.mul_add(F8::splat(e[1]), F8::splat(e[0]));
            if on.all() {
                return near;
            }
            let x = c.mul_add(F8::splat(half_s), F8::splat(e[4]));
            let a = x - x.floor();
            F8::select(on, near, (F8::one() - (a + a - 1.0).abs() * 3.0).abs() - 1.0)
        });
        let cross = g[0].max(g[1]).min(g[1].max(g[2])).min(g[2].max(g[0]));
        let hole = cross * inv3s;
        level = F8::select(hole.gt(d), F8::splat((k + 1) as f32), level);
        d = d.max(hole);
    }
    (d, level * (1.0 / norm.max(1) as f32))
}

/// The Mandelbulb's distance estimate, any power (trigonometric form), and
/// its orbit trap: the nearest the orbit comes to the centre.
#[inline(always)]
fn mandelbulb(p: V8, power: f32, iters: u32) -> (F8, F8) {
    let bail = F8::splat(2.0);
    let n = F8::splat(power);
    let mut z = p;
    let mut dr = F8::one();
    let mut r = z.length();
    // from the first iteration on: the starting radius is about the same all
    // over the surface
    let mut trap = F8::splat(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rs = r.max(F8::splat(1e-6));
        let theta = len2(z.x, z.z).atan2(z.y) * power;
        let phi = z.z.atan2(z.x) * power;
        let rp1 = rs.powf(n - 1.0);
        let ndr = (rp1 * dr * power) + 1.0;
        let zr = rp1 * rs;
        let st = theta.sin();
        let nz = V8::new(zr * st * phi.cos(), zr * theta.cos(), zr * st * phi.sin()) + p;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    let r = r.max(F8::splat(1e-6));
    (r.ln() * r * 0.5 / dr, (trap * 0.5).saturate())
}

/// The Mandelbox's distance estimate (box fold, sphere fold, scale), and its
/// orbit trap: the nearest the folded orbit comes to the centre.
#[inline(always)]
fn mandelbox(p: V8, scale: f32, min_r: f32, iters: u32) -> (F8, F8) {
    let min2 = min_r * min_r;
    let mut z = p;
    let mut dr = F8::one();
    let mut trap = F8::splat(1e9);
    for _ in 0..iters {
        let fold = |c: F8| c.clamp(-1.0, 1.0) * 2.0 - c;
        z = V8::new(fold(z.x), fold(z.y), fold(z.z));
        let r2 = z.length_sq();
        trap = trap.min(r2);
        // r2 < min2: scale by 1 / min2; min2 <= r2 < 1: by 1 / r2
        let k = F8::select(r2.lt(F8::splat(min2)), F8::splat(1.0 / min2), F8::one() / r2.max(F8::splat(1e-12)));
        let k = F8::select(r2.lt(F8::one()), k, F8::one());
        z = z * k;
        dr *= k;
        z = z * scale + p;
        dr = dr.mul_add(F8::splat(scale.abs()), F8::one());
    }
    (z.length() / dr.abs(), trap.sqrt().saturate())
}

/// The quaternion Julia set's distance estimate (iq's form), and its orbit
/// trap: the nearest the orbit comes to the centre.
#[inline(always)]
fn julia(p: V8, c: [f32; 4], iters: u32) -> (F8, F8) {
    let (mut a, mut b, mut cc, mut d) = (p.x, p.y, p.z, F8::zero());
    let mut md2 = F8::one();
    let mut mz2 = d.mul_add(d, cc.mul_add(cc, b.mul_add(b, a * a)));
    let mut trap = mz2;
    let mut act = mz2.lt(F8::splat(16.0));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let nmd2 = md2 * mz2 * 4.0;
        // z = z^2 + c
        let na = a * a - b * b - cc * cc - d * d + c[0];
        let nb = (a * b).mul_add(F8::splat(2.0), F8::splat(c[1]));
        let nc = (a * cc).mul_add(F8::splat(2.0), F8::splat(c[2]));
        let nd = (a * d).mul_add(F8::splat(2.0), F8::splat(c[3]));
        a = F8::select(act, na, a);
        b = F8::select(act, nb, b);
        cc = F8::select(act, nc, cc);
        d = F8::select(act, nd, d);
        md2 = F8::select(act, nmd2, md2);
        mz2 = d.mul_add(d, cc.mul_add(cc, b.mul_add(b, a * a)));
        trap = F8::select(act, trap.min(mz2), trap);
        act &= mz2.lt(F8::splat(16.0));
    }
    let mz2 = mz2.max(F8::splat(1e-12));
    ((mz2 / md2).sqrt() * mz2.ln() * 0.25, trap.sqrt().saturate())
}

/// The Sierpinski tetrahedron by folding: three reflections, scale 2 about
/// the vertex (1, 1, 1), then the distance to the base tetrahedron; and its
/// orbit trap: the share of the folds that reflected the point.
#[inline(always)]
fn sierpinski(p: V8, iters: u32, norm: u32) -> (F8, F8) {
    let mut z = p;
    let zero = F8::zero();
    let mut folds = F8::zero();
    for _ in 0..iters {
        let m = (z.x + z.y).lt(zero);
        folds += F8::one().and(m);
        let (x, y) = (F8::select(m, -z.y, z.x), F8::select(m, -z.x, z.y));
        z = V8::new(x, y, z.z);
        let m = (z.x + z.z).lt(zero);
        folds += F8::one().and(m);
        let (x, zz) = (F8::select(m, -z.z, z.x), F8::select(m, -z.x, z.z));
        z = V8::new(x, z.y, zz);
        let m = (z.y + z.z).lt(zero);
        folds += F8::one().and(m);
        let (y, zz) = (F8::select(m, -z.z, z.y), F8::select(m, -z.y, z.z));
        z = V8::new(z.x, y, zz);
        z = V8::new(z.x * 2.0 - 1.0, z.y * 2.0 - 1.0, z.z * 2.0 - 1.0);
    }
    // the regular tetrahedron with vertices (1,1,1), (-1,-1,1), (-1,1,-1), (1,-1,-1)
    let t = ((z.x + z.y).abs() - z.z).max((z.x - z.y).abs() + z.z) - 1.0;
    (t * 0.577_350_26 * (0.5f32).powi(iters as i32), folds * (1.0 / (3 * norm.max(1)) as f32))
}

/// iq's Apollonian packing: fold into the [-1, 1] cell, invert in the unit
/// sphere (`fold` above 1 swells the spheres), repeat; the distance to a
/// plane through the result, scaled back. Orbit trap: the nearest the folded
/// point comes to the centre.
#[inline(always)]
fn apollonian(p: V8, fold: f32, iters: u32) -> (F8, F8) {
    let mut z = p;
    let mut scale = F8::one();
    let mut orb = F8::splat(1e9);
    let wrap = |c: F8| {
        let h = c.mul_add(F8::splat(0.5), F8::splat(0.5));
        (h - h.floor()).mul_add(F8::splat(2.0), F8::splat(-1.0))
    };
    for _ in 0..iters {
        z = V8::new(wrap(z.x), wrap(z.y), wrap(z.z));
        let r2 = z.length_sq().max(F8::splat(1e-12));
        orb = orb.min(r2);
        let k = F8::splat(fold) / r2;
        z = z * k;
        scale *= k;
    }
    (z.y.abs() * 0.25 / scale, (orb * 6.0).saturate())
}

/// A kaleidoscopic IFS (Knighty's Menger form): fold into the octant and
/// sort the axes, turn by `rot`, scale by `s` about `off`; the distance to
/// the unit box, scaled back. Orbit trap: the nearest the orbit comes to the
/// centre, in the object's units.
#[inline(always)]
fn kifs(p: V8, s: f32, off: [f32; 3], rot: [f32; 9], iters: u32) -> (F8, F8) {
    let (mut x, mut y, mut z) = (p.x, p.y, p.z);
    let mut scale = 1.0f32;
    let mut orb = F8::splat(1e9);
    let k = s - 1.0;
    for _ in 0..iters {
        (x, y, z) = (x.abs(), y.abs(), z.abs());
        // largest first: x >= y >= z
        let m = x.lt(y);
        (x, y) = (F8::select(m, y, x), F8::select(m, x, y));
        let m = x.lt(z);
        (x, z) = (F8::select(m, z, x), F8::select(m, x, z));
        let m = y.lt(z);
        (y, z) = (F8::select(m, z, y), F8::select(m, y, z));
        let (rx, ry, rz) = (
            z.mul_add(F8::splat(rot[2]), y.mul_add(F8::splat(rot[1]), x * rot[0])),
            z.mul_add(F8::splat(rot[5]), y.mul_add(F8::splat(rot[4]), x * rot[3])),
            z.mul_add(F8::splat(rot[8]), y.mul_add(F8::splat(rot[7]), x * rot[6])),
        );
        x = rx * s - off[0] * k;
        y = ry * s - off[1] * k;
        z = rz * s - off[2] * k;
        z = F8::select(z.lt(F8::splat(-0.5 * off[2] * k)), z + off[2] * k, z);
        scale *= s;
        orb = orb.min(z.mul_add(z, y.mul_add(y, x * x)) * (1.0 / (scale * scale)));
    }
    let q = V8::new(x.abs() - 1.0, y.abs() - 1.0, z.abs() - 1.0);
    let boxd = q.max_s(F8::zero()).length() + q.max_comp().min(F8::zero());
    (boxd * (1.0 / scale), (orb.sqrt() * 1.5).saturate())
}

/// Knighty's pseudo-Kleinian: box fold into `cbox`, invert inside radius²
/// `size`, repeat; the distance to the ring-shaped set (the shadertoy form's
/// 0.92784: its rings are lacier than the original's thin 0.01), scaled back.
/// Orbit trap: the nearest the folded point comes to the centre.
#[inline(always)]
fn kleinian(p: V8, cbox: [f32; 3], size: f32, iters: u32) -> (F8, F8) {
    let mut z = p;
    let mut total = F8::one();
    let mut orb = F8::splat(1e9);
    for _ in 0..iters {
        let fold = |c: F8, b: f32| c.clamp(-b, b) * 2.0 - c;
        z = V8::new(fold(z.x, cbox[0]), fold(z.y, cbox[1]), fold(z.z, cbox[2]));
        let r2 = z.length_sq().max(F8::splat(1e-12));
        orb = orb.min(r2);
        let k = (F8::splat(size) / r2).max(F8::one());
        z = z * k;
        total *= k;
    }
    let rxy = len2(z.x, z.y);
    let d = (rxy - 0.927_84).max((rxy * z.z).abs() / z.length().max(F8::splat(1e-12)));
    (d / total, orb.sqrt().saturate())
}
