//! Signed-distance programs: a shape tree compiled into a flat list of
//! register ops, evaluated eight points at a time by render/vm.rs (and,
//! later, translated to WGSL for the GPU backend). Point registers hold
//! (transformed) sample positions, distance registers hold distances and the
//! material of the nearest leaf.
//!
//! The compiler also works out each tree's bounding box (to limit marching to
//! the box) and how much its distance estimate may overshoot (twist, bend and
//! displacement are not true distances), as a step factor below 1.

use super::json::{NodeDef, ShapeDef};
use super::nurbs::Nurbs;
use crate::math::{Aabb, Mat3, Vec3};
use std::sync::Arc;

/// Registers per kind. Trees deeper than this fail to compile.
pub const NREG: usize = 24;

/// No anchor table entry (Op::Menger / Op::Slice `at`).
pub const NO_ANCHOR: u16 = u16::MAX;
/// The most levels a Menger sponge reaches anchored (as set, 12: f32's limit
/// a metre from its centre).
pub const MENGER_LEVELS: usize = 20;
/// Floats per anchor table entry: a Menger sponge's (the map to its frame,
/// its box, and per level and axis: the fold at the anchor, its slope, the
/// stretch it holds over, and the anchor's residue), a slice's.
pub const MENGER_STRIDE: usize = 9 + 6 + 5 * 3 * MENGER_LEVELS;
pub const SLICE_STRIDE: usize = 4;

/// Where a point register's points are, as an affine map of the object's
/// (relative to Object.org): `a * p + b`, in f64.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf {
    pub a: [[f64; 3]; 3],
    pub b: [f64; 3],
}

impl Xf {
    pub const IDENTITY: Xf = Xf { a: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], b: [0.0; 3] };

    /// Followed by `q = m * (p - t)` (Op::Affine).
    fn then(&self, m: [f32; 9], t: [f32; 3]) -> Xf {
        let mut out = Xf { a: [[0.0; 3]; 3], b: [0.0; 3] };
        for r in 0..3 {
            for c in 0..3 {
                out.a[r][c] = (0..3).map(|k| m[r * 3 + k] as f64 * self.a[k][c]).sum();
            }
            out.b[r] = (0..3).map(|k| m[r * 3 + k] as f64 * (self.b[k] - t[k] as f64)).sum();
        }
        out
    }

    pub fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        std::array::from_fn(|r| (0..3).map(|c| self.a[r][c] * p[c]).sum::<f64>() + self.b[r])
    }
}

/// What a frame's anchor table holds for an op (render/mod.rs anchors).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnchorKind {
    Menger,
    /// the slice's normal and offset in its register's frame
    Slice { n: [f32; 3], off: f32 },
}

/// An op evaluated from the anchor (Program::anchors): its register's map
/// and where its entry starts in the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub xf: Xf,
    pub kind: AnchorKind,
    pub at: u16,
}

/// How a fractal resolves finer detail close up: `k - per * ln(thr)`
/// iterations make detail as small as the threshold `thr` (world metres) at
/// the leaf (its scale in `k`), each iteration `exp(1 / per)` times finer;
/// never fewer than its own count, never more than `cap`. The renderer picks
/// `thr` once a frame per object, from the camera's distance to it (render/mod.rs
/// fractal_detail): the same shape for every ray, normal, shadow and AO sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lod {
    pub k: f32,
    pub per: f32,
    pub cap: u32,
}

impl Lod {
    /// Only the iterations as set: for fractals whose further iterations
    /// reshape what is there rather than add smaller copies to it (escape
    /// time and inversion fractals turn to smears and dust close up).
    pub const NONE: Lod = Lod { k: 0.0, per: 0.0, cap: 0 };

    /// The iterations for threshold `ln_thr` (its log; infinite: as set).
    #[inline(always)]
    pub fn iters(self, iters: u32, ln_thr: f32) -> u32 {
        if !ln_thr.is_finite() {
            return iters;
        }
        let n = (self.k - self.per * ln_thr).ceil().clamp(0.0, self.cap as f32) as u32;
        iters.max(n)
    }
}

