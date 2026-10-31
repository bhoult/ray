//! Mandelbox relatives: a box fold, or an abs fold and a shift, then a
//! sphere fold or an inversion, a scale and a constant, repeated. They are
//! the walk-through fractals: their defaults have rooms, vaults and
//! corridors inside.

use super::util::s;
use super::{Formula, K, Param};
use crate::math::Vec3;
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "amazing_surf",
        category: "Fractals: box folds",
        desc: "Amazing Surf: a Mandelbox whose box fold works on x and y only, so z is never folded back and on its own the set is a flat fractal sheet. Turned at every step (here a quarter turn about y), the sheet folds into a tower of thin plates with slots and corridors between them. Formula proposed by Kali (Fractal Forums, 2011), with DarkBeam's additions in Mandelbulb 3D; ported from Mandelbulber 2.",
        params: &[
            Param::num("fold", &[1.0, 1.0], 0.1, 3.0, "Where the box fold turns back on x and y (Mandelbulber's 1): larger widens the plates and the whole tower; unequal values stretch it."),
            Param::num("fold_scale", &[2.0], -4.0, 4.0, "The scale after each fold (1.6 to 2.5 is typical; avoid -1 to 1): lower spreads the plates further apart and grows the tower; negative scales crumple it."),
            Param::num("min_radius", &[0.84], 0.05, 1.0, "Radius of the sphere fold's inner sphere: smaller cuts deeper round holes into the plates. (Mandelbulber's minimum radius of 0.5 is 0.84 here: it compares that value's square root with r².)"),
            Param::num("angles", &[0.0, 90.0, 0.0], -180.0, 180.0, "Degrees turned about x, y and z at every step: what lifts the flat sheet into 3D. Quarter turns give stacked plates; 0 is the bare sheet; odd angles give rocky lumps."),
            Param::whole("cylinder", 0.0, 0.0, 1.0, "1: the sphere fold measures the radius on x and y only (Mandelbulber's cylinder fold): the holes become slots."),
            Param::num("thickness", &[0.02], 0.0, 0.2, "Thickens the sheets, in the shape's units: 0 leaves the bare fractal surface, which renders as fuzz; 0.01 to 0.05 makes solid plates."),
        ],
        iterations: (10, 40, "Fold iterations: more = finer plates and holes; past about 14 the plates break into dust unless thickness grows."),
        bound: 8.0,
        lod: None,
        step: 0.8,
        example: r#"{"type":"amazing_surf","scale":0.24}"#,
        prep: Some(amazing_surf_prep),
        cpu: amazing_surf,
        wgsl: AMAZING_SURF,
        wgsl_fn: "amazing_surf",
    },
    Formula {
        name: "kalibox",
        category: "Fractals: box folds",
        desc: "Kali's Kalibox: a Julia-set Mandelbox whose box fold is an abs fold and a shift, so each step mirrors space into one octant and moves it before the sphere fold and scale. Its outside is often a closed box; these defaults open it into a gothic pillar of vaulted halls with ornamented walls. Kali (Fragmentarium, 2012; ABoxModKali in Mandelbulb 3D).",
        params: &[
            Param::num("fold_scale", &[-1.54], -4.0, 4.0, "The scale after each fold (avoid -1 to 1): negative values (-1.5 to -2) give vaulted halls, positive ones (1.5 to 2.2) boxy cities."),
            Param::num("min_radius", &[0.56], 0.1, 1.0, "Radius of the sphere fold's inner sphere: smaller makes deeper round niches and more ornament."),
            Param::num("trans", &[-0.81, -0.85, -0.92], -3.0, 3.0, "The shift after the abs fold, per axis: it sets where the halls open and how large they are. Small changes move whole walls."),
            Param::num("julia", &[-0.55, -0.3, -0.8], -3.0, 3.0, "The Julia constant added at every step, per axis: it decides which faces of the box stay closed and which open into halls."),
            Param::num("point", &[0.0], 0.0, 1.0, "How much of the point itself is added at every step: 0 is Kali's Julia form, 1 Mandelbulb 3D's ABoxModKali (a Mandelbrot form, which grows past the bound with these defaults)."),
            Param::num("turn", &[0.0], -180.0, 180.0, "Degrees turned about y before each fold: a few degrees twist the halls; larger turns grow a hairy, rocky skin."),
        ],
        iterations: (14, 40, "Fold iterations: more = finer ornament, linear cost."),
        bound: 4.0,
        lod: None,
        step: 0.8,
        example: r#"{"type":"kalibox","scale":0.6}"#,
        prep: Some(kalibox_prep),
        cpu: kalibox,
        wgsl: KALIBOX,
        wgsl_fn: "kalibox",
    },
    Formula {
        name: "mandalay",
        category: "Fractals: box folds",
        desc: "The Mandalay box: a Mandelbox whose box fold is DarkBeam's Mandalay fold, which carves each face into stepped frames, so the set grows into a cathedral: ornamented panels outside, vaults and arches in chambers within. DarkBeam's fold as Knighty adapted it (Fractal Forums); ported from Mandelbulber 2's Mandalay Box V1.",
        params: &[
            Param::num("fold_scale", &[1.8], -4.0, 4.0, "The scale after each fold (avoid -1 to 1): 1.5 to 2.2 gives the cathedral, higher breaks it into separate towers, negative values a larger, crumpled palace."),
            Param::num("min_radius", &[0.5], 0.1, 1.0, "Radius of the sphere fold's inner sphere: smaller gives deeper vaults and more arches; near 0.75 the facade turns into plain stacked blocks."),
            Param::num("fold", &[0.5, 0.5, 0.5], 0.0, 2.0, "The Mandalay fold's size per axis: the width of the stepped frames carved into each face. Unequal values make it a tower or a hall."),
            Param::num("offset", &[0.0, 0.0, 0.0], -2.0, 2.0, "Shifts where each axis's fold steps back: small values (±0.5) add balconies and recesses to the frames."),
            Param::num("julia", &[0.0, 0.0, 0.0], -3.0, 3.0, "A constant added at every step: with point at 0 it is a Julia set (Mandelbulber's example uses -0.9, 0.7, 0 with fold_scale 1.6)."),
            Param::num("point", &[1.0], 0.0, 1.0, "How much of the point itself is added at every step: 1 is the Mandelbrot form (the cathedral), 0 the Julia form."),
        ],
        iterations: (12, 40, "Fold iterations: more = finer ornament, linear cost."),
        bound: 4.0,
        lod: None,
        // (the Mandalay fold stretches space a little: the estimate runs up
        // to about twice as steep as the distance in places)
        step: 0.6,
        example: r#"{"type":"mandalay","scale":1.1}"#,
        prep: Some(mandalay_prep),
        cpu: mandalay,
        wgsl: MANDALAY,
        wgsl_fn: "mandalay",
    },
    Formula {
        name: "kaliset",
        category: "Fractals: box folds",
        desc: "Kali's Kaliset in 3D: mirror space into one octant with abs, invert it in the unit sphere, add a constant, again and again. Far points all land near the constant, so it fills space; cutting away every point whose orbit falls into a hole about the centre leaves a lace of domed chambers and bony struts. It fills all of space, so it is cut to a box here; fly inside it. Kali (Fractal Forums, 2011); after Mandelbulber 2's Kalisets1.",
        params: &[
            Param::num("inversion", &[1.0], 0.3, 3.0, "Radius² of the sphere it inverts in: above 1 swells the chambers, below 1 thins the struts into lace."),
            Param::num("julia", &[-0.5, -0.5, -0.5], -2.0, 2.0, "The constant added at every step (the Kaliset's c): -0.3 to -1 per axis; unequal values break the cubic symmetry."),
            Param::num("hole", &[0.5], 0.02, 1.0, "Radius of the hole about the centre: orbits that fall in it are cut away. Larger opens bigger chambers; small leaves a solid block pitted with bubbles."),
            Param::num("point", &[0.0], 0.0, 1.0, "How much of the point itself is added at every step: 0 is the Julia form; more gives a Mandelbrot-like form."),
            Param::num("angles", &[0.0, 0.0, 0.0], -180.0, 180.0, "Degrees turned about x, y and z at every step: a few degrees twist the chambers; 0 keeps the cubic symmetry."),
        ],
        iterations: (8, 30, "Iterations: each cuts away the next, smaller generation of holes; past 10 the walls are pitted with dust."),
        bound: f32::INFINITY,
        lod: None,
        step: 0.8,
        example: r#"{"type":"intersection","scale":0.6,"children":[{"type":"box","size":[4,4,4]},{"type":"kaliset"}]}"#,
        prep: Some(kaliset_prep),
        cpu: kaliset,
        wgsl: KALISET,
        wgsl_fn: "kaliset",
    },
];

