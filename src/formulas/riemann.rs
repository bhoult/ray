//! Bulbs through other coordinates: msltoe's and Hobold's Riemann sphere
//! maps, Nylander's Hopfbrot (the bulb's power map in the Hopf coordinates of
//! four dimensions, cut to three), and a Mandelbox fold before a power 2
//! bulb step.
//!
//! Each step of each raises the radius to an exact power (the direction goes
//! elsewhere), so the distance estimate is the potential's, 0.5 r ln r / dr,
//! with dr the radius's running derivative: it agrees with itself whichever
//! step the orbit escapes at, if the escape radius is large. (The full
//! gradient of these non-conformal maps swings too wildly to estimate with.)
//! The Riemann maps stretch their angles more than their radius near their
//! poles' axis, where they tear, so their steps are well below 1.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, V8};
use std::f32::consts::PI;

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "riemann_sphere",
        category: "Fractals: bulbs",
        desc: "msltoe's Riemann sphere fractal: each step projects the point's direction stereographically onto a plane, folds the plane into a grid of cells with |sin|, projects back, and raises the radius to a power that grows towards the projection's pole. A bell-shaped body covered in a curved grid of studs, with clusters of pyramids at its rim and on its face. Formula by msltoe (Fractal Forums, 2011); ported from Mandelbulber 2.",
        params: &[
            Param::num("radius", &[1.0], 0.5, 1.0, "The sphere the direction is put on before the projection (0.5 to 1): 1 is the true Riemann sphere; smaller values pull the clusters away from the pole and stretch the body into a pointed heart."),
            Param::num("offset", &[0.0, 0.0], -3.2, 3.2, "Phases of the two sine folds (radians, -3.2 to 3.2): slide the grid of pyramid clusters across the surface."),
            Param::num("c", &[0.0, 1.0, 0.0], -1.5, 1.5, "A constant added at every step (each component -1.5 to 1.5): (0, 1, 0), along the pole, gives the classic clustered bell; zero leaves a plain ball; sideways values lean and melt it."),
            Param::num("mandel", &[0.0], 0.0, 1.0, "How much of the point itself is added at every step (0 to 1): 0 is msltoe's Julia-like form, 1 a Mandelbrot-like one, lopsided and lumpier."),
        ],
        iterations: (8, 24, "Escape-time iterations: more = finer pyramids on the pyramids."),
        bound: 1.7,
        lod: None,
        step: 0.7,
        example: r#"{"type":"riemann_sphere","scale":0.9}"#,
        prep: None,
        cpu: riemann_sphere,
        wgsl: RIEMANN_SPHERE,
        wgsl_fn: "riemann_sphere",
    },
    Formula {
        name: "riemann_hobold",
        category: "Fractals: bulbs",
        desc: "Hobold's Riemann sphere bulb, done without trigonometry: the direction is projected from the sphere's pole onto two great circles, each projected point's angle is doubled by complex squaring, and the two planes through the pole that the doubled points define meet the sphere at the new direction. A round, quilted ball caged in a curved lattice, with a star of ribbed lobes around one pole (the example is turned to show both). Formula by hobold (Fractal Forums, 2010, later reworked without trigonometry); ported from Mandelbulber 2.",
        params: &[
            Param::num("radius", &[0.8], 0.5, 1.5, "The sphere the direction is put on before the projection (0.5 to 1.5): 1 is the exact construction; Mandelbulber's 0.8 bends it into the ribbed lattice."),
            Param::num("offset", &[0.0, 0.0], -1.0, 1.0, "Tweaks to the two planes' normals (each -1 to 1): skew the lattice along x and z."),
            Param::num("angle", &[0.0], -180.0, 180.0, "Turns the two planes' normals into each other (degrees): twists the lattice."),
            Param::num("c", &[0.0, 0.0, 0.0], -1.0, 1.0, "A constant added at every step besides the point (each component -1 to 1): shifts it off its symmetric form."),
            Param::whole("doublings", 3.0, 1.0, 4.0, "How many times the angles are doubled (1 to 4): 3 multiplies them by 8, as the original; 2 (by 4) gives a simpler shape with fewer lobes, 4 (by 16) a finer one."),
            Param::num("power", &[16.0], 5.0, 16.0, "The radius is raised to half this power each step (5 to 16): at 16 it keeps pace with the angles' eightfold turn, a round, quilted ball; Mandelbulber's 8 is the spikier original, all hair and lobes, whose estimate is rougher (lower the node's step if it frays)."),
        ],
        iterations: (8, 24, "Escape-time iterations: more = finer ribs."),
        bound: 1.5,
        lod: None,
        step: 0.4,
        example: r#"{"type":"riemann_hobold","scale":0.9,"rotation":[-60,40,0]}"#,
        prep: Some(prep_angle),
        cpu: riemann_hobold,
        wgsl: RIEMANN_HOBOLD,
        wgsl_fn: "riemann_hobold",
    },
    Formula {
        name: "hopfbrot",
        category: "Fractals: bulbs",
        desc: "The Hopfbrot: the Mandelbulb's power map in four dimensions, with the 3-sphere in Hopf coordinates (two circles' angles and the angle between the circles each multiplied by the power), cut by a 3D slice. Its x-y plane, facing you, holds the Mandelbrot set of the same power (the Mandelbrot set itself at power 2); behind it the body swells into a ribbed barrel fringed with bulbs, different for every tilt of the slice. Named by Paul Nylander; described by Oliver Knill (2023).",
        params: &[
            Param::num("power", &[3.0], 2.0, 8.0, "The exponent (2 to 8): odd powers keep the map continuous, so their surfaces are clean (3 has the cubic Mandelbrot set in its x-y plane); 2 is Knill's, with the Mandelbrot set itself, but torn by seams where the map jumps. Higher powers give more, smaller lobes."),
            Param::num("tilt", &[90.0], -180.0, 180.0, "Which 3D slice of the four dimensions (degrees): where the depth axis points in the plane of the last two coordinates. 90 is Knill's z = 0 cut, 0 the w = 0 cut; values between blend them."),
            Param::num("slice", &[0.0], -1.0, 1.0, "How far the slice sits from the centre along the hidden fourth axis (-1 to 1): away from 0 the body thins, shrinks and breaks apart."),
        ],
        iterations: (10, 30, "Escape-time iterations: more = finer detail, at three atan2, three sines, three cosines and a pow per point each."),
        bound: 2.0,
        lod: None,
        step: 0.7,
        example: r#"{"type":"hopfbrot","scale":0.75}"#,
        prep: Some(prep_tilt),
        cpu: hopfbrot,
        wgsl: HOPFBROT,
        wgsl_fn: "hopfbrot",
    },
    Formula {
        name: "bulbbox",
        category: "Fractals: box folds",
        desc: "A Mandelbox and Mandelbulb hybrid in one step: the Mandelbox's box and sphere folds and a scale, then the power 2 Mandelbulb's squaring, plus the point. A craggy floating block of rippled ledges, caves and bulb-like knots, flower patterns on its top. After Mandelbulber's Box Fold Bulb Pow 2 (its FoldingIntPow2), with the scale freed as in its v2 and the bulb's trigonometric squaring in place of the triplex.",
        params: &[
            Param::num("fold", &[1.0], 0.5, 1.2, "The box fold's limit (0.5 to 1.2): coordinates beyond it are reflected back. It sets the block's size: larger values make a bigger block with more ledges."),
            Param::num("min_radius", &[0.5], 0.1, 0.9, "The sphere fold's inner radius (0.1 to 0.9): it reshapes the fine detail in the hollows more than the overall form."),
            Param::num("fold_scale", &[0.6], 0.45, 1.0, "The scale between the folds and the squaring (0.45 to 1; the original's 1 leaves only dust): 0.5 makes a solid craggy block, 0.6 carves caves into it, and from about 0.7 it thins towards dust."),
            Param::num("z_factor", &[1.0], 0.5, 5.0, "Stretches the squaring's vertical axis (0.5 to 5; the original's is 5): 1 is the plain power 2 bulb; below 1 the walls stretch into flat ledges, above it they thin into sheets and threads."),
        ],
        iterations: (10, 30, "Escape-time iterations: more = finer detail."),
        bound: 6.5,
        lod: None,
        step: 0.5,
        example: r#"{"type":"bulbbox","scale":0.22}"#,
        prep: None,
        cpu: bulbbox,
        wgsl: BULBBOX,
        wgsl_fn: "bulbbox",
    },
];

