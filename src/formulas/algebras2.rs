//! Mandelbrot sets in other algebras: squarings in three and four
//! dimensions that are neither the bulb's spherical one nor the
//! quaternions', each iterated as z = z² + c from the point.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "aexion",
        category: "Fractals: algebras",
        desc: "Aexion's quadray Mandelbrot set: the point in four tetrahedral (quadray) coordinates, folded by abs and shifted, squared in four dimensions as two complex squares that trade imaginary parts. At the default shift it falls apart into six lobes along the axes, his 'Mandelbrot Six'. Formula by Aexion (Fractal Forums, 2011); ported from Mandelbulber 2.",
        params: &[Param::num("shift", &[-1.3], -1.6, 0.0, "Added to each of the four quadray coordinates (-1.6 to 0): about -1.3 is Aexion's 'Mandelbrot Six', six lobes, four round the middle and two at the poles; lower parts them further; towards 0 they merge into one small octahedral star (his OctoMandelbrot, a fifth of the size).")],
        iterations: (20, 40, "Escape-time iterations: few fatten the lobes and join them in the middle; past about 20 little changes. A few multiplications each."),
        bound: 2.0,
        lod: None,
        step: 1.0,
        example: r#"{"type":"aexion","scale":0.6}"#,
        prep: None,
        cpu: aexion,
        wgsl: AEXION,
        wgsl_fn: "aexion",
    },
    Formula {
        name: "bristorbrot",
        category: "Fractals: algebras",
        desc: "Doug Bristor's 3D Mandelbrot set, squared in his 'rotational' algebra of two imaginary units whose products depend on their order, so the two imaginary axes mix: the Mandelbrot set's outline along x, swept into a swirled, lopsided solid with a long antenna. Formula by Doug Bristor (Fractal Forums, 2009); ported from Mandelbulber 2.",
        params: &[Param::num("twist", &[1.0], 0.0, 2.0, "How much the two imaginary axes mix (0 to 2): 1 is Bristor's algebra; 0 spins the Mandelbrot set about its axis into a smooth solid of revolution; above 1 it shears further.")],
        iterations: (12, 30, "Escape-time iterations: more = finer detail, at a few multiplications each."),
        bound: 1.6,
        lod: None,
        step: 0.8,
        example: r#"{"type":"bristorbrot","scale":0.6}"#,
        prep: None,
        cpu: bristorbrot,
        wgsl: BRISTORBROT,
        wgsl_fn: "bristorbrot",
    },
    Formula {
        name: "buffalo",
        category: "Fractals: algebras",
        desc: "The 3D Buffalo: the power 2 Mandelbulb's squaring with some of its components made positive (abs) before or after, a shaggy 3D cousin of the 2D Buffalo and Burning Ship (the default holds the 2D Buffalo in its xy plane, facing the camera). Formula by youhn (Fractal Forums, 2015); ported from Mandelbulber 2.",
        params: &[
            Param { name: "abs", default: &[1.0, 1.0, 0.0], min: 0.0, max: 1.0, whole: true, doc: "Which components of the squared point are made positive (1) or left as they are (0): x, y, z, with z towards the default camera. [1, 1, 0] puts the 2D Buffalo in the xy plane; [0, 0, 0] is the plain power 2 Mandelbulb; each other choice is another member of the family." },
            Param { name: "pre_abs", default: &[0.0, 0.0, 0.0], min: 0.0, max: 1.0, whole: true, doc: "Which components are made positive before the squaring (1) or left as they are (0): x, y, z. More members of the family (some the same as an abs choice)." },
            Param::whole("flip_z", 0.0, 0.0, 1.0, "The sign of the squared point's z: 0 as the power 2 Mandelbulb, 1 flipped (no change when z is made positive anyway)."),
        ],
        iterations: (12, 30, "Escape-time iterations: more = finer detail, at a square root and a division each."),
        bound: 2.1,
        lod: None,
        step: 0.6,
        example: r#"{"type":"buffalo","scale":0.6}"#,
        prep: None,
        cpu: buffalo,
        wgsl: BUFFALO,
        wgsl_fn: "buffalo",
    },
    Formula {
        name: "mandelcup",
        category: "Fractals: algebras",
        desc: "The Mandelcup: each point's direction projected onto a plane, squared there as a complex number and wrapped back onto the sphere, its distance squared. A grail-shaped cup on a stem, smooth outside, with layered ornament in the bowl and at the foot. Formula by pupukuusikko (Fractal Forums, 2015); ported from Mandelbulber 2.",
        params: &[
            Param::num("stretch", &[2.0], 0.25, 4.0, "Multiplies the projected square (0.25 to 4): 2, the original's, is the goblet; 1 (Mandelbulber's default) a straighter vase; higher a rounder, shallower bowl."),
            Param::num("shape", &[0.0], -1.0, 1.0, "Added to the direction's height before the projection (-1 to 1): 0 is the plain cup; positive closes it into a rounded pod, negative opens a wide, shallow bowl on a thin stem."),
        ],
        iterations: (12, 30, "Escape-time iterations: more = finer detail, at a square root and a division each."),
        bound: 2.2,
        lod: None,
        step: 0.6,
        example: r#"{"type":"mandelcup","scale":0.6}"#,
        prep: None,
        cpu: mandelcup,
        wgsl: MANDELCUP,
        wgsl_fn: "mandelcup",
    },
];

