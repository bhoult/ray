//! The scene for the GPU: one flat buffer of objects, BVH, materials, lights,
//! emitters, terrain records and the numbers of every distance program; one
//! buffer of terrain maps; and the shader, with a WGSL function per distinct
//! program *structure* (the op sequence and registers). The numbers are read
//! from the buffer, so editing a parameter only re-uploads data, objects of
//! the same structure share their function, and the shader is recompiled
//! only when a new structure appears.

use crate::math::{Mat3, Vec3};
use crate::scene::sdf::{Lod, NO_ANCHOR, Op};
use crate::scene::{Scene, Shape};
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;

pub const LIBRARY: &str = include_str!("lib.wgsl");

/// Record sizes in words (must match lib.wgsl).
pub const OBJ: usize = 48;
const MAT: usize = 30;
const TER: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    pub n_objects: u32,
    pub off_objects: u32,
    pub n_nodes: u32,
    pub off_nodes: u32,
    pub off_items: u32,
    pub n_unbounded: u32,
    pub off_unbounded: u32,
    pub off_mats: u32,
    pub n_lights: u32,
    pub off_lights: u32,
    pub n_emitters: u32,
    pub off_emitters: u32,
    pub n_terrains: u32,
    pub off_terrains: u32,
    pub n_volumes: u32,
    pub off_volumes: u32,
}

pub struct Upload {
    /// the generated part of the shader (the library is prepended)
    pub generated: String,
    pub scene: Vec<u32>,
    pub terrain: Vec<u32>,
    pub layout: Layout,
    /// per object, where its anchor table's header word is in `scene` (0:
    /// none); each frame writes the header (1: a table now) and the table
    pub tables: Vec<u32>,
    /// the terrain maps `terrain` holds, in order; and whether they are the
    /// ones the GPU already has (then `terrain` is left empty: its buffer
    /// stays, and a scene that moves above a big terrain does not upload it
    /// again every frame)
    pub terrains: Vec<Arc<crate::scene::terrain::Terrain>>,
    pub terrain_same: bool,
}

impl Upload {
    pub fn shader(&self) -> String {
        LIBRARY.replace("//GENERATED", &self.generated)
    }
}

fn f(v: f32) -> u32 {
    v.to_bits()
}

fn push3(b: &mut Vec<u32>, v: Vec3) {
    b.extend([f(v.x), f(v.y), f(v.z)]);
}

/// Words of one volume cloud's header.
const VOL: usize = 36;

fn put3(b: &mut [u32], at: usize, v: Vec3, w: u32) {
    b[at..at + 4].copy_from_slice(&[f(v.x), f(v.y), f(v.z), w]);
}

/// The WGSL of one program (distance only, or with tags) and its numbers.
struct Emit {
    code: String,
    params: Vec<f32>,
    tags: bool,
    /// its NURBS leaves: where in the numbers each one's five words go (the
    /// surface's offsets, flags and grid, known once it is placed), and which
    /// surface of the program it is
    surfs: Vec<(usize, u8)>,
    /// the formula catalog's kernels it calls (formulas/mod.rs ids)
    formulas: Vec<u16>,
}

impl Emit {
    /// A number from the program's block: word k, read four at a time (`S4[bq + k / 4]`).
    fn p(&mut self, v: f32) -> String {
        self.params.push(v);
        let k = self.params.len() - 1;
        format!("S4[bq + {}u].{}", k / 4, ["x", "y", "z", "w"][k % 4])
    }

    fn p3(&mut self, v: [f32; 3]) -> String {
        let (x, y, z) = (self.p(v[0]), self.p(v[1]), self.p(v[2]));
        format!("vec3f({x}, {y}, {z})")
    }

    /// A u32 (tag, iteration count, axis mask) bit-cast into the block.
    fn pu(&mut self, v: u32) -> String {
        self.params.push(f32::from_bits(v));
        let k = self.params.len() - 1;
        format!("bitcast<u32>(S4[bq + {}u].{})", k / 4, ["x", "y", "z", "w"][k % 4])
    }

    /// A fractal's iteration count: as set, more close up (lib.wgsl more_iters).
    fn iters(&mut self, iters: u32, lod: Lod) -> String {
        let (i, k, per) = (self.pu(iters), self.p(lod.k), self.p(lod.per));
        format!("more_iters({i}, {k}, {per}, {}u)", lod.cap)
    }

    fn line(&mut self, s: &str) {
        self.code.push_str("    ");
        self.code.push_str(s);
        self.code.push('\n');
    }

