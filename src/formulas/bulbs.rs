//! Mandelbulb relatives: the bulb's power map with a different constant,
//! sign or algebra.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "juliabulb",
        category: "Fractals: bulbs",
        desc: "The Mandelbulb's Julia set: the same power map in spherical coordinates, with a fixed constant added at every step instead of the point itself. Each constant gives a different sculpture; the default stacks ringed, filigreed tiers like an ornate urn. Juliabulbs came with the Mandelbulb itself in 2009 (Paul Nylander, Daniel White and others on Fractal Forums).",
        params: &[
            Param::num("power", &[8.0], 2.0, 16.0, "The exponent (2 to 16): 8 is the classic bulb's. Higher powers give more, finer lobes."),
            Param::num("c", &[0.2, -0.64, 0.2], -1.5, 1.5, "The constant added at every step: small changes reshape it completely. Near the origin it is a lumpy ball; with a large vertical part (as the default) it stacks into ringed tiers; much past 0.8 out it breaks into islands."),
        ],
        iterations: (10, 32, "Escape-time iterations: more = finer detail, at a pow, two atan2, a sin and a cos per point each."),
        bound: 1.4,
        lod: None,
        step: 0.9,
        example: r#"{"type":"juliabulb","scale":0.6}"#,
        prep: None,
        cpu: juliabulb,
        wgsl: JULIABULB,
        wgsl_fn: "juliabulb",
    },
    Formula {
        name: "mandelbar",
        category: "Fractals: bulbs",
        desc: "The 3D Mandelbar, or Tricorn bulb: z is conjugated before it is squared, so the slice facing you is the three-cornered Tricorn and the level slice is the Mandelbrot set, joined by smooth, swirling folds. Eric Baird's formula, rendered by Paul Nylander in 2009; as Mandelbulber 2's Mandelbar.",
        params: &[Param::whole("power", 2.0, 2.0, 8.0, "The power (whole, 2 to 8): 2 is the Tricorn. Higher powers give the multicorns, whose upright slice has power + 1 corners and whose level slice is the Multibrot set of that power.")],
        iterations: (12, 32, "Escape-time iterations: more = finer bulbs along the needle and the arms, at a few multiplications per power per point each."),
        bound: 1.8,
        lod: None,
        step: 0.9,
        example: r#"{"type":"mandelbar","scale":0.6}"#,
        prep: Some(mandelbar_prep),
        cpu: mandelbar,
        wgsl: MANDELBAR,
        wgsl_fn: "mandelbar",
    },
    Formula {
        name: "burning_ship",
        category: "Fractals: bulbs",
        desc: "The Burning Ship in 3D: the Mandelbulb's power map with every component made positive first, so the slice facing you is Michelitsch and Rössler's Burning Ship (1992), masts up, and in depth it grows into a jagged hull. As laser blaster posted it to Fractal Forums (2014) and Mandelbulber 2's Mandelbulb Abs has it.",
        params: &[
            Param::num("power", &[2.0], 2.0, 8.0, "The exponent (2 to 8): 2 is the Burning Ship. 3 and 4 fold it into a rounder hull with more points; higher powers into a knobbly ball."),
            Param::whole("polar", 0.0, 0.0, 1.0, "0: the original form (laser blaster's and Mandelbulber's): latitudes doubled about a pole running front to back, a rounder hull with swirled rings behind. 1: polar angles doubled about the keel instead: a crisper, more crystalline hull, and a truer estimate."),
        ],
        iterations: (12, 32, "Escape-time iterations: more = finer spires and smaller ships along the keel, at a pow, two atan2, a sin and a cos per point each."),
        bound: 1.7,
        lod: None,
        step: 0.6,
        example: r#"{"type":"burning_ship","scale":0.6}"#,
        prep: Some(burning_ship_prep),
        cpu: burning_ship,
        wgsl: BURNING_SHIP,
        wgsl_fn: "burning_ship",
    },
    Formula {
        name: "power2_bulb",
        category: "Fractals: bulbs",
        desc: "The first 3D Mandelbulb: Daniel White's 2007 triplex square, the quadratic bulb Paul Nylander rendered in July 2009 before higher powers made the Mandelbulb famous. The slice facing you is exactly the Mandelbrot set; in depth it swells into smeared, whipped-cream folds.",
        params: &[
            Param::whole("julia", 0.0, 0.0, 1.0, "0: the bulb itself. 1: its Julia set for the constant c instead, the first Juliabulb (Nylander, July 2009)."),
            Param::num("c", &[-0.12, 0.74, 0.0], -1.5, 1.5, "The Julia set's constant (when julia is 1). With a z of 0 the slice facing you is that 2D Julia set, puffed into 3D around it; the default is the Douady rabbit's."),
        ],
        iterations: (12, 32, "Escape-time iterations: more = finer bulbs and smears, at a few multiplications and a square root per point each."),
        bound: 2.0,
        lod: None,
        step: 0.9,
        example: r#"{"type":"power2_bulb","scale":0.6}"#,
        prep: None,
        cpu: power2_bulb,
        wgsl: POWER2_BULB,
        wgsl_fn: "power2_bulb",
    },
];

