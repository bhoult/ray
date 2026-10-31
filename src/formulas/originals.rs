//! New formulas, made for this catalog: two from the geometry of the
//! 3-sphere, brought into our space by stereographic projection (a fibration
//! of it by torus knots; the 120-cell's symmetry group), and one from Zorich's
//! three-dimensional exponential map.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[SEIFERT, INDRA, ZORICH];

// ---- the Seifert gasket ----------------------------------------------------

const SEIFERT: Formula = Formula {
    name: "seifert_gasket",
    category: "Fractals: originals",
    desc: "A ring of knotted strands of every size, packed like the circles of an Apollonian gasket: the gasket on the orbit sphere of the 3-sphere's fibration by torus knots, lifted back along the fibres and projected into space. Every strand is a torus knot (a trefoil by default) and any two are linked. Made for this catalog.",
    params: &[
        Param::whole("turns", 2.0, 1.0, 5.0, "How many times each strand winds around the ring's axis (1 to 5). With twists it picks the knot: 2 and 3 make trefoils, 1 and 1 plain rings that link like a Hopf link."),
        Param::whole("twists", 3.0, 1.0, 8.0, "How many times each strand winds around the ring's tube (1 to 8): 3 makes trefoils, 5 cinquefoils; more twists, tighter coils."),
        Param::whole("strands", 3.0, 3.0, 8.0, "The big strands around the core (3 to 8): the gasket starts from this many circles in a ring around a central one."),
        Param::num("gap", &[0.35], 0.0, 0.7, "Space around each strand, as a share of its thickness (0 to 0.7): 0 packs them solid into one fat ring; 0.35 shows each knot; more makes them wiry."),
        Param::num("spacing", &[0.025], 0.002, 0.1, "The least space between strands, in the shape's units (0.002 to 0.1): strands thinner than about twice this are left out, so smaller values show finer strands."),
    ],
    iterations: (20, 40, "Inversions followed into the gasket's gaps: more = smaller strands in the gaps (spacing also sets how fine it goes)."),
    bound: 2.05,
    lod: None,
    step: 0.75,
    example: r#"{"type":"seifert_gasket","scale":0.5,"rotation":[-60,0,0]}"#,
    prep: Some(seifert_prep),
    cpu: seifert_gasket,
    wgsl: SEIFERT_GASKET,
    wgsl_fn: "seifert_gasket",
};

/// The ring's outer radius in the shape's units, whatever the knot.
const SEIFERT_SIZE: f64 = 2.0;

/// The envelope (the solid torus |z2|^p <= |z1|^q of S³, a round torus in
/// space), the gasket's circles and the shape's scale, from the parameters.
fn seifert_prep(k: &mut [f32; K]) {
    let (p, q, n) = (k[0] as i32, k[1] as i32, k[2] as f64);
    // a = |z1| on the envelope: b^p = a^q with a² + b² = 1
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..60 {
        let a = 0.5 * (lo + hi);
        let b = (1.0 - a * a).sqrt();
        if b.powi(p) > a.powi(q) {
            lo = a;
        } else {
            hi = a;
        }
    }
    let a = 0.5 * (lo + hi);
    let b = (1.0 - a * a).sqrt();
    let sn = (std::f64::consts::PI / n).sin();
    k[5] = (((1.0 + b) / a) / SEIFERT_SIZE) as f32;
    k[6] = (1.0 / a) as f32;
    k[7] = (b / a) as f32;
    k[8] = (sn / (1.0 + sn)) as f32;
    k[9] = (std::f64::consts::PI / n).cos() as f32;
    k[10] = sn as f32;
    k[11] = (std::f64::consts::TAU / n) as f32;
}

/// (re, im)^n, n small.
#[inline(always)]
fn cpow(re: F8, im: F8, n: u32) -> (F8, F8) {
    let (mut a, mut b) = (F8::one(), F8::zero());
    for _ in 0..n {
        (a, b) = (a * re - b * im, a * im + b * re);
    }
    (a, b)
}

