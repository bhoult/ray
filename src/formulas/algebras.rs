//! Mandelbrot sets in other number systems: z² + c with the point for c,
//! squared as a quaternion, a bicomplex number or one of Davenport's
//! commutative hypercomplex numbers.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

/// Where the sets' real axis sits: c = p.x - 0.75 centres the Mandelbrot
/// set's -2..0.25 (in the shapes' x).
const RE: f32 = -0.75;

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "quaternion",
        category: "Fractals: algebras",
        desc: "The quaternion Mandelbrot set (Gomatam et al. and Bedding and Briggs, 1995): z² + c with quaternions, sliced in 3D. Squaring a quaternion keeps it in the plane of the real axis and its own imaginary part, so the set is the Mandelbrot set spun about its real axis: a lathe-turned ornament of discs and rings with the antenna for a spire. Moving the slice along the fourth axis cuts it down to a dish.",
        params: &[Param::num(
            "w",
            &[0.0],
            -1.2,
            1.2,
            "The slice's fourth (k) coordinate: 0 is the classic spun Mandelbrot set; away from 0 only the parts of the set further than |w| from its real axis are left, pulled in to the axis, so the antenna and the left bulbs go first and the rest thins to a dish and rings (nothing is left past about 1.1).",
        )],
        iterations: (14, 40, "Escape-time iterations: more = sharper bulbs and finer rings."),
        bound: 1.3,
        lod: None,
        step: 1.0,
        example: r#"{"type":"quaternion","scale":0.9}"#,
        prep: None,
        cpu: quaternion,
        wgsl: QUATERNION,
        wgsl_fn: "quaternion",
    },
    Formula {
        name: "tetrabrot",
        category: "Fractals: algebras",
        desc: "Dominic Rochon's Tetrabrot (2000): the bicomplex Mandelbrot set's slice through 1, i1 and i2. A bicomplex number is two complex numbers squared independently, so the shape is two Mandelbrot sets drawn out along the diagonals and intersected: a faceted, cubist Mandelbrot with diamond cross-sections and box-like bulbs.",
        params: &[
            Param::whole("power", 2.0, 2.0, 8.0, "The exponent (2 to 8): 2 is Rochon's Tetrabrot; higher powers give the Tetrabrots of the Multibrot sets, blocky bodies with power - 1 lobes about the centre."),
            Param::num(
                "w",
                &[0.0],
                -1.0,
                1.0,
                "The slice's fourth (j = i1 i2) coordinate: it slides the two Mandelbrot sets the shape is cut from apart along the real axis, so it shrinks and its facets break up into crossed bulbs (try 0.2 to 0.4).",
            ),
        ],
        iterations: (16, 40, "Escape-time iterations: more = sharper, finer edges and bulbs."),
        bound: 1.3,
        lod: None,
        step: 1.0,
        example: r#"{"type":"tetrabrot","scale":0.9}"#,
        prep: Some(tetrabrot_prep),
        cpu: tetrabrot,
        wgsl: TETRABROT,
        wgsl_fn: "tetrabrot",
    },
    Formula {
        name: "hypercomplex",
        category: "Fractals: algebras",
        desc: "Clyde Davenport's hypercomplex Mandelbrot set: z² + c with commutative quaternions (i² = j² = -1, k² = +1), sliced through 1, i and k. These are the bicomplex numbers again, so the set is two Mandelbrot sets crossed, but this slice draws them out along the real axis: a stepped arrowhead of stacked slabs (Rochon's Arrowheadbrot). Turning the slice towards j gives the Tetrabrot.",
        params: &[
            Param::num("angle", &[0.0], 0.0, 90.0, "The slice's third axis, turned from k (0: the arrowhead) towards j (90: the Tetrabrot), in degrees; in between, the crossed Mandelbrot sets lean and the bulbs become slanted wedges."),
            Param::num("w", &[0.0], -1.0, 1.0, "The slice's fourth coordinate: it slides the two Mandelbrot sets apart, so the shape shrinks and splits into crossed bulbs."),
        ],
        iterations: (16, 40, "Escape-time iterations: more = sharper, finer edges and bulbs."),
        bound: 1.3,
        lod: None,
        step: 1.0,
        example: r#"{"type":"hypercomplex","scale":0.9}"#,
        prep: Some(hypercomplex_prep),
        cpu: hypercomplex,
        wgsl: HYPERCOMPLEX,
        wgsl_fn: "hypercomplex",
    },
];

