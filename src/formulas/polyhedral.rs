//! Geometric folds: Knighty's polyhedral kaleidoscopic IFS, Baird's Koch
//! curve in 3D, and a torus knot laid as a rope of ropes. Each folds space
//! into one cell of a symmetry and scales it about a point of the cell.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, V8};
use core::f32::consts::{PI, TAU};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "polyhedral",
        category: "Fractals: geometric",
        desc: "Knighty's polyhedral kaleidoscopic IFS: space folded into one cell of the tetrahedral, octahedral or icosahedral mirror group, then scaled away from a vertex of the cell's choosing, again and again. The vertex and scale give the icosahedral, dodecahedral and octahedral flakes, the Sierpinski tetrahedron and much in between; each copy ends as the polyhedron its vertex makes, or a ball. Formula by Knighty (Fractal Forums, 2010-2011), after Syntopia's Fragmentarium examples.",
        params: &[
            Param::whole("symmetry", 5.0, 3.0, 5.0, "The mirror group: 3 the tetrahedron's, 4 the octahedron's (and cube's), 5 the icosahedron's (and dodecahedron's)."),
            Param::num(
                "vertex",
                &[0.0, 0.0, 1.0],
                0.0,
                1.0,
                "Where the copies are scaled from, as weights on the mirror cell's three corners: [1,0,0] the midpoint of an edge, [0,1,0] a vertex of the tetrahedron, octahedron or icosahedron, [0,0,1] the centre of its face (a vertex of the dual: the cube, the dodecahedron). Mixtures give truncated forms. Icosahedral flake: symmetry 5, [0,1,0], fold_scale 2.618; dodecahedral: 5, [0,0,1], 3.618; octahedral: 4, [0,1,0], 2; Sierpinski tetrahedron: 3, [0,1,0], 2.",
            ),
            Param::num("fold_scale", &[3.618], 1.2, 4.5, "How much smaller each generation of copies is (1.2 to 4.5): at the flakes' values the copies just touch; less fuses them into a solid, more scatters them into dust."),
            Param::num("size", &[1.0], 0.2, 1.3, "The size of the shape each copy ends as, the vertex at 1 (0.2 to 1.3): smaller pulls the copies apart into beads, larger swells them together."),
            Param::num("roundness", &[0.0], 0.0, 1.0, "What each copy ends as: 0 the polyhedron of that vertex (the icosahedron, the dodecahedron, a truncated form), 1 a ball; between, a blend."),
            Param::num("twist", &[0.0], -180.0, 180.0, "Degrees each generation turns about the axis through its vertex (-180 to 180): small turns wind the copies into whorls."),
        ],
        iterations: (6, 14, "Generations of copies, each fold_scale times smaller: more = finer detail (close up, more are added), at a fold of 3 × `symmetry` reflections each."),
        bound: 1.3,
        lod: Some((2.0, 14)),
        step: 1.0,
        example: r#"{"type":"polyhedral","rotation":[20,30,0],"scale":1.0}"#,
        prep: Some(polyhedral_prep),
        cpu: polyhedral,
        wgsl: POLYHEDRAL,
        wgsl_fn: "polyhedral",
    },
    Formula {
        name: "koch",
        category: "Fractals: geometric",
        desc: "Eric Baird's Koch curve in three dimensions, the \"delta\" fractal: a triangular body whose copies hinge up out of its plane at the Koch angle, again and again, so that its edges and profile are Koch curves and its body a crumpled crystal. Formula by Eric Baird, kaleidoscopic form by Knighty (Fragmentarium); ported from Mandelbulber 2 (Koch V3).",
        params: &[Param::num(
            "angle",
            &[70.0],
            60.0,
            90.0,
            "The Koch angle in degrees (60 to 90): 60 lies flat, a planar Koch tree; more raises the copies out of the plane into a crumpled delta; towards 90 they pack into a solid. Each generation is 2(1 + cos angle) times smaller.",
        )],
        iterations: (8, 20, "Generations of copies, each 2(1 + cos angle) times smaller: more = finer detail (close up, more are added)."),
        bound: 1.02,
        lod: Some((2.0, 20)),
        step: 1.0,
        example: r#"{"type":"koch","rotation":[-35,0,90],"scale":1.2}"#,
        prep: Some(koch_prep),
        cpu: koch,
        wgsl: KOCH,
        wgsl_fn: "koch",
    },
    Formula {
        name: "knot",
        category: "Fractals: geometric",
        desc: "A torus knot whose tube is a rope: strands twisted about its core, each of them a rope of finer strands twisted the other way, and so on down, as rope is laid. The knot is DarkBeam's trefoil routine as Knighty refined it (Fractal Forums, 2012); the rope of ropes is ray's own.",
        params: &[
            Param::whole("laps", 2.0, 1.0, 8.0, "Times the knot goes round the hole (1 to 8): 2 laps with 3 turns is the trefoil. When laps and turns share a factor it falls into that many linked loops."),
            Param::whole("turns", 3.0, 0.0, 8.0, "Times it winds through the hole (0 to 8): 2 laps and 3 turns is the trefoil, 2 and 5 the cinquefoil, 3 and 4 a knot of 8 crossings."),
            Param::num("radius", &[0.45], 0.1, 0.6, "How far the knot strays from its core circle, of radius 1 (0.1 to 0.6): small hugs a ring, large flings the loops wide."),
            Param::num("tube", &[0.18], 0.02, 0.3, "The rope's radius (0.02 to 0.3): keep it under half the gap between passes, or the rope runs into itself."),
            Param::whole("strands", 3.0, 2.0, 6.0, "Strands in each rope, at every level (2 to 6): 3 is common laid rope."),
            Param::whole("twist", 6.0, -24.0, 24.0, "Turns the strands make about their rope's core each lap of the knot (-24 to 24); each finer level turns proportionally faster, the other way. Negative: the other hand; 0: straight strands."),
        ],
        iterations: (4, 8, "Levels: 1 a plain tube, each one more a rope of finer strands (close up, more are added); each costs an atan2, a sin and a cos."),
        bound: 1.95,
        lod: Some((1.8, 8)),
        step: 1.0,
        example: r#"{"type":"knot","rotation":[-30,20,0],"scale":0.6}"#,
        prep: Some(knot_prep),
        cpu: knot,
        wgsl: KNOT,
        wgsl_fn: "knot",
    },
];