/// The juliabulb's distance estimate (trigonometric form, as vm.rs
/// mandelbulb with c for the point), and its orbit trap: the nearest the
/// orbit comes to the centre.
fn juliabulb(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let power = k[0];
    let (cx, cy, cz) = (s(k[1]), s(k[2]), s(k[3]));
    let n1 = s(power - 1.0);
    let bail = s(2.0);
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
        let theta = len2(z.x, z.z).atan2(z.y) * power;
        let phi = z.z.atan2(z.x) * power;
        let rp1 = rs.powf(n1);
        let ndr = rp1 * dr * power;
        let zr = rp1 * rs;
        let st = theta.sin();
        let nz = V8::new(zr * st * phi.cos() + cx, zr * theta.cos() + cy, zr * st * phi.sin() + cz);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap * 0.5).saturate())
}

const JULIABULB: &str = r#"
fn juliabulb(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let k = a.yzw;
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 2.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let theta = atan2(length(z.xz), z.y) * power;
        let phi = atan2(z.z, z.x) * power;
        let rp1 = pow(rs, power - 1.0);
        dr = rp1 * dr * power;
        let zr = rp1 * rs;
        let st = sin(theta);
        z = vec3f(zr * st * cos(phi), zr * cos(theta), zr * st * sin(phi)) + k;
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap * 0.5, 0.0, 1.0));
}
"#;

/// Bailout radius of the quadratic sets. They reach out to 2 (the needle's
/// tip), where a bailout of 2 would make the estimate overshoot, and the
/// log estimate is continuous across a step only once c is small beside r.
/// Their traps are the nearest the orbit comes to the centre, less 0.3: on
/// their surfaces the nearest is about 0.2 to 1.2, so the trap spans 0..1.
const BAIL: f32 = 256.0;

/// k[1]: how far along x the Mandelbar is moved to centre it (the Tricorn's
/// needle reaches -2 and its arms 1; the multicorns are centred already).
fn mandelbar_prep(k: &mut [f32; K]) {
    k[1] = if k[0] == 2.0 { -0.5 } else { 0.0 };
}

/// The 3D Mandelbar (Eric Baird's, as Nylander and Mandelbulber have it):
/// z ↦ conj(z)ⁿ + c with x real and y, z both imaginary, the conjugate
/// flipping y only, so the xy plane is the multicorn and the xz plane the
/// Multibrot. The power is the complex power of (x, |yz|) by repeated
/// multiplication (exact, no trigonometry). Running-derivative estimate
/// (|zⁿ| = rⁿ).
fn mandelbar(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let n = k[0] as u32;
    let c = V8::new(p.x + k[1], p.y, p.z);
    let mut z = c;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(BAIL));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rho = len2(z.y, z.z);
        let (mut wr, mut wi, mut rp) = (z.x, rho, F8::one());
        for _ in 1..n {
            (wr, wi) = (wr * z.x - wi * rho, wr * rho + wi * z.x);
            rp *= r;
        }
        // (wi has a factor rho: on the x axis it is 0 over 1e-12)
        let q = wi / rho.max(s(1e-12));
        let nz = V8::new(wr, -(q * z.y), q * z.z) + c;
        let ndr = (rp * dr * n as f32 + 1.0).min(s(1e30));
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(s(BAIL));
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr, (trap - 0.3).saturate())
}

const MANDELBAR: &str = r#"
fn mandelbar(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let n = u32(a.x);
    let k = vec3f(p.x + a.y, p.y, p.z);
    var z = k;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 256.0) {
            break;
        }
        let rho = length(z.yz);
        var wr = z.x;
        var wi = rho;
        var rp = 1.0;
        for (var j = 1u; j < n; j++) {
            let t = wr * z.x - wi * rho;
            wi = wr * rho + wi * z.x;
            wr = t;
            rp *= r;
        }
        let q = wi / max(rho, 1e-12);
        z = vec3f(wr, -(q * z.y), q * z.z) + k;
        dr = min(rp * dr * f32(n) + 1.0, 1e30);
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / dr, clamp(trap - 0.3, 0.0, 1.0));
}
"#;

/// k[2..5]: the offset that centres the Burning Ship (power 2, in either
/// form), fading out by power 3, whose hulls are centred already.
fn burning_ship_prep(k: &mut [f32; K]) {
    let t = (3.0 - k[0]).clamp(0.0, 1.0);
    let off = if k[1] > 0.5 { [-0.44, -0.34, -0.4] } else { [-0.475, -0.325, -0.625] };
    for i in 0..3 {
        k[2 + i] = off[i] * t;
    }
}