/// The Seifert gasket's distance estimate and orbit trap (the depth of the
/// strand's circle in the gasket).
///
/// The point goes to S³ by inverse stereographic projection, then to the
/// orbit sphere's coordinate u = z2^p / z1^q (constant along each (p, q)
/// torus-knot fibre). There the Apollonian gasket inside the unit circle is
/// folded: turned into the wedge of one big circle, then inverted in the
/// circle through the tangency points of the gap it is in, until it lands in
/// a big circle or the central one. The strand is that circle narrowed by
/// `gap` of its radius plus `spacing`; the distance in u is carried back by
/// the inversions' factors and |∇u| (u's gradient in space, exact for the
/// holomorphic u). Outside the envelope it is the torus' own distance.
fn seifert_gasket(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (pp, qq) = (k[0] as u32, k[1] as u32);
    let (g, kn) = (k[3], k[5]);
    // the least gap in stereographic units
    let w0 = s(k[4] * kn);
    let (tor_r, tor_t) = (k[6], k[7]);
    let r = k[8];
    let (co, si, step) = (k[9], k[10], k[11]);
    let c = 1.0 - r;
    let rc = 1.0 - 2.0 * r;
    // the dual circles of the outer and inner gaps, centred on the wedge's mirror
    let mo = (1.0 - c * c * co * co) / (2.0 * r * co);
    let ro = mo - c * co;
    let mi = (c * c * co * co - rc * rc) / (2.0 * r * co);
    let ri = (mi - c * co).abs();
    let (oxo, oyo, oxi, oyi) = (s(mo * co), s(mo * si), s(mi * co), s(mi * si));
    let (ro2, ri2) = (s(ro * ro), s(ri * ri));

    let (x, y, z) = (p.x * kn, p.y * kn, p.z * kn);
    let r2 = z.mul_add(z, y.mul_add(y, x * x));
    let inv = F8::one() / (r2 + 1.0);
    let (x1, x2, x3, x4) = (x * inv * 2.0, y * inv * 2.0, z * inv * 2.0, (r2 - 1.0) * inv);
    let tor = len2(len2(x, y) - tor_r, z) - tor_t;
    // u = z2^p conj(z1)^q / |z1|^(2q)
    let (ar, ai) = cpow(x3, x4, pp);
    let (br, bi) = cpow(x1, -x2, qq);
    // (|z1|² is over a quarter inside the envelope: the clamp only keeps
    // the far side of the axis finite)
    let m1_raw = x2.mul_add(x2, x1 * x1);
    let m1 = m1_raw.max(s(0.01));
    let m2 = x4.mul_add(x4, x3 * x3).max(s(1e-12));
    let mut m1q = F8::one();
    for _ in 0..qq {
        m1q *= m1;
    }
    let (ur, ui) = ((ar * br - ai * bi) / m1q, (ar * bi + ai * br) / m1q);
    let um = len2(ur, ui);
    let inside = um.lt(F8::one()) & m1_raw.gt(s(0.01));
    // |∇u| in space: |u| |(q / z1, p / z2)| times the projection's factor 2 / (1 + r²)
    let grad = um * (s((qq * qq) as f32) / m1 + s((pp * pp) as f32) / m2).sqrt() * inv * 2.0;

    let (mut u, mut v) = (ur, ui);
    let mut dd = F8::one();
    let mut out = w0;
    let mut depth = F8::zero();
    let mut act: M8 = inside;
    for i in 0..iters {
        if act.none() {
            break;
        }
        // into the wedge 0 <= angle <= pi / n of the big circle on the x axis
        let th = v.atan2(u);
        let th = (th - (th * (1.0 / step)).round() * step).abs();
        let mag = len2(u, v);
        u = F8::select(act, mag * th.cos(), u);
        v = F8::select(act, mag * th.sin(), v);
        let kt = (dd * grad).max(s(1e-20));
        let wk = w0 * kt;
        // the narrowed strand (of radius `keep`: below 0 it is gone, and
        // this still bounds the distance to anything), or the space across
        // the circle (strands outside it are at least `spacing` in)
        let strand = |dist: F8, rad: f32| {
            let keep = s(rad * (1.0 - g)) - wk;
            (dist - keep).min(s(rad) - dist + wk) / kt
        };
        let db = len2(u - c, v);
        let in_b = act & db.lt(s(r));
        let in_c = act.and_not(in_b) & mag.lt(s(rc));
        out = F8::select(in_b, strand(db, r), out);
        out = F8::select(in_c, strand(mag, rc), out);
        depth = F8::select(in_b, s(i as f32), depth);
        depth = F8::select(in_c, s(i as f32 + 0.5), depth);
        act = act.and_not(in_b | in_c);
        // in a gap: invert in its dual circle
        let (eo, fo) = (u - oxo, v - oyo);
        let (ei, fi) = (u - oxi, v - oyi);
        let do2 = fo.mul_add(fo, eo * eo).max(s(1e-20));
        let di2 = fi.mul_add(fi, ei * ei).max(s(1e-20));
        let in_o = act & do2.lt(ro2);
        let in_i = act.and_not(in_o) & di2.lt(ri2);
        // (on a tangency point, in neither: as deep as it goes)
        act &= in_o | in_i;
        let ko = ro2 / do2;
        let ki = ri2 / di2;
        u = F8::select(in_o, eo.mul_add(ko, oxo), F8::select(in_i, ei.mul_add(ki, oxi), u));
        v = F8::select(in_o, fo.mul_add(ko, oyo), F8::select(in_i, fi.mul_add(ki, oyi), v));
        dd = F8::select(in_o, dd * ko, F8::select(in_i, dd * ki, dd));
    }
    let d = F8::select(inside, out, tor + w0).max(s(-1.0));
    (d * (1.0 / kn), (depth * (1.0 / 6.0)).saturate())
}