/// Escaped orbits stop at this radius² (Mandelbulber's bailout for the box
/// folds is 100).
const BAIL2: f32 = 100.0 * 100.0;

/// The unit quaternion (x, y, z, w) turning by `deg` degrees about x, then
/// y, then z.
fn euler_quat(deg: [f32; 3]) -> [f32; 4] {
    let h = deg.map(|d| (d.to_radians() * 0.5).sin_cos());
    let qx = [h[0].0, 0.0, 0.0, h[0].1];
    let qy = [0.0, h[1].0, 0.0, h[1].1];
    let qz = [0.0, 0.0, h[2].0, h[2].1];
    qmul(qz, qmul(qy, qx))
}

fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

/// `v` turned by the unit quaternion `q`: v + 2 u × (u × v + w v).
#[inline(always)]
fn qrot(v: V8, q: [f32; 4]) -> V8 {
    let u = V8::splat(Vec3::new(q[0], q[1], q[2]));
    let t = u.cross(v) + v * q[3];
    v + u.cross(t) * 2.0
}

/// min_radius to its square (the fold compares it with r²); the angles to a
/// quaternion at k[4..8], cylinder and thickness moved to k[8..10].
fn amazing_surf_prep(k: &mut [f32; K]) {
    k[3] *= k[3];
    let (cyl, thick) = (k[7], k[8]);
    let q = euler_quat([k[4], k[5], k[6]]);
    k[4..8].copy_from_slice(&q);
    k[8] = cyl;
    k[9] = thick;
}