/// The distance estimate 0.5 r ln r / dr at the orbit's last point, blended
/// toward the one at the point before as the last radius goes from the
/// bailout to its square (ln_b: the bailout's log). An orbit that escapes one
/// iteration later then gives nearly the same estimate, where it would jump
/// by as much as |z| squares inexactly (or c turns it).
#[inline(always)]
fn escape_de(r: F8, dr: F8, rp: F8, drp: F8, ln_b: f32) -> F8 {
    let (r, rp) = (r.max(s(1e-6)), rp.max(s(1e-6)));
    let lr = r.ln();
    let d = lr * r * 0.5 / dr;
    let dp = rp.ln() * rp * 0.5 / drp;
    let t = (lr.max(s(1e-30)) * (1.0 / ln_b)).log2().clamp(0.0, 1.0);
    (dp - d).mul_add(t, d)
}

/// Aexion's quadray set: the constant is the point in quadray coordinates
/// (abs folded, shifted; the map stretches by 2), and the orbit starts
/// there. The squaring is two complex squares, (x, y) and (z, w), with their
/// imaginary parts swapped and each pair turned 45 degrees and scaled by
/// sqrt 2, so |z| grows by between r² and sqrt 2 r². The derivative grows
/// by twice that growth over r: the strict bound, 2 sqrt 2 times the larger
/// pair's length, would make an estimate that is already cautious (the
/// surface is slow to escape) about twice as cautious again.
fn aexion(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let shift = s(k[0]);
    let cx = (p.x + p.y + p.z).abs() + shift;
    let cy = (p.z - p.x - p.y).abs() + shift;
    let cz = (p.y - p.x - p.z).abs() + shift;
    let cw = (p.x - p.y - p.z).abs() + shift;
    let (mut x, mut y, mut z, mut w) = (cx, cy, cz, cw);
    let mut dr = s(2.0);
    let bail = s(1e8);
    let mut r2 = w.mul_add(w, z.mul_add(z, y.mul_add(y, x * x)));
    let (mut r2p, mut drp) = (r2, dr);
    let mut trap = s(1e9);
    let mut act: M8 = r2.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (m1, m2) = (x.mul_add(x, y * y), z.mul_add(z, w * w));
        let g = (m1.mul_add(m1, m2 * m2) * 2.0 / (m1 + m2).max(s(1e-12))).sqrt();
        let ndr = (g * 2.0).mul_add(dr, s(2.0));
        r2p = F8::select(act, r2, r2p);
        drp = F8::select(act, dr, drp);
        let a = x * x - y * y;
        let b = 2.0 * w * z;
        let c = z * z - w * w;
        let d = 2.0 * x * y;
        x = F8::select(act, a + b + cx, x);
        y = F8::select(act, b - a + cy, y);
        z = F8::select(act, c + d + cz, z);
        w = F8::select(act, d - c + cw, w);
        dr = F8::select(act, ndr, dr);
        r2 = w.mul_add(w, z.mul_add(z, y.mul_add(y, x * x)));
        trap = F8::select(act, trap.min(r2), trap);
        act &= r2.lt(bail);
    }
    (escape_de(r2.sqrt(), dr, r2p.sqrt(), drp, 1e4f32.ln()), (trap.sqrt() - 0.4).saturate())
}

