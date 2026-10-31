//! Kleinian groups: limit sets of groups of sphere inversions and Möbius
//! maps (true Kleinian, Schottky) and a variant of Knighty's pseudo-Kleinian.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "jos_kleinian",
        category: "Fractals: spheres and tori",
        desc: "A true Kleinian group's limit set: Jos Leys's escape-time algorithm for Maskit's slice with Knighty's distance estimate (Fractal Forums, 2016-17; ported from Mandelbulber 2). Floors of great domes with necklaces of ever smaller spheres strung up between them into columns, layer upon layer. It fills all of space, so it is cut to a box here; fly inside it.",
        params: &[
            Param::num("klein", &[1.9, 0.0], -3.0, 3.0, "The group's parameter (Jos Leys's KleinR and KleinI; Maskit's t = -b + ai). a is the height of a layer (1.8 to 2.1, at least 0.5): below 2 the necklaces open into columns of beads, at 2 they pinch to points. b (-0.2 to 0.2) curls them into seahorse tails, though away from 0 the estimate is rougher."),
            Param::num("box", &[1.0, 1.0, 1.0], 0.2, 4.0, "Half sizes of the cell each step wraps the point into, along x, y and z: 1 is the group's own period, and other values cut and repeat it. y also spaces the layers: 5 or more leaves one layer between a solid floor and ceiling."),
            Param::whole("spheres", 1.0, 0.0, 1.0, "1 draws the solid spheres on both faces of each layer (the classic look); 0 only the floor's images, a finer and dustier lattice."),
            Param::num("clamp", &[0.2, 1.0], 0.001, 4.0, "Clamps on the distance estimate: the most one step may be (0.2), and the least the maps' scale may count as (1). Smaller is safer and slower; neither changes the shape."),
            Param::num("inversion_centre", &[0.0, 1.0, 0.0], -4.0, 4.0, "The centre of a sphere inversion applied before anything else (when inversion_radius is above 0). About a point inside a layer it bends the stacked layers into a tower of nested bubbles; with a single layer (box y 5 or more), spheres 0 and the centre above the layer, it folds the whole infinite layer into one bounded cluster."),
            Param::num("inversion_radius", &[0.0], 0.0, 4.0, "That inversion's radius (0 to 4): 0 turns it off."),
        ],
        iterations: (20, 64, "Steps of the group's maps: more = smaller spheres in the necklaces, linear cost."),
        bound: f32::INFINITY,
        lod: None,
        step: 0.8,
        example: r#"{"type":"intersection","scale":0.4,"children":[{"type":"box","size":[6,1.5,6]},{"type":"jos_kleinian","position":[0,-0.55,0]}]}"#,
        prep: Some(jos_kleinian_prep),
        cpu: jos_kleinian,
        wgsl: JOS_KLEINIAN,
        wgsl_fn: "jos_kleinian",
    },
    Formula {
        name: "schottky",
        category: "Fractals: spheres and tori",
        desc: "A Schottky group's limit set in 3D, in the manner of Indra's Pearls (Mumford, Series and Wright): kissing spheres at the corners of a regular solid, each holding the others' images under inversion in it, and so on down, so that necklaces of pearls within pearls gather on the limit set. Its own construction, with a strict distance bound carried back through the inversions.",
        params: &[
            Param::whole("layout", 3.0, 0.0, 3.0, "Where the spheres sit: 0 the corners of a tetrahedron (4), 1 an octahedron (6), 2 a cube (8), 3 an icosahedron (12)."),
            Param::num("radius", &[1.0], 0.3, 1.0, "The spheres' radius as a part of the radius at which neighbours kiss (0.3 to 1): at 1 the pearls string into necklaces; below it they part, and the set breaks into separate snowflake clusters, a Cantor dust."),
            Param::num("centre", &[0.0], 0.0, 1.0, "A sphere in the middle, as a part of the size at which it kisses all the others (0 to 1): 0 leaves it out; its images thicken the web between the necklaces."),
        ],
        iterations: (4, 24, "Levels of inversion: each puts the spheres' images inside every pearl, so more = smaller pearls (past 5 they are finer than the pixels from afar), at a pass over the spheres each."),
        bound: 1.85,
        lod: None,
        step: 0.9,
        example: r#"{"type":"schottky","scale":1.2}"#,
        prep: None,
        cpu: schottky,
        wgsl: SCHOTTKY,
        wgsl_fn: "schottky",
    },
    Formula {
        name: "kleinian_mod",
        category: "Fractals: spheres and tori",
        desc: "Pseudo-Kleinian Mod 3 (mclarekin, Mandelbulber 2, 2020), a variant of Knighty and Theli-at's pseudo-Kleinian: the same box folds and inversions, but measured to a floor plate and struts along the axes rather than to rings, so it grows into storeys of domed floors held up by columns of beads, an endless arcade. It fills all of space, so it is cut to a box here; fly inside it.",
        params: &[
            Param::num("fold_box", &[0.92436, 0.90756, 0.92436], 0.1, 2.0, "Half sizes of the box fold along x, y and z (0.7 to 1.1): smaller pinches the columns to cusps, larger widens the halls."),
            Param::num("inversion", &[1.0], 0.1, 4.0, "The inversion sphere's radius squared (0.5 to 1.5): larger swells the beads and domes."),
            Param::num("box_offset", &[0.0, 0.0, 0.0], -1.0, 1.0, "Pushes each coordinate away from the centre before the fold (0 to 0.1): the columns melt into ribbed trunks; much more fills the space solid."),
            Param::num("offset", &[0.0, 0.0, 0.0], -1.0, 1.0, "Added after each inversion (-0.1 to 0.1): warps and melts the arcades."),
            Param::num("strut", &[0.0], 0.0, 0.6, "The radius of the struts along the axes (0 to 0.6): they hide in the columns until about 0.4, then swell into drums and discs."),
            Param::num("plate", &[0.0], 0.0, 0.5, "The floor plate's half thickness (0 to 0.2): thicker facets the domes like umbrellas."),
        ],
        iterations: (8, 20, "Fold steps: more = finer detail, linear cost."),
        bound: f32::INFINITY,
        lod: None,
        step: 0.8,
        example: r#"{"type":"intersection","scale":0.5,"children":[{"type":"box","size":[4,4,4]},{"type":"kleinian_mod"}]}"#,
        prep: None,
        cpu: kleinian_mod,
        wgsl: KLEINIAN_MOD,
        wgsl_fn: "kleinian_mod",
    },
];