// ---- polyhedral ----------------------------------------------------------

fn polyhedral_prep(k: &mut [f32; K]) {
    let n = k[0];
    let cospin = (PI / n).cos();
    let scospin = (0.75 - cospin * cospin).max(0.0).sqrt();
    let isin = 1.0 / (PI / n).sin();
    // the fundamental triangle's corners: the 2-fold, n-fold and 3-fold axes
    let pbc = [scospin * isin, 0.0, 0.5 * isin];
    let ica = 1.0 / 0.75f32.sqrt();
    let pca = [0.0, scospin * ica, cospin * ica];
    let (u, v, w) = (k[1], k[2], k[3]);
    let mut q = [v * pbc[0], w * pca[1], u + v * pbc[2] + w * pca[2]];
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2]).sqrt();
    if l < 1e-6 {
        q = pbc;
    } else {
        q = q.map(|c| c / l);
    }
    let tw = k[7].to_radians();
    k[1] = q[0];
    k[2] = q[1];
    k[3] = q[2];
    k[7] = tw.cos();
    k[8] = tw.sin();
    k[9] = cospin;
    k[10] = scospin;
    k[11] = isin;
}

/// Knighty's fold into the fundamental domain of the order-n polyhedral
/// reflection group: the planes x = 0, y = 0 and the one normal to `nc`.
/// Also how many of the folds reflected it.
#[inline(always)]
fn polyhedral_fold(mut z: V8, n: u32, nc: [f32; 3]) -> (V8, F8) {
    let zero = F8::zero();
    let mut flips = F8::zero();
    for _ in 0..n {
        flips += F8::one().and(z.x.lt(zero)) + F8::one().and(z.y.lt(zero));
        let (x, y) = (z.x.abs(), z.y.abs());
        let t = z.z.mul_add(s(nc[2]), y.mul_add(s(nc[1]), x * nc[0])).min(zero) * -2.0;
        flips += F8::one().and(t.gt(zero));
        z = V8::new(t.mul_add(s(nc[0]), x), t.mul_add(s(nc[1]), y), t.mul_add(s(nc[2]), z.z));
    }
    (z, flips)
}

