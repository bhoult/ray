//! Escape-time formulas from Fractal Forums' hunt for a true 3D Mandelbrot:
//! Xenodreambuie's folded bulb, Benesi's pine tree, Makin's quadratic and
//! Ide's formula, ported from Mandelbulber 2. The last three are far from
//! conformal, so they carry the orbit's whole Jacobian rather than a scalar
//! derivative (Mandelbulber takes Ide's by finite differences instead), and
//! blend their estimate across the bailout, where it would otherwise jump.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};
use std::f32::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "xenodreambuie",
        category: "Fractals: more",
        desc: "Xenodreambuie's Mandelbulb: the power map in true spherical coordinates, its polar angle folded into the upper hemisphere before it is multiplied, so the bulb comes out mirrored top to bottom and set with stacked, petal-like domes. Formula by Garth Thornton (xenodreambuie, author of XenoDream; Fractal Forums, 2009); ported from Mandelbulber 2.",
        params: &[
            Param::num("power", &[9.0], 2.0, 16.0, "The exponent (2 to 16): 9 is Mandelbulber's default. Higher powers give more, finer domes around the equator and over the poles."),
            Param::num("polar_offset", &[0.0], -180.0, 180.0, "Degrees added to the polar angle before the fold (-180 to 180): small offsets break the top-bottom mirror and twist the domes into crowns and shells."),
            Param::num("azimuth_offset", &[0.0], -180.0, 180.0, "Degrees added to the angle around the axis before it is multiplied (-180 to 180): turns the lobes about the axis at every step, shifting and shearing the pattern."),
        ],
        iterations: (10, 32, "Escape-time iterations: more = finer detail, at a pow, two atan2, two sin and two cos per point each."),
        bound: 1.4,
        lod: None,
        step: 0.9,
        example: r#"{"type":"xenodreambuie","scale":0.9}"#,
        prep: Some(xenodreambuie_prep),
        cpu: xenodreambuie,
        wgsl: XENODREAMBUIE,
        wgsl_fn: "xenodreambuie",
    },
    Formula {
        name: "pine_tree",
        category: "Fractals: more",
        desc: "Benesi's pine tree: a quadratic Mandelbrot spun about its needle, the angle around it doubled and the two cross axes swapped at every step; stood on its needle it is a tiered, drooping conifer topped by a spire of minibrots. Formula by M. Benesi (Fractal Forums, 2010-2015) after Paul Nylander's Christmas-tree Mandelbrot (2009); ported from Mandelbulber 2.",
        params: &[
            Param::num("c_scale", &[1.0, 0.25, 0.25], -2.0, 2.0, "How much of the point's height and its two cross coordinates is added at each step (each -2 to 2): Mandelbulber's 1, 0, 0 is a smooth, banded pagoda; raising the last two drapes and tears the tiers, and 1, 1, 1 (the full Mandelbrot type) is a ragged storm."),
            Param::num("twist", &[0.0], -180.0, 180.0, "Degrees the cross section turns about the needle at every step (-180 to 180): spirals the tiers."),
        ],
        iterations: (14, 30, "Escape-time iterations: more = finer tiers and a longer spire, at nine products of the Jacobian per point each."),
        bound: 1.5,
        lod: None,
        step: 0.25,
        example: r#"{"type":"pine_tree","scale":0.6}"#,
        prep: Some(pine_tree_prep),
        cpu: pine_tree,
        wgsl: PINE_TREE,
        wgsl_fn: "pine_tree",
    },
    Formula {
        name: "makin",
        category: "Fractals: more",
        desc: "Makin's quadratic: (x² + 2yz, -y² - 2xz, -z² + 2xy) plus the point, a 3D squaring that comes out as a swept, three-cornered sail under a spire, its edges frayed into spikes. Formula by David Makin (Ultra Fractal, by 2010; Mandelbulb 3D's Makin3D-2, 2011); ported from Mandelbulber 2.",
        params: &[
            Param::num("julia", &[0.0], 0.0, 1.0, "From the Mandelbrot type (0: the point is added at each step) to the Julia set of c (1); in between morphs one into the other."),
            Param::num("c", &[-0.5, 0.5, 0.5], -1.5, 1.5, "The Julia constant (each -1.5 to 1.5), used as far as julia is above 0: near the origin a solid, cushion-like lump, further out swirled and holed, then broken into dust."),
        ],
        iterations: (14, 30, "Escape-time iterations: more = finer spikes, at nine products of the Jacobian per point each."),
        bound: 2.0,
        lod: None,
        step: 0.4,
        example: r#"{"type":"makin","scale":0.6}"#,
        prep: None,
        cpu: makin,
        wgsl: MAKIN,
        wgsl_fn: "makin",
    },
    Formula {
        name: "ides",
        category: "Fractals: more",
        desc: "Ide's formula: (x² - (y² + z²)/2, 2xyz, z² - (x² + y²)/2) plus the point, x and z first shrunk by a tenth: a mirror-symmetric, moth-like body of hollow wings, ribs and hair-fine spikes. Trafassel's \"Formula 21\" (Fractal Forums, 2010); ported from Mandelbulber 2.",
        params: &[
            Param::num("mult", &[1.0, 2.0, 1.0], -3.0, 3.0, "The multipliers of x², xyz and z² (each -3 to 3): 1, 2, 1 is Ide's; the middle one sets how spiky the ribs are."),
            Param::num("sub", &[0.5, 0.5], -2.0, 2.0, "How much of the other two squares is taken from x and from z (each -2 to 2): lower values grow the body and its spikes, higher shrink it to a compact, blade-edged core."),
        ],
        iterations: (12, 30, "Escape-time iterations: more = finer spikes, at nine products of the Jacobian per point each."),
        bound: 3.1,
        lod: None,
        step: 0.3,
        example: r#"{"type":"ides","scale":0.4}"#,
        prep: None,
        cpu: ides,
        wgsl: IDES,
        wgsl_fn: "ides",
    },
];