const AEXION: &str = r#"
fn aexion(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let k = abs(vec4f(p.x + p.y + p.z, p.z - p.x - p.y, p.y - p.x - p.z, p.x - p.y - p.z)) + a.x;
    var z = k;
    var dr = 2.0;
    var r2 = dot(z, z);
    var r2p = r2;
    var drp = dr;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r2 >= 1e8) {
            break;
        }
        r2p = r2;
        drp = dr;
        let m1 = dot(z.xy, z.xy);
        let m2 = dot(z.zw, z.zw);
        let g = sqrt((m1 * m1 + m2 * m2) * 2.0 / max(m1 + m2, 1e-12));
        dr = g * 2.0 * dr + 2.0;
        let q = z.x * z.x - z.y * z.y;
        let t = 2.0 * z.w * z.z;
        let u = z.z * z.z - z.w * z.w;
        let v = 2.0 * z.x * z.y;
        z = vec4f(q + t, t - q, u + v, v - u) + k;
        r2 = dot(z, z);
        trap = min(trap, r2);
    }
    let r = max(sqrt(r2), 1e-6);
    let rp = max(sqrt(r2p), 1e-6);
    let d = log(r) * r * 0.5 / dr;
    let t = clamp(log2(max(log(r), 1e-30) / 9.2103404), 0.0, 1.0);
    return vec2f(d + (log(rp) * rp * 0.5 / drp - d) * t, clamp(sqrt(trap) - 0.4, 0.0, 1.0));
}
"#;

/// Where the bristorbrot's middle is (along x, its Mandelbrot set's axis).
const BRISTOR_X: f32 = -0.5;

/// The bristorbrot: z = (x² - y² - z², y (2x - t z), z (2x + t y)) + c, t the
/// twist (1: Bristor's). |z| does not square exactly, nor is the stretch the
/// same every way (up to 3 r against the conformal 2 r): the derivative
/// grows by twice the radius's growth, 2 |f(z)| / r, and the step allows
/// for the rest.
fn bristorbrot(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let t = s(k[0]);
    let c = V8::new(p.x + BRISTOR_X, p.y, p.z);
    let bail = s(100.0);
    let mut z = c;
    let mut dr = F8::one();
    let mut r = z.length();
    let (mut rp, mut drp) = (r, dr);
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        rp = F8::select(act, r, rp);
        drp = F8::select(act, dr, drp);
        let x2 = z.x + z.x;
        let f = V8::new(z.x * z.x - z.y * z.y - z.z * z.z, z.y * t.neg_mul_add(z.z, x2), z.z * t.mul_add(z.y, x2));
        let ndr = (f.length() / r.max(s(1e-12)) * dr).mul_add(s(2.0), F8::one());
        z = V8::select(act, f + c, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    (escape_de(r, dr, rp, drp, 100f32.ln()), (trap - 0.35).saturate())
}

const BRISTORBROT: &str = r#"
fn bristorbrot(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let t = a.x;
    let k = vec3f(p.x - 0.5, p.y, p.z);
    var z = k;
    var dr = 1.0;
    var r = length(z);
    var rp = r;
    var drp = dr;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 100.0) {
            break;
        }
        rp = r;
        drp = dr;
        let x2 = z.x + z.x;
        let f = vec3f(z.x * z.x - z.y * z.y - z.z * z.z, z.y * (x2 - t * z.z), z.z * (x2 + t * z.y));
        dr = length(f) / max(r, 1e-12) * dr * 2.0 + 1.0;
        z = f + k;
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    let rq = max(rp, 1e-6);
    let d = log(rr) * rr * 0.5 / dr;
    let s = clamp(log2(max(log(rr), 1e-30) / 4.6051702), 0.0, 1.0);
    return vec2f(d + (log(rq) * rq * 0.5 / drp - d) * s, clamp(trap - 0.35, 0.0, 1.0));
}
"#;