fn polyhedral(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let n = k[0] as u32;
    let v = [k[1], k[2], k[3]];
    let (sc, size, rnd, ct, st) = (k[4], k[5], k[6], k[7], k[8]);
    let (cospin, scospin, isin) = (k[9], k[10], k[11]);
    let nc = [-0.5, -cospin, scospin];
    let (vx, vy, vz) = (s(v[0]), s(v[1]), s(v[2]));
    let mut z = p;
    let mut trap = F8::zero();
    let mut w = 1.0 / (3 * n) as f32;
    for _ in 0..iters {
        let (zf, flips) = polyhedral_fold(z, n, nc);
        z = zf;
        trap = flips.mul_add(s(w), trap);
        w *= 0.4;
        // turned about the vertex's axis (Rodrigues)
        let along = z.z.mul_add(vz, z.y.mul_add(vy, z.x * vx)) * (1.0 - ct);
        let cr = V8::new(vy * z.z - vz * z.y, vz * z.x - vx * z.z, vx * z.y - vy * z.x);
        z = V8::new(along.mul_add(vx, cr.x.mul_add(s(st), z.x * ct)), along.mul_add(vy, cr.y.mul_add(s(st), z.y * ct)), along.mul_add(vz, cr.z.mul_add(s(st), z.z * ct)));
        // scaled about the vertex
        z = V8::new(z.x * sc - v[0] * (sc - 1.0), z.y * sc - v[1] * (sc - 1.0), z.z * sc - v[2] * (sc - 1.0));
    }
    z = polyhedral_fold(z, n, nc).0;
    let dsph = z.length() - size;
    // the Wythoff polyhedron of vertex size * v: faces normal to the corners
    let pbc = [scospin * isin, 0.0, 0.5 * isin];
    let ica = 1.154_700_5;
    let pca = [0.0, scospin * ica, cospin * ica];
    let d0 = z.z - size * v[2];
    let d1 = z.z.mul_add(s(pbc[2]), z.x * pbc[0]) - size * (v[0] * pbc[0] + v[2] * pbc[2]);
    let d2 = z.z.mul_add(s(pca[2]), z.y * pca[1]) - size * (v[1] * pca[1] + v[2] * pca[2]);
    let dpoly = d0.max(d1).max(d2);
    let d = (dsph - dpoly).mul_add(s(rnd), dpoly);
    // (the trap: the reflection counts, the first generations' most, stretched over where they mostly fall)
    (d * sc.powi(-(iters as i32)), trap.mul_add(s(0.96), s(-0.16)).saturate())
}

const POLYHEDRAL: &str = r#"
fn polyhedral_fold(p: vec3f, n: u32, nc: vec3f) -> vec4f {
    var z = p;
    var flips = 0.0;
    for (var i = 0u; i < n; i++) {
        flips += select(0.0, 1.0, z.x < 0.0) + select(0.0, 1.0, z.y < 0.0);
        z = vec3f(abs(z.x), abs(z.y), z.z);
        let t = -2.0 * min(dot(z, nc), 0.0);
        flips += select(0.0, 1.0, t > 0.0);
        z += t * nc;
    }
    return vec4f(z, flips);
}

fn polyhedral(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let n = u32(a.x);
    let v = a.yzw;
    let sc = b.x;
    let size = b.y;
    let rnd = b.z;
    let ct = b.w;
    let st = c.x;
    let cospin = c.y;
    let scospin = c.z;
    let isin = c.w;
    let nc = vec3f(-0.5, -cospin, scospin);
    var z = p;
    var trap = 0.0;
    var w = 1.0 / f32(3u * n);
    for (var i = 0u; i < iters; i++) {
        let zf = polyhedral_fold(z, n, nc);
        z = zf.xyz;
        trap += zf.w * w;
        w *= 0.4;
        z = z * ct + cross(v, z) * st + v * (dot(z, v) * (1.0 - ct));
        z = z * sc - v * (sc - 1.0);
    }
    z = polyhedral_fold(z, n, nc).xyz;
    let dsph = length(z) - size;
    let pbc = vec3f(scospin * isin, 0.0, 0.5 * isin);
    let pca = vec3f(0.0, scospin, cospin) * 1.1547005;
    let d0 = z.z - size * v.z;
    let d1 = dot(z, pbc) - size * dot(v, pbc);
    let d2 = dot(z, pca) - size * dot(v, pca);
    let dpoly = max(max(d0, d1), d2);
    let d = dpoly + (dsph - dpoly) * rnd;
    return vec2f(d * pow(sc, -f32(iters)), clamp(trap * 0.96 - 0.16, 0.0, 1.0));
}
"#;

// ---- koch (Baird delta) ---------------------------------------------------