// ---- the Jacobian (CPU) -----------------------------------------------------

/// A 3x3 matrix in eight lanes: a map's derivative by rows, or the orbit's
/// Jacobian by columns (its derivatives by p.x, p.y and p.z).
type M3 = [V8; 3];

/// The Jacobian's entries are held to this (no lane overflows; an entry
/// that big is a distance far below any step).
const JMAX: f32 = 1e16;

#[inline(always)]
fn identity() -> M3 {
    let (o, l) = (F8::zero(), F8::one());
    [V8::new(l, o, o), V8::new(o, l, o), V8::new(o, o, l)]
}

/// The chain rule's step: the map's derivative `a` (rows) times the
/// Jacobian `j` (columns), held to ±JMAX.
#[inline(always)]
fn chain(a: &M3, j: &M3) -> M3 {
    let col = |v: V8| {
        let w = V8::new(a[0].dot(v), a[1].dot(v), a[2].dot(v));
        V8::new(w.x.clamp(-JMAX, JMAX), w.y.clamp(-JMAX, JMAX), w.z.clamp(-JMAX, JMAX))
    };
    [col(j[0]), col(j[1]), col(j[2])]
}

#[inline(always)]
fn select3(m: M8, t: &M3, f: &M3) -> M3 {
    [V8::select(m, t[0], f[0]), V8::select(m, t[1], f[1]), V8::select(m, t[2], f[2])]
}

/// The Jacobian's Frobenius norm: at least its largest stretch, and about
/// that once the orbit has run a few steps; for Ide's, whose potential's
/// gradient jumps about too much to march by.
#[inline(always)]
fn frobenius(j: &M3) -> F8 {
    (j[0].length_sq() + j[1].length_sq() + j[2].length_sq()).sqrt()
}

/// The escape potential's gradient, |Jᵀz| / r: the stretch that matters
/// (the Jacobian's largest can be far more: Benesi's pine tree stretches
/// about its needle up to twice as much as along it at every step, which
/// as a whole-Jacobian estimate swells it into fuzz).
#[inline(always)]
fn gradient(j: &M3, z: V8) -> F8 {
    let u = z * (F8::one() / z.length().max(s(1e-6)));
    V8::new(j[0].dot(u), j[1].dot(u), j[2].dot(u)).length()
}

