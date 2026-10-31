//! Four-dimensional fractals sliced in 3D: each iterates the point
//! (x, y, z, w) with w a parameter, so sliding w through a scene morphs the
//! slice. The 4D distance estimate bounds the distance within the slice too.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "menger4d",
        category: "Fractals: 4D slices",
        desc: "A slice through a four-dimensional Menger sponge: mirror folds sort its four axes, then the two largest and the smallest are pushed toward the corners and the third is folded about its middle, so the holes cross the slice at angles. Syntopia's and DarkBeam's Menger4 for Mandelbulb 3D, ported from Mandelbulber 2.",
        params: &[
            Param::num("w", &[0.0], -1.2, 1.2, "The slice's 4th coordinate, -1 to 1 (past that empty; -w matches w). 0 is a cube with ragged edges and cross-shaped holes; 0.1 to 0.2 one square tunnel through each face and fine pores; 0.33 a grid of holes; 0.45 nearly solid; 0.6 a ring of eight square holes on each face; towards 1 a rugged, crumbling block."),
            Param::num("fold_scale", &[3.0], 1.5, 4.0, "How much smaller each level's copies are: 3 is the sponge's; below it the copies overlap into solid lattices, above it they part into dust."),
            Param::num("offset", &[1.0, 1.0, 1.0, 0.5], 0.05, 1.5, "Where the copies sit along each sorted axis, largest first (the first is also the sponge's half size): lowering one pulls its copies in and fills holes; the 4th (0.5, half way) is what makes the 4D holes cross the slices."),
        ],
        iterations: (5, 10, "Levels of holes: each adds holes three times finer (more close up, on their own)."),
        bound: 1.5,
        lod: Some((3.0, 10)),
        step: 1.0,
        example: r#"{"type":"menger4d","w":0.2,"scale":0.6}"#,
        prep: Some(menger4d_prep),
        cpu: menger4d,
        wgsl: MENGER4D,
        wgsl_fn: "menger4d",
    },
    Formula {
        name: "sierpinski4d",
        category: "Fractals: 4D slices",
        desc: "A slice through a four-dimensional Sierpinski solid: six mirror folds and a halving about a corner, so eight half-size copies sit on the corners of a 16-cell. Its slices run from a cuboctahedral lattice at w = 0 to the Sierpinski tetrahedron at w = ±1. Syntopia's and DarkBeam's formula for Mandelbulb 3D, ported from Mandelbulber 2.",
        params: &[
            Param::num("w", &[0.0], -1.2, 1.2, "The slice's 4th coordinate, -1 to 1 (past that empty). 0 is a cuboctahedron of Sierpinski triangles; 0.5 four half-size cuboctahedra on a tetrahedron's corners, 0.75 sixteen quarter-size ones (1 - 1/2^k gives 4^k); between those, spiky lace; 1 the Sierpinski tetrahedron, -1 its mirror image."),
            Param::num("fold_scale", &[2.0], 1.5, 3.0, "How much smaller each level's copies are: 2 makes them touch; below it they overlap into a solid, above it they part into dust."),
        ],
        iterations: (10, 20, "Levels of copies: each halves them (more close up, on their own)."),
        bound: 1.0,
        lod: Some((2.0, 20)),
        step: 1.0,
        example: r#"{"type":"sierpinski4d","scale":0.8}"#,
        prep: None,
        cpu: sierpinski4d,
        wgsl: SIERPINSKI4D,
        wgsl_fn: "sierpinski4d",
    },
    Formula {
        name: "mandelbox4d",
        category: "Fractals: 4D slices",
        desc: "A slice through the four-dimensional Mandelbox: box folds on all four axes, a sphere fold in 4D, scale and add the point. At w = 0 it is the ordinary Mandelbox; other slices keep its cube but trade its round pits for square studs and back. Tglad's Mandelbox (2010) in 4D, from Mandelbulber 2's ABox 4D.",
        params: &[
            Param::num("w", &[0.0], -6.0, 6.0, "The slice's 4th coordinate: 0 is the ordinary Mandelbox. The slice stays a cube (out to ±2 at negative fold_scale, ±6 at 2) while its relief changes; at -2: round pits near 0, square studs by 0.75, a plain smooth cube at ±1 (w keeps the 4D radius past the sphere fold), pits and bubbles again from 1.2, then specks and nothing past ±2."),
            Param::num("fold_scale", &[-2.0], -3.0, 3.0, "The scale each step: negative values (-1.5 to -2.5) give crumpled cubes 4 across, positive ones (2 to 3) boxy cities up to 12 across; near ±1 it fattens into a blob."),
            Param::num("min_radius", &[0.5], 0.05, 1.0, "The sphere fold's inner radius: smaller values give finer, sharper detail; 1 turns the sphere fold off."),
            Param::num("fold", &[1.0, 1.0, 1.0, 1.0], 0.25, 2.0, "The box fold's limit on x, y, z and w: lower values fold more tightly and add detail; the w limit sets how the 4th axis folds in."),
        ],
        iterations: (12, 40, "Escape-time iterations: more = finer, sharper detail."),
        bound: 12.0,
        lod: None,
        step: 1.0,
        example: r#"{"type":"mandelbox4d","w":1.5,"scale":0.4}"#,
        prep: Some(mandelbox4d_prep),
        cpu: mandelbox4d,
        wgsl: MANDELBOX4D,
        wgsl_fn: "mandelbox4d",
    },
    Formula {
        name: "kleinian4d",
        category: "Fractals: 4D slices",
        desc: "A slice through a four-dimensional pseudo-Kleinian: box folds and a 4D sphere inversion, drawn around a pillar (the vertical axis) and a floor (the y = 0 plane), so the slice is a hall of floors, columns, domes and arches. TGlad's formula (fractalforums.org), ported from Mandelbulber 2.",
        params: &[
            Param::num("w", &[0.0], -2.0, 2.0, "The slice's 4th coordinate: 0 is the 3D form, a closed block of walls with round windows; 0.2 goblet-topped columns on a floor; 0.4 arches; 0.6 floating crowns; near 0.9 a bare floor with beads. It repeats every 2 x the w fold (1.8 by default): -w and w match, and so do 1.8 - w and w."),
            Param::num("fold", &[0.92436, 0.90756, 0.92436, 0.9], 0.25, 2.0, "The box fold's limit on x, y (up), z and w: half the repeating cell on each axis. Just under 1 (Knighty's classic 0.92436, 0.90756) keeps the spheres from lining up with the cells, for richer halls; 1 gives tidy, sparse ones."),
            Param::num("inversion", &[1.5], 0.25, 2.0, "The sphere inversion's radius²: larger fills the halls with walls, domes and round windows (and keeps more of it away from w = 0); smaller leaves bare floors and beads."),
            Param::num("radius", &[0.1], 0.0, 0.5, "The columns' radius (round the vertical axis, in the folded space): 0 leaves only the floors and domes."),
            Param::num("thickness", &[0.02], 0.0, 0.2, "Thickens the plates and columns alike."),
        ],
        iterations: (10, 30, "Fold and inversion steps: more = finer plates and columns."),
        bound: f32::INFINITY,
        lod: None,
        step: 1.0,
        example: r#"{"type":"intersection","scale":0.5,"children":[{"type":"box","size":[4,3,4]},{"type":"kleinian4d","w":0.3,"iterations":10}]}"#,
        prep: None,
        cpu: kleinian4d,
        wgsl: KLEINIAN4D,
        wgsl_fn: "kleinian4d",
    },
];