const SEIFERT_GASKET: &str = r#"
fn seifert_gasket_cpow(z: vec2f, n: u32) -> vec2f {
    var a = vec2f(1.0, 0.0);
    for (var i = 0u; i < n; i++) {
        a = vec2f(a.x * z.x - a.y * z.y, a.x * z.y + a.y * z.x);
    }
    return a;
}

fn seifert_gasket_strand(dist: f32, rad: f32, g: f32, wk: f32, kt: f32) -> f32 {
    let keep = rad * (1.0 - g) - wk;
    return min(dist - keep, rad - dist + wk) / kt;
}

fn seifert_gasket(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let pp = u32(a.x);
    let qq = u32(a.y);
    let g = a.w;
    let kn = b.y;
    let w0 = b.x * kn;
    let r = c.x;
    let co = c.y;
    let si = c.z;
    let step = c.w;
    let cc = 1.0 - r;
    let rc = 1.0 - 2.0 * r;
    let mo = (1.0 - cc * cc * co * co) / (2.0 * r * co);
    let ro = mo - cc * co;
    let mi = (cc * cc * co * co - rc * rc) / (2.0 * r * co);
    let ri = abs(mi - cc * co);
    let oo = vec2f(mo * co, mo * si);
    let oi = vec2f(mi * co, mi * si);
    let q = p * kn;
    let r2 = dot(q, q);
    let inv = 1.0 / (r2 + 1.0);
    let x1 = q.x * inv * 2.0;
    let x2 = q.y * inv * 2.0;
    let x3 = q.z * inv * 2.0;
    let x4 = (r2 - 1.0) * inv;
    let tor = length(vec2f(length(q.xy) - b.z, q.z)) - b.w;
    let za = seifert_gasket_cpow(vec2f(x3, x4), pp);
    let zb = seifert_gasket_cpow(vec2f(x1, -x2), qq);
    let m1_raw = x1 * x1 + x2 * x2;
    let m1 = max(m1_raw, 0.01);
    let m2 = max(x3 * x3 + x4 * x4, 1e-12);
    var m1q = 1.0;
    for (var i = 0u; i < qq; i++) {
        m1q *= m1;
    }
    let u0 = vec2f(za.x * zb.x - za.y * zb.y, za.x * zb.y + za.y * zb.x) / m1q;
    let um = length(u0);
    let grad = um * sqrt(f32(qq * qq) / m1 + f32(pp * pp) / m2) * inv * 2.0;
    var uv = u0;
    var dd = 1.0;
    var out = w0;
    var depth = 0.0;
    let outside = um >= 1.0 || m1_raw <= 0.01;
    var done = outside;
    for (var i = 0u; i < iters; i++) {
        if (done) {
            break;
        }
        var th = atan2(uv.y, uv.x);
        th = abs(th - round(th * (1.0 / step)) * step);
        let mag = length(uv);
        uv = vec2f(mag * cos(th), mag * sin(th));
        let kt = max(dd * grad, 1e-20);
        let wk = w0 * kt;
        let db = length(uv - vec2f(cc, 0.0));
        let in_b = db < r;
        let in_c = !in_b && mag < rc;
        let sd = seifert_gasket_strand(select(mag, db, in_b), select(rc, r, in_b), g, wk, kt);
        out = select(out, sd, in_b || in_c);
        depth = select(depth, f32(i) + select(0.0, 0.5, in_c), in_b || in_c);
        let eo = uv - oo;
        let ei = uv - oi;
        let do2 = max(dot(eo, eo), 1e-20);
        let di2 = max(dot(ei, ei), 1e-20);
        let in_o = !(in_b || in_c) && do2 < ro * ro;
        let in_i = !(in_b || in_c) && !in_o && di2 < ri * ri;
        done = !(in_o || in_i);
        let ko = ro * ro / do2;
        let ki = ri * ri / di2;
        uv = select(select(uv, ei * ki + oi, in_i), eo * ko + oo, in_o);
        dd *= select(select(1.0, ki, in_i), ko, in_o);
    }
    let d = select(out, tor + w0, outside);
    return vec2f(max(d, -1.0) / kn, clamp(depth * (1.0 / 6.0), 0.0, 1.0));
}
"#;