/// The estimate half r ln r / f (f the gradient or the Jacobian's size)
/// from the escaping step (r, f) and the one
/// before it (rp, fp), blended by how far the escape overshot the bailout
/// (in ln ln r: 0 just past it, 1 when the step before was just short of
/// it), so it does not jump where the escape count changes; 0 for lanes
/// that never escaped (`inside`: bounded orbits say nothing of distance).
#[inline(always)]
fn blended_de(r: F8, f: F8, rp: F8, fp: F8, bail: f32, inside: M8) -> F8 {
    let de = |r: F8, f: F8| {
        let r = r.max(s(1e-6));
        r.ln() * r * 0.5 / f.max(s(1e-12))
    };
    let lnln = |r: F8| r.max(s(1e-6)).ln().max(s(1e-6)).ln();
    let t = ((lnln(r) - bail.ln().ln()) / (lnln(r) - lnln(rp)).max(s(1e-6))).saturate();
    let d = de(r, f);
    F8::select(inside, F8::zero(), (de(rp, fp) - d).mul_add(t, d))
}

// ---- Xenodreambuie ------------------------------------------------------------

/// The angle offsets from degrees to radians.
fn xenodreambuie_prep(k: &mut [f32; K]) {
    k[1] = k[1].to_radians();
    k[2] = k[2].to_radians();
}

/// Xenodreambuie's bulb (Mandelbulber's, its z axis our y): the polar angle
/// from the axis, offset, folded into -90..90 degrees, then multiplied; the
/// scalar derivative as the Mandelbulb's. Trap: the nearest the orbit comes
/// to the equator, the fold's mirror plane.
fn xenodreambuie(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let n = k[0];
    let (alpha, beta) = (s(k[1]), s(k[2]));
    let n1 = s(n - 1.0);
    // (not the Mandelbulb's 2: the estimate steps a little where the first
    // iteration starts, which this keeps outside the bound)
    let bail = s(4.0);
    let mut z = p;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-6));
        let rp1 = rs.powf(n1);
        let ndr = (rp1 * dr * n) + 1.0;
        let zr = rp1 * rs;
        let th = (z.z.atan2(z.x) + beta) * n;
        let ph = len2(z.x, z.z).atan2(z.y) + alpha;
        let ph = F8::select(ph.abs().gt(s(FRAC_PI_2)), s(PI).copysign(ph) - ph, ph) * n;
        let sph = ph.sin();
        let nz = V8::new(zr * th.cos() * sph, zr * ph.cos(), zr * th.sin() * sph) + p;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(z.y.abs()), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap * 1.5).saturate())
}

const XENODREAMBUIE: &str = r#"
fn xenodreambuie(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let n = a.x;
    let alpha = a.y;
    let beta = a.z;
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 4.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let rp1 = pow(rs, n - 1.0);
        dr = rp1 * dr * n + 1.0;
        let zr = rp1 * rs;
        let th = (atan2(z.z, z.x) + beta) * n;
        var ph = atan2(length(z.xz), z.y) + alpha;
        if (abs(ph) > 1.5707964) {
            ph = sign(ph) * 3.1415927 - ph;
        }
        ph = ph * n;
        let sph = sin(ph);
        z = vec3f(zr * cos(th) * sph, zr * cos(ph), zr * sin(th) * sph) + p;
        r = length(z);
        trap = min(trap, abs(z.y));
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap * 1.5, 0.0, 1.0));
}
"#;

// ---- Benesi's pine tree -------------------------------------------------------

/// The twist from degrees to its cosine and sine.
fn pine_tree_prep(k: &mut [f32; K]) {
    let t = k[3].to_radians();
    k[3] = t.cos();
    k[4] = t.sin();
}