fn koch_prep(k: &mut [f32; K]) {
    let beta = k[0].to_radians() * 0.5;
    let tc = beta.tan();
    let tsq = (3.0 * tc * tc - 1.0).max(0.0).sqrt() * 0.25;
    let l = 1.0 / (1.0 + 4.0 * tsq * tsq).sqrt();
    let c = beta.cos();
    k[1] = tsq;
    k[2] = l;
    k[3] = -2.0 * tsq * l;
    k[4] = 4.0 * c * c;
}

fn koch(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (tsq, f0, f2, bd) = (k[1], k[2], k[3], k[4]);
    const C6: f32 = 0.866_025_4;
    let zero = F8::zero();
    let mut z = p;
    // (the trap: which copies were hinged up, the first generations' most)
    let mut trap = F8::zero();
    let mut w = 0.5f32;
    for _ in 0..iters {
        // the triangle's symmetry, and the mirror through its plane
        let (y, zz) = (z.y.abs(), z.z.abs());
        let t = (z.x * C6 - y * 0.5).min(zero) * 2.0;
        let x = t.neg_mul_add(s(C6), z.x);
        let y = t.mul_add(s(0.5), y).abs();
        // the Koch fold: the raised piece onto the flat one, about their hinge
        let (x1, z1) = (x - 0.5, zz - tsq);
        let t = z1.mul_add(s(f2), x1 * f0).min(zero) * 2.0;
        trap += F8::splat(w).and(t.lt(zero));
        w *= 0.5;
        let x = t.neg_mul_add(s(f0), x1) + 0.5;
        let zz = t.neg_mul_add(s(f2), z1) + tsq;
        // scaled about the corner (1, 0, 0)
        z = V8::new((x - 1.0) * bd + 1.0, y * bd, zz * bd);
    }
    // ending in the unit ball: it holds the fractal, and its own images
    let d = (z.length() - 1.0) * bd.powi(-(iters as i32));
    (d, trap.saturate())
}

const KOCH: &str = r#"
fn koch(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let tsq = a.y;
    let f0 = a.z;
    let f2 = a.w;
    let bd = b.x;
    var z = p;
    var trap = 0.0;
    var w = 0.5;
    for (var i = 0u; i < iters; i++) {
        var y = abs(z.y);
        let zz = abs(z.z);
        var t = min(z.x * 0.8660254 - y * 0.5, 0.0) * 2.0;
        var x = z.x - t * 0.8660254;
        y = abs(y + t * 0.5);
        let x1 = x - 0.5;
        let z1 = zz - tsq;
        t = min(x1 * f0 + z1 * f2, 0.0) * 2.0;
        trap += select(0.0, w, t < 0.0);
        w *= 0.5;
        x = x1 - t * f0 + 0.5;
        let z2 = z1 - t * f2 + tsq;
        z = vec3f((x - 1.0) * bd + 1.0, y * bd, z2 * bd);
    }
    let d = (length(z) - 1.0) * pow(bd, -f32(iters));
    return vec2f(d, clamp(trap, 0.0, 1.0));
}
"#;

// ---- knot ------------------------------------------------------------------

fn knot_prep(k: &mut [f32; K]) {
    let laps = k[0].max(1.0);
    let n = k[4];
    let sn = (PI / n).sin();
    // the strands' radius, of the tube's: a little over touching
    let f = (sn / (1.0 + sn) * 1.1).min(0.95);
    let wind = k[1] / laps;
    k[0] = laps;
    k[1] = (TAU / laps).cos();
    k[6] = f;
    k[7] = (TAU / laps).sin();
    k[8] = wind;
    k[9] = k[2] + k[3];
    k[10] = (TAU / n).cos();
    k[11] = (TAU / n).sin();
}

/// floor(x + 0.5): the same rounding on both devices.
#[inline(always)]
fn knot_round(x: F8) -> F8 {
    (x + 0.5).floor()
}

/// The distance to a tube of radius 1 at (u, v) of the cross-section, in
/// metres (`lscale` per unit). The twist stretches space by up to
/// sqrt(1 + (m lscale / rho)²); close to the tube, whose lean (u, v) has
/// allowed for, hardly at all, so the allowance grows over a tube radius.
#[inline(always)]
fn knot_tube(u: F8, v: F8, m: F8, lscale: f32, irho: F8) -> F8 {
    let x = len2(u, v) - 1.0;
    let g = m * irho * x.clamp(0.0, 1.0) * lscale;
    x * lscale / g.mul_add(g, F8::one()).sqrt()
}