/// Where the buffalo's middle is, in its own (Mandelbulber's) axes.
const BUFFALO_C: [f32; 3] = [-0.4, -0.3, 0.0];

/// The buffalo: Mandelbulber's power 2 bulb (in its own axes: its axis z
/// faces the camera, so the 2D Buffalo in the xy plane does) with abs on
/// chosen components before and after. |z| squares exactly, so the
/// derivative grows by 2 r; near the axis the squaring stretches more (and
/// a Jacobian's estimate dips sharply wherever an orbit passes it), which
/// the step allows for.
fn buffalo(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let on = |i: usize| k[i] > 0.5;
    let up = if on(6) { s(2.0) } else { s(-2.0) };
    let c = V8::new(p.x + BUFFALO_C[0], p.y + BUFFALO_C[1], p.z + BUFFALO_C[2]);
    let bail = s(100.0);
    let mut z = c;
    let mut dr = F8::one();
    let mut r = z.length();
    let (mut rp, mut drp) = (r, dr);
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let ndr = (r * dr).mul_add(s(2.0), F8::one());
        rp = F8::select(act, r, rp);
        drp = F8::select(act, dr, drp);
        let x = if on(3) { z.x.abs() } else { z.x };
        let y = if on(4) { z.y.abs() } else { z.y };
        let h = if on(5) { z.z.abs() } else { z.z };
        let (x2, y2) = (x * x, y * y);
        let rho2 = (x2 + y2).max(s(1e-12));
        let f = F8::one() - h * h / rho2;
        let nx = (x2 - y2) * f;
        let ny = (x + x) * y * f;
        let nh = up * h * rho2.sqrt();
        let nx = if on(0) { nx.abs() } else { nx };
        let ny = if on(1) { ny.abs() } else { ny };
        let nh = if on(2) { nh.abs() } else { nh };
        z = V8::select(act, V8::new(nx, ny, nh) + c, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    (escape_de(r, dr, rp, drp, 100f32.ln()), ((trap - 0.45) * 0.9).saturate())
}

const BUFFALO: &str = r#"
fn buffalo(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let up = select(-2.0, 2.0, b.z > 0.5);
    let k = p + vec3f(-0.4, -0.3, 0.0);
    var z = k;
    var dr = 1.0;
    var r = length(z);
    var rp = r;
    var drp = dr;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 100.0) {
            break;
        }
        rp = r;
        drp = dr;
        dr = r * dr * 2.0 + 1.0;
        let x = select(z.x, abs(z.x), a.w > 0.5);
        let y = select(z.y, abs(z.y), b.x > 0.5);
        let h = select(z.z, abs(z.z), b.y > 0.5);
        let x2 = x * x;
        let y2 = y * y;
        let rho2 = max(x2 + y2, 1e-12);
        let f = 1.0 - h * h / rho2;
        var n = vec3f((x2 - y2) * f, (x + x) * y * f, up * h * sqrt(rho2));
        n = select(n, abs(n), a.xyz > vec3f(0.5));
        z = n + k;
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    let rq = max(rp, 1e-6);
    let d = log(rr) * rr * 0.5 / dr;
    let s = clamp(log2(max(log(rr), 1e-30) / 4.6051702), 0.0, 1.0);
    return vec2f(d + (log(rq) * rq * 0.5 / drp - d) * s, clamp((trap - 0.45) * 0.9, 0.0, 1.0));
}
"#;