// ---- Indra's cells ---------------------------------------------------------

const INDRA: Formula = Formula {
    name: "indra_cells",
    category: "Fractals: originals",
    desc: "Rooms of the 120-cell, the 4D polytope of 120 dodecahedra, seen in our space by stereographic projection: pentagon-framed rooms that grow toward the edge, each holding a pearl, a small inside-out copy of the whole, and each room in it a pearl of its own. A Kleinian group made of the 120-cell's symmetries and one sphere inversion. Made for this catalog.",
    params: &[
        Param::num("pearl", &[0.5], 0.1, 0.9, "The pearl in each room, as a share of the room's inner radius (0.1 to 0.9): small pearls hold a tiny inside-out world; large ones crowd it against the walls."),
        Param::num("thickness", &[0.04], 0.005, 0.2, "The walls' thickness, as a share of the room's inner radius (0.005 to 0.2)."),
        Param::num("frame", &[0.16], 0.0, 1.0, "The width of the frame around each pentagonal window, as a share of the room's inner radius (0 to 1): 0 leaves only the edges, 1 closes the windows into solid walls."),
        Param::whole("centre", 0.0, 0.0, 3.0, "What sits at the centre of the view: 0 a room (the dodecahedron's symmetry), 1 a window, 2 an edge, 3 a corner where four rooms meet."),
    ],
    iterations: (5, 8, "Pearls within pearls: each level is a room's inside-out world, a few times smaller than the last."),
    bound: f32::INFINITY,
    lod: None,
    step: 0.7,
    example: r#"{"type":"intersection","scale":1.0,"children":[{"type":"sphere","radius":1.0},{"type":"indra_cells"}]}"#,
    prep: Some(indra_prep),
    cpu: indra_cells,
    wgsl: INDRA_CELLS,
    wgsl_fn: "indra_cells",
};

/// The 120-cell's fold, in R⁴ = (w, x, y, z): the dodecahedron's mirrors
/// through the cell centre (1, 0, 0, 0) are x = 0, y = 0 (with z = 0, which
/// abs also folds) and the plane of INDRA_R2; INDRA_NU is the face between
/// the centre cell and its neighbour 36° away along (0, 1, φ), which makes
/// them the Coxeter group H4.
const INDRA_R2: [f32; 3] = [-0.809_017, -0.5, 0.309_017];
const INDRA_NU: [f32; 3] = [-0.309_017, 0.5, 0.809_017];
/// A room's inner radius on S³ (18°).
const INDRA_IN: f32 = std::f32::consts::PI / 10.0;

/// Radians from the shares; the pearl's inside-out world's outer radius; the
/// turn that puts the chosen centre at the origin.
fn indra_prep(k: &mut [f32; K]) {
    let (pearl, t, frame, centre) = (k[0] * INDRA_IN, k[1] * INDRA_IN, k[2] * INDRA_IN, k[3] as i32);
    k[0] = pearl;
    k[1] = t;
    k[2] = frame;
    // the walls nearest the pearl (the room's inner radius less the wall),
    // inverted in it: everything inside the pearl is within this radius
    k[3] = 2.0 * ((pearl * 0.5).tan().powi(2) / ((INDRA_IN - t) * 0.5).tan()).atan();
    // (the corners of the fold's chamber: a face, an edge and a vertex of the centre room)
    let (deg, n): (f32, [f32; 3]) = match centre {
        1 => (18.0, [0.0, 0.525_731_1, 0.850_650_8]),
        2 => (20.905_2, [0.0, 0.0, 1.0]),
        3 => (22.238_8, [0.356_822_1, 0.0, 0.934_172_4]),
        _ => (0.0, [0.0, 0.0, 1.0]),
    };
    k[4] = n[0];
    k[5] = n[1];
    k[6] = n[2];
    k[7] = deg.to_radians().cos();
    k[8] = deg.to_radians().sin();
}