/// riemann_hobold's angle (degrees, k[3]) into its cosine there and its sine
/// in k[11]; and in k[9] its escape radius, as large as the power allows (a
/// step then reaches at most 1e8): the larger, the steadier the estimate.
fn prep_angle(k: &mut [f32; K]) {
    let a = k[3].to_radians();
    k[3] = a.cos();
    k[11] = a.sin();
    k[9] = (2.0 * 1e8f32.ln() / k[8]).exp();
}

/// hopfbrot's tilt (degrees, k[1]) into its cosine there and its sine in
/// k[11]; in k[10] the shift along x that centres it (the power 2 set
/// reaches from -2 to 1/2; from power 3 on they are centred); and in k[9] its
/// escape radius, as large as the power allows (a step then reaches at most
/// 1e8).
fn prep_tilt(k: &mut [f32; K]) {
    let a = k[1].to_radians();
    k[1] = a.cos();
    k[11] = a.sin();
    k[10] = (3.0 - k[0]).max(0.0) * -0.5;
    k[9] = (1e8f32.ln() / k[0]).exp();
}

/// Steps past the iterations an orbit already beyond its old (small) escape
/// radius may take to reach the large one, so that the estimate stays steady
/// where the iterations run out.
const TAIL: u32 = 4;

/// The largest running derivative kept (beyond, the estimate is 0 anyway):
/// it keeps the products finite.
const DR_MAX: f32 = 1e30;

