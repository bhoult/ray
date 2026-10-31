//! More Mandelbulb relatives, ported from Mandelbulber 2: Benesi's power-2
//! bulb, Kosalos's tweaked bulb, the Lambdabulb and the sine/cosine bulb.
//! Mandelbulber is z-up: kosalos and sincos_bulb take (x, y, z) as its
//! (x, -z, y) so their poles point up, benesi stands its axis up, and the
//! Lambdabulb keeps Mandelbulber's frame (its example tips it to the camera).

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "benesi",
        category: "Fractals: bulbs",
        desc: "Matt Benesi's power-2 bulb: the Mandelbrot set spun about its real axis, the plane across the axis squared like a complex number, and x's sign flipped where it outgrows that plane's radius. Stood on end it is a draped, fluted spire with a needle on top. Formula by Matt Benesi (Fractal Forums); ported from Mandelbulber 2.",
        params: &[
            Param::num("julia", &[0.0], 0.0, 1.0, "0: the Mandelbrot-type set (each point adds itself); 1: the Julia set of `c`; in between morphs one into the other."),
            Param::num("c", &[0.85, -0.34, 0.41], -1.5, 1.5, "The Julia constant (with julia above 0): Mandelbulber's example's. Small changes reshape it completely."),
            Param::num("offset", &[0.0, 0.0, 0.0], -0.5, 0.5, "Added to x, y and z after each square: x stretches or pinches the bulbs along the axis, y and z skew them (-0.5 to 0.5 is useful)."),
        ],
        iterations: (20, 64, "Escape-time iterations: more = finer detail, cheaply (no trigonometry)."),
        bound: 2.9,
        lod: None,
        step: 0.9,
        example: r#"{"type":"benesi","scale":0.6}"#,
        prep: None,
        cpu: benesi,
        wgsl: BENESI,
        wgsl_fn: "benesi",
    },
    Formula {
        name: "kosalos",
        category: "Fractals: bulbs",
        desc: "Kosalos's bulb (fractalforums.org; in Mandelbulber 2 since 2019): a power-9 bulb whose polar angle is squashed the more, the less the orbit moved on its last step. The first step moves least, so it is squashed most, which stacks the bulb into tiers like a pagoda under a star-shaped crown. Ported from Mandelbulber 2.",
        params: &[
            Param::num("power", &[9.0], 3.0, 16.0, "The exponent (3 to 16): 9 is Mandelbulber's default, and the number of tiers' ribs. Higher powers give more, finer lobes."),
            Param::num("tweak", &[0.2], -4.0, 4.0, "How hard a short step squashes the polar angle: 0 is a plain bulb, 0.1 to 0.3 stacks it into tiers, beyond about ±1 it flattens into a ribbed pumpkin."),
            Param::num("tweak_offset", &[0.0], 0.0, 10.0, "Added to the step's length before it divides the tweak: 1 to 5 softens the first step's squash, back towards a plain bulb."),
            Param::num("lift", &[0.0], -1.0, 1.0, "Added to the vertical component after each step: shifts the lobes up or down the axis."),
            Param::whole("add_point", 0.0, 0.0, 1.0, "0: each step adds the power to the previous orbit point (Mandelbulber's default); 1: to the starting point, like the Mandelbulb."),
        ],
        iterations: (10, 32, "Escape-time iterations: more = finer detail, at a pow, two atan2, a sin and a cos per point each."),
        bound: 2.0,
        lod: None,
        step: 0.9,
        example: r#"{"type":"kosalos","scale":0.6}"#,
        prep: Some(kosalos_prep),
        cpu: kosalos,
        wgsl: KOSALOS,
        wgsl_fn: "kosalos",
    },
    Formula {
        name: "lambda_bulb",
        category: "Fractals: bulbs",
        desc: "The Lambdabulb: the logistic map's Julia set made 3D, z = c(z - z^n) in Paul Nylander's triplex numbers, Ron Barnett's idea (Fractal Forums, 2009). A twisted, cratered lump whose rims are seamed with tiny buds. Ported from Mandelbulber 2.",
        params: &[
            Param::num("power", &[4.0], 2.0, 12.0, "The exponent n (2 to 12): Nylander's Lambdabulb uses 4. Higher powers give more lobes."),
            Param::num("c", &[1.0625, 0.2375, 0.0], -2.0, 2.0, "The constant multiplied in at every step, as a triplex number (x and y across its axis, z along it): Ron Barnett's 1.0625 + 0.2375i. Its length sets how fat it is (much under 1 swells it past its box), its angle the twist; z tips it out of the plane."),
        ],
        iterations: (12, 40, "Escape-time iterations: more = finer detail, at a pow, two atan2, a sin and a cos per point each."),
        bound: 2.0,
        lod: None,
        step: 0.9,
        example: r#"{"type":"lambda_bulb","scale":0.6,"rotation":[-50,0,0]}"#,
        prep: Some(lambda_prep),
        cpu: lambda_bulb,
        wgsl: LAMBDA_BULB,
        wgsl_fn: "lambda_bulb",
    },
    Formula {
        name: "sincos_bulb",
        category: "Fractals: bulbs",
        desc: "A Mandelbulb whose polar angle is taken by acos, asin or a blend of the two and put back as a latitude, so sine and cosine trade places. The default blend, a quarter of the way to asin, halves the polar angle's power: onion domes draped in folds and studded with buds. Graeme McLaren's variant (Mandelbulber 2, 2021) of Daniel White and Paul Nylander's bulb.",
        params: &[
            Param::num("power", &[9.0], 3.0, 16.0, "The exponent (3 to 16): 9 is Mandelbulber's default. Higher powers give more, finer lobes."),
            Param::num("mix", &[0.25], 0.0, 1.0, "Blends the polar angle from acos (0: sine and cosine fully swapped, which looks much like the ordinary bulb) to asin (1: the ordinary bulb). Between, the angle's power is n(1 - 2 mix): 0.25 halves it into onion domes, 0.15 to 0.35 is useful, and at 0.5 the bulb collapses."),
            Param::num("z_scale", &[1.0], 0.75, 1.5, "Scales the vertical component after each step: below 1 squashes the domes, above 1 stretches them."),
            Param::num("offset", &[0.0, 0.0, 0.0], -0.5, 0.5, "Added after each step (x, y, z with z up): shifts the orbit off centre and makes it lopsided."),
            Param::num("angles", &[0.0, 0.0], -180.0, 180.0, "Degrees added to the longitude and the polar angle before the power: they twist and tilt the lobes."),
        ],
        iterations: (10, 32, "Escape-time iterations: more = finer detail, at a pow, two atan2, a sin and a cos per point each."),
        bound: 2.0,
        lod: None,
        step: 0.9,
        example: r#"{"type":"sincos_bulb","scale":0.6}"#,
        prep: Some(sincos_prep),
        cpu: sincos_bulb,
        wgsl: SINCOS_BULB,
        wgsl_fn: "sincos_bulb",
    },
];

