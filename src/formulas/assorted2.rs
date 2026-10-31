//! More catalog fractals: a distance IFS (knighty's mdifs and Darkbeam's
//! dIFS: Menger sponges copied by folds), msltoe's symmetrised bulbs (his
//! Sym4 and eiffie's eight-fold one), and Jeannot's Mandelnest.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};
use core::f32::consts::PI;

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "difs",
        category: "Fractals: more",
        desc: "A distance IFS: a Menger sponge copied smaller and folded outward ring after ring, so a big sponge stands on a twisting skirt of ever smaller ones that thins to a carpet. Each copy is a true sponge, not a smear. knighty's mdifs (Fragmentarium, 2012) and Darkbeam's dIFS formulas for Mandelbulb 3D; ported from Mandelbulber 2's DIFS Menger.",
        params: &[
            Param::num("fold_scale", &[2.3], 1.5, 3.0, "How much smaller each ring of copies is than the last (1.5 to 3): about 2 packs them edge to edge into a carpet, higher leaves gaps that show the branching, lower crowds them into a dense mound."),
            Param::num("offset", &[2.0, -1.2, 2.0], -2.0, 2.0, "Where the rings go: x and z how far out each ring is folded (0 to 2; negative folds the rings back into the big sponge), y the height the rings sink to (negative: below the big sponge; positive: they climb above it; with fold_up, how far above and below)."),
            Param::num("size", &[0.55], 0.1, 1.0, "The sponge's half size (0.1 to 1): small leaves the copies apart, about 0.5 lets them touch, larger merges them into a stepped mass."),
            Param::whole("sponge_levels", 3.0, 0.0, 6.0, "The sponge's own Menger levels (0 to 6): 0 copies plain cubes; each level carves a finer layer of holes."),
            Param::num("turn", &[12.0], -180.0, 180.0, "Degrees each ring turns about the vertical: 0 lines the copies up in a square; 10 to 30 twists the skirt into spiral arms."),
            Param::whole("fold_up", 0.0, 0.0, 1.0, "1 folds the vertical too: the rings grow above and below the big sponge, a cluster in all eight directions instead of a mound."),
        ],
        iterations: (8, 12, "Rings of copies: each adds copies another fold_scale times smaller; past 8 they are below a pixel from afar (close up more come in by themselves)."),
        bound: 3.0,
        lod: Some((1.5, 12)),
        step: 1.0,
        example: r#"{"type":"difs","scale":0.5}"#,
        prep: Some(difs_prep),
        cpu: difs,
        wgsl: DIFS,
        wgsl_fn: "difs",
    },
    Formula {
        name: "msltoe_sym",
        category: "Fractals: more",
        desc: "msltoe's Sym4 bulb: before each squaring the point's coordinates are sorted by size and given one sign, so the bulb grows with the cube's symmetry. Its Julia sets are ornate carved knots of bulbs. From msltoe's symmetric squarings (Fractal Forums, 2010) as Mandelbulb 3D's Sym4; ported from Mandelbulber 2.",
        params: &[
            Param::whole("julia", 1.0, 0.0, 1.0, "1: the Julia set of `c`; 0: the Mandelbrot form (each point its own constant), a lopsided whipped shape."),
            Param::num("c", &[-1.0, 0.0, 0.0], -1.5, 1.5, "The Julia constant (with julia 1): on the x axis it keeps the symmetry, -0.9 to -1.3 gives the richest knots; off the axis it warps, and far from the centre it falls to dust."),
            Param::num("twist", &[0.0], -180.0, 180.0, "Degrees the square turns about the x axis at each step: 0 is msltoe's; 10 to 30 opens the knot into lace."),
        ],
        iterations: (10, 30, "Escape-time iterations: more = finer bulbs on the bulbs."),
        bound: 2.2,
        lod: None,
        step: 1.0,
        example: r#"{"type":"msltoe_sym","scale":0.6}"#,
        prep: Some(twist_prep),
        cpu: msltoe_sym,
        wgsl: MSLTOE_SYM,
        wgsl_fn: "msltoe_sym",
    },
    Formula {
        name: "eiffie",
        category: "Fractals: more",
        desc: "eiffie's bulb: msltoe's squaring with the point first folded into a one-sixteenth wedge about the vertical axis, so it grows eight-fold, like a fluted stone vase on a spike. Formula by eiffie (Fractal Forums, 2015), from msltoe's eight-fold fold of 2010; ported from Mandelbulber 2.",
        params: &[
            Param::num("y_scale", &[1.0], 0.8, 1.2, "Stretches the squaring's sideways term (0.8 to 1.2): 1 is eiffie's; lower swells the vase and frays its rim into fur, higher shrinks it fast (at 1.2 little is left)."),
            Param::whole("julia", 0.0, 0.0, 1.0, "0: the Mandelbrot form (each point its own constant); 1: the Julia set of `c`."),
            Param::num("c", &[0.0, -1.0, 0.0], -1.5, 1.5, "The Julia constant (with julia 1): on the vertical axis the Julia set keeps the eight-fold symmetry ((0, -1, 0) is a knobbed spindle, eight-pointed in section); (0, 0, -1) flattens it into a rosette ringed by satellites."),
        ],
        iterations: (10, 30, "Escape-time iterations: more = finer detail."),
        bound: 2.5,
        lod: None,
        step: 0.7,
        example: r#"{"type":"eiffie","scale":0.6}"#,
        prep: None,
        cpu: eiffie,
        wgsl: EIFFIE,
        wgsl_fn: "eiffie",
    },
    Formula {
        name: "mandelnest",
        category: "Fractals: more",
        desc: "Jeannot's Mandelnest: the angle of each coordinate to its own axis is multiplied by the power, so the bulb has a cube's symmetry instead of the Mandelbulb's poles: a ball of nested, pinched lobes with a porthole on each axis. Formula by Jeannot (Fractal Forums, Shadertoy); ported from Mandelbulber 2.",
        params: &[
            Param::num("power", &[9.0], 5.0, 16.0, "The power each axis's angle is multiplied by (5 to 16): 9 is Jeannot's; higher packs in more, smaller lobes. Odd whole powers keep the cube's mirror symmetry; others break it."),
            Param::num("shift", &[0.0], -1.0, 1.0, "A phase added to every multiplied angle, in half turns (-1 to 1): 0 is Jeannot's; 0.1 to 0.3 swells some lobes and sinks others into a blockier, more cubic form."),
            Param::whole("julia", 0.0, 0.0, 1.0, "0: the Mandelbrot form (each point its own constant); 1: the Julia set of `c`."),
            Param::num("c", &[-0.8, 0.0, 0.0], -1.5, 1.5, "The Julia constant (with julia 1): (-0.8, 0, 0) is a banded barrel; nearer the centre, a lumpy ball."),
        ],
        iterations: (8, 30, "Escape-time iterations: more = finer lobes along the seams."),
        bound: 1.6,
        lod: None,
        step: 0.8,
        example: r#"{"type":"mandelnest","scale":0.8}"#,
        prep: Some(mandelnest_prep),
        cpu: mandelnest,
        wgsl: MANDELNEST,
        wgsl_fn: "mandelnest",
    },
];