/// Amazing Surf (Mandelbulber's amazing_surf.cl, and the point it adds with
/// x and y swapped): box fold x and y, sphere fold and scale in one factor,
/// turn, add the point. The estimate is the linear one less the thickness.
/// Orbit trap: the nearest the folded orbit comes to the centre.
fn amazing_surf(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (fx, fy) = (s(k[0]), s(k[1]));
    let scale = s(k[2]);
    let min2 = s(k[3]);
    let q = [k[4], k[5], k[6], k[7]];
    let cyl = k[8] > 0.5;
    let add = V8::new(p.y, p.x, p.z);
    let mut z = p;
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = M8::all_true();
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let x = (z.x + fx).abs() - (z.x - fx).abs() - z.x;
        let y = (z.y + fy).abs() - (z.y - fy).abs() - z.y;
        let rxy = y.mul_add(y, x * x);
        let rr = if cyl { rxy } else { z.z.mul_add(z.z, rxy) };
        let m = scale / F8::select(rr.lt(min2), min2, rr.min(F8::one()));
        let nz = qrot(V8::new(x * m, y * m, z.z * m), q) + add;
        let ndr = dr.mul_add(m.abs(), F8::one());
        trap = F8::select(act, trap.min(rr), trap);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        act &= z.length_sq().lt(s(BAIL2));
    }
    (z.length() / dr - k[9], ((trap.sqrt() - 0.3) * 0.75).saturate())
}

const AMAZING_SURF: &str = r#"
fn amazing_surf_qrot(v: vec3f, q: vec4f) -> vec3f {
    let t = cross(q.xyz, v) + v * q.w;
    return v + cross(q.xyz, t) * 2.0;
}

fn amazing_surf(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let fx = a.x;
    let fy = a.y;
    let scale = a.z;
    let min2 = a.w;
    let q = b;
    let cyl = c.x > 0.5;
    let add = vec3f(p.y, p.x, p.z);
    var z = p;
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        let x = abs(z.x + fx) - abs(z.x - fx) - z.x;
        let y = abs(z.y + fy) - abs(z.y - fy) - z.y;
        let rxy = y * y + x * x;
        var rr = z.z * z.z + rxy;
        if (cyl) {
            rr = rxy;
        }
        var dv = min(rr, 1.0);
        if (rr < min2) {
            dv = min2;
        }
        let m = scale / dv;
        trap = min(trap, rr);
        z = amazing_surf_qrot(vec3f(x * m, y * m, z.z * m), q) + add;
        dr = dr * abs(m) + 1.0;
        if (dot(z, z) >= 10000.0) {
            break;
        }
    }
    return vec2f(length(z) / dr - c.y, clamp((sqrt(trap) - 0.3) * 0.75, 0.0, 1.0));
}
"#;

/// min_radius to its square, the turn to its cosine and sine at k[9..11].
fn kalibox_prep(k: &mut [f32; K]) {
    k[1] *= k[1];
    let (sn, cs) = k[9].to_radians().sin_cos();
    k[9] = cs;
    k[10] = sn;
}