/// A torus knot of `laps` and `turns` whose tube is a rope of strands,
/// each a rope of finer strands, and so on: DarkBeam's knot (the meridian
/// plane turned as it goes round, folded to the nearest strand), then at
/// every level the strand's cross-section turned and folded the same way,
/// by the strand's own length. A strand leans across the plane it is cut in:
/// its section is taken square to it (the lean's component shortened), and
/// the point's place along it moved to its nearest point's.
/// The estimate: at every level the distance to the tube folded to, but no
/// further than the nearest tube another fold would have chosen (its
/// sibling's, or an ancestor's sibling's), the best of the levels.
fn knot(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (laps, c0, rg, rt, n, tw, f, s0, wind, outer, cn, sn) = (k[0], k[1], k[2], k[3], k[4], k[5], k[6], k[7], k[8], k[9], k[10], k[11]);
    let a = 1.0 - f;
    let rho = len2(p.x, p.y).max(s(1e-6));
    let irho = F8::one() / rho;
    let th = p.y.atan2(p.x);
    let (u0, v0) = (rho - 1.0, p.z);
    let r0 = len2(u0, v0);
    // (the solid torus about the core circle holds it all)
    let mut d = r0 - outer;
    // the knot: the meridian plane turned by wind * th, folded to the nearest strand
    let ang = th.mul_add(s(wind), v0.atan2(u0));
    let ang = knot_round(ang * (laps / TAU)).neg_mul_add(s(TAU / laps), ang);
    let (ca, sa) = (ang.cos(), ang.sin());
    let (fu, fv) = (r0 * ca, r0 * sa);
    // the strand's lean: rg * wind across for rho along
    let kl = rg * wind;
    let mut arc = rho.mul_add(rho, s(kl * kl)).sqrt();
    let iarc = F8::one() / arc;
    let (cl, sl) = ((rho * iarc).max(s(0.05)), iarc * kl);
    let mut phi = (fv * sl * cl).neg_mul_add(irho, th);
    let irt = 1.0 / rt;
    let mut u = (fu - rg) * irt;
    let mut v = fv * cl * irt;
    // the bound on |d(u, v)/d th|
    let mut m = r0 * (wind.abs() * irt);
    let mut lscale = rt;
    d = d.max(knot_tube(u, v, m, lscale, irho));
    // the other strands: the nearest, in its own frame, shortened as much as any strand leans
    let mut other = if laps > 1.5 {
        let lmax = kl / (1.0 - rg);
        let cmin = (1.0 / (1.0 + lmax * lmax).sqrt()).max(0.05);
        let (du, dv) = (fu - rg * c0, fv.abs() - rg * s0);
        knot_tube(dv.mul_add(s(s0), du * c0) * irt, dv.mul_add(s(c0), -(du * s0)) * (cmin * irt), m, lscale, irho)
    } else {
        s(1e9)
    };
    let mut trap = F8::zero();
    let mut digit = 1.0f32;
    let mut om = tw;
    let inv_f = 1.0 / f;
    for _ in 1..iters {
        // the rope: `strands` strands about the strand's axis, turning w times a lap
        let w = (om + 0.5).floor();
        let r = len2(u, v);
        let ang = phi.mul_add(s(w), v.atan2(u));
        let sec = knot_round(ang * (n / TAU));
        let ang = sec.neg_mul_add(s(TAU / n), ang);
        let (ca, sa) = (ang.cos(), ang.sin());
        let (fu, fv) = (r * ca, r * sa);
        m = r.mul_add(s(w.abs()), m) * inv_f;
        // the strands lean: w * a across (in metres) for arc along
        let kl = w * a * lscale;
        let hyp = arc.mul_add(arc, s(kl * kl)).sqrt();
        let ih = F8::one() / hyp;
        let (cl, sl) = ((arc * ih).max(s(0.05)), ih * kl);
        phi = (fv * lscale * sl * cl).neg_mul_add(F8::one() / arc, phi);
        arc = hyp;
        lscale *= f;
        u = (fu - a) * inv_f;
        v = fv * cl * inv_f;
        // the nearest sibling, in its own frame
        let (du, dv) = (fu - a * cn, fv.abs() - a * sn);
        let sib = knot_tube(dv.mul_add(s(sn), du * cn) * inv_f, dv.mul_add(s(cn), -(du * sn)) * cl * inv_f, m, lscale, irho);
        d = d.max(knot_tube(u, v, m, lscale, irho).min(other));
        other = other.min(sib);
        // (the trap: the strand's index at every level, a digit each)
        let idx = sec - (sec * (1.0 / n)).floor() * n;
        digit /= n;
        trap = idx.mul_add(s(digit), trap);
        // the next level's strands twist the other way, as a rope's do
        om = -om * inv_f;
    }
    let trap = if iters > 1 { trap } else { sa.mul_add(s(0.5), s(0.5)) };
    (d, trap.saturate())
}