impl Op {
    /// A fractal's level of detail.
    pub fn lod(&self) -> Option<Lod> {
        match *self {
            Op::Menger { lod, .. }
            | Op::Mandelbulb { lod, .. }
            | Op::Mandelbox { lod, .. }
            | Op::Julia { lod, .. }
            | Op::Sierpinski { lod, .. }
            | Op::Apollonian { lod, .. }
            | Op::Kifs { lod, .. }
            | Op::Kleinian { lod, .. }
            | Op::Formula { lod, .. } => Some(lod),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    // ---- point ops: p[dst] = f(p[src])
    /// q = m * (p - t)
    Affine { dst: u8, src: u8, m: [f32; 9], t: [f32; 3] },
    /// p - period * round(p / period), the cell index clamped to +-limit (limit < 0: infinite);
    /// period 0 leaves the axis alone
    Repeat { dst: u8, src: u8, period: [f32; 3], limit: [f32; 3] },
    /// |p| on the axes in the mask (bit 0 = x)
    Mirror { dst: u8, src: u8, axes: u8 },
    Twist { dst: u8, src: u8, k: f32 },
    Bend { dst: u8, src: u8, k: f32 },
    // ---- leaves: d[d] = sdf(p[p]); `mat` is the leaf's tag: node id << 16 | material
    Sphere { d: u8, p: u8, c: [f32; 3], r: f32, mat: u32 },
    Box { d: u8, p: u8, b: [f32; 3], r: f32, mat: u32 },
    Plane { d: u8, p: u8, n: [f32; 3], h: f32, mat: u32 },
    Torus { d: u8, p: u8, big: f32, small: f32, mat: u32 },
    Cylinder { d: u8, p: u8, r: f32, h: f32, mat: u32 },
    Cone { d: u8, p: u8, r1: f32, r2: f32, h: f32, mat: u32 },
    Capsule { d: u8, p: u8, a: [f32; 3], b: [f32; 3], r: f32, mat: u32 },
    Ellipsoid { d: u8, p: u8, r: [f32; 3], mat: u32 },
    /// a NURBS surface inside a boolean or a modifier: its signed distance
    /// (Program.surfaces[surf]; render/nurbs.rs sdf8)
    Nurbs { d: u8, p: u8, surf: u8, mat: u32 },
    Octahedron { d: u8, p: u8, s: f32, mat: u32 },
    Gyroid { d: u8, p: u8, f: f32, t: f32, mat: u32 },
    // (fractals: `iters` as set, more close up as `lod` says: see Lod)
    /// `at`: its anchor table entry (NO_ANCHOR: none)
    Menger { d: u8, p: u8, iters: u32, lod: Lod, mat: u32, at: u16 },
    Mandelbulb { d: u8, p: u8, power: f32, iters: u32, lod: Lod, mat: u32 },
    Mandelbox { d: u8, p: u8, scale: f32, min_r: f32, iters: u32, lod: Lod, mat: u32 },
    Julia { d: u8, p: u8, c: [f32; 4], iters: u32, lod: Lod, mat: u32 },
    Sierpinski { d: u8, p: u8, iters: u32, lod: Lod, mat: u32 },
    Apollonian { d: u8, p: u8, fold: f32, iters: u32, lod: Lod, mat: u32 },
    /// `rot` row-major
    Kifs { d: u8, p: u8, s: f32, off: [f32; 3], rot: [f32; 9], iters: u32, lod: Lod, mat: u32 },
    Kleinian { d: u8, p: u8, cbox: [f32; 3], size: f32, iters: u32, lod: Lod, mat: u32 },
    /// one of the formula catalog's (formulas/mod.rs): `f` its id, `k` its
    /// parameters packed
    Formula { d: u8, p: u8, f: u16, k: [f32; crate::formulas::K], iters: u32, lod: Lod, mat: u32 },
    // ---- d[a] = op(d[a], d[b])
    Union { a: u8, b: u8 },
    Intersect { a: u8, b: u8 },
    Subtract { a: u8, b: u8 },
    SmoothUnion { a: u8, b: u8, k: f32 },
    SmoothIntersect { a: u8, b: u8, k: f32 },
    SmoothSubtract { a: u8, b: u8, k: f32 },
    // ---- distance modifiers
    Mul { d: u8, s: f32 },
    Round { d: u8, r: f32 },
    Shell { d: u8, t: f32 },
    Displace { d: u8, p: u8, amp: f32, freq: f32, octaves: u32 },
    /// d -= depth * pattern(kind, p * freq): the pattern's projection axis
    /// follows the point's direction from the node's origin
    Relief { d: u8, p: u8, kind: i32, freq: f32, depth: f32, warp: f32, sharp: f32 },
    /// d = max(d, n.p - off), or with `half` > 0 the slab |n.p - off| <= half;
    /// `at` as Menger's
    Slice { d: u8, p: u8, n: [f32; 3], off: f32, half: f32, at: u16 },
}

/// A compiled tree: evaluate `ops` with the sample point in p[0]; the
/// distance ends in d[0]. Node ids number the tree's nodes in pre-order;
/// `paths[id]` is the child-index path to node `id` and `ranges[id]` the ops
/// its subtree compiled to.
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub ops: Vec<Op>,
    pub leaves: usize,
    pub paths: Vec<Vec<usize>>,
    pub ranges: Vec<(u32, u32)>,
    /// every leaf's tag and world bounding box (its own instance; repeats not included)
    pub leaf_bounds: Vec<(u32, Aabb)>,
    /// it has fractals that iterate further close up (see Lod)
    pub lod: bool,
    /// the ops that can be evaluated from an anchor near the camera, where
    /// f32 is finer (render/mod.rs anchors), and the floats of their table
    pub anchors: Vec<Anchor>,
    pub table_len: usize,
    /// every leaf and slice is anchored: close to the anchor the whole
    /// distance is as fine as the offset from it (the marches' floor follows)
    pub anchored: bool,
    /// the NURBS surfaces its Op::Nurbs leaves measure (by index)
    pub surfaces: Vec<Arc<Nurbs>>,
}

/// A leaf's tag: which node it came from and its material.
pub fn tag(node: usize, mat: u16) -> u32 {
    ((node.min(0x7fff) as u32) << 16) | mat as u32
}

/// What a material reference resolves to.
pub trait MaterialSource {
    fn resolve(&mut self, m: &super::json::MatRef) -> Result<u16, String>;
}

pub struct Compiler<'a, M: MaterialSource> {
    pub ops: Vec<Op>,
    np: usize,
    nd: usize,
    pub step: f32,
    pub leaves: usize,
    mats: &'a mut M,
    /// the path to the node being compiled, its id, and every node's path and op range
    path: Vec<usize>,
    cur: usize,
    paths: Vec<Vec<usize>>,
    ranges: Vec<(u32, u32)>,
    /// the transforms from the root down to the current node, and every leaf's world bounds
    xf: Vec<(Mat3, f32, Vec3)>,
    leaf_bounds: Vec<(u32, Aabb)>,
    /// each point register's points as an affine map of the object's (None:
    /// through a fold, repeat, twist or bend), and the anchored ops so far
    maps: [Option<Xf>; NREG],
    anchors: Vec<Anchor>,
    table_len: usize,
    /// the NURBS surfaces of its leaves, and where to get them built (the
    /// scene's cache: an edit recompiles the scene, and a surface is cut
    /// into its pieces once)
    surfaces: Vec<Arc<Nurbs>>,
    cache: Option<&'a mut super::TerrainCache>,
}