/// Kali's Kalibox (Fragmentarium's Kalibox.frag, plus Mandelbulb 3D's
/// point term): turn, abs fold and shift, sphere fold, scale, add the Julia
/// constant (and the point); Rrrola's estimate, as Kali's. Orbit trap: the
/// nearest the folded orbit comes to the centre.
fn kalibox(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let scale = k[0];
    let min2 = k[1];
    let tr = V8::splat(Vec3::new(k[2], k[3], k[4]));
    let add = V8::splat(Vec3::new(k[5], k[6], k[7])) + p * k[8];
    let (cs, sn) = (s(k[9]), s(k[10]));
    let sc = s(scale / min2);
    let asc = s(scale.abs() / min2);
    let mut z = p;
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = M8::all_true();
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let u = V8::new(cs.mul_add(z.x, sn * z.z), z.y, cs.mul_add(z.z, -(sn * z.x)));
        let w = u.abs() + tr;
        let r2 = w.length_sq().max(s(1e-12));
        // 1 / min2 inside the inner sphere, 1 / r2 between it and the unit
        // sphere, 1 outside (Rrrola's clamp, with the 1 / min2 in sc)
        let f = (s(min2) / r2).max(s(min2)).min(F8::one());
        let nz = w * (f * sc) + add;
        let ndr = dr.mul_add(f * asc, F8::one());
        trap = F8::select(act, trap.min(r2), trap);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        act &= z.length_sq().lt(s(BAIL2));
    }
    // (Rrrola's last term: only for scales past 1, where it is small)
    let tail = if scale.abs() > 1.0 { scale.abs().powf(1.0 - iters as f32) } else { 0.0 };
    ((z.length() - (scale - 1.0).abs()) / dr - tail, (trap.sqrt() * 0.85).saturate())
}

const KALIBOX: &str = r#"
fn kalibox(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let scale = a.x;
    let min2 = a.y;
    let tr = vec3f(a.z, a.w, b.x);
    let add = b.yzw + p * c.x;
    let cs = c.y;
    let sn = c.z;
    let sc = scale / min2;
    let asc = abs(scale) / min2;
    var z = p;
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        let u = vec3f(cs * z.x + sn * z.z, z.y, cs * z.z - sn * z.x);
        let w = abs(u) + tr;
        let r2 = max(dot(w, w), 1e-12);
        let f = min(max(min2 / r2, min2), 1.0);
        trap = min(trap, r2);
        z = w * (f * sc) + add;
        dr = dr * (f * asc) + 1.0;
        if (dot(z, z) >= 10000.0) {
            break;
        }
    }
    let tail = select(0.0, pow(abs(scale), 1.0 - f32(iters)), abs(scale) > 1.0);
    return vec2f((length(z) - abs(scale - 1.0)) / dr - tail, clamp(sqrt(trap) * 0.85, 0.0, 1.0));
}
"#;

/// min_radius to its square.
fn mandalay_prep(k: &mut [f32; K]) {
    k[1] *= k[1];
}

/// DarkBeam's Mandalay fold of one coordinate in the positive octant
/// (Mandelbulber's mandalay_box_v1.cl, its swaps written out): `a` the
/// coordinate, `o` the larger of the other two, `f` the fold size, `g` the
/// offset.
#[inline(always)]
fn mandalay_fold(a: F8, o: F8, f: f32, g: f32) -> F8 {
    let v = ((a - f).abs() - f).max(o - 4.0 * f);
    let v1 = (a - (2.0 * f + g)).max(o);
    v.min(v1).min(a)
}

/// The Mandalay box (Knighty's form of DarkBeam's fold, Mandelbulber's
/// mandalay_box_v1.cl): fold each axis by the Mandalay fold in the positive
/// octant and restore the signs, sphere fold, scale, add the point
/// (weighted) and the Julia constant; the linear estimate. Orbit trap: the
/// nearest the folded orbit comes to the centre.
fn mandalay(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let scale = k[0];
    let inv_min = s(1.0 / k[1]);
    let fo = [k[2], k[3], k[4]];
    let g = [k[5], k[6], k[7]];
    let add = V8::splat(Vec3::new(k[8], k[9], k[10])) + p * k[11];
    let sc = s(scale);
    let asc = s(scale.abs());
    let neg = |v: F8, c: F8| F8::select(c.lt(F8::zero()), -v, v);
    let mut z = p;
    let mut dr = F8::one();
    let mut trap = s(1e9);
    let mut act = M8::all_true();
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let a = z.abs();
        let x = mandalay_fold(a.x, a.y.max(a.z), fo[0], g[0]);
        let y = mandalay_fold(a.y, a.x.max(a.z), fo[1], g[1]);
        let zz = mandalay_fold(a.z, a.x.max(a.y), fo[2], g[2]);
        let w = V8::new(neg(x, z.x), neg(y, z.y), neg(zz, z.z));
        let r2 = w.length_sq().max(s(1e-12));
        let t = (F8::one() / r2).max(F8::one()).min(inv_min);
        let nz = w * (t * sc) + add;
        let ndr = dr.mul_add(t * asc, F8::one());
        trap = F8::select(act, trap.min(r2), trap);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        act &= z.length_sq().lt(s(BAIL2));
    }
    (z.length() / dr, trap.sqrt().mul_add(s(1.6), s(-0.15)).saturate())
}