/// Benesi's power-2 bulb (Mandelbulber's benesi.cl, its point added after
/// the step, here blended with a Julia constant): x² - (y² + z²) along the
/// axis and the yz plane squared, scaled by -2|x| over its radius. Its axis
/// stands up (Mandelbulber's x is -y here, its y -z and its z x), the needle
/// on top, and the Mandelbrot-type set is centred. Its trap: the nearest the
/// orbit comes to the axis.
fn benesi(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let j = k[0];
    let q = V8::new(0.08 * (1.0 - j) - p.y, 0.46 * (1.0 - j) - p.z, p.x);
    let add = V8::new(q.x * (1.0 - j) + k[1] * j, q.y * (1.0 - j) + k[2] * j, q.z * (1.0 - j) + k[3] * j);
    let (ox, oy, oz) = (s(k[4]), s(k[5]), s(k[6]));
    // (the sign flip is on where the added point's x is not negative)
    let flip = add.x.ge(F8::zero());
    let bail = s(100.0);
    let mut z = q;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let ndr = dr * r * 2.0 + 1.0;
        let (xx, yy, zz) = (z.x * z.x, z.y * z.y, z.z * z.z);
        let r1 = yy + zz;
        let tp = r1.sqrt();
        let x0 = xx - r1;
        let tx = F8::select(flip & z.x.ge(tp), -x0 - ox, x0 + ox);
        let m = z.x.abs() * -2.0 / tp.max(s(1e-20));
        let nz = V8::new(tx, m * (yy - zz) + oy, m * 2.0 * z.y * z.z + oz) + add;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(len2(z.y, z.z)), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), trap.saturate())
}

const BENESI: &str = r#"
fn benesi(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let j = a.x;
    let q = vec3f(0.08 * (1.0 - j) - p.y, 0.46 * (1.0 - j) - p.z, p.x);
    let add = q * (1.0 - j) + a.yzw * j;
    let off = b.xyz;
    let flip = add.x >= 0.0;
    var z = q;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 100.0) {
            break;
        }
        dr = dr * r * 2.0 + 1.0;
        let zz = z * z;
        let r1 = zz.y + zz.z;
        let tp = sqrt(r1);
        let x0 = zz.x - r1;
        let tx = select(x0 + off.x, -x0 - off.x, flip && z.x >= tp);
        let m = abs(z.x) * -2.0 / max(tp, 1e-20);
        z = vec3f(tx, m * (zz.y - zz.z) + off.y, m * 2.0 * z.y * z.z + off.z) + add;
        r = length(z);
        trap = min(trap, length(z.yz));
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;