// ---- menger4d ---------------------------------------------------------------

/// k: w, fold scale, offset[4]; adds in k[6..11] the x, y, w offsets times
/// (scale - 1), the z fold's centre, and the box's half size (the first
/// offset: past it the largest axis only grows).
fn menger4d_prep(k: &mut [f32; K]) {
    let sc = k[1];
    let m = sc - 1.0;
    k[6] = k[2] * m;
    k[7] = k[3] * m;
    k[8] = 0.5 * k[4] * m / sc;
    k[9] = k[5] * m;
    k[10] = k[2];
}

/// Mandelbulber's Menger 4D: fold into the positive orthant, sort the axes
/// largest first, scale the largest two and the smallest away from the
/// offset and fold the third about its centre; the distance to the box,
/// scaled back. Orbit trap: how often w sorted above x or z (Mandelbulber's
/// fold colour), per level.
fn menger4d(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let sc = k[1];
    let (ox, oy, h, ow, m) = (k[6], k[7], k[8], k[9], k[10]);
    let (mut x, mut y, mut z, mut w) = (p.x, p.y, p.z, s(k[0]));
    let mut scale = 1.0f32;
    let mut swaps = F8::zero();
    for _ in 0..iters {
        (x, y, z, w) = (x.abs(), y.abs(), z.abs(), w.abs());
        // largest first: x >= y >= z >= w
        (x, y) = (x.max(y), x.min(y));
        (x, z) = (x.max(z), x.min(z));
        (y, z) = (y.max(z), y.min(z));
        swaps += F8::one().and(x.lt(w));
        (x, w) = (x.max(w), x.min(w));
        (y, w) = (y.max(w), y.min(w));
        swaps += F8::one().and(z.lt(w));
        (z, w) = (z.max(w), z.min(w));
        x = x * sc - ox;
        y = y * sc - oy;
        w = w * sc - ow;
        z = (s(h) - (z - h).abs()) * sc;
        scale *= sc;
    }
    let (qx, qy, qz, qw) = (x.abs() - m, y.abs() - m, z.abs() - m, w.abs() - m);
    let zero = F8::zero();
    let out = dot4(qx.max(zero), qy.max(zero), qz.max(zero), qw.max(zero)).sqrt();
    let d = out + qx.max(qy).max(qz.max(qw)).min(zero);
    (d * (1.0 / scale), (swaps * (1.0 / iters.max(1) as f32)).saturate())
}