// ---- difs

/// The turn as its cosine and sine, the vertical offset made positive when
/// folded (below the centre a fold has no copies to make), and the shift
/// that centres the mound vertically.
fn difs_prep(k: &mut [f32; K]) {
    let size = k[4];
    let a = k[6].to_radians();
    k[8] = a.cos();
    k[9] = a.sin();
    // (the rings sink or climb to offset.y; folded vertically they spread
    // evenly above and below)
    if k[7] > 0.5 {
        k[2] = k[2].abs();
        k[10] = 0.0;
    } else {
        k[10] = 0.5 * (k[2].min(-size) + k[2].max(size));
    }
}

/// knighty's Menger sponge (mdifs menger1) of half size `size`: its levels
/// folded in, then the box's distance. (Every level, with no bailout: one
/// would make the estimate jump where it stops.)
#[inline(always)]
fn difs_sponge(q: V8, size: f32, levels: u32) -> F8 {
    let mut z = q * (1.0 / size);
    for _ in 0..levels {
        let a = z.abs();
        // sorted so y is the least, then y folded about 1/3
        let x = a.x.max(a.y);
        let y0 = a.x.min(a.y);
        let y1 = y0.min(a.z);
        let zz = y0.max(a.z);
        let y = F8::select(y1.lt(s(1.0 / 3.0)), s(2.0 / 3.0) - y1, y1);
        z = V8::new(x * 3.0 - 2.0, y * 3.0 - 2.0, zz * 3.0 - 2.0);
    }
    let b = V8::new(z.x.abs() - 1.0, z.y.abs() - 1.0, z.z.abs() - 1.0);
    let m = b.max_comp();
    let d = F8::select(m.gt(F8::zero()), b.max_s(F8::zero()).length(), m);
    d * (size / 3f32.powi(levels as i32))
}