// ---- jos_kleinian ----------------------------------------------------------

/// A layer at least half a unit high (the wrap divides by it), and the first
/// inversion's radius, squared.
fn jos_kleinian_prep(k: &mut [f32; K]) {
    k[0] = k[0].max(0.5);
    k[11] *= k[11];
}

/// x wrapped into [lo, lo + w) (Mandelbulber's wrap).
#[inline(always)]
fn jos_wrap(c: F8, w: f32, lo: f32) -> F8 {
    let t = c - lo;
    t - (t / w).floor() * w + lo
}

/// Jos Leys's Kleinian (Mandelbulber's jos_kleinian.cl): wrap into the
/// group's cell, turn the part above the separation line half round about
/// (-b/2, a/2), apply the generator z -> t + 1/z (inversion and turn);
/// Knighty's estimate, the height above the floor (or below the ceiling)
/// over the maps' scale, clamped. Unlike Mandelbulber it stops at a 2-cycle
/// (as Jos Leys's original does) and takes the smaller of the last two
/// steps' estimates. Orbit trap: the nearest the wrapped orbit comes to the
/// centre (as Mandelbulber colours it).
fn jos_kleinian(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (a, b) = (k[0], k[1]);
    let bx = [k[2], k[3], k[4]];
    let both = k[5] > 0.5;
    let (cy, cdf) = (k[6], k[7]);
    let f = if b > 0.0 {
        1.0
    } else if b < 0.0 {
        -1.0
    } else {
        0.0
    };
    let sk = f * std::f32::consts::PI / bx[0];
    let (wx, wy, wz) = (2.0 * bx[0], a * bx[1], 2.0 * bx[2]);
    let (lx, ly, lz) = (-bx[0], 1.0 - bx[1], -bx[2]);
    let mut z = p;
    let mut df = F8::one();
    if k[11] > 0.0 {
        let c = V8::new(s(k[8]), s(k[9]), s(k[10]));
        let q = z - c;
        let m = s(k[11]) / q.length_sq().max(s(1e-12));
        z = q * m + c;
        df = m;
    }
    let mut trap = s(1e9);
    let mut act = M8::all_true();
    // the two iterates before (a 2-cycle ends it, as in Jos Leys's original),
    // and the estimates at the last two: the cycle's two phases differ, and
    // the smaller of them keeps neighbouring points that stop in different
    // phases from disagreeing
    let (mut z1, mut z2) = (z + V8::one(), z - V8::one());
    let (mut cur, mut prev) = (s(1e9), s(1e9));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (mut x, mut y, mut w) = (jos_wrap(z.x, wx, lx), jos_wrap(z.y, wy, ly), jos_wrap(z.z, wz, lz));
        let line = ((x + b * 0.5) * sk).sin().mul_add(s(0.2 * a), s(0.5 * a));
        let up = y.ge(line);
        x = F8::select(up, -b - x, x);
        y = F8::select(up, a - y, y);
        w = F8::select(up, -w, w);
        let rr = w.mul_add(w, y.mul_add(y, x * x));
        trap = F8::select(act, trap.min(rr.mul_add(rr, rr).sqrt()), trap);
        let ir = F8::one() / rr.max(s(1e-12));
        let nz = V8::new(x * ir - b, a - y * ir, -w * ir);
        z = V8::select(act, nz, z);
        df = F8::select(act, df * ir, df);
        let h = if both { z.y.min(a - z.y) } else { z.y };
        let de = h.min(s(cy)) / df.max(s(cdf));
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, de, cur);
        act &= z.length_sq().lt(s(100.0)) & (z - z2).length_sq().ge(s(1e-5));
        z2 = z1;
        z1 = z;
    }
    (cur.min(prev), (trap * 0.6).saturate())
}