/// The potential's distance estimate from the final radius and running
/// derivative.
#[inline(always)]
fn estimate(r: F8, dr: F8) -> F8 {
    let r = r.max(s(1e-6));
    r * r.ln() * 0.5 / dr.max(s(1e-12))
}

/// riemann_sphere's escape radius; the radius within which its orbit counts
/// for the trap and must be to run past the iterations; and the ln of the
/// radius a step may reach (beyond, the lane is done, its ln|z| and dr / |z|
/// worked out in log space).
const RS_BAIL: f32 = 1e3;
const RS_NEAR: f32 = 2.0;
const RS_HUGE: f32 = 27.6;
/// pi / 8: where r^(2p) is small the -1/4 holds the radius up while the
/// angles still stretch (by pi p / 4 r at most: the projection, the fold and
/// back), so the radial stretch 2p r^(2p) / r alone would shrink the
/// derivative, and blow the estimate up, near the set. In units of 2p / r.
const RS_HOLD: f32 = PI / 8.0;

/// msltoe's Riemann sphere map (Mandelbulber's RiemannSphereMsltoe, with
/// its pole on +y): the direction, put on a sphere, is projected
/// stereographically to (s, t), each folded by |sin(pi x)|, projected back;
/// the radius becomes r^(2p) - 1/4 with p = 1 + s² + t² the projection's
/// stretch (capped at 36); plus c and `mandel` times the point. Trap: the
/// nearest the orbit comes to the equator's plane.
fn riemann_sphere(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (radius, oa, ob) = (k[0], k[1], k[2]);
    let w = k[6];
    let mut z = p;
    let mut r = z.length();
    let mut dr = F8::one();
    // (ln|z| and dr / |z| as the estimate takes them)
    let mut l = r.max(s(1e-6)).ln();
    let mut rel = dr / r.max(s(1e-6));
    let mut trap = s(1e9);
    let mut act = r.lt(s(RS_BAIL));
    for i in 0..iters + TAIL {
        if i >= iters {
            act &= r.gt(s(RS_NEAR));
        }
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-12));
        let u = z * (s(radius) / rs);
        // (the pole's neighbourhood is capped by the power anyway)
        let q = F8::one() / (F8::one() - u.y).max(s(1e-4));
        let (s0, t0) = (u.x * q, u.z * q);
        let pw = s0.mul_add(s0, t0.mul_add(t0, F8::one())).min(s(36.0));
        let ss = (s0 * PI + oa).sin().abs();
        let tt = (t0 * PI + ob).sin().abs();
        // r^(2p) in log space
        let lp = pw * (rs * rs).ln();
        let huge = lp.gt(s(RS_HUGE));
        let rp = lp.min(s(RS_HUGE)).exp();
        let s2 = ss.mul_add(ss, tt * tt);
        let f = (rp - 0.25) / (s2 + 1.0);
        let nz = V8::new(ss * f * 2.0 + p.x * w + k[3], (s2 - 1.0) * f + p.y * w + k[4], tt * f * 2.0 + p.z * w + k[5]);
        // the radius's stretch, and the angles' where -1/4 holds the radius up
        let g = pw * 2.0 / rs;
        let hrel = g * dr;
        let ndr = (g * (rp + RS_HOLD) * dr + w).min(s(DR_MAX));
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        let rl = r.max(s(1e-6));
        l = F8::select(act, F8::select(huge, lp, rl.ln()), l);
        rel = F8::select(act, F8::select(huge, hrel, dr / rl), rel);
        trap = F8::select(act & r.lt(s(RS_NEAR)), trap.min(z.y.abs()), trap);
        act &= r.lt(s(RS_BAIL)) & !huge;
    }
    (l * 0.5 / rel.max(s(1e-12)), trap.saturate())
}