/// Benesi's pine tree, in Mandelbulber's frame: x² - (y² + z²) along the
/// needle (x); across it 2x/ρ (2yz, y² - z²), the angle about the needle
/// doubled and y and z swapped; plus the point, scaled per axis and swapped
/// alike; then the twist. Our y is its -x, so the needle points up, and the
/// tree is centred. Trap: the nearest the orbit comes to the needle.
fn pine_tree(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let m = [k[0], k[1], k[2]];
    let (ca, sa) = (s(k[3]), s(k[4]));
    let p = V8::new(-p.y - 0.68, p.x, p.z);
    let c = V8::new(p.x * m[0], p.z * m[2], p.y * m[1]);
    let bail = 8.0;
    let twist = |v: V8| V8::new(v.x, v.y * ca + v.z * sa, v.z * ca - v.y * sa);
    let mut z = p;
    let mut j = identity();
    let mut r = z.length();
    let mut f = F8::one();
    let (mut rp, mut fp) = (r, f);
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(bail));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (x2, y2, z2) = (z.x * z.x, z.y * z.y, z.z * z.z);
        let t = y2 + z2;
        let iq = F8::one() / t.max(s(1e-30)).sqrt();
        let iq3 = iq * iq * iq;
        let h = z.x * 2.0 * iq;
        let nz = twist(V8::new(x2 - t, h * 2.0 * z.y * z.z, h * (y2 - z2)) + c);
        let a = [
            V8::new(z.x * 2.0, z.y * -2.0, z.z * -2.0),
            V8::new(z.y * z.z * 4.0 * iq, z.x * 4.0 * z2 * z.z * iq3, z.x * 4.0 * y2 * z.y * iq3),
            V8::new((y2 - z2) * 2.0 * iq, z.x * 2.0 * z.y * (z2 * 3.0 + y2) * iq3, z.x * -2.0 * z.z * (y2 * 3.0 + z2) * iq3),
        ];
        let mut nj = chain(&a, &j);
        nj[0].x += m[0];
        nj[1].z += m[1];
        nj[2].y += m[2];
        let nj = [twist(nj[0]), twist(nj[1]), twist(nj[2])];
        rp = F8::select(act, r, rp);
        fp = F8::select(act, f, fp);
        z = V8::select(act, nz, z);
        j = select3(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        // (at least a tenth of the Jacobian's size: where the orbit grazes
        // the needle the gradient alone swings about too much to march by)
        f = gradient(&j, z).max(frobenius(&j) * 0.1);
        trap = F8::select(act, trap.min(len2(z.y, z.z)), trap);
        act &= r.lt(s(bail));
    }
    (blended_de(r, f, rp, fp, bail, act), (trap * 1.5).saturate())
}

const PINE_TREE: &str = r#"
fn pine_tree_twist(v: vec3f, ca: f32, sa: f32) -> vec3f {
    return vec3f(v.x, v.y * ca + v.z * sa, v.z * ca - v.y * sa);
}

fn pine_tree_chain(r0: vec3f, r1: vec3f, r2: vec3f, v: vec3f) -> vec3f {
    return clamp(vec3f(dot(r0, v), dot(r1, v), dot(r2, v)), vec3f(-1e16), vec3f(1e16));
}

fn pine_tree_de(r: f32, f: f32) -> f32 {
    let rr = max(r, 1e-6);
    return log(rr) * rr * 0.5 / max(f, 1e-12);
}

fn pine_tree_lnln(r: f32) -> f32 {
    return log(max(log(max(r, 1e-6)), 1e-6));
}