const MENGER4D: &str = r#"
fn menger4d(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let sc = a.y;
    let ox = b.z;
    let oy = b.w;
    let h = c.x;
    let ow = c.y;
    let m = c.z;
    var z = vec4f(p, a.x);
    var scale = 1.0;
    var swaps = 0.0;
    for (var i = 0u; i < iters; i++) {
        z = abs(z);
        var t = z.x;
        z.x = max(t, z.y);
        z.y = min(t, z.y);
        t = z.x;
        z.x = max(t, z.z);
        z.z = min(t, z.z);
        t = z.y;
        z.y = max(t, z.z);
        z.z = min(t, z.z);
        t = z.x;
        swaps += select(0.0, 1.0, t < z.w);
        z.x = max(t, z.w);
        z.w = min(t, z.w);
        t = z.y;
        z.y = max(t, z.w);
        z.w = min(t, z.w);
        t = z.z;
        swaps += select(0.0, 1.0, t < z.w);
        z.z = max(t, z.w);
        z.w = min(t, z.w);
        z.x = z.x * sc - ox;
        z.y = z.y * sc - oy;
        z.w = z.w * sc - ow;
        z.z = (h - abs(z.z - h)) * sc;
        scale *= sc;
    }
    let q = abs(z) - vec4f(m);
    let d = length(max(q, vec4f(0.0))) + min(max(max(q.x, q.y), max(q.z, q.w)), 0.0);
    return vec2f(d * (1.0 / scale), clamp(swaps * (1.0 / f32(max(iters, 1u))), 0.0, 1.0));
}
"#;

// ---- sierpinski4d -----------------------------------------------------------

/// Mandelbulber's Sierpinski 4D: six folds (x_i + x_j < 0: reflect), scale
/// about the corner (1, 1, 1, 1); the distance to the 16-cell on the corners
/// with an even count of -1 (|x_i| <= 1, |sum - 2 x_i| <= 2), scaled back.
/// Orbit trap: the share of the folds that reflected the point.
fn sierpinski4d(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let sc = k[1];
    let off = sc - 1.0;
    let (mut x, mut y, mut z, mut w) = (p.x, p.y, p.z, s(k[0]));
    let zero = F8::zero();
    let mut folds = F8::zero();
    let mut scale = 1.0f32;
    // a + b < 0: (a, b) = (-b, -a)
    let fold = |a: F8, b: F8, folds: &mut F8| -> (F8, F8) {
        let m: M8 = (a + b).lt(zero);
        *folds += F8::one().and(m);
        (F8::select(m, -b, a), F8::select(m, -a, b))
    };
    for _ in 0..iters {
        (x, y) = fold(x, y, &mut folds);
        (x, z) = fold(x, z, &mut folds);
        (y, z) = fold(y, z, &mut folds);
        (x, w) = fold(x, w, &mut folds);
        (y, w) = fold(y, w, &mut folds);
        (z, w) = fold(z, w, &mut folds);
        x = x * sc - off;
        y = y * sc - off;
        z = z * sc - off;
        w = w * sc - off;
        scale *= sc;
    }
    let t = x + y + z + w;
    let faces = x.abs().max(y.abs()).max(z.abs().max(w.abs())) - 1.0;
    let corners = (t - x * 2.0).abs().max((t - y * 2.0).abs()).max((t - z * 2.0).abs().max((t - w * 2.0).abs())) * 0.5 - 1.0;
    (faces.max(corners) * (1.0 / scale), folds * (1.0 / (6 * iters.max(1)) as f32))
}