/// Indra's cells: the distance estimate and the orbit trap (how many pearls
/// deep the nearest wall is).
///
/// The point goes to S³ by inverse stereographic projection (the origin to
/// the centre room's centre (1, 0, 0, 0)), turned so the chosen centre is
/// there. Each level folds it into the centre room by the 120-cell's
/// symmetries (H4: the dodecahedron's own mirrors, then the face to the
/// neighbour room when it is past it, until it is not), measures the
/// pentagonal window frames, and if it is inside the pearl (a cap about the
/// room's centre) inverts it there: the pearl's inside is the whole outside
/// turned inside out. Distances on S³ go back by the inversions' factor
/// (first order, as all Kleinian estimates), then to space through the exact
/// chord of the projection: |p - q| = chord · √(1+|p|²) √(1+|q|²) / 2 with
/// √(1+|q|²) at least √(1+|p|²) - |p - q|, which gives the lower bound
/// chord (1+|p|²) / (2 + chord √(1+|p|²)).
fn indra_cells(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (rho, t, border, rin) = (k[0], k[1], k[2], k[3]);
    let (nx, ny, nz, ct, st) = (k[4], k[5], k[6], k[7], k[8]);
    let (cr, tr2, cin) = (1.0 / rho.cos(), rho.tan() * rho.tan(), (rho * 0.999).cos());
    let r2 = p.length_sq();
    let inv = F8::one() / (r2 + 1.0);
    let (mut w, mut x, mut y, mut z) = ((F8::one() - r2) * inv, p.x * inv * 2.0, p.y * inv * 2.0, p.z * inv * 2.0);
    // turn the plane of (1, 0, 0, 0) and (0, n) by the centre's angle
    let a = w;
    let b = z.mul_add(s(nz), y.mul_add(s(ny), x * nx));
    let (da, db) = (a * (ct - 1.0) - b * st, b * (ct - 1.0) + a * st);
    w += da;
    x = db.mul_add(s(nx), x);
    y = db.mul_add(s(ny), y);
    z = db.mul_add(s(nz), z);
    let [r2x, r2y, r2z] = INDRA_R2.map(s);
    let [nuw, nuy, nuz] = INDRA_NU.map(s);
    let mut dd = F8::one();
    let mut best = s(1e9);
    let mut level = F8::zero();
    let mut act: M8 = M8::all_true();
    for l in 0..iters {
        if act.none() {
            break;
        }
        // into the centre room's chamber
        for _ in 0..6 {
            for _ in 0..3 {
                (x, y, z) = (x.abs(), y.abs(), z.abs());
                let d = z.mul_add(r2z, y.mul_add(r2y, x * r2x)).min(F8::zero()) * 2.0;
                x = d.neg_mul_add(r2x, x);
                y = d.neg_mul_add(r2y, y);
                z = d.neg_mul_add(r2z, z);
            }
            let d = z.mul_add(nuz, y.mul_add(nuy, w * nuw));
            let past = d.gt(F8::zero());
            if past.none() {
                break;
            }
            let d = d.max(F8::zero()) * 2.0;
            w = d.neg_mul_add(nuw, w);
            y = d.neg_mul_add(nuy, y);
            z = d.neg_mul_add(nuz, z);
        }
        // the window frame: the face's slab, within `border` of its edge (the y = 0 mirror)
        let face = asin(-z.mul_add(nuz, y.mul_add(nuy, w * nuw)));
        let wall = (face - t).max(asin(y) - border) / dd;
        let nearer = act & wall.lt(best);
        level = F8::select(nearer, s(l as f32), level);
        best = F8::select(act, best.min(wall), best);
        // in the pearl (a hair smaller, so a point on its rim, which the
        // inversion leaves there, is not inverted again and again): invert
        // in it, about (sec ρ, 0, 0, 0), radius tan ρ. Outside it, all it
        // holds is beyond its inside-out walls, within `rin` of its centre;
        // at the last level that ball stands for what it holds
        let inside = act & w.gt(s(cin));
        let held = (acos(w) - rin) / dd;
        let last = if l + 1 == iters { act } else { M8::none_set() };
        best = F8::select(act.and_not(inside) | last, best.min(held), best);
        act = inside.and_not(last);
        let e = w - cr;
        let kk = s(tr2) / z.mul_add(z, y.mul_add(y, x.mul_add(x, e * e))).max(s(1e-20));
        w = F8::select(act, e.mul_add(kk, s(cr)), w);
        x = F8::select(act, x * kk, x);
        y = F8::select(act, y * kk, y);
        z = F8::select(act, z * kk, z);
        dd = F8::select(act, dd * kk, dd);
    }
    // arc to chord to space
    let a = (r2 + 1.0).sqrt();
    let chord = F8::select(best.gt(F8::zero()), (best.min(s(std::f32::consts::PI)) * 0.5).sin() * 2.0, best);
    let d = F8::select(chord.gt(F8::zero()), chord * (r2 + 1.0) / chord.mul_add(a, s(2.0)), chord * (r2 + 1.0) * 0.5);
    (d, (level * (1.0 / 3.0)).saturate())
}