/// The quaternion Mandelbrot set's distance estimate (iq's, as vm.rs julia,
/// with the point for c and +1 in the derivative), and its orbit trap: the
/// nearest the orbit comes to the centre.
fn quaternion(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (c0, c1, c2, c3) = (p.x + RE, p.y, p.z, s(k[0]));
    let (mut a, mut b, mut cc, mut d) = (c0, c1, c2, c3);
    let mut dr = F8::one();
    let mut mz2 = d.mul_add(d, cc.mul_add(cc, b.mul_add(b, a * a)));
    let mut trap = mz2;
    let mut act = mz2.lt(s(1e6));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let ndr = (mz2.sqrt() * dr).mul_add(s(2.0), F8::one());
        let na = a * a - b * b - cc * cc - d * d + c0;
        let nb = (a * b).mul_add(s(2.0), c1);
        let nc = (a * cc).mul_add(s(2.0), c2);
        let nd = (a * d).mul_add(s(2.0), c3);
        a = F8::select(act, na, a);
        b = F8::select(act, nb, b);
        cc = F8::select(act, nc, cc);
        d = F8::select(act, nd, d);
        dr = F8::select(act, ndr, dr);
        mz2 = d.mul_add(d, cc.mul_add(cc, b.mul_add(b, a * a)));
        trap = F8::select(act, trap.min(mz2), trap);
        act &= mz2.lt(s(1e6));
    }
    // (an orbit still bounded is inside: 0, not the formula, which is
    // meaningless there)
    let r = mz2.max(s(1e-12)).sqrt();
    (F8::select(act, F8::zero(), r * r.ln() * 0.5 / dr), trap.sqrt().saturate())
}

const QUATERNION: &str = r#"
fn quaternion(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let k = vec4f(p.x - 0.75, p.y, p.z, a.x);
    var z = k;
    var dr = 1.0;
    var mz2 = dot(z, z);
    var trap = mz2;
    for (var i = 0u; i < iters; i++) {
        if (mz2 >= 1e6) {
            break;
        }
        dr = sqrt(mz2) * dr * 2.0 + 1.0;
        z = vec4f(z.x * z.x - z.y * z.y - z.z * z.z - z.w * z.w + k.x, z.x * z.y * 2.0 + k.y, z.x * z.z * 2.0 + k.z, z.x * z.w * 2.0 + k.w);
        mz2 = dot(z, z);
        trap = min(trap, mz2);
    }
    let r = sqrt(max(mz2, 1e-12));
    return vec2f(select(r * log(r) * 0.5 / dr, 0.0, mz2 < 1e6), clamp(sqrt(trap), 0.0, 1.0));
}
"#;

/// The real axis's offset into k[2]: the Mandelbrot set's centre for power
/// 2, the origin for the Multibrot sets (symmetric about it).
fn tetrabrot_prep(k: &mut [f32; K]) {
    k[2] = if k[0] == 2.0 { RE } else { 0.0 };
}

/// One complex iteration of z^power + c, and its derivative's size
/// (|dz| <- power |z|^(power-1) |dz| + 1: a bound, so the estimate stays
/// conservative); squaring directly for power 2.
#[inline(always)]
fn tetrabrot_step(x: F8, y: F8, power: f32, cx: F8, cy: F8, dr: F8) -> (F8, F8, F8) {
    if power == 2.0 {
        let r = len2(x, y);
        (x * x - y * y + cx, (x * y).mul_add(s(2.0), cy), (r * dr).mul_add(s(2.0), F8::one()))
    } else {
        let r = len2(x, y).max(s(1e-12));
        let rp1 = r.powf(s(power - 1.0));
        let t = y.atan2(x) * power;
        let rp = rp1 * r;
        (rp.mul_add(t.cos(), cx), rp.mul_add(t.sin(), cy), (rp1 * dr).mul_add(s(power), F8::one()))
    }
}