/// The tweak's strength and offset, as the kernels use them (Mandelbulber's
/// are hundredths).
fn kosalos_prep(k: &mut [f32; K]) {
    k[1] *= 0.01;
    k[2] *= 0.01;
}

/// Kosalos's bulb (Mandelbulber's mandelbulb_kosalos.cl): z-up, its polar
/// angle's radial part scaled by 1 - tweak / (|last step| + offset), the
/// step measured from the previous orbit point (the first from the point
/// itself, nudged by 0.001 on each axis). Its trap: the nearest the orbit
/// comes to the equator, as |z| / r.
fn kosalos(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let power = k[0];
    let (tw, toff, lift) = (s(k[1]), s(k[2]), s(k[3]));
    let add_point = k[4] > 0.5;
    let n1 = s(power - 1.0);
    let bail = s(10.0);
    let c = V8::new(p.x, -p.z, p.y);
    let mut z = c;
    let mut old = c;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-6));
        let d = V8::new(old.x - z.x + 0.001, old.y - z.y + 0.001, old.z - z.z + 0.001);
        old = z;
        let tweak = F8::one() - tw / (d.length() + toff).max(s(1e-6));
        // (the tweak stretches the polar angle by |t| r² / (z² + t² xy²): the
        // derivative takes it where it is above 1)
        let (xy2, z2) = (z.x * z.x + z.y * z.y, z.z * z.z);
        let st = (tweak.abs() * (xy2 + z2) / tweak.mul_add(tweak * xy2, z2).max(s(1e-12))).max(F8::one());
        let theta = (xy2.sqrt() * tweak).atan2(z.z) * power;
        let phi = z.y.atan2(z.x) * power;
        let rp1 = rs.powf(n1);
        let pwr = rp1 * rs;
        let ss = theta.sin() * pwr;
        let f = V8::new(ss * phi.cos(), ss * phi.sin(), pwr * theta.cos() + lift);
        let (nz, ndr) = if add_point { (f + c, rp1 * st * power * dr + 1.0) } else { (z + f, rp1 * st * power * dr + dr + 1.0) };
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(z.z.abs() / r.max(s(1e-6))), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), trap.saturate())
}

const KOSALOS: &str = r#"
fn kosalos(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let tw = a.y;
    let toff = a.z;
    let lift = a.w;
    let add_point = b.x > 0.5;
    let k = vec3f(p.x, -p.z, p.y);
    var z = k;
    var old = k;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let d = old - z + vec3f(0.001);
        old = z;
        let tweak = 1.0 - tw / max(length(d) + toff, 1e-6);
        let xy2 = z.x * z.x + z.y * z.y;
        let z2 = z.z * z.z;
        let st = max(abs(tweak) * (xy2 + z2) / max(tweak * (tweak * xy2) + z2, 1e-12), 1.0);
        let theta = atan2(sqrt(xy2) * tweak, z.z) * power;
        let phi = atan2(z.y, z.x) * power;
        let rp1 = pow(rs, power - 1.0);
        let pwr = rp1 * rs;
        let ss = sin(theta) * pwr;
        let f = vec3f(ss * cos(phi), ss * sin(phi), pwr * cos(theta) + lift);
        if (add_point) {
            z = f + k;
            dr = rp1 * st * power * dr + 1.0;
        } else {
            z = z + f;
            dr = rp1 * st * power * dr + dr + 1.0;
        }
        r = length(z);
        trap = min(trap, abs(z.z) / max(r, 1e-6));
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;

/// The constant's x and y kept off the axis (as Mandelbulber does: the
/// triplex product divides by their length), then that length and the
/// constant's (the derivative's scale) in k[4], k[5].
fn lambda_prep(k: &mut [f32; K]) {
    for i in [1, 2] {
        if k[i].abs() < 1e-6 {
            k[i] = if k[i] > 0.0 { 1e-6 } else { -1e-6 };
        }
    }
    k[4] = (k[1] * k[1] + k[2] * k[2]).sqrt();
    k[5] = (k[1] * k[1] + k[2] * k[2] + k[3] * k[3]).sqrt();
}

/// The Lambdabulb (Mandelbulber's mandelbulb_lambda.cl, in its frame): z -
/// z^n by the latitude (asin) power, then multiplied by the constant in
/// Nylander's triplex product. Julia-type: the point only starts the orbit.
/// Its trap: the nearest the orbit comes to the axis.
fn lambda_bulb(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let power = k[0];
    let (lx, ly, lz) = (s(k[1]), s(k[2]), s(k[3]));
    let (r2, lam) = (s(k[4]), k[5]);
    let n1 = s(power - 1.0);
    let bail = s(10.0);
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
        let th = z.z.atan2(len2(z.x, z.y)) * power;
        let ph = z.y.atan2(z.x) * power;
        let rp1 = rs.powf(n1);
        let rp = rp1 * rs;
        let ct = th.cos() * rp;
        let w = V8::new(z.x - ct * ph.cos(), z.y - ct * ph.sin(), z.z - th.sin() * rp);
        let r1 = len2(w.x, w.y);
        let t = F8::one() - w.z * lz / (r1 * r2).max(s(1e-20));
        let nz = V8::new(t * (w.x * lx - w.y * ly), t * (lx * w.y + w.x * ly), r2 * w.z + r1 * lz);
        let ndr = (rp1 * power + 1.0) * dr * lam;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(len2(z.x, z.y)), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap * 0.6).saturate())
}