const JOS_KLEINIAN: &str = r#"
fn jos_kleinian_wrap(c: f32, w: f32, lo: f32) -> f32 {
    let t = c - lo;
    return t - floor(t / w) * w + lo;
}

fn jos_kleinian(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let ka = a.x;
    let kb = a.y;
    let bx = vec3f(a.z, a.w, b.x);
    let both = b.y > 0.5;
    let cy = b.z;
    let cdf = b.w;
    let f = select(select(0.0, -1.0, kb < 0.0), 1.0, kb > 0.0);
    let sk = f * 3.14159265 / bx.x;
    let wv = vec3f(2.0 * bx.x, ka * bx.y, 2.0 * bx.z);
    let lv = vec3f(-bx.x, 1.0 - bx.y, -bx.z);
    var z = p;
    var df = 1.0;
    if (c.w > 0.0) {
        let q = z - c.xyz;
        let m = c.w / max(dot(q, q), 1e-12);
        z = q * m + c.xyz;
        df = m;
    }
    var trap = 1e9;
    var z1 = z + vec3f(1.0);
    var z2 = z - vec3f(1.0);
    var cur = 1e9;
    var prev = 1e9;
    for (var i = 0u; i < iters; i++) {
        var x = jos_kleinian_wrap(z.x, wv.x, lv.x);
        var y = jos_kleinian_wrap(z.y, wv.y, lv.y);
        var w = jos_kleinian_wrap(z.z, wv.z, lv.z);
        let line = sin((x + kb * 0.5) * sk) * (0.2 * ka) + 0.5 * ka;
        if (y >= line) {
            x = -kb - x;
            y = ka - y;
            w = -w;
        }
        let rr = x * x + y * y + w * w;
        trap = min(trap, sqrt(rr * rr + rr));
        let ir = 1.0 / max(rr, 1e-12);
        z = vec3f(x * ir - kb, ka - y * ir, -w * ir);
        df *= ir;
        var h = z.y;
        if (both) {
            h = min(z.y, ka - z.y);
        }
        prev = cur;
        cur = min(h, cy) / max(df, cdf);
        if (dot(z, z) >= 100.0 || dot(z - z2, z - z2) < 1e-5) {
            break;
        }
        z2 = z1;
        z1 = z;
    }
    return vec2f(min(cur, prev), clamp(trap * 0.6, 0.0, 1.0));
}
"#;