/// The distance to the union of every ring's sponge (each one's distance in
/// its ring's frame over the ring's scale: exact, as the folds and turns are
/// isometries), and the trap: which ring is nearest. (Every ring, with no
/// bailout: a bound standing in for the rings past one would make the
/// estimate jump where it takes over.)
fn difs(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (sc, ox, oy, oz, size) = (k[0], k[1], k[3], k[2], k[4]);
    let levels = k[5] as u32;
    let up = k[7] > 0.5;
    let (ca, sa) = (k[8], k[9]);
    // (Mandelbulber's z is up)
    let mut z = V8::new(p.x, p.z, p.y + k[10]);
    let mut d = s(1e9);
    let mut ring = F8::zero();
    let mut dd = 1.0f32;
    for i in 0..iters {
        let dm = difs_sponge(z, size, levels) * dd;
        ring = F8::select(dm.lt(d), s(i as f32), ring);
        d = d.min(dm);
        // folded, scaled about the offset, turned
        let fx = z.x.abs() * sc - (sc - 1.0) * ox;
        let fy = z.y.abs() * sc - (sc - 1.0) * oy;
        let fz = if up { z.z.abs() } else { z.z } * sc - (sc - 1.0) * oz;
        z = V8::new(fx * ca - fy * sa, fx * sa + fy * ca, fz);
        dd /= sc;
    }
    (d, (ring * 0.125).saturate())
}

const DIFS: &str = r#"
fn difs_sponge(q: vec3f, size: f32, levels: u32) -> f32 {
    var z = q * (1.0 / size);
    for (var i = 0u; i < levels; i++) {
        let a = abs(z);
        let x = max(a.x, a.y);
        let y0 = min(a.x, a.y);
        let y1 = min(y0, a.z);
        let zz = max(y0, a.z);
        let y = select(y1, 2.0 / 3.0 - y1, y1 < 1.0 / 3.0);
        z = vec3f(x * 3.0 - 2.0, y * 3.0 - 2.0, zz * 3.0 - 2.0);
    }
    let b = abs(z) - vec3f(1.0);
    let m = max(b.x, max(b.y, b.z));
    let d = select(m, length(max(b, vec3f(0.0))), m > 0.0);
    return d * (size / pow(3.0, f32(levels)));
}

fn difs(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let sc = a.x;
    let o = a.ywz;
    let size = b.x;
    let levels = u32(b.y);
    let up = b.w > 0.5;
    let ca = c.x;
    let sa = c.y;
    var z = vec3f(p.x, p.z, p.y + c.z);
    var d = 1e9;
    var ring = 0.0;
    var dd = 1.0;
    for (var i = 0u; i < iters; i++) {
        let dm = difs_sponge(z, size, levels) * dd;
        ring = select(ring, f32(i), dm < d);
        d = min(d, dm);
        let fx = abs(z.x) * sc - (sc - 1.0) * o.x;
        let fy = abs(z.y) * sc - (sc - 1.0) * o.y;
        let fz = select(z.z, abs(z.z), up) * sc - (sc - 1.0) * o.z;
        z = vec3f(fx * ca - fy * sa, fx * sa + fy * ca, fz);
        dd /= sc;
    }
    return vec2f(d, clamp(ring * 0.125, 0.0, 1.0));
}
"#;

// ---- the bulbs' escape

/// Where the bulbs' orbits count as escaped: far out, so the log estimate
/// changes smoothly from one escape count to the next (msltoe's squaring
/// does not square the radius exactly: at Mandelbulber's 10 eiffie's
/// estimate jumps between counts).
const BULB_BAIL: f32 = 1000.0;
/// Mandelnest's escape: Mandelbulber's (its radius is raised to an exact
/// power, so the estimate is smooth without a far bailout).
const MANDELNEST_BAIL: f32 = 10.0;

/// The twist as its cosine and sine.
fn twist_prep(k: &mut [f32; K]) {
    let a = k[4].to_radians();
    k[5] = a.cos();
    k[6] = a.sin();
}

// ---- msltoe_sym