fn pine_tree(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let m = a.xyz;
    let ca = a.w;
    let sa = b.x;
    let q = vec3f(-p.y - 0.68, p.x, p.z);
    let k = vec3f(q.x * m.x, q.z * m.z, q.y * m.y);
    var z = q;
    var j0 = vec3f(1.0, 0.0, 0.0);
    var j1 = vec3f(0.0, 1.0, 0.0);
    var j2 = vec3f(0.0, 0.0, 1.0);
    var r = length(z);
    var f = 1.0;
    var rp = r;
    var fp = f;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 8.0) {
            break;
        }
        let x2 = z.x * z.x;
        let y2 = z.y * z.y;
        let z2 = z.z * z.z;
        let t = y2 + z2;
        let iq = 1.0 / sqrt(max(t, 1e-30));
        let iq3 = iq * iq * iq;
        let h = z.x * 2.0 * iq;
        let r0 = vec3f(z.x * 2.0, z.y * -2.0, z.z * -2.0);
        let r1 = vec3f(z.y * z.z * 4.0 * iq, z.x * 4.0 * z2 * z.z * iq3, z.x * 4.0 * y2 * z.y * iq3);
        let r2 = vec3f((y2 - z2) * 2.0 * iq, z.x * 2.0 * z.y * (z2 * 3.0 + y2) * iq3, z.x * -2.0 * z.z * (y2 * 3.0 + z2) * iq3);
        j0 = pine_tree_twist(pine_tree_chain(r0, r1, r2, j0) + vec3f(m.x, 0.0, 0.0), ca, sa);
        j1 = pine_tree_twist(pine_tree_chain(r0, r1, r2, j1) + vec3f(0.0, 0.0, m.y), ca, sa);
        j2 = pine_tree_twist(pine_tree_chain(r0, r1, r2, j2) + vec3f(0.0, m.z, 0.0), ca, sa);
        rp = r;
        fp = f;
        z = pine_tree_twist(vec3f(x2 - t, h * 2.0 * z.y * z.z, h * (y2 - z2)) + k, ca, sa);
        r = length(z);
        let u = z / max(r, 1e-6);
        f = max(length(vec3f(dot(j0, u), dot(j1, u), dot(j2, u))), sqrt(dot(j0, j0) + dot(j1, j1) + dot(j2, j2)) * 0.1);
        trap = min(trap, length(z.yz));
    }
    let tr = clamp(trap * 1.5, 0.0, 1.0);
    if (r < 8.0) {
        return vec2f(0.0, tr);
    }
    let d = pine_tree_de(r, f);
    let t = clamp((pine_tree_lnln(r) - pine_tree_lnln(8.0)) / max(pine_tree_lnln(r) - pine_tree_lnln(rp), 1e-6), 0.0, 1.0);
    return vec2f(d + (pine_tree_de(rp, fp) - d) * t, tr);
}
"#;

// ---- Makin's quadratic --------------------------------------------------------

/// Makin3D-2: (x² + 2yz, -y² - 2xz, -z² + 2xy) plus the point (or, as far
/// as julia goes, the constant c). Turned a quarter about y, so the default
/// view looks along its sail with the spire up, and centred. Trap: the
/// nearest the orbit comes to its y axis.
fn makin(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let w = k[0];
    let p = V8::new(p.z - 0.27, p.y + 0.45, 0.27 - p.x);
    let c = p * (1.0 - w) + V8::new(s(k[1] * w), s(k[2] * w), s(k[3] * w));
    let bail = 8.0;
    let mut z = p;
    let mut j = identity();
    let mut r = z.length();
    let mut f = F8::one();
    let (mut rp, mut fp) = (r, f);
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(bail));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (x2, y2, z2) = (z.x * z.x, z.y * z.y, z.z * z.z);
        let nz = V8::new(x2 + z.y * z.z * 2.0, -y2 - z.x * z.z * 2.0, -z2 + z.x * z.y * 2.0) + c;
        let (x, y, zz) = (z.x * 2.0, z.y * 2.0, z.z * 2.0);
        let a = [V8::new(x, zz, y), V8::new(-zz, -y, -x), V8::new(y, x, -zz)];
        let mut nj = chain(&a, &j);
        nj[0].x += 1.0 - w;
        nj[1].y += 1.0 - w;
        nj[2].z += 1.0 - w;
        rp = F8::select(act, r, rp);
        fp = F8::select(act, f, fp);
        z = V8::select(act, nz, z);
        j = select3(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        f = gradient(&j, z);
        trap = F8::select(act, trap.min(len2(z.x, z.z)), trap);
        act &= r.lt(s(bail));
    }
    (blended_de(r, f, rp, fp, bail, act), (trap * 1.5).saturate())
}

const MAKIN: &str = r#"
fn makin_chain(r0: vec3f, r1: vec3f, r2: vec3f, v: vec3f) -> vec3f {
    return clamp(vec3f(dot(r0, v), dot(r1, v), dot(r2, v)), vec3f(-1e16), vec3f(1e16));
}

fn makin_de(r: f32, f: f32) -> f32 {
    let rr = max(r, 1e-6);
    return log(rr) * rr * 0.5 / max(f, 1e-12);
}

fn makin_lnln(r: f32) -> f32 {
    return log(max(log(max(r, 1e-6)), 1e-6));
}