const MANDALAY: &str = r#"
fn mandalay_fold(a: f32, o: f32, f: f32, g: f32) -> f32 {
    let v = max(abs(a - f) - f, o - 4.0 * f);
    let v1 = max(a - (2.0 * f + g), o);
    return min(min(v, v1), a);
}

fn mandalay(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let scale = a.x;
    let inv_min = 1.0 / a.y;
    let fo = vec3f(a.z, a.w, b.x);
    let g = b.yzw;
    let add = c.xyz + p * c.w;
    var z = p;
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        let q = abs(z);
        let x = mandalay_fold(q.x, max(q.y, q.z), fo.x, g.x);
        let y = mandalay_fold(q.y, max(q.x, q.z), fo.y, g.y);
        let zz = mandalay_fold(q.z, max(q.x, q.y), fo.z, g.z);
        let w = vec3f(select(x, -x, z.x < 0.0), select(y, -y, z.y < 0.0), select(zz, -zz, z.z < 0.0));
        let r2 = max(dot(w, w), 1e-12);
        let t = min(max(1.0 / r2, 1.0), inv_min);
        trap = min(trap, r2);
        z = w * (t * scale) + add;
        dr = dr * (t * abs(scale)) + 1.0;
        if (dot(z, z) >= 10000.0) {
            break;
        }
    }
    return vec2f(length(z) / dr, clamp(sqrt(trap) * 1.6 - 0.15, 0.0, 1.0));
}
"#;

/// The angles to a quaternion at k[6..10].
fn kaliset_prep(k: &mut [f32; K]) {
    let q = euler_quat([k[6], k[7], k[8]]);
    k[6..10].copy_from_slice(&q);
}

/// Kali's 3D Kaliset (Mandelbulber's kalisets1.cl): abs, invert in the
/// sphere of radius² `inversion`, add the Julia constant (and the point,
/// weighted), turn. Far points all land near the constant, so it fills
/// space; what is cut away are the holes, the points whose orbit comes
/// within `hole` of the centre (where the inversion would fling it out).
/// The estimate is the depth into the deepest such hole, (hole - |z|) /
/// |dz| at its step, negative in the solid. Orbit trap: the orbit's mean radius (Kali's
/// colouring of the 2D sets).
fn kaliset(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let inv = s(k[0]);
    let add = V8::splat(Vec3::new(k[1], k[2], k[3])) + p * k[5];
    let hole = s(k[4]);
    let w = s(k[5]);
    let q = [k[6], k[7], k[8], k[9]];
    let mut z = p;
    let mut dr = F8::one();
    let mut d = s(-1e9);
    let mut sum = F8::zero();
    let mut n = F8::zero();
    let mut act = M8::all_true();
    // (every hole is empty space, so the orbit runs on past the first it
    // falls into: the estimate stays continuous; the trap stops there)
    for _ in 0..iters {
        let r = z.length();
        d = d.max((hole - r) / dr);
        sum += r.and(act);
        n += F8::one().and(act);
        act &= r.ge(hole);
        let m = inv / (r * r).max(s(1e-12));
        z = qrot(z.abs() * m + add, q);
        dr = dr.mul_add(m, w).min(s(1e30));
    }
    (d, ((sum / n - 0.7) * 2.5).saturate())
}

const KALISET: &str = r#"
fn kaliset_qrot(v: vec3f, q: vec4f) -> vec3f {
    let t = cross(q.xyz, v) + v * q.w;
    return v + cross(q.xyz, t) * 2.0;
}

fn kaliset(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let inv = a.x;
    let hole = b.x;
    let w = b.y;
    let add = a.yzw + p * w;
    let q = vec4f(b.z, b.w, c.x, c.y);
    var z = p;
    var dr = 1.0;
    var d = -1e9;
    var sum = 0.0;
    var n = 0.0;
    var inside = true;
    for (var i = 0u; i < iters; i++) {
        let r = length(z);
        d = max(d, (hole - r) / dr);
        if (inside) {
            sum += r;
            n += 1.0;
        }
        inside = inside && r >= hole;
        let m = inv / max(r * r, 1e-12);
        z = kaliset_qrot(abs(z) * m + add, q);
        dr = min(dr * m + w, 1e30);
    }
    return vec2f(d, clamp((sum / n - 0.7) * 2.5, 0.0, 1.0));
}
"#;