/// msltoe's Sym4: the axes sorted by size (x the largest, then y), y and z
/// given x's sign, then the quaternion-like square (x² - y² - z², 2xy, 2xz)
/// turned about x, + c. |z|² is exactly squared, so the log estimate (with
/// the derivative 2 r dr + 1) is sound; the trap is the nearest the orbit
/// comes to the centre.
fn msltoe_sym(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let c = if k[0] > 0.5 { V8::new(s(k[1]), s(k[2]), s(k[3])) } else { p };
    let (ca, sa) = (k[5], k[6]);
    let bail = s(BULB_BAIL);
    let mut z = p;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let ndr = r * dr * 2.0 + 1.0;
        let sw = z.x.abs().lt(z.z.abs());
        let (x, zz) = (F8::select(sw, z.z, z.x), F8::select(sw, z.x, z.z));
        let sw = x.abs().lt(z.y.abs());
        let (x, y) = (F8::select(sw, z.y, x), F8::select(sw, x, z.y));
        let sw = y.abs().lt(zz.abs());
        let (y, zz) = (F8::select(sw, zz, y), F8::select(sw, y, zz));
        let zz = F8::select((x * zz).lt(F8::zero()), -zz, zz);
        let y = F8::select((x * y).lt(F8::zero()), -y, y);
        let (yy, w) = (x * y * 2.0, x * zz * 2.0);
        let nz = V8::new(x * x - y * y - zz * zz, yy * ca - w * sa, yy * sa + w * ca) + c;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap * 0.5).saturate())
}


const MSLTOE_SYM: &str = r#"
fn msltoe_sym(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let k = select(p, a.yzw, a.x > 0.5);
    let ca = b.y;
    let sa = b.z;
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 1000.0) {
            break;
        }
        dr = r * dr * 2.0 + 1.0;
        // sorted by size, y and z given x's sign
        var sw = abs(z.x) < abs(z.z);
        let x0 = select(z.x, z.z, sw);
        let z0 = select(z.z, z.x, sw);
        sw = abs(x0) < abs(z.y);
        let x = select(x0, z.y, sw);
        let y0 = select(z.y, x0, sw);
        sw = abs(y0) < abs(z0);
        let y1 = select(y0, z0, sw);
        let z1 = select(z0, y0, sw);
        let zz = select(z1, -z1, x * z1 < 0.0);
        let y = select(y1, -y1, x * y1 < 0.0);
        let yy = x * y * 2.0;
        let w = x * zz * 2.0;
        z = vec3f(x * x - y * y - zz * zz, yy * ca - w * sa, yy * sa + w * ca) + k;
        r = length(z);
        trap = min(trap, r);
    }
    let rs = max(r, 1e-6);
    return vec2f(log(rs) * rs * 0.5 / max(dr, 1e-12), clamp(trap * 0.5, 0.0, 1.0));
}
"#;

// ---- eiffie

/// eiffie's estimate factor: 0.5 in theory, but this squaring's estimate
/// measures at most 0.75 of the true distance along rays (it does not square
/// the radius exactly), so a little more marches, and shades (occlusion
/// reads the estimate), truer.
const EIFFIE_DE: f32 = 0.75;

/// msltoe's squaring: the xy plane's complex square times cos² of the
/// latitude, and the latitude's doubling on z; `ys` stretches y.
#[inline(always)]
fn eiffie_square(z: V8, ys: F8) -> V8 {
    let (x2, y2, z2) = (z.x * z.x, z.y * z.y, z.z * z.z);
    let m = F8::one() - z2 / (x2 + y2 + z2).max(s(1e-12));
    V8::new((x2 - y2) * m, z.x * z.y * m * ys * 2.0, z.z * (x2 + y2).sqrt() * 2.0)
}

/// eiffie's bulb, its axis vertical (Mandelbulber's x is our y): the yz
/// angle folded into 0..pi/8, msltoe's square, + c; the log estimate and
/// the trap (the nearest the orbit comes to the centre).
fn eiffie(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let ys = s(k[0]);
    let q = V8::new(p.y, p.z, p.x);
    let c = if k[1] > 0.5 { V8::new(s(k[3]), s(k[4]), s(k[2])) } else { q };
    let bail = s(BULB_BAIL);
    let mut z = q;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let ndr = r * dr * 2.0 + 1.0;
        let a = z.z.atan2(z.y) + (PI + PI / 8.0);
        let psi = (a - (a * (4.0 / PI)).floor() * (PI / 4.0) - PI / 8.0).abs();
        let l = len2(z.y, z.z);
        let nz = eiffie_square(V8::new(z.x, psi.cos() * l, psi.sin() * l), ys) + c;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * EIFFIE_DE / dr.max(s(1e-12)), (trap * 0.5).saturate())
}