fn makin(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let w = a.x;
    let q = vec3f(p.z - 0.27, p.y + 0.45, 0.27 - p.x);
    let k = q * (1.0 - w) + a.yzw * w;
    var z = q;
    var j0 = vec3f(1.0, 0.0, 0.0);
    var j1 = vec3f(0.0, 1.0, 0.0);
    var j2 = vec3f(0.0, 0.0, 1.0);
    var r = length(z);
    var f = 1.0;
    var rp = r;
    var fp = f;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 8.0) {
            break;
        }
        let r0 = vec3f(z.x * 2.0, z.z * 2.0, z.y * 2.0);
        let r1 = vec3f(-z.z * 2.0, -z.y * 2.0, -z.x * 2.0);
        let r2 = vec3f(z.y * 2.0, z.x * 2.0, -z.z * 2.0);
        j0 = makin_chain(r0, r1, r2, j0) + vec3f(1.0 - w, 0.0, 0.0);
        j1 = makin_chain(r0, r1, r2, j1) + vec3f(0.0, 1.0 - w, 0.0);
        j2 = makin_chain(r0, r1, r2, j2) + vec3f(0.0, 0.0, 1.0 - w);
        rp = r;
        fp = f;
        z = vec3f(z.x * z.x + z.y * z.z * 2.0, -z.y * z.y - z.x * z.z * 2.0, -z.z * z.z + z.x * z.y * 2.0) + k;
        r = length(z);
        let u = z / max(r, 1e-6);
        f = length(vec3f(dot(j0, u), dot(j1, u), dot(j2, u)));
        trap = min(trap, length(z.xz));
    }
    let tr = clamp(trap * 1.5, 0.0, 1.0);
    if (r < 8.0) {
        return vec2f(0.0, tr);
    }
    let d = makin_de(r, f);
    let t = clamp((makin_lnln(r) - makin_lnln(8.0)) / max(makin_lnln(r) - makin_lnln(rp), 1e-6), 0.0, 1.0);
    return vec2f(d + (makin_de(rp, fp) - d) * t, tr);
}
"#;

// ---- Ide's formula -------------------------------------------------------------

/// Ide's formula: x and z shrunk by a tenth (ides_shrink), then
/// (m₁x² - s₁(y² + z²), m₂xyz, m₃z² - s₂(x² + y²)) plus the point; bailout
/// 10, as Mandelbulber's; estimated by the Jacobian's size. Turned 135 degrees about y, so its mirror plane
/// (x = z) faces the default view, and centred. Trap: the nearest the orbit
/// comes to the planes x = 0 and z = 0.
fn ides(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (m, sub) = ([k[0], k[1], k[2]], [k[3], k[4]]);
    let (x, z) = (p.x * FRAC_1_SQRT_2, (p.z + 0.3) * FRAC_1_SQRT_2);
    let p = V8::new(-x - z, p.y, x - z);
    let bail = 10.0;
    let mut z = p;
    let mut j = identity();
    let mut r = z.length();
    let mut f = s(3f32.sqrt());
    let (mut rp, mut fp) = (r, f);
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(bail));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (x, sx) = ides_shrink(z.x);
        let (zz, sz) = ides_shrink(z.z);
        let y = z.y;
        let (x2, y2, z2) = (x * x, y * y, zz * zz);
        let nz = V8::new(x2 * m[0] - (y2 + z2) * sub[0], x * y * zz * m[1], z2 * m[2] - (x2 + y2) * sub[1]) + p;
        // (the shrink scales the derivative's x and z columns)
        let a = [
            V8::new(x * (2.0 * m[0]) * sx, y * (-2.0 * sub[0]), zz * (-2.0 * sub[0]) * sz),
            V8::new(y * zz * m[1] * sx, x * zz * m[1], x * y * m[1] * sz),
            V8::new(x * (-2.0 * sub[1]) * sx, y * (-2.0 * sub[1]), zz * (2.0 * m[2]) * sz),
        ];
        let mut nj = chain(&a, &j);
        nj[0].x += 1.0;
        nj[1].y += 1.0;
        nj[2].z += 1.0;
        rp = F8::select(act, r, rp);
        fp = F8::select(act, f, fp);
        z = V8::select(act, nz, z);
        j = select3(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        f = frobenius(&j);
        trap = F8::select(act, trap.min(z.x.abs().min(z.z.abs())), trap);
        act &= r.lt(s(bail));
    }
    (blended_de(r, f, rp, fp, bail, act), (trap * 1.5).saturate())
}