/// asin, for |v| <= 1 (clamped): atan2(v, √(1 - v²)).
#[inline(always)]
fn asin(v: F8) -> F8 {
    let v = v.clamp(-1.0, 1.0);
    v.atan2(v.neg_mul_add(v, F8::one()).max(F8::zero()).sqrt())
}

/// acos, for |v| <= 1 (clamped).
#[inline(always)]
fn acos(v: F8) -> F8 {
    let v = v.clamp(-1.0, 1.0);
    v.neg_mul_add(v, F8::one()).max(F8::zero()).sqrt().atan2(v)
}

const INDRA_CELLS: &str = r#"
const indra_cells_r2: vec3f = vec3f(-0.809017, -0.5, 0.309017);
const indra_cells_nu: vec3f = vec3f(-0.309017, 0.5, 0.809017);

fn indra_cells_asin(v: f32) -> f32 {
    let c = clamp(v, -1.0, 1.0);
    return atan2(c, sqrt(max(1.0 - c * c, 0.0)));
}

fn indra_cells_acos(v: f32) -> f32 {
    let c = clamp(v, -1.0, 1.0);
    return atan2(sqrt(max(1.0 - c * c, 0.0)), c);
}

fn indra_cells(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let rho = a.x;
    let t = a.y;
    let border = a.z;
    let rin = a.w;
    let n = b.xyz;
    let ct = b.w;
    let st = c.x;
    let cr = 1.0 / cos(rho);
    let tr2 = tan(rho) * tan(rho);
    let r2 = dot(p, p);
    let inv = 1.0 / (r2 + 1.0);
    var w = (1.0 - r2) * inv;
    var v = p * inv * 2.0;
    let pa = w;
    let pb = dot(v, n);
    w += pa * (ct - 1.0) - pb * st;
    v += n * (pb * (ct - 1.0) + pa * st);
    var dd = 1.0;
    var best = 1e9;
    var level = 0.0;
    for (var l = 0u; l < iters; l++) {
        for (var i = 0u; i < 6u; i++) {
            for (var j = 0u; j < 3u; j++) {
                v = abs(v);
                v -= indra_cells_r2 * (min(dot(v, indra_cells_r2), 0.0) * 2.0);
            }
            let d = w * indra_cells_nu.x + v.y * indra_cells_nu.y + v.z * indra_cells_nu.z;
            if (d <= 0.0) {
                break;
            }
            w -= indra_cells_nu.x * d * 2.0;
            v = vec3f(v.x, v.y - indra_cells_nu.y * d * 2.0, v.z - indra_cells_nu.z * d * 2.0);
        }
        let face = indra_cells_asin(-(w * indra_cells_nu.x + v.y * indra_cells_nu.y + v.z * indra_cells_nu.z));
        let wall = max(face - t, indra_cells_asin(v.y) - border) / dd;
        level = select(level, f32(l), wall < best);
        best = min(best, wall);
        if (w <= cos(rho * 0.999) || l + 1u == iters) {
            best = min(best, (indra_cells_acos(w) - rin) / dd);
            break;
        }
        let e = w - cr;
        let kk = tr2 / max(dot(v, v) + e * e, 1e-20);
        w = e * kk + cr;
        v *= kk;
        dd *= kk;
    }
    let ar = sqrt(r2 + 1.0);
    let chord = select(best, sin(min(best, 3.14159265) * 0.5) * 2.0, best > 0.0);
    let d = select(chord * (r2 + 1.0) * 0.5, chord * (r2 + 1.0) / (chord * ar + 2.0), chord > 0.0);
    return vec2f(d, clamp(level * (1.0 / 3.0), 0.0, 1.0));
}
"#;