// ---- schottky --------------------------------------------------------------

/// The spheres' centres, a unit from the middle: the corners of a
/// tetrahedron, an octahedron, a cube and an icosahedron, one after another.
const SCHOTTKY_CORNERS: [[f32; 3]; 30] = {
    const T: f32 = 0.577_350_3;
    const A: f32 = 0.525_731_1;
    const B: f32 = 0.850_650_8;
    [
        [T, T, T],
        [T, -T, -T],
        [-T, T, -T],
        [-T, -T, T],
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
        [T, T, T],
        [-T, T, T],
        [T, -T, T],
        [-T, -T, T],
        [T, T, -T],
        [-T, T, -T],
        [T, -T, -T],
        [-T, -T, -T],
        [0.0, A, B],
        [0.0, -A, B],
        [0.0, A, -B],
        [0.0, -A, -B],
        [A, B, 0.0],
        [-A, B, 0.0],
        [A, -B, 0.0],
        [-A, -B, 0.0],
        [B, 0.0, A],
        [B, 0.0, -A],
        [-B, 0.0, A],
        [-B, 0.0, -A],
    ]
};

/// Each layout's first corner in SCHOTTKY_CORNERS, its count, and the radius
/// at which neighbouring spheres kiss (half the solid's edge).
const SCHOTTKY_LAYOUTS: [(usize, usize, f32); 4] = [(0, 4, 0.816_496_6), (4, 6, core::f32::consts::FRAC_1_SQRT_2), (10, 8, 0.577_350_3), (18, 12, 0.525_731_1)];

/// Inversions in spheres at the corners of a regular solid (and, if it is
/// on, one in the middle). What is drawn is the spheres `iters` levels down:
/// each step inverts the point in the sphere nearest it other than the last
/// one, whose inside then holds the rest of the target, and the target in
/// the other spheres is no nearer than the second nearest. Distances are
/// carried back to the start as strict lower bounds, accumulated as 1/d0 <=
/// a/d + b: an inversion of radius R about c, the point a distance q from c
/// before it, takes a distance d after it to at least R²d/(r(r + d)) with r =
/// R²/q, or near the centre (where that bound vanishes) to at least dq/M,
/// all the spheres lying within M of c. Orbit trap: how much the steps that
/// began inside a sphere magnified, i.e. how small the pearl is.
fn schottky(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (first, n, kiss) = SCHOTTKY_LAYOUTS[(k[0].round() as usize).min(3)];
    let corners = &SCHOTTKY_CORNERS[first..first + n];
    let r = k[1] * kiss;
    let rc = k[2] * (1.0 - r);
    let centre = rc > 0.0;
    let big = s(1e9);
    let mut z = p;
    let (mut a, mut b) = (F8::one(), F8::zero());
    let mut best = big;
    let mut last = s(-1.0);
    let mut mag = F8::one();
    for _ in 0..iters {
        // the nearest sphere other than the last, and the second nearest
        let (mut d1, mut d2) = (big, big);
        let (mut i1, mut c1, mut r1) = (s(-1.0), V8::zero(), s(r));
        for (j, v) in corners.iter().enumerate() {
            let dj = F8::select(last.eq(s(j as f32)), big, (z - V8::new(s(v[0]), s(v[1]), s(v[2]))).length() - r);
            let m = dj.lt(d1);
            d2 = F8::select(m, d1, d2.min(dj));
            d1 = F8::select(m, dj, d1);
            i1 = F8::select(m, s(j as f32), i1);
            c1 = V8::select(m, V8::new(s(v[0]), s(v[1]), s(v[2])), c1);
        }
        if centre {
            let dj = F8::select(last.eq(s(n as f32)), big, z.length() - rc);
            let m = dj.lt(d1);
            d2 = F8::select(m, d1, d2.min(dj));
            d1 = F8::select(m, dj, d1);
            i1 = F8::select(m, s(n as f32), i1);
            c1 = V8::select(m, V8::zero(), c1);
            r1 = F8::select(m, s(rc), r1);
        }
        let d2 = d2.max(s(1e-12));
        best = best.min(d2 / d2.mul_add(b, a));
        let q2 = (z - c1).length_sq().max(s(1e-12));
        let q = q2.sqrt();
        let rr = r1 * r1;
        let f = rr / q2;
        // (all the spheres lie within M = |c| + 1 + r of c)
        let mm = F8::select(i1.eq(s(n as f32)), s(1.0 + r), s(2.0 + r));
        let near = q.lt(rr * 2.0 / mm);
        b = a.mul_add(F8::select(near, F8::zero(), F8::one() / q), b);
        a *= F8::select(near, mm / q, f);
        mag = F8::select(d1.lt(F8::zero()), mag * f, mag);
        z = (z - c1) * f + c1;
        last = i1;
    }
    // the rest of the target lies in the spheres other than the last
    let mut d = big;
    for (j, v) in corners.iter().enumerate() {
        d = d.min(F8::select(last.eq(s(j as f32)), big, (z - V8::new(s(v[0]), s(v[1]), s(v[2]))).length() - r));
    }
    if centre {
        d = d.min(F8::select(last.eq(s(n as f32)), big, z.length() - rc));
    }
    let de = F8::select(d.lt(F8::zero()), d / a, (d / d.mul_add(b, a)).min(best));
    (de, (mag.ln() * 0.1).saturate())
}