const RIEMANN_SPHERE: &str = r#"
fn riemann_sphere(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let radius = a.x;
    let oa = a.y;
    let ob = a.z;
    let k = vec3f(a.w, b.x, b.y);
    let w = b.z;
    var z = p;
    var r = length(z);
    var dr = 1.0;
    var l = log(max(r, 1e-6));
    var rel = dr / max(r, 1e-6);
    var trap = 1e9;
    for (var i = 0u; i < iters + 4u; i++) {
        if (r >= 1e3 || (i >= iters && r <= 2.0)) {
            break;
        }
        let rs = max(r, 1e-12);
        let u = z * (radius / rs);
        let q = 1.0 / max(1.0 - u.y, 1e-4);
        let s0 = u.x * q;
        let t0 = u.z * q;
        let pw = min(s0 * s0 + t0 * t0 + 1.0, 36.0);
        let ss = abs(sin(s0 * 3.14159265 + oa));
        let tt = abs(sin(t0 * 3.14159265 + ob));
        let lp = pw * log(rs * rs);
        let rp = exp(min(lp, 27.6));
        let s2 = ss * ss + tt * tt;
        let f = (rp - 0.25) / (s2 + 1.0);
        z = vec3f(ss * f * 2.0, (s2 - 1.0) * f, tt * f * 2.0) + p * w + k;
        let g = pw * 2.0 / rs;
        let hrel = g * dr;
        dr = min(g * (rp + 0.3927) * dr + w, 1e30);
        r = length(z);
        let rl = max(r, 1e-6);
        if (lp > 27.6) {
            l = lp;
            rel = hrel;
            break;
        }
        l = log(rl);
        rel = dr / rl;
        if (r < 2.0) {
            trap = min(trap, abs(z.y));
        }
    }
    return vec2f(l * 0.5 / max(rel, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;

/// The radius within which riemann_hobold's orbit counts for the trap and
/// must be to run past the iterations (its escape radius is in k[9]).
const RH_NEAR: f32 = 2.5;

/// One of Hobold's circle projections: the point (a, y) on the sphere seen
/// from the pole (0, 1) as a point on the unit circle at its central angle,
/// that angle doubled `n` times; returns the normal (Ky - 1, -Ka) of the
/// plane through the pole at the doubled angle. (v = 1 - y.)
#[inline(always)]
fn hobold_plane(a: F8, v: F8, n: u32) -> (F8, F8) {
    let den = v.mul_add(v, a * a).max(s(1e-12));
    let mut ka = a * v * 2.0 / den;
    let mut ky = F8::one() - v * v * 2.0 / den;
    for _ in 0..n {
        let na = ka * ky * -2.0;
        ky = ka.mul_add(ka, -(ky * ky));
        ka = na;
    }
    (ky - 1.0, -ka)
}

/// Hobold's Riemann sphere map (Mandelbulber's RiemannSphereHoboldPow8 and
/// Multi, doublings only), with the division by the first normal's x folded
/// into the solve so that it stays finite. The new direction is on the unit
/// sphere, the radius r^(power / 2). Trap: the nearest the orbit comes to
/// the pole's axis.
fn riemann_hobold(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (radius, oa, ob, ca, sa) = (k[0], k[1], k[2], k[3], k[11]);
    let n = k[7] as u32;
    let (h, bail) = (k[8] * 0.5, k[9]);
    let mut z = p;
    let mut r = z.length();
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = r.lt(s(bail));
    for i in 0..iters + TAIL {
        if i >= iters {
            act &= r.gt(s(RH_NEAR));
        }
        if act.none() {
            break;
        }
        let rs = r.max(s(1e-12));
        let u = z * (s(radius) / rs);
        let v = F8::one() - u.y;
        let (n1x, n1y) = hobold_plane(u.x, v, n);
        let (n2z, n2y) = hobold_plane(u.z, v, n);
        let (n1x, n2z) = (n1x + oa, n2z + ob);
        let (n1x, n2z) = (n1x * ca + n2z * sa, n2z * ca - n1x * sa);
        let (a2, c2) = (n1x * n1x, n2z * n2z);
        let den = (n1y.mul_add(n1y, a2) * c2 + a2 * n2y * n2y).max(s(1e-20));
        let t = n1x * n2z * 2.0 / den;
        let rp = rs.powf(s(h));
        let nz = V8::new(n1y * n2z * t * rp + p.x + k[4], (F8::one() - n1x * n2z * t) * rp + p.y + k[5], n2y * n1x * t * rp + p.z + k[6]);
        let ndr = (rp / rs * h * dr + 1.0).min(s(DR_MAX));
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        trap = F8::select(act & r.lt(s(RH_NEAR)), trap.min(len2(z.x, z.z)), trap);
        act &= r.lt(s(bail));
    }
    (estimate(r, dr), trap.saturate())
}

const RIEMANN_HOBOLD: &str = r#"
fn riemann_hobold_plane(a: f32, v: f32, n: u32) -> vec2f {
    let den = max(v * v + a * a, 1e-12);
    var ka = a * v * 2.0 / den;
    var ky = 1.0 - v * v * 2.0 / den;
    for (var i = 0u; i < n; i++) {
        let na = ka * ky * -2.0;
        ky = ka * ka - ky * ky;
        ka = na;
    }
    return vec2f(ky - 1.0, -ka);
}

fn riemann_hobold(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let radius = a.x;
    let oa = a.y;
    let ob = a.z;
    let ca = a.w;
    let sa = c.w;
    let k = b.xyz;
    let n = u32(b.w);
    let h = c.x * 0.5;
    let bail = c.y;
    var z = p;
    var r = length(z);
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters + 4u; i++) {
        if (r >= bail || (i >= iters && r <= 2.5)) {
            break;
        }
        let rs = max(r, 1e-12);
        let u = z * (radius / rs);
        let v = 1.0 - u.y;
        let p1 = riemann_hobold_plane(u.x, v, n);
        let p2 = riemann_hobold_plane(u.z, v, n);
        let n1x0 = p1.x + oa;
        let n2z0 = p2.x + ob;
        let n1x = n1x0 * ca + n2z0 * sa;
        let n2z = n2z0 * ca - n1x0 * sa;
        let n1y = p1.y;
        let n2y = p2.y;
        let a2 = n1x * n1x;
        let c2 = n2z * n2z;
        let den = max((n1y * n1y + a2) * c2 + a2 * n2y * n2y, 1e-20);
        let t = n1x * n2z * 2.0 / den;
        let rp = pow(rs, h);
        z = vec3f(n1y * n2z * t, 1.0 - n1x * n2z * t, n2y * n1x * t) * rp + p + k;
        dr = min(rp / rs * h * dr + 1.0, 1e30);
        r = length(z);
        if (r < 2.5) {
            trap = min(trap, length(z.xz));
        }
    }
    let rr = max(r, 1e-6);
    return vec2f(rr * log(rr) * 0.5 / max(dr, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;

/// The radius within which hopfbrot's orbit counts for the trap and must be
/// to run past the iterations (its escape radius is in k[9]).
const HB_NEAR: f32 = 2.0;

/// The Hopfbrot cut to 3D: the point is c = (x + shift, y, z cos t - slice
/// sin t, z sin t + slice cos t) in four dimensions, and z <- |z|^n U(n a1,
/// n a2, n phi) + c from z = c, with U the Hopf parametrisation (cos phi
/// e^(i a1), sin phi e^(i a2)) of the unit 3-sphere. Trap: the nearest the
/// orbit comes to the Mandelbrot set's plane (the first two coordinates').
fn hopfbrot(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (power, ct, slice, st, shift, bail) = (k[0], k[1], k[2], k[11], k[10], k[9]);
    let c = [p.x + shift, p.y, p.z * ct - slice * st, p.z.mul_add(s(st), s(slice * ct))];
    let mut z = c;
    let len4 = |z: &[F8; 4]| z[3].mul_add(z[3], z[2].mul_add(z[2], z[1].mul_add(z[1], z[0] * z[0]))).sqrt();
    let mut r = len4(&z);
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = r.lt(s(bail));
    for i in 0..iters + TAIL {
        if i >= iters {
            act &= r.gt(s(HB_NEAR));
        }
        if act.none() {
            break;
        }
        let r1 = len2(z[0], z[1]);
        let r2 = len2(z[2], z[3]);
        let a1 = z[1].atan2(z[0]) * power;
        let a2 = z[3].atan2(z[2]) * power;
        let ph = r2.atan2(r1) * power;
        let rs = r.max(s(1e-12));
        let rp = rs.powf(s(power));
        let (cp, sp) = (ph.cos() * rp, ph.sin() * rp);
        let n = [cp * a1.cos() + c[0], cp * a1.sin() + c[1], sp * a2.cos() + c[2], sp * a2.sin() + c[3]];
        let ndr = (rp / rs * power * dr + 1.0).min(s(DR_MAX));
        for j in 0..4 {
            z[j] = F8::select(act, n[j], z[j]);
        }
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, len4(&z), r);
        trap = F8::select(act & r.lt(s(HB_NEAR)), trap.min(len2(z[2], z[3])), trap);
        act &= r.lt(s(bail));
    }
    (estimate(r, dr), trap.saturate())
}

const HOPFBROT: &str = r#"
fn hopfbrot(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let power = a.x;
    let ct = a.y;
    let slice = a.z;
    let bail = c.y;
    let shift = c.z;
    let st = c.w;
    let k = vec4f(p.x + shift, p.y, p.z * ct - slice * st, p.z * st + slice * ct);
    var z = k;
    var r = length(z);
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters + 4u; i++) {
        if (r >= bail || (i >= iters && r <= 2.0)) {
            break;
        }
        let r1 = length(z.xy);
        let r2 = length(z.zw);
        let a1 = atan2(z.y, z.x) * power;
        let a2 = atan2(z.w, z.z) * power;
        let ph = atan2(r2, r1) * power;
        let rs = max(r, 1e-12);
        let rp = pow(rs, power);
        let cp = cos(ph) * rp;
        let sp = sin(ph) * rp;
        z = vec4f(cp * cos(a1), cp * sin(a1), sp * cos(a2), sp * sin(a2)) + k;
        dr = min(rp / rs * power * dr + 1.0, 1e30);
        r = length(z);
        if (r < 2.0) {
            trap = min(trap, length(z.zw));
        }
    }
    let rr = max(r, 1e-6);
    return vec2f(rr * log(rr) * 0.5 / max(dr, 1e-12), clamp(trap, 0.0, 1.0));
}
"#;

/// bulbbox's escape radius, and the radius within which its orbit counts for
/// the trap and must be to run past the iterations.
const BB_BAIL: f32 = 1e4;
const BB_NEAR: f32 = 4.0;

/// After Mandelbulber's BoxFoldBulbPow2: the box fold at `fold`, the sphere
/// fold (fixed radius 1), the scale (free, as in its v2) and its 2, the power
/// 2 Mandelbulb squaring with +y as its axis, that axis times `z_factor`,
/// plus the point. The squaring is the bulb's trigonometric form (polar
/// angle and azimuth doubled), not the original's triplex (latitude doubled):
/// that one sends its axis to 0, tearing the space around it into needles no
/// estimate can follow; this one stretches by at most 2r everywhere. Its
/// radius is the folded one's squared times a factor of the direction (1
/// when `z_factor` is 1), so the running derivative takes the squaring's as
/// 2 |new| / |old|. Trap: the nearest the box-folded orbit comes to the
/// centre (as the Mandelbox's).
fn bulbbox(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (fold, min_r, scale, zf) = (k[0], k[1], k[2], k[3]);
    let min2 = min_r * min_r;
    let bfold = |c: F8| F8::select(c.abs().gt(s(fold)), s(2.0 * fold).copysign(c) - c, c);
    let mut z = p;
    let mut r = z.length();
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = r.lt(s(BB_BAIL));
    for i in 0..iters + TAIL {
        if i >= iters {
            act &= r.gt(s(BB_NEAR));
        }
        if act.none() {
            break;
        }
        let f = V8::new(bfold(z.x), bfold(z.y), bfold(z.z));
        let r2 = f.length_sq();
        trap = F8::select(act & r.lt(s(BB_NEAR)), trap.min(r2.sqrt()), trap);
        // r2 < min2: scale by 1 / min2; min2 <= r2 < 1: by 1 / r2
        let kk = F8::select(r2.lt(s(min2)), s(1.0 / min2), F8::one() / r2.max(s(1e-12)));
        let kk = F8::select(r2.lt(F8::one()), kk, F8::one()) * (2.0 * scale);
        let wv = f * kk;
        let (x2, z2) = (wv.x * wv.x, wv.z * wv.z);
        let rho2 = (x2 + z2).max(s(1e-12));
        let t = wv.y * 2.0 / rho2.sqrt();
        let q = V8::new((x2 - z2) * t, (wv.y * wv.y - rho2) * zf, wv.x * wv.z * t * 2.0);
        let nz = q + p;
        let ndr = (dr * kk * q.length() * 2.0 / wv.length().max(s(1e-12)) + 1.0).min(s(DR_MAX));
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        act &= r.lt(s(BB_BAIL));
    }
    // (far out a step takes r to (2 scale r)², so the potential of 4 scale² r,
    // which squares exactly, is the one that agrees across the steps)
    let r = r.max(s(1e-6));
    let d = r * (r * (4.0 * scale * scale)).max(s(1e-6)).ln() * 0.5 / dr.max(s(1e-12));
    (d, (trap - 0.45).saturate())
}

const BULBBOX: &str = r#"
fn bulbbox_fold(c: f32, fold: f32) -> f32 {
    return select(c, sign(c) * 2.0 * fold - c, abs(c) > fold);
}

fn bulbbox(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let fold = a.x;
    let min2 = a.y * a.y;
    let scale = a.z;
    let zf = a.w;
    var z = p;
    var r = length(z);
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters + 4u; i++) {
        if (r >= 1e4 || (i >= iters && r <= 4.0)) {
            break;
        }
        let f = vec3f(bulbbox_fold(z.x, fold), bulbbox_fold(z.y, fold), bulbbox_fold(z.z, fold));
        let r2 = dot(f, f);
        if (r < 4.0) {
            trap = min(trap, sqrt(r2));
        }
        var kk = 1.0;
        if (r2 < min2) {
            kk = 1.0 / min2;
        } else if (r2 < 1.0) {
            kk = 1.0 / max(r2, 1e-12);
        }
        kk = kk * (2.0 * scale);
        let w = f * kk;
        let x2 = w.x * w.x;
        let z2 = w.z * w.z;
        let rho2 = max(x2 + z2, 1e-12);
        let t = w.y * 2.0 / sqrt(rho2);
        let q = vec3f((x2 - z2) * t, (w.y * w.y - rho2) * zf, w.x * w.z * t * 2.0);
        z = q + p;
        dr = min(dr * kk * length(q) * 2.0 / max(length(w), 1e-12) + 1.0, 1e30);
        r = length(z);
    }
    let rr = max(r, 1e-6);
    return vec2f(rr * log(max(rr * 4.0 * scale * scale, 1e-6)) * 0.5 / max(dr, 1e-12), clamp(trap - 0.45, 0.0, 1.0));
}
"#;