/// The Tetrabrot's distance estimate: in idempotent coordinates the point
/// x + y i1 + z i2 + w j is the pair (x + w) + (y - z) i and (x - w) + (y +
/// z) i, each squared on its own (see `bicomplex`).
fn tetrabrot(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let re = p.x + k[2];
    bicomplex((re + k[1], p.y - p.z), (re - k[1], p.y + p.z), k[0], iters)
}

/// A bicomplex Multibrot set's distance estimate from the point's two
/// idempotent components, each iterated as a complex number: the set is
/// the points whose two orbits both stay bounded, and the distance to it
/// sqrt((d1² + d2²) / 2) of the two complex estimates (Brouillette, Parisé
/// and Rochon, 2019; an orbit still bounded counts 0). Its orbit trap: the
/// nearest either orbit comes to the centre.
#[inline(always)]
fn bicomplex(a: (F8, F8), b: (F8, F8), power: f32, iters: u32) -> (F8, F8) {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (mut x1, mut y1, mut x2, mut y2) = (ax, ay, bx, by);
    let (mut dr1, mut dr2) = (F8::one(), F8::one());
    let bail = s(1e6);
    let mut r1 = y1.mul_add(y1, x1 * x1);
    let mut r2 = y2.mul_add(y2, x2 * x2);
    let mut trap = r1.min(r2);
    let mut act1: M8 = r1.lt(bail);
    let mut act2: M8 = r2.lt(bail);
    for _ in 0..iters {
        if (act1 | act2).none() {
            break;
        }
        let (nx, ny, nd) = tetrabrot_step(x1, y1, power, ax, ay, dr1);
        x1 = F8::select(act1, nx, x1);
        y1 = F8::select(act1, ny, y1);
        dr1 = F8::select(act1, nd, dr1);
        let (nx, ny, nd) = tetrabrot_step(x2, y2, power, bx, by, dr2);
        x2 = F8::select(act2, nx, x2);
        y2 = F8::select(act2, ny, y2);
        dr2 = F8::select(act2, nd, dr2);
        r1 = y1.mul_add(y1, x1 * x1);
        r2 = y2.mul_add(y2, x2 * x2);
        trap = F8::select(act1, trap.min(r1), trap);
        trap = F8::select(act2, trap.min(r2), trap);
        act1 &= r1.lt(bail);
        act2 &= r2.lt(bail);
    }
    let de = |r2: F8, dr: F8, act: M8| {
        let r = r2.max(s(1e-12)).sqrt();
        F8::select(act, F8::zero(), (r * r.ln() * 0.5 / dr).max(F8::zero()))
    };
    let (d1, d2) = (de(r1, dr1, act1), de(r2, dr2, act2));
    ((d2.mul_add(d2, d1 * d1) * 0.5).sqrt(), trap.sqrt().saturate())
}

const TETRABROT: &str = r#"
fn tetrabrot_step(z: vec2f, power: f32, k: vec2f, dr: f32) -> vec3f {
    if (power == 2.0) {
        let r = length(z);
        return vec3f(z.x * z.x - z.y * z.y + k.x, z.x * z.y * 2.0 + k.y, r * dr * 2.0 + 1.0);
    }
    let r = max(length(z), 1e-12);
    let rp1 = pow(r, power - 1.0);
    let t = atan2(z.y, z.x) * power;
    let rp = rp1 * r;
    return vec3f(rp * cos(t) + k.x, rp * sin(t) + k.y, rp1 * dr * power + 1.0);
}