const SCHOTTKY: &str = r#"
fn schottky_corner(l: u32, i: u32) -> vec3f {
    let t = 0.5773503;
    if (l == 0u) {
        let x = select(t, -t, i >= 2u);
        let y = select(t, -t, (i & 1u) == 1u);
        return vec3f(x, y, select(t, -t, (x < 0.0) != (y < 0.0)));
    }
    if (l == 1u) {
        let sg = select(1.0, -1.0, (i & 1u) == 1u);
        return vec3f(select(0.0, sg, i < 2u), select(0.0, sg, i == 2u || i == 3u), select(0.0, sg, i >= 4u));
    }
    if (l == 2u) {
        return vec3f(select(t, -t, (i & 1u) == 1u), select(t, -t, (i & 2u) == 2u), select(t, -t, (i & 4u) == 4u));
    }
    let s1 = select(1.0, -1.0, (i & 1u) == 1u);
    let s2 = select(1.0, -1.0, (i & 2u) == 2u);
    let a = 0.5257311;
    let b = 0.8506508;
    if (i < 4u) {
        return vec3f(0.0, a * s1, b * s2);
    }
    if (i < 8u) {
        return vec3f(a * s1, b * s2, 0.0);
    }
    return vec3f(b * s2, 0.0, a * s1);
}

fn schottky(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let l = min(u32(round(a.x)), 3u);
    var n = 12u;
    var kiss = 0.5257311;
    if (l == 0u) {
        n = 4u;
        kiss = 0.8164966;
    } else if (l == 1u) {
        n = 6u;
        kiss = 0.7071068;
    } else if (l == 2u) {
        n = 8u;
        kiss = 0.5773503;
    }
    let r = a.y * kiss;
    let rc = a.z * (1.0 - r);
    let centre = rc > 0.0;
    var z = p;
    var ca = 1.0;
    var cb = 0.0;
    var best = 1e9;
    var last = 99u;
    var mag = 1.0;
    for (var it = 0u; it < iters; it++) {
        var d1 = 1e9;
        var d2 = 1e9;
        var i1 = 99u;
        var c1 = vec3f(0.0);
        var r1 = r;
        for (var j = 0u; j < n; j++) {
            let v = schottky_corner(l, j);
            let dj = select(length(z - v) - r, 1e9, last == j);
            if (dj < d1) {
                d2 = d1;
                d1 = dj;
                i1 = j;
                c1 = v;
            } else {
                d2 = min(d2, dj);
            }
        }
        if (centre) {
            let dj = select(length(z) - rc, 1e9, last == n);
            if (dj < d1) {
                d2 = d1;
                d1 = dj;
                i1 = n;
                c1 = vec3f(0.0);
                r1 = rc;
            } else {
                d2 = min(d2, dj);
            }
        }
        d2 = max(d2, 1e-12);
        best = min(best, d2 / (d2 * cb + ca));
        let q2 = max(dot(z - c1, z - c1), 1e-12);
        let q = sqrt(q2);
        let rr = r1 * r1;
        let f = rr / q2;
        let mm = select(2.0 + r, 1.0 + r, i1 == n);
        let near = q < rr * 2.0 / mm;
        cb = ca * select(1.0 / q, 0.0, near) + cb;
        ca *= select(f, mm / q, near);
        if (d1 < 0.0) {
            mag *= f;
        }
        z = (z - c1) * f + c1;
        last = i1;
    }
    var d = 1e9;
    for (var j = 0u; j < n; j++) {
        let v = schottky_corner(l, j);
        d = min(d, select(length(z - v) - r, 1e9, last == j));
    }
    if (centre) {
        d = min(d, select(length(z) - rc, 1e9, last == n));
    }
    var de = min(d / (d * cb + ca), best);
    if (d < 0.0) {
        de = d / ca;
    }
    return vec2f(de, clamp(log(mag) * 0.1, 0.0, 1.0));
}
"#;