const SIERPINSKI4D: &str = r#"
fn sierpinski4d(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let sc = a.y;
    let off = sc - 1.0;
    var z = vec4f(p, a.x);
    var folds = 0.0;
    var scale = 1.0;
    for (var i = 0u; i < iters; i++) {
        if (z.x + z.y < 0.0) {
            z = vec4f(-z.y, -z.x, z.z, z.w);
            folds += 1.0;
        }
        if (z.x + z.z < 0.0) {
            z = vec4f(-z.z, z.y, -z.x, z.w);
            folds += 1.0;
        }
        if (z.y + z.z < 0.0) {
            z = vec4f(z.x, -z.z, -z.y, z.w);
            folds += 1.0;
        }
        if (z.x + z.w < 0.0) {
            z = vec4f(-z.w, z.y, z.z, -z.x);
            folds += 1.0;
        }
        if (z.y + z.w < 0.0) {
            z = vec4f(z.x, -z.w, z.z, -z.y);
            folds += 1.0;
        }
        if (z.z + z.w < 0.0) {
            z = vec4f(z.x, z.y, -z.w, -z.z);
            folds += 1.0;
        }
        z = z * sc - vec4f(off);
        scale *= sc;
    }
    let t = z.x + z.y + z.z + z.w;
    let az = abs(z);
    let faces = max(max(az.x, az.y), max(az.z, az.w)) - 1.0;
    let e = abs(vec4f(t) - z * 2.0);
    let corners = max(max(e.x, e.y), max(e.z, e.w)) * 0.5 - 1.0;
    return vec2f(max(faces, corners) * (1.0 / scale), folds * (1.0 / f32(6u * max(iters, 1u))));
}
"#;

// ---- mandelbox4d ------------------------------------------------------------

/// k: w, fold scale, min radius, fold[4]; to: the min radius², |scale|.
fn mandelbox4d_prep(k: &mut [f32; K]) {
    k[7] = k[2] * k[2];
    k[8] = k[1].abs();
}

/// Mandelbulber's ABox 4D: box fold each axis at `fold`, sphere fold in 4D
/// (by 1 / min² inside min, 1 / r² up to 1), scale, add (p, w); escapes past
/// 1e5 (Mandelbulber's 100 leaves the estimate jumping between escape bands
/// far out). The distance |z| / |dz|. Orbit trap: the nearest the folded orbit
/// comes to the centre.
fn mandelbox4d(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (sc, fx, fy, fz, fw, min2, asc) = (k[1], k[3], k[4], k[5], k[6], k[7], k[8]);
    let cw = s(k[0]);
    let (mut x, mut y, mut z, mut w) = (p.x, p.y, p.z, cw);
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut r = dot4(x, y, z, w).sqrt();
    let mut act: M8 = r.lt(s(1e5));
    let fold = |c: F8, f: f32| (c + f).abs() - (c - f).abs() - c;
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (nx, ny, nz, nw) = (fold(x, fx), fold(y, fy), fold(z, fz), fold(w, fw));
        let r2 = dot4(nx, ny, nz, nw);
        // r2 < min2: by 1 / min2; min2 <= r2 < 1: by 1 / r2
        let kk = F8::select(r2.lt(s(min2)), s(1.0 / min2), F8::one() / r2.max(s(1e-12)));
        let kk = F8::select(r2.lt(F8::one()), kk, F8::one());
        let ks = kk * sc;
        x = F8::select(act, nx * ks + p.x, x);
        y = F8::select(act, ny * ks + p.y, y);
        z = F8::select(act, nz * ks + p.z, z);
        w = F8::select(act, nw * ks + cw, w);
        dr = F8::select(act, (dr * kk).mul_add(s(asc), F8::one()), dr);
        trap = F8::select(act, trap.min(r2), trap);
        r = F8::select(act, dot4(x, y, z, w).sqrt(), r);
        act &= r.lt(s(1e5));
    }
    (r / dr.abs().max(s(1e-12)), trap.sqrt().saturate())
}