/// Where the mandelcup's middle is, in its own (Mandelbulber's) axes.
const CUP_C: [f32; 3] = [-0.29, 0.0, 0.36];

/// The mandelcup: the direction's (x, y) over its height (a gnomonic
/// projection, height shifted by the shape), squared as a complex number and
/// scaled, back onto the sphere by inverse stereographic projection, times
/// r². Written over a common denominator so that the equator, where the
/// projection goes to infinity, stays finite. Its axis (Mandelbulber's z)
/// points down y and its mirror plane faces the camera. |z| squares
/// exactly, so the derivative grows by 2 r; the projection stretches up to
/// twice that (and without bound at the equator, which a Jacobian's
/// estimate shows as sharp dips), which the step allows for.
fn mandelcup(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let shape = s(k[1]);
    let kk = s(k[0] * (1.0 + k[1] * k[1]));
    let c = V8::new(p.z + CUP_C[0], CUP_C[1] - p.x, CUP_C[2] - p.y);
    let bail = s(100.0 * 100.0);
    let mut z = c;
    let mut dr = F8::one();
    let mut r2 = z.length_sq();
    let (mut r2p, mut drp) = (r2, dr);
    let mut trap = s(1e9);
    let mut act: M8 = r2.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let r = r2.sqrt();
        let ndr = (r * dr).mul_add(s(2.0), F8::one());
        r2p = F8::select(act, r2, r2p);
        drp = F8::select(act, dr, drp);
        let q = shape.mul_add(r, z.z);
        let q2 = q * q;
        let a = kk * (z.x * z.x - z.y * z.y);
        let b = kk * (z.x + z.x) * z.y;
        let ab = a.mul_add(a, b * b);
        let m = r2 / q2.mul_add(q2, ab).max(s(1e-30));
        let n = V8::new(a * q2 * 2.0 * m, b * q2 * 2.0 * m, (ab - q2 * q2) * m);
        z = V8::select(act, n + c, z);
        dr = F8::select(act, ndr, dr);
        r2 = z.length_sq();
        trap = F8::select(act, trap.min(r2), trap);
        act &= r2.lt(bail);
    }
    (escape_de(r2.sqrt(), dr, r2p.sqrt(), drp, 100f32.ln()), ((trap.sqrt() - 0.45) * 0.9).saturate())
}

const MANDELCUP: &str = r#"
fn mandelcup(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let shape = a.y;
    let kk = a.x * (1.0 + a.y * a.y);
    let k = vec3f(p.z - 0.29, -p.x, 0.36 - p.y);
    var z = k;
    var dr = 1.0;
    var r2 = dot(z, z);
    var r2p = r2;
    var drp = dr;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r2 >= 1e4) {
            break;
        }
        r2p = r2;
        drp = dr;
        let r = sqrt(r2);
        dr = r * dr * 2.0 + 1.0;
        let q = shape * r + z.z;
        let q2 = q * q;
        let u = kk * (z.x * z.x - z.y * z.y);
        let v = kk * (z.x + z.x) * z.y;
        let uv = u * u + v * v;
        let m = r2 / max(uv + q2 * q2, 1e-30);
        z = vec3f(u * q2 * 2.0 * m, v * q2 * 2.0 * m, (uv - q2 * q2) * m) + k;
        r2 = dot(z, z);
        trap = min(trap, r2);
    }
    let rr = max(sqrt(r2), 1e-6);
    let rq = max(sqrt(r2p), 1e-6);
    let d = log(rr) * rr * 0.5 / dr;
    let s = clamp(log2(max(log(rr), 1e-30) / 4.6051702), 0.0, 1.0);
    return vec2f(d + (log(rq) * rq * 0.5 / drp - d) * s, clamp((sqrt(trap) - 0.45) * 0.9, 0.0, 1.0));
}
"#;