// ---- kleinian_mod ----------------------------------------------------------

/// Pseudo-Kleinian Mod 3 (Mandelbulber's pseudo_kleinian_mod3.cl): push
/// each coordinate out by the box offset, box fold into `fold_box`, invert
/// inside radius² `inversion`, add the offset; the distance (halved, as
/// there) to struts along the three axes and a plate, scaled back. The plate
/// lies across y here, not z: Mandelbulber's z is up, ray's y, and across y
/// it is a floor. Orbit trap: the nearest the folded point comes to the
/// centre.
fn kleinian_mod(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let cb = [k[0], k[1], k[2]];
    let size = k[3];
    let bo = [k[4], k[5], k[6]];
    let off = [k[7], k[8], k[9]];
    let (strut, plate) = (k[10], k[11]);
    let mut z = p;
    let mut de = F8::one();
    let mut orb = s(1e9);
    let push = |c: F8, o: f32| c - F8::splat(o).copysign(c);
    let fold = |c: F8, b: f32| c.clamp(-b, b) * 2.0 - c;
    for _ in 0..iters {
        z = V8::new(push(z.x, bo[0]), push(z.y, bo[1]), push(z.z, bo[2]));
        z = V8::new(fold(z.x, cb[0]), fold(z.y, cb[1]), fold(z.z, cb[2]));
        let r2 = z.length_sq().max(s(1e-12));
        orb = orb.min(r2);
        let m = (s(size) / r2).max(F8::one());
        z = V8::new(z.x * m + off[0], z.y * m + off[1], z.z * m + off[2]);
        de *= m;
    }
    let (xx, yy, zz) = (z.x * z.x, z.y * z.y, z.z * z.z);
    let d1 = (xx + yy).min(yy + zz).min(zz + xx).sqrt() - strut;
    let d2 = z.y.abs() - plate;
    (d1.min(d2) * 0.5 / de, (orb.sqrt() * 0.7).saturate())
}

const KLEINIAN_MOD: &str = r#"
fn kleinian_mod_push(c: f32, o: f32) -> f32 {
    return c - select(o, -o, c < 0.0);
}

fn kleinian_mod(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let cb = a.xyz;
    let size = a.w;
    let bo = b.xyz;
    let off = vec3f(b.w, c.x, c.y);
    let strut = c.z;
    let plate = c.w;
    var z = p;
    var de = 1.0;
    var orb = 1e9;
    for (var i = 0u; i < iters; i++) {
        z = vec3f(kleinian_mod_push(z.x, bo.x), kleinian_mod_push(z.y, bo.y), kleinian_mod_push(z.z, bo.z));
        z = clamp(z, -cb, cb) * 2.0 - z;
        let r2 = max(dot(z, z), 1e-12);
        orb = min(orb, r2);
        let m = max(size / r2, 1.0);
        z = z * m + off;
        de *= m;
    }
    let zz = z * z;
    let d1 = sqrt(min(min(zz.x + zz.y, zz.y + zz.z), zz.z + zz.x)) - strut;
    let d2 = abs(z.y) - plate;
    return vec2f(min(d1, d2) * 0.5 / de, clamp(sqrt(orb) * 0.7, 0.0, 1.0));
}
"#;