const LAMBDA_BULB: &str = r#"
fn lambda_bulb(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let l = a.yzw;
    let r2 = b.x;
    let lam = b.y;
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let th = atan2(z.z, length(z.xy)) * power;
        let ph = atan2(z.y, z.x) * power;
        let rp1 = pow(rs, power - 1.0);
        let rp = rp1 * rs;
        let ct = cos(th) * rp;
        let w = vec3f(z.x - ct * cos(ph), z.y - ct * sin(ph), z.z - sin(th) * rp);
        let r1 = length(w.xy);
        let t = 1.0 - w.z * l.z / max(r1 * r2, 1e-20);
        z = vec3f(t * (w.x * l.x - w.y * l.y), t * (l.x * w.y + w.x * l.y), r2 * w.z + r1 * l.z);
        dr = (rp1 * power + 1.0) * dr * lam;
        r = length(z);
        trap = min(trap, length(z.xy));
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap * 0.6, 0.0, 1.0));
}
"#;

/// The angle as the kernels use it: acos·(1 - mix) + asin·mix is
/// acos·(1 - 2 mix) + mix·π/2 (asin = π/2 - acos), so with the offsets and
/// the power folded in: k[1] = (1 - 2 mix)·n, k[7] = (mix·π/2 + polar)·n,
/// k[6] = longitude·n (radians), k[8] the derivative's scale for z_scale.
fn sincos_prep(k: &mut [f32; K]) {
    let n = k[0];
    let mix = k[1];
    k[1] = (1.0 - 2.0 * mix) * n;
    k[6] = k[6].to_radians() * n;
    k[7] = (mix * std::f32::consts::FRAC_PI_2 + k[7].to_radians()) * n;
    k[8] = k[2].abs().max(1.0);
}

/// The sine/cosine bulb (Mandelbulber's mandelbulb_sin_cos.cl, its acos /
/// asin blend): z-up, the polar angle taken by acos, asin or between and
/// put back as a latitude (cos for the xy radius, sin for z), then the offset and the
/// point added and z scaled. Its trap: the nearest the orbit comes to the
/// pole axis, as |xy| / r.
fn sincos_bulb(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let power = k[0];
    let (tk, t0, p0) = (k[1], s(k[7]), s(k[6]));
    let zs = k[2];
    let dzs = k[8];
    let c = V8::new(p.x + k[3], -p.z + k[4], p.y + k[5]);
    let n1 = s(power - 1.0);
    let bail = s(10.0);
    let mut z = V8::new(p.x, -p.z, p.y);
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-6));
        let th = len2(z.x, z.y).atan2(z.z) * tk + t0;
        let ph = z.y.atan2(z.x) * power + p0;
        let rp1 = rs.powf(n1);
        let ndr = (rp1 * dr * power + 1.0) * dzs;
        let ct = th.cos() * rp1 * rs;
        let nz = V8::new(ct * ph.cos() + c.x, ct * ph.sin() + c.y, (th.sin() * rp1 * rs + c.z) * zs);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(len2(z.x, z.y) / r.max(s(1e-6))), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), trap.saturate())
}

const SINCOS_BULB: &str = r#"
fn sincos_bulb(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let tk = a.y;
    let zs = a.z;
    let p0 = b.z;
    let t0 = b.w;
    let dzs = c.x;
    let k = vec3f(p.x, -p.z, p.y) + vec3f(a.w, b.x, b.y);
    var z = vec3f(p.x, -p.z, p.y);
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let th = atan2(length(z.xy), z.z) * tk + t0;
        let ph = atan2(z.y, z.x) * power + p0;
        let rp1 = pow(rs, power - 1.0);
        dr = (rp1 * dr * power + 1.0) * dzs;
        let ct = cos(th) * rp1 * rs;
        z = vec3f(ct * cos(ph) + k.x, ct * sin(ph) + k.y, (sin(th) * rp1 * rs + k.z) * zs);
        r = length(z);
        trap = min(trap, length(z.xy) / max(r, 1e-6));
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;