// ---- the Zorich pagodas -----------------------------------------------------

const ZORICH: Formula = Formula {
    name: "zorich_pagodas",
    category: "Fractals: originals",
    desc: "A floor of domed pagodas crowned with tiers of spiky pinnacles, each ringed by smaller ones, in a lattice that runs on for ever: the points that escape upward under a scaled Zorich map, the three-dimensional analogue of the exponential function (V. A. Zorich, 1967; the dynamics studied by Bergweiler and others). Made for this catalog.",
    params: &[
        Param::num("lambda", &[0.42], 0.2, 0.5, "The map's factor (0.2 to 0.5): small values grow low domes with a few spikes; around 0.4 tall tiered pagodas; toward 0.5 they branch into a tangled lace."),
        Param::num("depth", &[3.0], 1.5, 4.0, "How far the escape level lies above the floor, in the map's units (1.5 to 4): low values leave domes with thin spines; high ones grow dense towers."),
    ],
    iterations: (6, 12, "Steps of the map: each adds a tier of smaller pinnacles around the larger ones."),
    bound: f32::INFINITY,
    lod: None,
    step: 0.25,
    example: r#"{"type":"intersection","scale":0.4,"position":[0,-0.9,0],"children":[{"type":"box","size":[6,4.4,6],"position":[0,1.8,0]},{"type":"zorich_pagodas"}]}"#,
    prep: None,
    cpu: zorich_pagodas,
    wgsl: ZORICH_PAGODAS,
    wgsl_fn: "zorich_pagodas",
};

/// Folds a coordinate into the Zorich map's beam [-1, 1] (period 4: odd
/// beams mirrored), and whether it was mirrored.
#[inline(always)]
fn zorich_fold(u: F8) -> (F8, M8) {
    let k = ((u + 1.0) * 0.5).floor();
    let w = u - k * 2.0;
    let odd = (k * 0.5).fract().gt(s(0.25));
    (F8::select(odd, -w, w), odd)
}

/// The Zorich pagodas' distance estimate and orbit trap (the step at which
/// the nearest part escapes).
///
/// Space is (x1, x2, x3) = (x, z, depth - y): the floor y <= 0 is x3 >= depth,
/// the points already escaped. Zorich's map is Z(x) = e^x3 h(x1, x2), h the
/// square [-1, 1]² projected radially onto the upper half of the unit sphere
/// (from the pyramid (x1, x2, 1 - max|xi|)), extended to the plane by
/// reflecting across the square's sides (each reflection mirrors the image in
/// x3 = 0). A point is in the shape when some iterate of x -> lambda Z(x)
/// reaches x3 >= depth. Each step's Lipschitz bound is lambda e^x3 max(1,
/// √2 / |pyramid point|) (h's derivative is at most √2 over the pyramid
/// point's length); the distance to step n's escape is (depth - x3) over
/// their product. For an exponential map that first-order estimate is off
/// both ways: uncapped it overshoots far below the escape level, and capped
/// too tightly (at 0.5) it creeps, so rays stall into grain. The gap is
/// capped at 2 and the step held at 0.25, which passes the steepness check
/// and renders clean across the parameters' ranges.
fn zorich_pagodas(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (lam, depth) = (k[0], k[1]);
    let (mut x1, mut x2, mut x3) = (p.x, p.z, s(depth) - p.y);
    let mut best = s(depth) - x3;
    let mut lip = F8::one();
    let mut level = F8::zero();
    let mut act: M8 = x3.lt(s(depth));
    for n in 1..=iters {
        if act.none() {
            break;
        }
        let (a, fa) = zorich_fold(x1);
        let (b, fb) = zorich_fold(x2);
        let m = a.abs().max(b.abs());
        let v3 = F8::one() - m;
        let nv = v3.mul_add(v3, b.mul_add(b, a * a)).sqrt();
        let e = x3.min(s(30.0)).exp() * lam;
        let h3 = v3 / nv;
        let h3 = F8::select(fa ^ fb, -h3, h3);
        x1 = F8::select(act, e * a / nv, x1);
        x2 = F8::select(act, e * b / nv, x2);
        x3 = F8::select(act, e * h3, x3);
        lip = F8::select(act, lip * e * (s(std::f32::consts::SQRT_2) / nv).max(F8::one()), lip);
        let d = (s(depth) - x3).min(s(2.0)) / lip.max(s(1e-30));
        let nearer = act & d.lt(best);
        level = F8::select(nearer, s(n as f32), level);
        best = F8::select(act, best.min(d), best);
        act &= x3.lt(s(depth));
    }
    (best, (level * (1.0 / 6.0)).saturate())
}