impl<'a, M: MaterialSource> Compiler<'a, M> {
    pub fn new(mats: &'a mut M) -> Self {
        Compiler {
            ops: Vec::new(),
            np: 1,
            nd: 0,
            step: 1.0,
            leaves: 0,
            mats,
            path: Vec::new(),
            cur: 0,
            paths: Vec::new(),
            ranges: Vec::new(),
            xf: Vec::new(),
            leaf_bounds: Vec::new(),
            maps: std::array::from_fn(|i| (i == 0).then_some(Xf::IDENTITY)),
            anchors: Vec::new(),
            table_len: 0,
            surfaces: Vec::new(),
            cache: None,
        }
    }

    /// Builds its NURBS leaves' surfaces through the scene's cache.
    pub fn with_cache(mut self, cache: &'a mut super::TerrainCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// An anchor table entry for an op reading register `p`, if its points
    /// are an affine map of the object's.
    fn anchor(&mut self, p: u8, kind: AnchorKind, stride: usize) -> u16 {
        let Some(xf) = self.maps[p as usize] else { return NO_ANCHOR };
        // (offsets are u16: a program too big for them is evaluated plainly)
        if self.table_len + stride >= NO_ANCHOR as usize {
            return NO_ANCHOR;
        }
        let at = self.table_len as u16;
        self.anchors.push(Anchor { xf, kind, at });
        self.table_len += stride;
        at
    }

    fn alloc_p(&mut self) -> Result<u8, String> {
        if self.np >= NREG {
            return Err(format!("shape tree too deep (more than {NREG} nested transforms)"));
        }
        self.np += 1;
        Ok((self.np - 1) as u8)
    }

    fn alloc_d(&mut self) -> Result<u8, String> {
        if self.nd >= NREG {
            return Err(format!("shape tree too wide/deep (more than {NREG} pending distances)"));
        }
        self.nd += 1;
        Ok((self.nd - 1) as u8)
    }

    pub fn finish(self) -> Program {
        let lod = self.ops.iter().any(|op| op.lod().is_some_and(|l| l.cap > 0));
        // (a leaf or modifier still in plain f32 would be noisier than the
        // floor an anchored object gets)
        let anchored = !self.anchors.is_empty()
            && self.ops.iter().all(|op| match op {
                Op::Menger { at, .. } | Op::Slice { at, .. } => *at != NO_ANCHOR,
                Op::Affine { .. } | Op::Union { .. } | Op::Intersect { .. } | Op::Subtract { .. } | Op::Mul { .. } => true,
                _ => false,
            });
        Program {
            ops: self.ops,
            leaves: self.leaves,
            paths: self.paths,
            ranges: self.ranges,
            leaf_bounds: self.leaf_bounds,
            lod,
            anchors: self.anchors,
            table_len: self.table_len,
            anchored,
            surfaces: self.surfaces,
        }
    }

    /// Compiles `node` reading points from register `p`, inheriting material
    /// `mat`: the distance register (top of the stack) and the node's bound in
    /// the parent's frame.
    pub fn node(&mut self, node: &NodeDef, p: u8, mat: u16) -> Result<(u8, Aabb), String> {
        let id = self.paths.len();
        self.paths.push(self.path.clone());
        self.ranges.push((0, 0));
        let (prev, start) = (self.cur, self.ops.len() as u32);
        self.cur = id;
        let r = self.node_inner(node, p, mat);
        self.cur = prev;
        self.ranges[id] = (start, self.ops.len() as u32);
        r
    }

    /// Compiles child `index` of the current node.
    fn child(&mut self, index: usize, node: &NodeDef, p: u8, mat: u16) -> Result<(u8, Aabb), String> {
        self.path.push(index);
        let r = self.node(node, p, mat);
        self.path.pop();
        r
    }

    fn node_inner(&mut self, node: &NodeDef, p: u8, mat: u16) -> Result<(u8, Aabb), String> {
        let mat = match &node.material {
            Some(m) => self.mats.resolve(m)?,
            None => mat,
        };
        let rot = node.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
        let s = node.scale.unwrap_or(1.0);
        if !(s > 0.0) {
            return Err("scale must be positive".into());
        }
        let t = node.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
        let identity = rot.is_identity() && s == 1.0 && t == Vec3::ZERO;
        if identity {
            return self.shape(&node.shape, p, mat);
        }
        let q = self.alloc_p()?;
        let m = rot.transpose().scale(1.0 / s);
        self.ops.push(Op::Affine { dst: q, src: p, m: m.to_array(), t: t.to_array() });
        self.maps[q as usize] = self.maps[p as usize].map(|x| x.then(m.to_array(), t.to_array()));
        self.xf.push((rot, s, t));
        let r = self.shape(&node.shape, q, mat);
        self.xf.pop();
        let (d, b) = r?;
        self.np -= 1;
        if s != 1.0 {
            self.ops.push(Op::Mul { d, s });
        }
        Ok((d, b.transform(rot, s, t)))
    }

    /// The level of detail of a fractal leaf here, whose detail grows `g`
    /// times finer each iteration, at most `cap` iterations.
    fn lod(&self, g: f32, cap: u32) -> Lod {
        // the leaf's metres per world metre
        let scale: f32 = self.xf.iter().map(|x| x.1).product();
        let per = 1.0 / g.max(1.05).ln();
        // (the finest detail two thresholds across)
        Lod { k: scale.ln() * per, per, cap }
    }

    fn leaf(&mut self, make: impl FnOnce(u8) -> Op, bound: Aabb) -> Result<(u8, Aabb), String> {
        let d = self.alloc_d()?;
        let op = make(d);
        let tag = leaf_tag(&op);
        self.ops.push(op);
        self.leaves += 1;
        if let Some(tag) = tag {
            self.note_leaf(tag, bound);
        }
        Ok((d, bound))
    }

    /// Records a leaf's bounds in world space (through every enclosing transform).
    fn note_leaf(&mut self, tag: u32, bound: Aabb) {
        let mut b = bound;
        for &(r, s, t) in self.xf.iter().rev() {
            b = b.transform(r, s, t);
        }
        self.leaf_bounds.push((tag, b));
    }

    fn children(&mut self, kids: &[NodeDef], p: u8, mat: u16, kind: u8, k: f32) -> Result<(u8, Aabb), String> {
        let kids: Vec<(usize, &NodeDef)> = kids.iter().enumerate().filter(|(_, c)| !c.disabled).collect();
        let Some((&(i0, first), rest)) = kids.split_first() else {
            return Err("a union / intersection / difference needs children".into());
        };
        let (a, mut bound) = self.child(i0, first, p, mat)?;
        for &(i, c) in rest {
            let (b, cb) = self.child(i, c, p, mat)?;
            let op = match (kind, k > 0.0) {
                (0, false) => Op::Union { a, b },
                (0, true) => Op::SmoothUnion { a, b, k },
                (1, false) => Op::Intersect { a, b },
                (1, true) => Op::SmoothIntersect { a, b, k },
                (_, false) => Op::Subtract { a, b },
                (_, true) => Op::SmoothSubtract { a, b, k },
            };
            self.ops.push(op);
            self.nd -= 1;
            bound = match kind {
                0 => bound.union(cb).grow(k * 0.25),
                1 => bound.intersect(cb).grow(k * 0.25),
                _ => bound.grow(k * 0.25),
            };
        }
        Ok((a, bound))
    }

    fn point_op(
        &mut self,
        make: impl FnOnce(u8) -> Op,
        child: &NodeDef,
        mat: u16,
    ) -> Result<(u8, Aabb), String> {
        let q = self.alloc_p()?;
        self.ops.push(make(q));
        self.maps[q as usize] = None;
        let r = self.child(0, child, q, mat)?;
        self.np -= 1;
        Ok(r)
    }

    fn shape(&mut self, shape: &ShapeDef, p: u8, mat: u16) -> Result<(u8, Aabb), String> {
        use ShapeDef as S;
        let tag = tag(self.cur, mat);
        match *shape {
            S::Sphere { radius } => self.leaf(|d| Op::Sphere { d, p, c: [0.0; 3], r: radius, mat: tag }, Aabb::cube(radius)),
            S::Box { size, rounding } => {
                if size.iter().any(|&v| !(v >= 0.0)) {
                    return Err("a box's size cannot be negative".into());
                }
                let h = Vec3::from_array(size) * 0.5;
                let r = rounding.min(h.x.min(h.y).min(h.z)).max(0.0);
                self.leaf(|d| Op::Box { d, p, b: (h - Vec3::splat(r)).to_array(), r, mat: tag }, Aabb::half(h))
            }
            S::Plane { normal, offset } => {
                let n = Vec3::from_array(normal).normalize();
                self.leaf(|d| Op::Plane { d, p, n: n.to_array(), h: offset, mat: tag }, plane_bound(n, offset))
            }
            S::Torus { radius, thickness } => {
                let r = radius + thickness;
                self.leaf(
                    |d| Op::Torus { d, p, big: radius, small: thickness, mat: tag },
                    Aabb::half(Vec3::new(r, thickness, r)),
                )
            }
            S::Cylinder { radius, height } => self.leaf(
                |d| Op::Cylinder { d, p, r: radius, h: height * 0.5, mat: tag },
                Aabb::half(Vec3::new(radius, height * 0.5, radius)),
            ),
            S::Cone { radius, top_radius, height } => {
                let r = radius.max(top_radius);
                self.leaf(
                    |d| Op::Cone { d, p, r1: radius, r2: top_radius, h: height * 0.5, mat: tag },
                    Aabb::half(Vec3::new(r, height * 0.5, r)),
                )
            }
            S::Capsule { a, b, radius } => {
                let (va, vb) = (Vec3::from_array(a), Vec3::from_array(b));
                let bound = Aabb::new(va.min(vb), va.max(vb)).grow(radius);
                self.leaf(|d| Op::Capsule { d, p, a, b, r: radius, mat: tag }, bound)
            }
            S::Ellipsoid { radii } => {
                // iq's approximation overshoots for very flat ellipsoids
                let r = Vec3::from_array(radii);
                self.step = self.step.min((r.x.min(r.y).min(r.z) / r.max_comp()).sqrt().max(0.3));
                self.leaf(|d| Op::Ellipsoid { d, p, r: radii, mat: tag }, Aabb::half(r))
            }
            S::Octahedron { size } => self.leaf(|d| Op::Octahedron { d, p, s: size, mat: tag }, Aabb::cube(size)),
            S::Gyroid { frequency, thickness } => {
                let f = frequency * core::f32::consts::TAU;
                self.leaf(|d| Op::Gyroid { d, p, f, t: thickness, mat: tag }, Aabb::INFINITE)
            }
            S::Menger { iterations } => {
                // (past 12 levels only anchored: vm.rs menger)
                let lod = self.lod(3.0, MENGER_LEVELS as u32);
                let at = self.anchor(p, AnchorKind::Menger, MENGER_STRIDE);
                self.leaf(|d| Op::Menger { d, p, iters: iterations.clamp(0, 12), lod, mat: tag, at }, Aabb::cube(1.0))
            }
            S::Mandelbulb { power, iterations } => {
                let power = power.clamp(2.0, 16.0);
                let lod = Lod::NONE;
                self.leaf(|d| Op::Mandelbulb { d, p, power, iters: iterations.clamp(1, 32), lod, mat: tag }, Aabb::cube(1.25))
            }
            S::Mandelbox { fold_scale, iterations, min_radius } => {
                let s = fold_scale;
                let lod = Lod::NONE;
                let r = if s > 1.0 { 2.0 * (s + 1.0) / (s - 1.0) } else { 4.0 };
                self.leaf(
                    |d| Op::Mandelbox { d, p, scale: s, min_r: min_radius.clamp(0.01, 1.0), iters: iterations.clamp(1, 40), lod, mat: tag },
                    Aabb::cube(r.min(12.0)),
                )
            }
            S::Julia { c, iterations } => {
                let lod = Lod::NONE;
                self.leaf(|d| Op::Julia { d, p, c, iters: iterations.clamp(1, 32), lod, mat: tag }, Aabb::cube(1.6))
            }
            S::Sierpinski { iterations } => {
                let lod = self.lod(2.0, 24);
                self.leaf(|d| Op::Sierpinski { d, p, iters: iterations.clamp(1, 20), lod, mat: tag }, Aabb::cube(1.0))
            }
            S::Apollonian { fold, iterations } => {
                let lod = Lod::NONE;
                self.leaf(|d| Op::Apollonian { d, p, fold: fold.clamp(0.5, 2.0), iters: iterations.clamp(1, 20), lod, mat: tag }, Aabb::INFINITE)
            }
            S::Kifs { fold_scale, offset, angles, iterations } => {
                let s = fold_scale.clamp(1.2, 6.0);
                let rot = crate::math::Mat3::euler_deg(angles).to_array();
                // folded, turned and scaled about the offset: within its reach
                let reach = 1.8 * offset.iter().fold(1.0f32, |m, &v| m.max(v.abs()));
                let lod = self.lod(s, 24);
                self.leaf(|d| Op::Kifs { d, p, s, off: offset, rot, iters: iterations.clamp(1, 12), lod, mat: tag }, Aabb::cube(reach))
            }
            S::Kleinian { fold_box, inversion, iterations } => {
                let lod = Lod::NONE;
                self.leaf(
                    |d| Op::Kleinian { d, p, cbox: fold_box, size: inversion.clamp(0.1, 4.0), iters: iterations.clamp(1, 20), lod, mat: tag },
                    Aabb::INFINITE,
                )
            }
            S::Formula(ref node) => {
                let f = node.formula();
                let lod = match f.lod {
                    Some((g, cap)) => self.lod(g, cap),
                    None => Lod::NONE,
                };
                self.step = self.step.min(f.step);
                let bound = if f.bound.is_finite() { Aabb::cube(f.bound) } else { Aabb::INFINITE };
                let (id, k, iters) = (node.id, node.pack(), node.iters());
                self.leaf(|d| Op::Formula { d, p, f: id, k, iters, lod, mat: tag }, bound)
            }
            S::Metaballs { ref balls, blend } => {
                if balls.is_empty() {
                    return Err("metaballs needs at least one ball".into());
                }
                let mut bound = Aabb::EMPTY;
                let a = self.alloc_d()?;
                for (i, b) in balls.iter().enumerate() {
                    let c = Vec3::from_array(b.center);
                    bound = bound.union(Aabb::cube(b.radius).transform(Mat3::IDENTITY, 1.0, c));
                    let d = if i == 0 { a } else { self.alloc_d()? };
                    self.ops.push(Op::Sphere { d, p, c: b.center, r: b.radius, mat: tag });
                    self.leaves += 1;
                    self.note_leaf(tag, Aabb::cube(b.radius).transform(Mat3::IDENTITY, 1.0, Vec3::from_array(b.center)));
                    if i > 0 {
                        self.ops.push(if blend > 0.0 { Op::SmoothUnion { a, b: d, k: blend } } else { Op::Union { a, b: d } });
                        self.nd -= 1;
                    }
                }
                Ok((a, bound.grow(blend * 0.25)))
            }
            S::Union { ref children, smooth } => self.children(children, p, mat, 0, smooth),
            S::Intersection { ref children, smooth } => self.children(children, p, mat, 1, smooth),
            S::Difference { ref children, smooth } => self.children(children, p, mat, 2, smooth),
            S::Repeat { period, count, ref child } => {
                let limit = match count {
                    Some(c) => c.map(|v| v as f32),
                    None => [-1.0; 3],
                };
                let (d, b) = self.point_op(|q| Op::Repeat { dst: q, src: p, period, limit }, child, mat)?;
                let mut out = b;
                for (axis, &per) in period.iter().enumerate() {
                    if per == 0.0 {
                        continue;
                    }
                    let (lo, hi) = if limit[axis] < 0.0 {
                        (f32::NEG_INFINITY, f32::INFINITY)
                    } else {
                        let e = per.abs() * limit[axis];
                        (b.min.get(axis) - e, b.max.get(axis) + e)
                    };
                    set_axis(&mut out, axis, lo, hi);
                }
                Ok((d, out))
            }
            S::Mirror { ref axes, ref child } => {
                let mut mask = 0u8;
                for ch in axes.chars() {
                    mask |= match ch {
                        'x' | 'X' => 1,
                        'y' | 'Y' => 2,
                        'z' | 'Z' => 4,
                        _ => return Err(format!("mirror axes '{axes}': use x, y and z")),
                    };
                }
                let (d, b) = self.point_op(|q| Op::Mirror { dst: q, src: p, axes: mask }, child, mat)?;
                let mut out = b;
                for axis in 0..3 {
                    if mask & (1 << axis) != 0 {
                        let e = b.min.get(axis).abs().max(b.max.get(axis).abs());
                        set_axis(&mut out, axis, -e, e);
                    }
                }
                Ok((d, out))
            }
            S::Twist { amount, ref child } => {
                let (d, b) = self.point_op(|q| Op::Twist { dst: q, src: p, k: amount }, child, mat)?;
                let r = radius_xz(b);
                self.step = self.step.min(1.0 / (1.0 + (amount * r).powi(2)).sqrt());
                Ok((d, Aabb::new(Vec3::new(-r, b.min.y, -r), Vec3::new(r, b.max.y, r))))
            }
            S::Bend { amount, ref child } => {
                let (d, b) = self.point_op(|q| Op::Bend { dst: q, src: p, k: amount }, child, mat)?;
                let r = b.min.length().max(b.max.length());
                self.step = self.step.min(1.0 / (1.0 + (amount * r).powi(2)).sqrt());
                Ok((d, Aabb::new(Vec3::new(-r, -r, b.min.z), Vec3::new(r, r, b.max.z))))
            }
            S::Round { radius, ref child } => {
                let (d, b) = self.child(0, child, p, mat)?;
                self.ops.push(Op::Round { d, r: radius });
                Ok((d, b.grow(radius)))
            }
            S::Shell { thickness, ref child } => {
                let (d, b) = self.child(0, child, p, mat)?;
                self.ops.push(Op::Shell { d, t: thickness });
                Ok((d, b.grow(thickness)))
            }
            S::Displace { amplitude, frequency, octaves, ref child } => {
                let (d, b) = self.child(0, child, p, mat)?;
                let octaves = octaves.clamp(1, 8);
                self.ops.push(Op::Displace { d, p, amp: amplitude, freq: frequency, octaves });
                // Perlin fBm's gradient grows with every octave
                self.step = self.step.min(1.0 / (1.0 + amplitude.abs() * frequency * 1.5 * octaves as f32));
                Ok((d, b.grow(amplitude.abs())))
            }
            S::Slice { normal, offset, thickness, ref child } => {
                let n = crate::math::Vec3::from_array(normal);
                if !(n.length() > 1e-6) {
                    return Err("slice: the normal must not be zero".into());
                }
                let (d, b) = self.child(0, child, p, mat)?;
                let n = n.normalize().to_array();
                let at = self.anchor(p, AnchorKind::Slice { n, off: offset }, SLICE_STRIDE);
                self.ops.push(Op::Slice { d, p, n, off: offset, half: thickness.max(0.0) * 0.5, at });
                Ok((d, b))
            }
            S::Relief { ref pattern, frequency, depth, warp, sharpness, ref child } => {
                use crate::scene::tex;
                let kind = tex::by_name(pattern).filter(|k| ![tex::NONE, tex::TERRAIN, tex::WATER, tex::ORBIT].contains(k)).ok_or_else(|| {
                    let names: Vec<&str> = tex::PATTERNS.iter().map(|p| p.0).filter(|n| !["none", "terrain", "water", "orbit"].contains(n)).collect();
                    format!("relief: unknown pattern '{pattern}' (one of {})", names.join(", "))
                })?;
                let (d, b) = self.child(0, child, p, mat)?;
                let sharp = sharpness.max(0.05);
                self.ops.push(Op::Relief { d, p, kind, freq: frequency, depth, warp, sharp });
                // the steepest slope of the pattern per cell: smooth ones a few,
                // edged ones (grout, mortar) far more; capped, so a step stays
                // useful (the steepest edges may then show slight seams)
                let slope = match kind {
                    tex::WAVES | tex::RIPPLES => 3.2,
                    tex::NOISE | tex::WEAVE | tex::SCALES => 6.0,
                    tex::STRIPES | tex::MARBLE | tex::WOOD => 12.0,
                    _ => 30.0,
                } * sharp;
                self.step = self.step.min(1.0 / (1.0 + depth.abs() * frequency.abs() * slope).min(14.0));
                Ok((d, b.grow(depth.abs())))
            }
            S::Terrain(_) | S::Water { .. } | S::Cloud { .. } => Err("terrain, water and clouds must be top-level objects".into()),
            // in a boolean or under a modifier: a leaf of the program, by its
            // signed distance (alone at the top level a surface is hit exactly)
            S::Nurbs { .. } | S::Revolve { .. } => {
                let build = || Nurbs::from_shape(shape).map(|s| s.expect("a surface"));
                let surf = match self.cache.as_deref_mut() {
                    Some(c) => c.surface(serde_json::to_string(shape).unwrap_or_default(), build)?,
                    None => Arc::new(build()?),
                };
                // its distance grid now (once per surface: the cache keeps it)
                crate::render::nurbs::grid(&surf);
                let i = match self.surfaces.iter().position(|s| Arc::ptr_eq(s, &surf)) {
                    Some(i) => i,
                    None => {
                        self.surfaces.push(surf.clone());
                        self.surfaces.len() - 1
                    }
                };
                if i > u8::MAX as usize {
                    return Err("more than 256 different NURBS surfaces in one object".into());
                }
                // (away from the surface its distance is the grid's, within a
                // third of the true one; near it, projected onto the nearest
                // piece: steps of two thirds keep the march short of it)
                self.step = self.step.min(0.65);
                self.leaf(|d| Op::Nurbs { d, p, surf: i as u8, mat: tag }, surf.aabb)
            }
            S::Emitter { .. } => Err("an emitter must be a top-level object".into()),
            // (the scene opens groups into objects of their own; this is the
            // whole group as one shape: the physics' body of a group)
            S::Group { ref children } => self.children(children, p, mat, 0, 0.0),
        }
    }
}

/// The tag of a leaf op.
fn leaf_tag(op: &Op) -> Option<u32> {
    match *op {
        Op::Sphere { mat, .. }
        | Op::Box { mat, .. }
        | Op::Plane { mat, .. }
        | Op::Torus { mat, .. }
        | Op::Cylinder { mat, .. }
        | Op::Cone { mat, .. }
        | Op::Capsule { mat, .. }
        | Op::Ellipsoid { mat, .. }
        | Op::Nurbs { mat, .. }
        | Op::Octahedron { mat, .. }
        | Op::Gyroid { mat, .. }
        | Op::Menger { mat, .. }
        | Op::Mandelbulb { mat, .. }
        | Op::Mandelbox { mat, .. }
        | Op::Julia { mat, .. }
        | Op::Sierpinski { mat, .. }
        | Op::Apollonian { mat, .. }
        | Op::Kifs { mat, .. }
        | Op::Kleinian { mat, .. }
        | Op::Formula { mat, .. } => Some(mat),
        _ => None,
    }
}

fn set_axis(b: &mut Aabb, axis: usize, lo: f32, hi: f32) {
    match axis {
        0 => (b.min.x, b.max.x) = (lo, hi),
        1 => (b.min.y, b.max.y) = (lo, hi),
        _ => (b.min.z, b.max.z) = (lo, hi),
    }
}

fn radius_xz(b: Aabb) -> f32 {
    let x = b.min.x.abs().max(b.max.x.abs());
    let z = b.min.z.abs().max(b.max.z.abs());
    (x * x + z * z).sqrt()
}

/// The half-space n.p <= h: bounded on an axis only when n is that axis.
fn plane_bound(n: Vec3, h: f32) -> Aabb {
    let mut b = Aabb::INFINITE;
    for axis in 0..3 {
        let c = n.get(axis);
        if (c.abs() - 1.0).abs() < 1e-6 {
            if c > 0.0 {
                set_axis(&mut b, axis, f32::NEG_INFINITY, h);
            } else {
                set_axis(&mut b, axis, -h, f32::INFINITY);
            }
        }
    }
    b
}