fn tetrabrot_de(z: vec2f, dr: f32, inside: bool) -> f32 {
    let r = sqrt(max(dot(z, z), 1e-12));
    return select(max(r * log(r) * 0.5 / dr, 0.0), 0.0, inside);
}

fn tetrabrot(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let re = p.x + a.z;
    let ka = vec2f(re + a.y, p.y - p.z);
    let kb = vec2f(re - a.y, p.y + p.z);
    var z1 = ka;
    var z2 = kb;
    var dr1 = 1.0;
    var dr2 = 1.0;
    var r1 = dot(z1, z1);
    var r2 = dot(z2, z2);
    var trap = min(r1, r2);
    var in1 = r1 < 1e6;
    var in2 = r2 < 1e6;
    for (var i = 0u; i < iters; i++) {
        if (!in1 && !in2) {
            break;
        }
        if (in1) {
            let n = tetrabrot_step(z1, power, ka, dr1);
            z1 = n.xy;
            dr1 = n.z;
            r1 = dot(z1, z1);
            trap = min(trap, r1);
            in1 = r1 < 1e6;
        }
        if (in2) {
            let n = tetrabrot_step(z2, power, kb, dr2);
            z2 = n.xy;
            dr2 = n.z;
            r2 = dot(z2, z2);
            trap = min(trap, r2);
            in2 = r2 < 1e6;
        }
    }
    let d1 = tetrabrot_de(z1, dr1, in1);
    let d2 = tetrabrot_de(z2, dr2, in2);
    return vec2f(sqrt((d1 * d1 + d2 * d2) * 0.5), clamp(sqrt(trap), 0.0, 1.0));
}
"#;

/// The slice's angle (degrees) into its cosine and sine, k[2] and k[3].
fn hypercomplex_prep(k: &mut [f32; K]) {
    let t = k[0].to_radians();
    k[2] = t.cos();
    k[3] = t.sin();
}

/// The hypercomplex set's distance estimate. Davenport's commutative
/// quaternions x + y i + u j + v k (i² = j² = -1, k² = 1, ij = k) are the
/// bicomplex numbers, so the point's idempotent components are (x + v) +
/// (y - u) i and (x - v) + (y + u) i, iterated as the Tetrabrot's are; the
/// slice's third axis is k turned towards j by the angle, and `w` moves it
/// along the fourth.
fn hypercomplex(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (cs, sn, w) = (k[2], k[3], s(k[1]));
    let u = p.z * sn + w * cs;
    let v = p.z * cs - w * sn;
    let re = p.x + RE;
    bicomplex((re + v, p.y - u), (re - v, p.y + u), 2.0, iters)
}

const HYPERCOMPLEX: &str = r#"
fn hypercomplex_orbit(k: vec2f, iters: u32) -> vec3f {
    var z = k;
    var dr = 1.0;
    var r2 = dot(z, z);
    var trap = r2;
    for (var i = 0u; i < iters; i++) {
        if (r2 >= 1e6) {
            break;
        }
        let r = sqrt(r2);
        z = vec2f(z.x * z.x - z.y * z.y + k.x, z.x * z.y * 2.0 + k.y);
        dr = r * dr * 2.0 + 1.0;
        r2 = dot(z, z);
        trap = min(trap, r2);
    }
    let r = sqrt(max(r2, 1e-12));
    return vec3f(select(max(r * log(r) * 0.5 / dr, 0.0), 0.0, r2 < 1e6), trap, 0.0);
}

fn hypercomplex(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let u = p.z * a.w + a.y * a.z;
    let v = p.z * a.z - a.y * a.w;
    let re = p.x - 0.75;
    let o1 = hypercomplex_orbit(vec2f(re + v, p.y - u), iters);
    let o2 = hypercomplex_orbit(vec2f(re - v, p.y + u), iters);
    let trap = min(o1.y, o2.y);
    return vec2f(sqrt((o1.x * o1.x + o2.x * o2.x) * 0.5), clamp(sqrt(trap), 0.0, 1.0));
}
"#;