    /// A leaf: the distance into d`d`, its tag into m`d` (and orbit trap 0
    /// into o`d`).
    fn leaf(&mut self, d: u8, expr: String, tag: u32) {
        let t = self.pu(tag);
        self.line(&format!("d{d} = {expr};"));
        if self.tags {
            self.line(&format!("m{d} = {t}; o{d} = 0.0;"));
        }
    }

    /// A fractal leaf: `call` gives (distance, orbit trap).
    fn fractal(&mut self, d: u8, call: String, tag: u32) {
        let t = self.pu(tag);
        if self.tags {
            self.line(&format!("{{ let r = {call}; d{d} = r.x; o{d} = r.y; m{d} = {t}; }}"));
        } else {
            self.line(&format!("d{d} = {call}.x;"));
        }
    }

    fn op(&mut self, op: &Op) {
        match *op {
            Op::Affine { dst, src, m, t } => {
                let t = self.p3(t);
                let r0 = self.p3([m[0], m[1], m[2]]);
                let r1 = self.p3([m[3], m[4], m[5]]);
                let r2 = self.p3([m[6], m[7], m[8]]);
                self.line(&format!("{{ let q = p{src} - {t}; p{dst} = vec3f(dot({r0}, q), dot({r1}, q), dot({r2}, q)); }}"));
            }
            Op::Repeat { dst, src, period, limit } => {
                let (pe, li) = (self.p3(period), self.p3(limit));
                self.line(&format!("p{dst} = rep3(p{src}, {pe}, {li});"));
            }
            Op::Mirror { dst, src, axes } => {
                let a = self.pu(axes as u32);
                self.line(&format!("p{dst} = mirror3(p{src}, {a});"));
            }
            Op::Twist { dst, src, k } => {
                let k = self.p(k);
                self.line(&format!("p{dst} = twist(p{src}, {k});"));
            }
            Op::Bend { dst, src, k } => {
                let k = self.p(k);
                self.line(&format!("p{dst} = bend(p{src}, {k});"));
            }
            Op::Sphere { d, p, c, r, mat } => {
                let (c, r) = (self.p3(c), self.p(r));
                self.leaf(d, format!("length(p{p} - {c}) - {r}"), mat);
            }
            Op::Box { d, p, b, r, mat } => {
                let (b, r) = (self.p3(b), self.p(r));
                self.leaf(d, format!("sd_box(p{p}, {b}, {r})"), mat);
            }
            Op::Plane { d, p, n, h, mat } => {
                let (n, h) = (self.p3(n), self.p(h));
                self.leaf(d, format!("dot(p{p}, {n}) - {h}"), mat);
            }
            Op::Torus { d, p, big, small, mat } => {
                let (a, b) = (self.p(big), self.p(small));
                self.leaf(d, format!("sd_torus(p{p}, {a}, {b})"), mat);
            }
            Op::Cylinder { d, p, r, h, mat } => {
                let (r, h) = (self.p(r), self.p(h));
                self.leaf(d, format!("sd_cylinder(p{p}, {r}, {h})"), mat);
            }
            Op::Cone { d, p, r1, r2, h, mat } => {
                let (a, b, h) = (self.p(r1), self.p(r2), self.p(h));
                self.leaf(d, format!("sd_cone(p{p}, {a}, {b}, {h})"), mat);
            }
            Op::Capsule { d, p, a, b, r, mat } => {
                let (a, b, r) = (self.p3(a), self.p3(b), self.p(r));
                self.leaf(d, format!("sd_capsule(p{p}, {a}, {b}, {r})"), mat);
            }
            Op::Ellipsoid { d, p, r, mat } => {
                let r = self.p3(r);
                self.leaf(d, format!("sd_ellipsoid(p{p}, {r})"), mat);
            }
            Op::Nurbs { d, p, surf, mat } => {
                // (filled in by build once the surface is placed: the program
                // stays the same function whichever surface it measures)
                self.surfs.push((self.params.len(), surf));
                let (a, b, c, fl, g) = (self.pu(0), self.pu(0), self.pu(0), self.pu(0), self.pu(0));
                self.leaf(d, format!("nb_sdf({a}, {b}, {c}, {fl}, {g}, p{p})"), mat);
            }
            Op::Octahedron { d, p, s, mat } => {
                let s = self.p(s);
                self.leaf(d, format!("(abs(p{p}.x) + abs(p{p}.y) + abs(p{p}.z) - {s}) * 0.57735026"), mat);
            }
            Op::Gyroid { d, p, f, t, mat } => {
                let (f, t) = (self.p(f), self.p(t));
                self.leaf(d, format!("sd_gyroid(p{p}, {f}, {t})"), mat);
            }
            Op::Menger { d, p, iters, lod, mat, at } => {
                let (i, n) = (self.iters(iters, lod), self.pu(iters));
                if at == NO_ANCHOR {
                    self.fractal(d, format!("menger(p{p}, {i}, {n})"), mat);
                } else {
                    let a = self.pu(at as u32);
                    self.fractal(d, format!("menger_a(p{p}, {i}, {n}, {a})"), mat);
                }
            }
            Op::Mandelbulb { d, p, power, iters, lod, mat } => {
                let (w, i) = (self.p(power), self.iters(iters, lod));
                self.fractal(d, format!("mandelbulb(p{p}, {w}, {i})"), mat);
            }
            Op::Mandelbox { d, p, scale, min_r, iters, lod, mat } => {
                let (s, m, i) = (self.p(scale), self.p(min_r), self.iters(iters, lod));
                self.fractal(d, format!("mandelbox(p{p}, {s}, {m}, {i})"), mat);
            }
            Op::Julia { d, p, c, iters, lod, mat } => {
                let (c0, c1, c2, c3) = (self.p(c[0]), self.p(c[1]), self.p(c[2]), self.p(c[3]));
                let i = self.iters(iters, lod);
                self.fractal(d, format!("julia(p{p}, vec4f({c0}, {c1}, {c2}, {c3}), {i})"), mat);
            }
            Op::Sierpinski { d, p, iters, lod, mat } => {
                let (i, n) = (self.iters(iters, lod), self.pu(iters));
                self.fractal(d, format!("sierpinski(p{p}, {i}, {n})"), mat);
            }
            Op::Apollonian { d, p, fold, iters, lod, mat } => {
                let (f, i) = (self.p(fold), self.iters(iters, lod));
                self.fractal(d, format!("apollonian(p{p}, {f}, {i})"), mat);
            }
            Op::Kifs { d, p, s, off, rot, iters, lod, mat } => {
                let (sc, o, i) = (self.p(s), self.p3(off), self.iters(iters, lod));
                let (r0, r1, r2) = (self.p3([rot[0], rot[1], rot[2]]), self.p3([rot[3], rot[4], rot[5]]), self.p3([rot[6], rot[7], rot[8]]));
                self.fractal(d, format!("kifs(p{p}, {sc}, {o}, {r0}, {r1}, {r2}, {i})"), mat);
            }
            Op::Kleinian { d, p, cbox, size, iters, lod, mat } => {
                let (b, sz, i) = (self.p3(cbox), self.p(size), self.iters(iters, lod));
                self.fractal(d, format!("kleinian(p{p}, {b}, {sz}, {i})"), mat);
            }
            Op::Formula { d, p, f, k, iters, lod, mat } => {
                let v: Vec<String> = k.iter().map(|&x| self.p(x)).collect();
                let i = self.iters(iters, lod);
                if !self.formulas.contains(&f) {
                    self.formulas.push(f);
                }
                let name = crate::formulas::get(f).wgsl_fn;
                let call = format!("{name}(p{p}, vec4f({}), vec4f({}), vec4f({}), {i})", v[0..4].join(", "), v[4..8].join(", "), v[8..12].join(", "));
                self.fractal(d, call, mat);
            }
            Op::Union { a, b } => {
                if self.tags {
                    self.line(&format!("if (d{b} < d{a}) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = min(d{a}, d{b});"));
            }
            Op::Intersect { a, b } => {
                if self.tags {
                    self.line(&format!("if (d{b} > d{a}) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = max(d{a}, d{b});"));
            }
            Op::Subtract { a, b } => {
                if self.tags {
                    self.line(&format!("if (-d{b} > d{a}) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = max(d{a}, -d{b});"));
            }
            Op::SmoothUnion { a, b, k } => {
                let k = self.p(k);
                self.line(&format!("{{ let k = {k}; let h = clamp((d{b} - d{a}) * (0.5 / k) + 0.5, 0.0, 1.0);"));
                if self.tags {
                    self.line(&format!("if (h < 0.5) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = mix(d{b}, d{a}, h) - h * (1.0 - h) * k; }}"));
            }
            Op::SmoothIntersect { a, b, k } => {
                let k = self.p(k);
                self.line(&format!("{{ let k = {k}; let h = clamp((d{b} - d{a}) * (-0.5 / k) + 0.5, 0.0, 1.0);"));
                if self.tags {
                    self.line(&format!("if (h < 0.5) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = mix(d{b}, d{a}, h) + h * (1.0 - h) * k; }}"));
            }
            Op::SmoothSubtract { a, b, k } => {
                let k = self.p(k);
                self.line(&format!("{{ let k = {k}; let h = clamp((d{b} + d{a}) * (-0.5 / k) + 0.5, 0.0, 1.0);"));
                if self.tags {
                    self.line(&format!("if (h > 0.5) {{ m{a} = m{b}; o{a} = o{b}; }}"));
                }
                self.line(&format!("d{a} = mix(d{a}, -d{b}, h) + h * (1.0 - h) * k; }}"));
            }
            Op::Mul { d, s } => {
                let s = self.p(s);
                self.line(&format!("d{d} *= {s};"));
            }
            Op::Round { d, r } => {
                let r = self.p(r);
                self.line(&format!("d{d} -= {r};"));
            }
            Op::Shell { d, t } => {
                let t = self.p(t);
                self.line(&format!("d{d} = abs(d{d}) - {t};"));
            }
            Op::Slice { d, p, n, off, half, at } => {
                let (n, o, h) = (self.p3(n), self.p(off), self.p(half));
                // (anchored: lib.wgsl anc_slice when the object has a table now)
                let s = if at == NO_ANCHOR { format!("dot(p{p}, {n}) - {o}") } else { format!("select(dot(p{p}, {n}) - {o}, anc_slice({}), ANC != 0u)", self.pu(at as u32)) };
                self.line(&format!("{{ let s = {s}; d{d} = max(d{d}, select(s, abs(s) - {h}, {h} > 0.0)); }}"));
            }
            Op::Relief { d, p, kind, freq, depth, warp, sharp } => {
                let (k, fr, dp, w, sh) = (self.pu(kind as u32), self.p(freq), self.p(depth), self.p(warp), self.p(sharp));
                self.line(&format!("d{d} = d{d} - {dp} * pattern({k}, p{p} * {fr}, p{p} * inverseSqrt(max(dot(p{p}, p{p}), 1e-12)), {w}, {sh}).x;"));
            }
            Op::Displace { d, p, amp, freq, octaves } => {
                let (a, fr, o) = (self.p(amp), self.p(freq), self.pu(octaves));
                self.line(&format!("d{d} = fbm3(p{p} * {fr}, {o}, 0x2545F491u) * {a} + d{d};"));
            }
        }
    }
}

/// The registers an op writes.
fn regs(op: &Op) -> (Option<u8>, Option<u8>) {
    match *op {
        Op::Affine { dst, .. } | Op::Repeat { dst, .. } | Op::Mirror { dst, .. } | Op::Twist { dst, .. } | Op::Bend { dst, .. } => {
            (Some(dst), None)
        }
        Op::Sphere { d, .. }
        | Op::Box { d, .. }
        | Op::Plane { d, .. }
        | Op::Torus { d, .. }
        | Op::Cylinder { d, .. }
        | Op::Cone { d, .. }
        | Op::Capsule { d, .. }
        | Op::Ellipsoid { d, .. }
        | Op::Nurbs { d, .. }
        | Op::Octahedron { d, .. }
        | Op::Gyroid { d, .. }
        | Op::Menger { d, .. }
        | Op::Mandelbulb { d, .. }
        | Op::Mandelbox { d, .. }
        | Op::Julia { d, .. }
        | Op::Sierpinski { d, .. }
        | Op::Apollonian { d, .. }
        | Op::Kifs { d, .. }
        | Op::Kleinian { d, .. }
        | Op::Formula { d, .. } => (None, Some(d)),
        _ => (None, None),
    }
}

/// A program's function body, its numbers, where its NURBS leaves' words go
/// in them (Emit::surfs), and the formula kernels it calls.
type Body = (String, Vec<f32>, Vec<(usize, u8)>, Vec<u16>);

/// The body of a program's function (distance only or with tags).
fn program(ops: &[Op], tags: bool) -> Body {
    let mut e = Emit { code: String::new(), params: Vec::new(), tags, surfs: Vec::new(), formulas: Vec::new() };
    let (mut np, mut nd) = (0u8, 0u8);
    for op in ops {
        let (p, d) = regs(op);
        np = np.max(p.unwrap_or(0));
        nd = nd.max(d.map_or(0, |d| d + 1));
        e.op(op);
    }
    // (the block starts on a multiple of 4 words: lib.wgsl S4)
    let mut head = String::from("    let bq = b >> 2u;\n");
    for i in 1..=np {
        let _ = writeln!(head, "    var p{i} = vec3f(0.0);");
    }
    for i in 0..nd.max(1) {
        let _ = writeln!(head, "    var d{i} = BIG;");
        if tags {
            let _ = writeln!(head, "    var m{i} = 0u;");
            let _ = writeln!(head, "    var o{i} = 0.0;");
        }
    }
    (head + &e.code, e.params, e.surfs, e.formulas)
}

/// An object's program: its function's number, its numbers, and where its
/// NURBS leaves' words go in them (Emit::surfs).
type ProgOf = (u32, Vec<f32>, Vec<(usize, u8)>);

/// A NURBS surface's patches, pieces and BVH nodes into `s` (the patches on
/// a multiple of 4 words: lib.wgsl S4): where each begins.
fn put_surface(s: &mut Vec<u32>, surf: &crate::scene::nurbs::Nurbs) -> (u32, u32, u32) {
    s.resize(s.len().next_multiple_of(4), 0);
    let off_p = s.len() as u32;
    for p in &surf.patches {
        for h in p {
            s.extend(h.map(f));
        }
    }
    let off_pc = s.len() as u32;
    for pc in &surf.pieces {
        let (lo, hi) = (pc.aabb.min, pc.aabb.max);
        s.extend([pc.patch, f(pc.u0), f(pc.u1), f(pc.v0), f(pc.v1), f(lo.x), f(lo.y), f(lo.z), f(hi.x), f(hi.y), f(hi.z), 0]);
    }
    let off_n = s.len() as u32;
    for n in &surf.nodes {
        s.extend([f(n.min.x), f(n.min.y), f(n.min.z), n.count, f(n.max.x), f(n.max.y), f(n.max.z), n.index]);
    }
    (off_p, off_pc, off_n)
}

/// The upload for `scene`; `held` are the terrain maps the GPU has already
/// (Upload::terrain_same).
pub fn build(scene: &Scene, held: &[Arc<crate::scene::terrain::Terrain>]) -> Upload {
    // ---- programs: one function per distinct structure
    let mut structures: HashMap<String, u32> = HashMap::new();
    let mut functions = String::new();
    let mut prog_of: Vec<ProgOf> = Vec::new();
    // (only the catalog's kernels the scene calls: the shader stays small)
    let mut kernels: Vec<u16> = Vec::new();
    for o in &scene.objects {
        let Shape::Sdf(p) = &o.shape else {
            prog_of.push((0, Vec::new(), Vec::new()));
            continue;
        };
        let (body, params, surfs, used) = program(&p.ops, false);
        for f in used {
            if !kernels.contains(&f) {
                kernels.push(f);
            }
        }
        let n = structures.len() as u32;
        let id = *structures.entry(body.clone()).or_insert_with(|| {
            let (tbody, ..) = program(&p.ops, true);
            let _ = write!(
                functions,
                "fn prog{n}(p0: vec3f, b: u32) -> f32 {{\n{body}    return d0;\n}}\n\nfn prog{n}_t(p0: vec3f, b: u32) -> DT {{\n{tbody}    return DT(d0, m0, o0);\n}}\n\n"
            );
            n
        });
        prog_of.push((id, params, surfs));
    }
    let mut generated = String::new();
    kernels.sort_unstable();
    for &f in &kernels {
        generated.push_str(crate::formulas::get(f).wgsl);
        generated.push('\n');
    }
    generated.push_str(&functions);
    generated.push_str("fn prog_dist(k: u32, b: u32, p: vec3f) -> f32 {\n    switch k {\n");
    for n in 0..structures.len() {
        let _ = writeln!(generated, "        case {n}u: {{ return prog{n}(p, b); }}");
    }
    generated.push_str("        default: { return BIG; }\n    }\n}\n\nfn prog_tag(k: u32, b: u32, p: vec3f) -> DT {\n    switch k {\n");
    for n in 0..structures.len() {
        let _ = writeln!(generated, "        case {n}u: {{ return prog{n}_t(p, b); }}");
    }
    generated.push_str("        default: { return DT(BIG, 0u, 0.0); }\n    }\n}\n");
    // the NURBS code compiles away from scenes without a surface (lib.wgsl's nb_*)
    let nurbs = scene.objects.iter().any(|o| match &o.shape {
        Shape::Nurbs { .. } => true,
        Shape::Sdf(p) => !p.surfaces.is_empty(),
        _ => false,
    });
    let _ = writeln!(generated, "\nconst HAS_NURBS: bool = {nurbs};");
    // and the haze's (its shadow rays inline the scene's distance once more)
    let _ = writeln!(generated, "const HAS_HAZE: bool = {};", scene.env.haze > 0.0);

    // ---- terrains (shared maps are uploaded once)
    let mut terrains: Vec<Arc<crate::scene::terrain::Terrain>> = Vec::new();
    let mut ter_of = vec![0u32; scene.objects.len()];
    for (i, o) in scene.objects.iter().enumerate() {
        if let Shape::Terrain(t) = &o.shape {
            let k = match terrains.iter().position(|x| Arc::ptr_eq(x, t)) {
                Some(k) => k,
                None => {
                    terrains.push(t.clone());
                    terrains.len() - 1
                }
            };
            ter_of[i] = k as u32;
        }
    }

    // ---- the scene buffer
    let mut s: Vec<u32> = Vec::new();
    let mut lay = Layout { n_objects: scene.objects.len() as u32, off_objects: 0, ..Default::default() };
    s.resize(scene.objects.len() * OBJ, 0);
    lay.off_nodes = s.len() as u32;
    lay.n_nodes = scene.bvh.nodes.len() as u32;
    for n in &scene.bvh.nodes {
        push3(&mut s, n.min);
        s.push(n.count);
        push3(&mut s, n.max);
        s.push(n.index);
    }
    lay.off_items = s.len() as u32;
    s.extend(&scene.bvh.items);
    lay.off_unbounded = s.len() as u32;
    lay.n_unbounded = scene.unbounded.len() as u32;
    s.extend(&scene.unbounded);
    lay.off_mats = s.len() as u32;
    for m in &scene.materials {
        let at = s.len();
        s.resize(at + MAT, 0);
        put3(&mut s, at, m.color, f(m.roughness));
        put3(&mut s, at + 4, m.color2, f(m.roughness2));
        put3(&mut s, at + 8, m.emission, f(m.metallic));
        put3(&mut s, at + 12, m.emission2, f(m.metallic2));
        put3(&mut s, at + 16, m.absorb, f(m.transmission));
        s[at + 20] = f(m.ior);
        s[at + 21] = m.tex as u32;
        s[at + 22] = f(m.tex_scale);
        s[at + 23] = f(m.warp);
        s[at + 24] = f(m.sharpness);
        s[at + 25] = f(m.bump);
        s[at + 26] = m.world as u32;
        s[at + 27] = f(m.translucency);
        s[at + 28] = m.bump_tex as u32;
        s[at + 29] = f(m.bump_scale);
    }
    lay.off_lights = s.len() as u32;
    lay.n_lights = scene.lights.len() as u32;
    for l in &scene.lights {
        push3(&mut s, l.pos);
        s.push(f(l.range));
        push3(&mut s, l.color);
        s.push(0);
    }
    lay.off_emitters = s.len() as u32;
    lay.n_emitters = scene.emitters.len() as u32;
    for e in &scene.emitters {
        push3(&mut s, e.pos);
        s.push(f(e.radius));
        push3(&mut s, e.color);
        s.push(f(e.range));
        s.extend([e.obj, 0, 0, 0]);
    }
    // the terrain maps go to their own buffer: heights, packed normals, cavity
    // AO (not again when the GPU has these maps already)
    let terrain_same = !terrains.is_empty() && terrains.len() == held.len() && terrains.iter().zip(held).all(|(a, b)| Arc::ptr_eq(a, b));
    let mut t: Vec<u32> = Vec::new();
    let mut len = 0usize;
    let mut put = |t: &mut Vec<u32>, words: &mut dyn Iterator<Item = u32>, n: usize| {
        let at = len as u32;
        len += n;
        if !terrain_same {
            t.extend(words);
        }
        at
    };
    lay.off_terrains = s.len() as u32;
    lay.n_terrains = terrains.len() as u32;
    for tr in &terrains {
        let at = s.len();
        s.resize(at + TER, 0);
        let off_h = put(&mut t, &mut tr.h.iter().map(|v| v.to_bits()), tr.h.len());
        let off_n = put(&mut t, &mut tr.nrm.iter().map(|&v| v as u32), tr.nrm.len());
        let off_ao = put(&mut t, &mut tr.ao.iter().map(|v| v.to_bits()), tr.ao.len());
        s[at..at + 16].copy_from_slice(&[
            tr.n as u32,
            f(tr.cell),
            f(tr.inv_cell),
            f(tr.origin_x),
            f(tr.origin_z),
            f(tr.relax),
            f(tr.min_h),
            f(tr.max_h),
            off_h,
            off_n,
            off_ao,
            f(tr.grass_top),
            f(tr.snow_line),
            f(tr.sand_line),
            0,
            0,
        ]);
        put3(&mut s, at + 16, tr.rock, 0);
        put3(&mut s, at + 20, tr.grass, 0);
        put3(&mut s, at + 24, tr.snow, 0);
        put3(&mut s, at + 28, tr.sand, 0);
    }
    if t.is_empty() {
        t.push(0);
    }
    // the objects, now that the programs' numbers can be placed
    let mut tables = vec![0u32; scene.objects.len()];
    for (i, o) in scene.objects.iter().enumerate() {
        let at = i * OBJ;
        let (prog, params, surfs) = &prog_of[i];
        s.resize(s.len().next_multiple_of(4), 0);
        let base = s.len() as u32;
        s.extend(params.iter().map(|v| v.to_bits()));
        // its NURBS leaves' surfaces after its numbers (each once), and where
        // they are into the words left for them
        if let Shape::Sdf(p) = &o.shape {
            let mut placed: Vec<Option<[u32; 5]>> = vec![None; p.surfaces.len()];
            for &(k, surf) in surfs {
                let w = match placed[surf as usize] {
                    Some(w) => w,
                    None => {
                        let sf = &p.surfaces[surf as usize];
                        let (off_p, off_pc, off_n) = put_surface(&mut s, sf);
                        // and its distance grid (lib.wgsl nb_sdf)
                        let g = crate::render::nurbs::grid(sf);
                        let off_g = s.len() as u32;
                        s.extend([f(g.lo.x), f(g.lo.y), f(g.lo.z), g.n as u32, f(g.cell.x), f(g.cell.y), f(g.cell.z), f(g.slack)]);
                        s.extend(g.values.iter().map(|&v| f(v)));
                        let w = [off_p, off_pc, off_n, sf.solid as u32 | ((sf.orient < 0.0) as u32) << 1, off_g];
                        placed[surf as usize] = Some(w);
                        w
                    }
                };
                let at = base as usize + k;
                s[at..at + 5].copy_from_slice(&w);
            }
        }
        let kind = match &o.shape {
            Shape::Sphere { .. } => 0,
            Shape::Box { .. } => 1,
            Shape::Plane { .. } => 2,
            Shape::Sdf(_) => 3,
            Shape::Terrain(_) => 4,
            Shape::Nurbs { .. } => 5,
        };
        let flags = o.shadow as u32 | (scene.emitter_flags.get(i).copied().unwrap_or(0) as u32) << 1;
        s[at..at + 8].copy_from_slice(&[kind, o.material as u32, flags, *prog, base, f(o.step), ter_of[i], 0]);
        put3(&mut s, at + 8, o.aabb.min, 0);
        put3(&mut s, at + 12, o.aabb.max, 0);
        match &o.shape {
            Shape::Sphere { c, r } => put3(&mut s, at + 16, *c, f(*r)),
            Shape::Box { c, rot, half } => {
                put3(&mut s, at + 16, *c, 0);
                put3(&mut s, at + 20, *half, 0);
                rows(&mut s, at + 24, *rot, Vec3::ZERO);
            }
            Shape::Plane { n, d } => put3(&mut s, at + 16, *n, f(*d)),
            // where it stands (its program works in metres from there), and
            // its fractals' detail, which each frame writes (none until then);
            // its anchor table, a header word (whether it has one this frame)
            // and the table, which each frame writes too; whether every leaf
            // is anchored
            Shape::Sdf(prog) => {
                put3(&mut s, at + 16, o.org, f(3.0e38));
                if prog.table_len > 0 {
                    tables[i] = s.len() as u32;
                    s.resize(s.len() + 1 + prog.table_len, 0);
                    s[at + 20] = tables[i];
                    s[at + 21] = prog.anchored as u32;
                }
            }
            // lib.wgsl's nb_*: the patches, pieces and nodes follow in S
            Shape::Nurbs { surf, c, rot, s: scale } => {
                let (off_p, off_pc, off_n) = put_surface(&mut s, surf);
                s[at + 7] = f(surf.size);
                put3(&mut s, at + 16, *c, f(*scale));
                let flags = surf.solid as u32 | ((surf.orient < 0.0) as u32) << 1;
                s[at + 20..at + 24].copy_from_slice(&[off_p, off_pc, off_n, flags]);
                rows(&mut s, at + 24, *rot, Vec3::ZERO);
            }
            _ => {}
        }
        rows(&mut s, at + 36, o.local_m, o.local_t);
    }
    // the volume clouds: a header each (lib.wgsl's vol_*), then their puffs
    lay.off_volumes = s.len() as u32;
    lay.n_volumes = scene.volumes.len() as u32;
    let headers = s.len();
    s.resize(headers + VOL * scene.volumes.len(), 0);
    for (i, v) in scene.volumes.iter().enumerate() {
        let at = headers + i * VOL;
        let puffs = s.len() as u32;
        for p in &v.puffs {
            s.extend(p.map(f));
        }
        put3(&mut s, at, v.aabb.min, f(v.sigma));
        put3(&mut s, at + 4, v.aabb.max, f(v.turbulence));
        put3(&mut s, at + 8, v.pos, f(v.scale));
        put3(&mut s, at + 12, v.to_local.m[0], f(v.frequency));
        put3(&mut s, at + 16, v.to_local.m[1], v.detail);
        put3(&mut s, at + 20, v.to_local.m[2], v.seed);
        put3(&mut s, at + 24, v.color, f(v.softness));
        s[at + 28..at + 33].copy_from_slice(&[f(v.blend), f(v.base), v.puffs.len() as u32, puffs, f(v.wisps)]);
    }
    // (whole vec4s: lib.wgsl S4 reads the last block's last words)
    s.resize(s.len().max(4).next_multiple_of(4), 0);
    Upload { generated, scene: s, terrain: t, layout: lay, tables, terrains, terrain_same }
}

/// A matrix as three vec4 rows, `w` carrying the translation.
fn rows(s: &mut [u32], at: usize, m: Mat3, t: Vec3) {
    put3(s, at, m.m[0], f(t.x));
    put3(s, at + 4, m.m[1], f(t.y));
    put3(s, at + 8, m.m[2], f(t.z));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::TerrainCache;

    fn upload(objects: serde_json::Value) -> Upload {
        let sc = Scene::from_value(&serde_json::json!({ "objects": objects }), &mut TerrainCache::default()).unwrap();
        assert!(sc.problems.is_empty(), "{:?}", sc.problems);
        build(&sc, &[])
    }

    fn functions(u: &Upload) -> usize {
        // (each has a distance function and a tagged one: prog0 and prog0_t)
        u.generated.matches("_t(p0: vec3f").count()
    }

    /// Distance programs of the same shape share one GPU function (their
    /// numbers differ, not their code): one with a leaf more is another. Pads
    /// of different leaf counts once made a shader of sixteen functions that
    /// took four minutes to build.
    #[test]
    fn programs_of_one_shape_share_a_function() {
        let pad = |n: usize, x: f32| {
            let kids: Vec<_> = (0..n).map(|k| serde_json::json!({ "type": "ellipsoid", "radii": [0.1, 0.02, 0.05], "position": [k as f32 * 0.1, 0, 0], "rotation": [0, k as f32 * 20.0, 0] })).collect();
            serde_json::json!({ "type": "union", "position": [x, 0, 0], "children": kids })
        };
        assert_eq!(functions(&upload(serde_json::json!([pad(5, 0.0), pad(5, 2.0), pad(5, 4.0)]))), 1);
        assert_eq!(functions(&upload(serde_json::json!([pad(5, 0.0), pad(6, 2.0)]))), 2);
    }

    /// The NURBS code is compiled in when a surface is only inside a
    /// boolean, and two programs measuring different surfaces still share
    /// their function (where each surface is goes in their numbers).
    #[test]
    fn nurbs_leaves_need_the_nurbs_code() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let ball = |r: f32| serde_json::json!({ "type": "revolve", "profile": [[0, -r], [r, -r, h], [r, 0], [r, r, h], [0, r]], "degree": 2, "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1] });
        let cut = |r: f32, x: f32| serde_json::json!({ "type": "difference", "position": [x, 0, 0], "children": [{ "type": "box", "size": [1, 1, 1] }, ball(r)] });
        let u = upload(serde_json::json!([cut(0.6, 0.0), cut(0.5, 3.0)]));
        assert!(u.generated.contains("const HAS_NURBS: bool = true;"));
        assert!(u.generated.contains("nb_sdf("));
        assert_eq!(functions(&u), 1);
        let none = upload(serde_json::json!([{ "type": "box" }]));
        assert!(none.generated.contains("const HAS_NURBS: bool = false;"));
    }
}