/// The 3D Burning Ship: the bulb's power map of the orbit's absolute
/// values (laser blaster's, Fractal Forums 2014, and Mandelbulber's
/// Mandelbulb Abs). In the latitude form (pole z) the equator maps to
/// itself by complex powering; in the polar form (pole x) a meridian plane
/// does. Either way the plane z = 0 is the 2D Burning Ship exactly. The
/// latitude form is the original; its map jumps across the pole, so its
/// estimate overshoots there (hence the step). The polar form is
/// continuous. y is flipped: the ship's masts up, as the 2D one is always
/// shown. Running-derivative estimate (the folds are isometries).
fn burning_ship(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let power = k[0];
    let polar = k[1] > 0.5;
    let n1 = s(power - 1.0);
    let c = V8::new(p.x + k[2], k[3] - p.y, p.z + k[4]);
    let mut z = c;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(BAIL));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let a = z.abs();
        let rs = r.max(s(1e-6));
        let rp1 = rs.powf(n1);
        let ndr = (rp1 * dr * power + 1.0).min(s(1e30));
        let zr = rp1 * rs;
        let zn = if polar {
            let th = len2(a.y, a.z).atan2(a.x) * power;
            let ph = a.z.atan2(a.y) * power;
            let st = th.sin() * zr;
            V8::new(zr * th.cos(), st * ph.cos(), st * ph.sin())
        } else {
            let th = a.z.atan2(len2(a.x, a.y)) * power;
            let ph = a.y.atan2(a.x) * power;
            let ct = th.cos() * zr;
            V8::new(ct * ph.cos(), ct * ph.sin(), zr * th.sin())
        };
        let nz = zn + c;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(s(BAIL));
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr, (trap - 0.3).saturate())
}

const BURNING_SHIP: &str = r#"
fn burning_ship(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let polar = a.y > 0.5;
    let k = vec3f(p.x + a.z, a.w - p.y, p.z + b.x);
    var z = k;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 256.0) {
            break;
        }
        let w = abs(z);
        let rs = max(r, 1e-6);
        let rp1 = pow(rs, power - 1.0);
        dr = min(rp1 * dr * power + 1.0, 1e30);
        let zr = rp1 * rs;
        if (polar) {
            let th = atan2(length(w.yz), w.x) * power;
            let ph = atan2(w.z, w.y) * power;
            let st = sin(th) * zr;
            z = vec3f(zr * cos(th), st * cos(ph), st * sin(ph)) + k;
        } else {
            let th = atan2(w.z, length(w.xy)) * power;
            let ph = atan2(w.y, w.x) * power;
            let ct = cos(th) * zr;
            z = vec3f(ct * cos(ph), ct * sin(ph), zr * sin(th)) + k;
        }
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / dr, clamp(trap - 0.3, 0.0, 1.0));
}
"#;

/// The quadratic Mandelbulb: Daniel White's triplex square (2007; pole z,
/// the latitude doubled and negated), in its exact non-trigonometric form.
/// The z = 0 plane is the Mandelbrot set. On the pole axis, where the map
/// has no limit, it takes the one along y = 0 (so |z²| stays r²). The bulb
/// is moved along x to centre it; the Julia sets are centred already.
/// Analytic estimate (|z²| = |z|², so the running derivative 2·r·dr (+ 1
/// for the bulb) holds).
fn power2_bulb(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let julia = k[0] > 0.5;
    let (mut z, c, plus) = if julia {
        (p, V8::new(s(k[1]), s(k[2]), s(k[3])), 0.0)
    } else {
        let c = V8::new(p.x - 0.62, p.y, p.z);
        (c, c, 1.0)
    };
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(s(BAIL));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rho2 = z.y.mul_add(z.y, z.x * z.x);
        let inv = F8::one() / (rho2 + 1e-20);
        let h = rho2 - z.z * z.z;
        let cos2 = (z.x * z.x - z.y * z.y + 1e-20) * inv;
        let sin2 = z.x * z.y * 2.0 * inv;
        let nz = V8::new(cos2 * h, sin2 * h, z.z * rho2.sqrt() * -2.0) + c;
        let ndr = r * dr * 2.0 + plus;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(s(BAIL));
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap - 0.3).saturate())
}

const POWER2_BULB: &str = r#"
fn power2_bulb(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let julia = a.x > 0.5;
    let m = vec3f(p.x - 0.62, p.y, p.z);
    var z = select(m, p, julia);
    let k = select(m, a.yzw, julia);
    let plus = select(1.0, 0.0, julia);
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 256.0) {
            break;
        }
        let rho2 = z.x * z.x + z.y * z.y;
        let inv = 1.0 / (rho2 + 1e-20);
        let h = rho2 - z.z * z.z;
        let cos2 = (z.x * z.x - z.y * z.y + 1e-20) * inv;
        let sin2 = z.x * z.y * 2.0 * inv;
        z = vec3f(cos2 * h, sin2 * h, z.z * sqrt(rho2) * -2.0) + k;
        dr = r * dr * 2.0 + plus;
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap - 0.3, 0.0, 1.0));
}
"#;