/// Mandelbulber's shrink of a coordinate by a tenth (trafassel's original
/// has none), and its slope. Mandelbulber's stops at 2.5, a jump no estimate
/// can follow; this eases into slope 1 by 3 (the set lies within 2.5).
#[inline(always)]
fn ides_shrink(x: F8) -> (F8, F8) {
    let u = x.abs();
    let t = (u - 2.5).clamp(0.0, 0.5);
    let g = (t * t).mul_add(s(0.1), u * 0.9) + (u - 3.0).max(F8::zero()) * 0.1;
    (g.copysign(x), t.mul_add(s(0.2), s(0.9)))
}

const IDES: &str = r#"
fn ides_chain(r0: vec3f, r1: vec3f, r2: vec3f, v: vec3f) -> vec3f {
    return clamp(vec3f(dot(r0, v), dot(r1, v), dot(r2, v)), vec3f(-1e16), vec3f(1e16));
}

fn ides_shrink(x: f32) -> vec2f {
    let u = abs(x);
    let t = clamp(u - 2.5, 0.0, 0.5);
    let g = t * t * 0.1 + u * 0.9 + max(u - 3.0, 0.0) * 0.1;
    return vec2f(sign(x) * g, t * 0.2 + 0.9);
}

fn ides_de(r: f32, f: f32) -> f32 {
    let rr = max(r, 1e-6);
    return log(rr) * rr * 0.5 / max(f, 1e-12);
}

fn ides_lnln(r: f32) -> f32 {
    return log(max(log(max(r, 1e-6)), 1e-6));
}

fn ides(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let m = a.xyz;
    let sub = vec2f(a.w, b.x);
    let h = vec2f(p.x, p.z + 0.3) * 0.70710678;
    let q = vec3f(-h.x - h.y, p.y, h.x - h.y);
    var z = q;
    var j0 = vec3f(1.0, 0.0, 0.0);
    var j1 = vec3f(0.0, 1.0, 0.0);
    var j2 = vec3f(0.0, 0.0, 1.0);
    var r = length(z);
    var f = sqrt(3.0);
    var rp = r;
    var fp = f;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let gx = ides_shrink(z.x);
        let gz = ides_shrink(z.z);
        let x = gx.x;
        let sx = gx.y;
        let zz = gz.x;
        let sz = gz.y;
        let y = z.y;
        let x2 = x * x;
        let y2 = y * y;
        let z2 = zz * zz;
        let r0 = vec3f(x * (2.0 * m.x) * sx, y * (-2.0 * sub.x), zz * (-2.0 * sub.x) * sz);
        let r1 = vec3f(y * zz * m.y * sx, x * zz * m.y, x * y * m.y * sz);
        let r2 = vec3f(x * (-2.0 * sub.y) * sx, y * (-2.0 * sub.y), zz * (2.0 * m.z) * sz);
        j0 = ides_chain(r0, r1, r2, j0) + vec3f(1.0, 0.0, 0.0);
        j1 = ides_chain(r0, r1, r2, j1) + vec3f(0.0, 1.0, 0.0);
        j2 = ides_chain(r0, r1, r2, j2) + vec3f(0.0, 0.0, 1.0);
        rp = r;
        fp = f;
        z = vec3f(x2 * m.x - (y2 + z2) * sub.x, x * y * zz * m.y, z2 * m.z - (x2 + y2) * sub.y) + q;
        r = length(z);
        f = sqrt(dot(j0, j0) + dot(j1, j1) + dot(j2, j2));
        trap = min(trap, min(abs(z.x), abs(z.z)));
    }
    let tr = clamp(trap * 1.5, 0.0, 1.0);
    if (r < 10.0) {
        return vec2f(0.0, tr);
    }
    let d = ides_de(r, f);
    let t = clamp((ides_lnln(r) - ides_lnln(10.0)) / max(ides_lnln(r) - ides_lnln(rp), 1e-6), 0.0, 1.0);
    return vec2f(d + (ides_de(rp, fp) - d) * t, tr);
}
"#;