const ZORICH_PAGODAS: &str = r#"
fn zorich_pagodas(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let lam = a.x;
    let depth = a.y;
    var x = vec3f(p.x, p.z, depth - p.y);
    var best = depth - x.z;
    var lip = 1.0;
    var level = 0.0;
    for (var n = 1u; n <= iters; n++) {
        if (x.z >= depth) {
            break;
        }
        let k = floor((x.xy + vec2f(1.0)) * 0.5);
        let w = x.xy - k * 2.0;
        let odd = fract(k * 0.5) > vec2f(0.25);
        let ab = select(w, -w, odd);
        let m = max(abs(ab.x), abs(ab.y));
        let v3 = 1.0 - m;
        let nv = sqrt(dot(ab, ab) + v3 * v3);
        let e = exp(min(x.z, 30.0)) * lam;
        let h3 = select(v3 / nv, -v3 / nv, odd.x != odd.y);
        x = vec3f(e * ab.x / nv, e * ab.y / nv, e * h3);
        lip = lip * e * max(1.41421356 / nv, 1.0);
        let d = min(depth - x.z, 2.0) / max(lip, 1e-30);
        level = select(level, f32(n), d < best);
        best = min(best, d);
    }
    return vec2f(best, clamp(level * (1.0 / 6.0), 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of each formula's parameters
    /// (it tries only the defaults): the estimate, times the step, changes at
    /// most 1.6 times as fast as the point moves.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[(&str, &[(usize, f32)])] = &[
            ("seifert_gasket", &[(0, 5.0), (1, 1.0)]),
            ("seifert_gasket", &[(0, 1.0), (1, 8.0)]),
            ("seifert_gasket", &[(0, 1.0), (1, 1.0), (2, 8.0)]),
            ("seifert_gasket", &[(0, 5.0), (1, 8.0), (2, 8.0)]),
            ("seifert_gasket", &[(3, 0.0), (4, 0.002)]),
            ("seifert_gasket", &[(3, 0.7), (4, 0.1)]),
            ("indra_cells", &[(0, 0.9), (1, 0.005)]),
            ("indra_cells", &[(0, 0.1), (1, 0.2), (2, 0.0)]),
            ("indra_cells", &[(2, 1.0), (3, 3.0)]),
            ("indra_cells", &[(3, 1.0)]),
            ("indra_cells", &[(3, 2.0)]),
            ("zorich_pagodas", &[(0, 0.2), (1, 1.5)]),
            ("zorich_pagodas", &[(0, 0.5), (1, 1.5)]),
            ("zorich_pagodas", &[(0, 0.2), (1, 4.0)]),
            ("zorich_pagodas", &[(0, 0.5), (1, 4.0)]),
        ];
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        for &(name, set) in corners {
            let id = find(name).unwrap();
            let f = super::super::get(id);
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            let node = FormulaNode { id, values, iterations: f.iterations.0 };
            let k = node.pack();
            for near in [1.0, 0.3] {
                let reach = if f.bound.is_finite() { f.bound * 1.2 } else { 3.0 } * near;
                let e = 1e-3 * reach;
                for _ in 0..1500 {
                    let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                    let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                    let (d, t) = (f.cpu)(p, &k, node.iters());
                    let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                    for i in 0..8 {
                        let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                        assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{name} {set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                        if a > 10.0 * e && b.is_finite() {
                            let steep = (b - a).abs() / e * f.step;
                            assert!(steep < 1.6, "{name} {set:?}: changes {steep:.2} times as fast as the point at {:?}", p.lane(i));
                        }
                    }
                }
            }
        }
    }
}