const EIFFIE: &str = r#"
fn eiffie(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let ys = a.x;
    let q = p.yzx;
    let k = select(q, vec3f(a.w, b.x, a.z), a.y > 0.5);
    let pi = 3.14159265;
    var z = q;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 1000.0) {
            break;
        }
        dr = r * dr * 2.0 + 1.0;
        // the yz angle folded into 0..pi/8
        let an = atan2(z.z, z.y) + (pi + pi / 8.0);
        let psi = abs(an - floor(an * (4.0 / pi)) * (pi / 4.0) - pi / 8.0);
        let l = length(z.yz);
        let f = vec3f(z.x, cos(psi) * l, sin(psi) * l);
        let f2 = f * f;
        let m = 1.0 - f2.z / max(f2.x + f2.y + f2.z, 1e-12);
        z = vec3f((f2.x - f2.y) * m, f.x * f.y * m * ys * 2.0, f.z * sqrt(f2.x + f2.y) * 2.0) + k;
        r = length(z);
        trap = min(trap, r);
    }
    let rs = max(r, 1e-6);
    return vec2f(log(rs) * rs * 0.75 / max(dr, 1e-12), clamp(trap * 0.5, 0.0, 1.0));
}
"#;

// ---- mandelnest

/// The power as packed, the phase shift in radians.
fn mandelnest_prep(k: &mut [f32; K]) {
    k[1] *= PI;
}

/// Jeannot's Mandelnest: each coordinate's angle to its axis, acos(z_i / r),
/// multiplied by the power (and shifted), its cosine taken; the three
/// normalised and scaled by r^(power - 1), + c. The derivative grows by
/// power r^(power - 2): Mandelbulber's also divides by the cosines' length,
/// which makes the estimate jump about; without it the estimate rarely
/// passes the true distance (at most 1.4 times it, measured along rays), so
/// the march steps 0.8 of it. The trap is the least length of the cosines,
/// low along the seams between the lobes.
fn mandelnest(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (n, shift) = (k[0], s(k[1]));
    let c = if k[2] > 0.5 { V8::new(s(k[3]), s(k[4]), s(k[5])) } else { p };
    let bail = s(MANDELNEST_BAIL);
    let mut z = p;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    // cos(shift + n acos(t)), acos as atan2(sqrt(1 - t²), t)
    let ch = |t: F8| {
        let t = t.clamp(-1.0, 1.0);
        (shift + t.neg_mul_add(t, F8::one()).max(F8::zero()).sqrt().atan2(t) * n).cos()
    };
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-12));
        let inv = F8::one() / rs;
        let v = V8::new(ch(z.x * inv), ch(z.y * inv), ch(z.z * inv));
        let lv = v.length().max(s(1e-6));
        let rp = rs.powf(s(n - 1.0));
        let ndr = dr * n * rp / rs + 1.0;
        let nz = v * (rp / lv) + c;
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act, trap.min(lv), trap);
        act &= r.lt(bail);
    }
    let r = r.max(s(1e-6));
    (r.ln() * r * 0.5 / dr.max(s(1e-12)), (trap * trap * (1.0 / 3.0)).saturate())
}

const MANDELNEST: &str = r#"
fn mandelnest_ch(t: f32, n: f32, shift: f32) -> f32 {
    let u = clamp(t, -1.0, 1.0);
    return cos(shift + atan2(sqrt(max(1.0 - u * u, 0.0)), u) * n);
}

fn mandelnest(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let n = a.x;
    let shift = a.y;
    let k = select(p, vec3f(a.w, b.x, b.y), a.z > 0.5);
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let rs = max(r, 1e-12);
        let inv = 1.0 / rs;
        let v = vec3f(mandelnest_ch(z.x * inv, n, shift), mandelnest_ch(z.y * inv, n, shift), mandelnest_ch(z.z * inv, n, shift));
        let lv = max(length(v), 1e-6);
        let rp = pow(rs, n - 1.0);
        dr = dr * n * rp / rs + 1.0;
        z = v * (rp / lv) + k;
        r = length(z);
        trap = min(trap, lv);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / max(dr, 1e-12), clamp(trap * trap * (1.0 / 3.0), 0.0, 1.0));
}
"#;