const KNOT: &str = r#"
fn knot_round(x: f32) -> f32 {
    return floor(x + 0.5);
}

fn knot_tube(u: f32, v: f32, m: f32, lscale: f32, irho: f32) -> f32 {
    let x = length(vec2f(u, v)) - 1.0;
    let g = m * irho * clamp(x, 0.0, 1.0) * lscale;
    return x * lscale / sqrt(g * g + 1.0);
}

fn knot(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let laps = a.x;
    let c0 = a.y;
    let rg = a.z;
    let rt = a.w;
    let n = b.x;
    let tw = b.y;
    let f = b.z;
    let s0 = b.w;
    let wind = c.x;
    let outer = c.y;
    let cn = c.z;
    let sn = c.w;
    let tau = 6.2831853;
    let aa = 1.0 - f;
    let rho = max(length(p.xy), 1e-6);
    let irho = 1.0 / rho;
    let th = atan2(p.y, p.x);
    let u0 = rho - 1.0;
    let v0 = p.z;
    let r0 = length(vec2f(u0, v0));
    var d = r0 - outer;
    var ang = th * wind + atan2(v0, u0);
    ang = ang - knot_round(ang * (laps / tau)) * (tau / laps);
    var ca = cos(ang);
    var sa = sin(ang);
    var fu = r0 * ca;
    var fv = r0 * sa;
    let kl0 = rg * wind;
    var arc = sqrt(rho * rho + kl0 * kl0);
    let cl0 = max(rho / arc, 0.05);
    let sl0 = kl0 / arc;
    var phi = th - fv * sl0 * cl0 * irho;
    let irt = 1.0 / rt;
    var u = (fu - rg) * irt;
    var v = fv * cl0 * irt;
    var m = r0 * (abs(wind) * irt);
    var lscale = rt;
    d = max(d, knot_tube(u, v, m, lscale, irho));
    var other = 1e9;
    if (laps > 1.5) {
        let lmax = kl0 / (1.0 - rg);
        let cmin = max(1.0 / sqrt(1.0 + lmax * lmax), 0.05);
        let du = fu - rg * c0;
        let dv = abs(fv) - rg * s0;
        other = knot_tube((du * c0 + dv * s0) * irt, (dv * c0 - du * s0) * (cmin * irt), m, lscale, irho);
    }
    var trap = 0.0;
    var digit = 1.0;
    var om = tw;
    let inv_f = 1.0 / f;
    for (var i = 1u; i < iters; i++) {
        let w = floor(om + 0.5);
        let r = length(vec2f(u, v));
        var an = phi * w + atan2(v, u);
        let sec = knot_round(an * (n / tau));
        an = an - sec * (tau / n);
        ca = cos(an);
        sa = sin(an);
        fu = r * ca;
        fv = r * sa;
        m = (r * abs(w) + m) * inv_f;
        let kl = w * aa * lscale;
        let hyp = sqrt(arc * arc + kl * kl);
        let cl = max(arc / hyp, 0.05);
        let sl = kl / hyp;
        phi = phi - fv * lscale * sl * cl / arc;
        arc = hyp;
        lscale *= f;
        u = (fu - aa) * inv_f;
        v = fv * cl * inv_f;
        let du = fu - aa * cn;
        let dv = abs(fv) - aa * sn;
        let sib = knot_tube((du * cn + dv * sn) * inv_f, (dv * cn - du * sn) * cl * inv_f, m, lscale, irho);
        d = max(d, min(knot_tube(u, v, m, lscale, irho), other));
        other = min(other, sib);
        let idx = sec - floor(sec * (1.0 / n)) * n;
        digit /= n;
        trap += idx * digit;
        om = -om * inv_f;
    }
    var tr = trap;
    if (iters <= 1u) {
        tr = sa * 0.5 + 0.5;
    }
    return vec2f(d, clamp(tr, 0.0, 1.0));
}
"#;