const MANDELBOX4D: &str = r#"
fn mandelbox4d(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let sc = a.y;
    let f = vec4f(a.w, b.x, b.y, b.z);
    let min2 = b.w;
    let asc = c.x;
    let pc = vec4f(p, a.x);
    var z = pc;
    var dr = 1.0;
    var trap = 1e9;
    var r = length(z);
    for (var i = 0u; i < iters; i++) {
        if (r >= 1e5) {
            break;
        }
        z = abs(z + f) - abs(z - f) - z;
        let r2 = dot(z, z);
        var kk = 1.0;
        if (r2 < min2) {
            kk = 1.0 / min2;
        } else if (r2 < 1.0) {
            kk = 1.0 / max(r2, 1e-12);
        }
        z = z * (kk * sc) + pc;
        dr = dr * kk * asc + 1.0;
        trap = min(trap, r2);
        r = length(z);
    }
    return vec2f(r / max(abs(dr), 1e-12), clamp(sqrt(trap), 0.0, 1.0));
}
"#;

// ---- kleinian4d -------------------------------------------------------------

/// TGlad's 4D pseudo-Kleinian (Mandelbulber's Pseudo Kleinian 4D): box fold
/// each axis at `fold`, invert in the sphere of radius² `inversion` (never
/// shrinking); the distance to the y axis (less `radius`) or the y = 0 plane,
/// less `thickness`, scaled back, halved as Mandelbulber does. Orbit trap:
/// the nearest the folded point comes to the centre.
fn kleinian4d(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (fx, fy, fz, fw, inv, radius, thick) = (k[1], k[2], k[3], k[4], k[5], k[6], k[7]);
    let (mut x, mut y, mut z, mut w) = (p.x, p.y, p.z, s(k[0]));
    let mut de = F8::one();
    let mut orb = s(1e9);
    let fold = |c: F8, f: f32| (c + f).abs() - (c - f).abs() - c;
    for _ in 0..iters {
        (x, y, z, w) = (fold(x, fx), fold(y, fy), fold(z, fz), fold(w, fw));
        let r2 = dot4(x, y, z, w).max(s(1e-12));
        orb = orb.min(r2);
        let kk = (s(inv) / r2).max(F8::one());
        (x, y, z, w) = (x * kk, y * kk, z * kk, w * kk);
        de *= kk;
    }
    // (Mandelbulber's z is up: here y)
    let d1 = w.mul_add(w, z.mul_add(z, x * x)).sqrt() - radius;
    let d2 = y.abs();
    ((d1.min(d2) - thick) * 0.5 / de, orb.sqrt().saturate())
}

const KLEINIAN4D: &str = r#"
fn kleinian4d(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let f = vec4f(a.y, a.z, a.w, b.x);
    let inv = b.y;
    var z = vec4f(p, a.x);
    var de = 1.0;
    var orb = 1e9;
    for (var i = 0u; i < iters; i++) {
        z = abs(z + f) - abs(z - f) - z;
        let r2 = max(dot(z, z), 1e-12);
        orb = min(orb, r2);
        let kk = max(inv / r2, 1.0);
        z = z * kk;
        de *= kk;
    }
    let d1 = sqrt(z.x * z.x + z.z * z.z + z.w * z.w) - b.z;
    let d2 = abs(z.y);
    return vec2f((min(d1, d2) - b.w) * 0.5 / de, clamp(sqrt(orb), 0.0, 1.0));
}
"#;

/// x² + y² + z² + w².
#[inline(always)]
fn dot4(x: F8, y: F8, z: F8, w: F8) -> F8 {
    w.mul_add(w, z.mul_add(z, y.mul_add(y, x * x)))
}
