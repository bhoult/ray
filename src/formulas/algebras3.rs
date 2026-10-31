//! Algebraic maps that are not conformal: Dudley's quadratic, the Collatz
//! map carried into 3D (plain and opened up) and a triplex Newton fractal.
//! Their stretch differs by direction, so a running scalar derivative
//! misjudges them; each carries the orbit's whole Jacobian instead
//! (Newton's by dual numbers) and bounds |∇r| by its Frobenius norm in the
//! potential's estimate 0.5 r ln r / |∇r|. How much an escape grows also
//! differs by direction, so that estimate jumps from one escape band to the
//! next; it is mixed from the last two iterates' to keep it continuous
//! (`finish`). (Mandelbulber's coastalbrot, meant for this family, is left
//! out: its growth factor sin(sin(sin(π/3 + πx))) rᵖ vanishes on planes all
//! through space, so its exterior is dust at every scale and no estimate
//! holds.)

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "quick_dudley",
        category: "Fractals: algebras",
        desc: "Dudley's quadratic: z squared in a three-dimensional algebra whose two imaginary units obey m² = n, n² = m and mn = −1, plus the point, as for the Mandelbrot set. A lopsided, wind-swept lump with a crest of spikes, frilled edges and a long needle. Formula by quick-dudley (Fractal Forums, 2012); ported from Mandelbulber 2.",
        params: &[
            Param::num("square", &[1.0, 1.0, 1.0], 0.25, 2.0, "Weights on the squared terms (x², z², y²) of the new x, y and z: 1 is Dudley's map. Lowering one flattens and spreads the shape along that axis. Useful from 0.5 to 1.5."),
            Param::num("cross", &[2.0, 2.0, 2.0], -3.0, 3.0, "Weights on the cross terms (yz, xy, xz) of the new x, y and z: 2 is Dudley's map. Larger pulls out longer spikes and frills, smaller rounds the shape off. Useful from 1.5 to 2.5."),
            Param::whole("julia", 0.0, 0.0, 1.0, "1 adds the constant c at every step instead of the point: one of the map's Julia sets, swirled and flatter, often broken into islands."),
            Param::num("c", &[-0.6, 0.3, 0.2], -1.5, 1.5, "The Julia constant (with julia 1): near the origin a single swirled body, further out it breaks into islands."),
        ],
        iterations: (9, 32, "Escape-time iterations: more = finer frills and fur at the edges (the shape gets hairier, not bigger), at a 3×3 matrix product each."),
        bound: 1.5,
        lod: None,
        step: 0.75,
        example: r#"{"type":"quick_dudley","scale":0.6}"#,
        prep: None,
        cpu: quick_dudley,
        wgsl: QUICK_DUDLEY,
        wgsl_fn: "quick_dudley",
    },
    Formula {
        name: "collatz",
        category: "Fractals: algebras",
        desc: "Mandelbulber's 3D Collatz fractal: the map (1 + 4z − (1 + 2z) cos πz) / 4, which carries the Collatz rule over to complex numbers, lifted into 3D with cos πz replaced by a half turn of z about (1, 1, 1) whose axis is left unnormalised (so it also stretches along the diagonal). A curved, three-cornered shell with layered ridges and fine fringed edges. After Claude Heiland-Allen's Collatz fractal (mathr, 2016); ported from Mandelbulber 2.",
        params: &[],
        iterations: (16, 40, "Escape-time iterations: more = finer ridges and fringes; past about 10 the shape barely changes."),
        bound: 1.45,
        lod: None,
        step: 0.35,
        example: r#"{"type":"collatz","scale":0.6}"#,
        prep: None,
        cpu: collatz,
        wgsl: COLLATZ,
        wgsl_fn: "collatz",
    },
    Formula {
        name: "collatz_mod",
        category: "Fractals: algebras",
        desc: "Mandelbulber's Collatz Mod: the 3D Collatz map with its turn, axis, offset and scale opened up, and the point optionally added as for a Mandelbrot set. At its default angle (0.8 of a half turn rather than a whole one) it twists into a three-armed pinwheel with fringed blades. Ported from Mandelbulber 2.",
        params: &[
            Param::num("angle", &[0.8], 0.0, 2.0, "The turn, in half turns: 1 is the plain Collatz map. Either side of 1 the shell twists into arms, one way or the other (0.8 and 1.2 are mirror images). Useful from 0.7 to 1.3."),
            Param::num("axis", &[1.0, 1.0, 1.0], -2.0, 2.0, "The turn's axis, unnormalised (its length also stretches the map), and the constant in the (axis + 2z) factor. Tilting it skews the arms; far from (1, 1, 1) the shape can outgrow its bounds."),
            Param::num("offset", &[1.0, 1.0, 1.0], -2.0, 2.0, "The constant added before scaling: changes the arms' lengths and where the cusps fall."),
            Param::num("step_scale", &[0.25], 0.1, 0.5, "The factor on each step: smaller shrinks the set, larger grows it. Useful from 0.2 to 0.3."),
            Param::num("c_mult", &[0.0, 0.0, 0.0], -1.0, 1.0, "How much of the point (taken z, y, x, as Mandelbulber adds it) is added each step: 0 gives the Julia-type shell; towards 0.5 it swirls into a Mandelbrot-type shape, whose estimate is rougher (expect some fuzz)."),
        ],
        iterations: (16, 40, "Escape-time iterations: more = finer arms and fringes."),
        bound: 1.5,
        lod: None,
        step: 0.35,
        example: r#"{"type":"collatz_mod","scale":0.6}"#,
        prep: Some(collatz_mod_prep),
        cpu: collatz_mod,
        wgsl: COLLATZ_MOD,
        wgsl_fn: "collatz_mod",
    },
    Formula {
        name: "newton",
        category: "Fractals: algebras",
        desc: "A Newton fractal in 3D: Newton's method for z³ = 1 in the triplex (Mandelbulb) algebra, turned inside out so that the basin of the root 1 escapes and the basins of the other two roots make the solid. A ribbed barrel whose ends are concentric rings. Formula by gannjondal (Fractal Forums); ported from Mandelbulber 2's Newton Pow3.",
        params: &[Param::num(
            "offset",
            &[0.0, 0.0, 0.0],
            -0.6,
            0.6,
            "Added after every step, like a Julia constant: along x it splits the barrel into two ribbed halves joined by filaments; sideways it twists and roughens it.",
        )],
        iterations: (16, 48, "Newton steps: the rings are sharp by about 12; more adds finer filaments where the basins meet."),
        bound: 1.25,
        lod: None,
        step: 0.9,
        example: r#"{"type":"newton","scale":0.6}"#,
        prep: None,
        cpu: newton,
        wgsl: NEWTON,
        wgsl_fn: "newton",
    },
];

/// A 3×3 matrix of lanes, row-major.
type M3 = [F8; 9];

fn eye() -> M3 {
    let (o, l) = (F8::zero(), F8::one());
    [l, o, o, o, l, o, o, o, l]
}

/// a b
#[inline(always)]
fn mm(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|n| {
        let (i, j) = (n / 3, n % 3);
        a[i * 3 + 2].mul_add(b[6 + j], a[i * 3 + 1].mul_add(b[3 + j], a[i * 3] * b[j]))
    })
}

/// a + w I
#[inline(always)]
fn plus_eye(mut a: M3, w: f32) -> M3 {
    a[0] += w;
    a[4] += w;
    a[8] += w;
    a
}

#[inline(always)]
fn sel(m: M8, a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|n| F8::select(m, a[n], b[n]))
}

/// The estimate at one iterate, 0.5 r ln r / |∇r|, with |∇r| = |Jᵀz| / r
/// bounded by J's Frobenius norm (n2 its square): Jᵀz alone nearly
/// vanishes where z lines up with a direction J barely stretches, and the
/// estimate then overshoots.
#[inline(always)]
fn de_norm(z: V8, n2: F8) -> F8 {
    let r2 = z.length_sq().max(s(1e-12));
    r2.sqrt() * r2.ln() * 0.25 / n2.sqrt().max(s(1e-20))
}

#[inline(always)]
fn de_frob(z: V8, j: &M3) -> F8 {
    de_norm(z, j.iter().fold(F8::zero(), |a, &v| v.mul_add(v, a)))
}

/// The distance: the escaping iterate's estimate (`cur`) mixed with the one
/// before (`prev`) by how far past the bailout the escape went (ln r): all
/// `cur` just past it, all `prev` from `ln_hi` on, where the next band's
/// points land (they escape from just under the bailout), so the bands
/// meet. Inside (the orbit has not escaped), 0.
#[inline(always)]
fn finish(prev: F8, cur: F8, r: F8, ln_bail: f32, ln_hi: f32, inside: M8) -> F8 {
    let lr = r.max(s(1e-12)).ln();
    let t = F8::smoothstep(ln_hi, ln_bail, lr);
    // (inside 0, but eased in from the bailout's root up, to meet the
    // iterates that escape at the last iteration; the estimate itself is
    // erratic there and would roughen the normals)
    let ease = F8::smoothstep(ln_bail * 0.5, ln_bail, lr);
    F8::select(inside, (cur * ease).max(F8::zero()), (cur - prev).mul_add(t, prev))
}

// Dudley's map grows at least 0.577 r² in every direction: from a bailout of
// 10 the next band lands past 40 (|c| < 3 here).
const QD_BAIL: f32 = 10.0;
const QD_LN_BAIL: f32 = core::f32::consts::LN_10;
const QD_LN_HI: f32 = 3.688879;
/// (the Mandelbrot-type set's centre, moved to the origin)
const QD_CENTRE: [f32; 3] = [-0.67, -0.2, -0.2];

/// Dudley's quadratic (x² − 2yz, z² + 2xy, y² + 2xz) + c, with weights
/// on its squares and cross terms (Mandelbulber's quick_dudley_mod).
fn quick_dudley(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (a, b) = ([k[0] * 2.0, k[1] * 2.0, k[2] * 2.0], [k[3], k[4], k[5]]);
    let julia = k[6] > 0.5;
    let p = if julia { p } else { p + V8::new(s(QD_CENTRE[0]), s(QD_CENTRE[1]), s(QD_CENTRE[2])) };
    let c = if julia { V8::new(s(k[7]), s(k[8]), s(k[9])) } else { p };
    let add = if julia { 0.0 } else { 1.0 };
    let bail = s(QD_BAIL);
    let mut z = p;
    let mut j = eye();
    let mut r = z.length();
    let mut cur = de_frob(z, &j);
    let mut prev = cur;
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (x, y, w) = (z.x, z.y, z.z);
        let f = [x * a[0], w * -b[0], y * -b[0], y * b[1], x * b[1], w * a[1], w * b[2], y * a[2], x * b[2]];
        let nj = plus_eye(mm(&f, &j), add);
        let nz = V8::new(x * x * k[0] - y * w * b[0], w * w * k[1] + x * y * b[1], y * y * k[2] + w * x * b[2]) + c;
        z = V8::select(act, nz, z);
        j = sel(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, de_frob(z, &j), cur);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    (finish(prev, cur, r, QD_LN_BAIL, QD_LN_HI, act), ((trap - 0.3) * 1.5).saturate())
}

const QUICK_DUDLEY: &str = r#"
fn quick_dudley_de(z: vec3f, j: mat3x3f) -> f32 {
    let r2 = max(dot(z, z), 1e-12);
    let n2 = dot(j[0], j[0]) + dot(j[1], j[1]) + dot(j[2], j[2]);
    return sqrt(r2) * log(r2) * 0.25 / max(sqrt(n2), 1e-20);
}

fn quick_dudley_finish(prev: f32, cur: f32, r: f32, lb: f32, lh: f32, inside: bool) -> f32 {
    var t = clamp((log(max(r, 1e-12)) - lh) / (lb - lh), 0.0, 1.0);
    t = t * t * (3.0 - t - t);
    var e = clamp((log(max(r, 1e-12)) - lb * 0.5) / (lb * 0.5), 0.0, 1.0);
    e = e * e * (3.0 - e - e);
    return select(prev + (cur - prev) * t, max(cur * e, 0.0), inside);
}

fn quick_dudley(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let sq = a.xyz;
    let a2 = sq * 2.0;
    let cr = vec3f(a.w, b.x, b.y);
    let julia = b.z > 0.5;
    let q = select(p + vec3f(-0.67, -0.2, -0.2), p, julia);
    let k = select(q, vec3f(b.w, c.x, c.y), julia);
    let add = select(1.0, 0.0, julia);
    let eye = mat3x3f(add, 0.0, 0.0, 0.0, add, 0.0, 0.0, 0.0, add);
    var z = q;
    var j = mat3x3f(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    var r = length(z);
    var cur = quick_dudley_de(z, j);
    var prev = cur;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        // (the map's Jacobian, by columns)
        let f = mat3x3f(z.x * a2.x, z.y * cr.y, z.z * cr.z, z.z * -cr.x, z.x * cr.y, z.y * a2.z, z.y * -cr.x, z.z * a2.y, z.x * cr.z);
        j = f * j + eye;
        z = vec3f(z.x * z.x * sq.x - z.y * z.z * cr.x, z.z * z.z * sq.y + z.x * z.y * cr.y, z.y * z.y * sq.z + z.z * z.x * cr.z) + k;
        r = length(z);
        prev = cur;
        cur = quick_dudley_de(z, j);
        trap = min(trap, r);
    }
    return vec2f(quick_dudley_finish(prev, cur, r, 2.302585, 3.688879, r < 10.0), clamp((trap - 0.3) * 1.5, 0.0, 1.0));
}
"#;

/// The Collatz map's parameters (Mandelbulber's collatz_mod): the turn
/// (cos, sin), its axis n (also the constant in n + 2z), the offset b, the
/// scale, and how much of the point (taken zyx, as Mandelbulber adds it)
/// is added.
struct Collatz {
    ca: f32,
    sa: f32,
    n: [f32; 3],
    b: [f32; 3],
    sc: f32,
    cm: [f32; 3],
}

// The plain map grows at least about 0.15 r² from a bailout of 100 (its
// quadratic part at least 0.196 r²).
const CZ_BAIL: f32 = 100.0;
const CZ_LN_BAIL: f32 = 4.605_17;
const CZ_LN_HI: f32 = 7.313_22;

/// z ← s (b + 4z − (n + 2z) ⊙ R(z)) + m ⊙ p_zyx, R the turn of z about n
/// (unnormalised, as Mandelbulber has it). The point is first scaled by
/// `grow` and moved by `centre` (the set's own size and place).
fn collatz_core(p: V8, q: &Collatz, iters: u32, grow: f32, centre: [f32; 3]) -> (F8, F8) {
    let (ca, sa, n, sc) = (q.ca, q.sa, q.n, q.sc);
    let ac = 1.0 - ca;
    let p = V8::new(p.x.mul_add(s(grow), s(centre[0])), p.y.mul_add(s(grow), s(centre[1])), p.z.mul_add(s(grow), s(centre[2])));
    let add = V8::new(p.z * q.cm[0], p.y * q.cm[1], p.x * q.cm[2]);
    // R's Jacobian: cos a I + sin a [n]× + (1 − cos a) n nᵀ
    let jr = [
        ca + ac * n[0] * n[0],
        ac * n[0] * n[1] - sa * n[2],
        ac * n[0] * n[2] + sa * n[1],
        ac * n[1] * n[0] + sa * n[2],
        ca + ac * n[1] * n[1],
        ac * n[1] * n[2] - sa * n[0],
        ac * n[2] * n[0] - sa * n[1],
        ac * n[2] * n[1] + sa * n[0],
        ca + ac * n[2] * n[2],
    ];
    let bail = s(CZ_BAIL);
    let mut z = p;
    let mut j = eye();
    let mut r = z.length();
    let mut cur = de_frob(z, &j);
    let mut prev = cur;
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let nd = z.z.mul_add(s(n[2]), z.y.mul_add(s(n[1]), z.x * n[0])) * ac;
        let cr = V8::new(z.z * n[1] - z.y * n[2], z.x * n[2] - z.z * n[0], z.y * n[0] - z.x * n[1]);
        let rot = V8::new(z.x * ca + cr.x * sa + nd * n[0], z.y * ca + cr.y * sa + nd * n[1], z.z * ca + cr.z * sa + nd * n[2]);
        let h = [z.x * 2.0 + n[0], z.y * 2.0 + n[1], z.z * 2.0 + n[2]];
        let rv = [rot.x, rot.y, rot.z];
        // f'(z) = s (4I − 2 diag(R) − diag(n + 2z) R')
        let f: M3 = std::array::from_fn(|k| {
            let (i, jj) = (k / 3, k % 3);
            let d = if i == jj { 4.0 - rv[i] * 2.0 } else { F8::zero() };
            (d - h[i] * jr[k]) * sc
        });
        let mut nj = mm(&f, &j);
        nj[2] += q.cm[0];
        nj[4] += q.cm[1];
        nj[6] += q.cm[2];
        let nz = V8::new((z.x * 4.0 + q.b[0] - h[0] * rot.x) * sc, (z.y * 4.0 + q.b[1] - h[1] * rot.y) * sc, (z.z * 4.0 + q.b[2] - h[2] * rot.z) * sc) + add;
        z = V8::select(act, nz, z);
        j = sel(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, de_frob(z, &j), cur);
        trap = F8::select(act, trap.min(r), trap);
        act &= r.lt(bail);
    }
    (finish(prev, cur, r, CZ_LN_BAIL, CZ_LN_HI, act) * (1.0 / grow), (trap.max(s(1e-6)).ln() * 0.5).saturate())
}

/// The plain map, z ← (1 + 4z − (1 + 2z) ⊙ R(z)) / 4, R the half turn
/// about (1, 1, 1); its set (with its fringes) is about 12 across about
/// (2.7, 2.7, 2.7).
fn collatz(p: V8, _k: &[f32; K], iters: u32) -> (F8, F8) {
    let q = Collatz { ca: -1.0, sa: 0.0, n: [1.0; 3], b: [1.0; 3], sc: 0.25, cm: [0.0; 3] };
    collatz_core(p, &q, iters, 4.4, [2.69; 3])
}

/// (cos, sin) of the angle (in half turns) into k[0], k[11].
fn collatz_mod_prep(k: &mut [f32; K]) {
    let a = k[0] as f64 * std::f64::consts::PI;
    k[0] = a.cos() as f32;
    k[11] = a.sin() as f32;
}

/// At its default angle (0.8) the set is about 17 across about (3.9, 3.9, 3.9).
fn collatz_mod(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let q = Collatz { ca: k[0], sa: k[11], n: [k[1], k[2], k[3]], b: [k[4], k[5], k[6]], sc: k[7], cm: [k[8], k[9], k[10]] };
    collatz_core(p, &q, iters, 6.2, [3.92; 3])
}

/// The Collatz kernel in WGSL, its function named $f.
macro_rules! collatz_wgsl {
    ($f:literal) => {
        concat!(
            "
fn ",
            $f,
            "(p: vec3f, ca: f32, sa: f32, n: vec3f, bb: vec3f, sc: f32, cm: vec3f, iters: u32, grow: f32, centre: vec3f) -> vec2f {
    let ac = 1.0 - ca;
    let q = p * grow + centre;
    let add = vec3f(q.z, q.y, q.x) * cm;
    let cmm = mat3x3f(0.0, 0.0, cm.z, 0.0, cm.y, 0.0, cm.x, 0.0, 0.0);
    // (R's Jacobian, by columns)
    let jr = mat3x3f(vec3f(ca, n.z * sa, -n.y * sa) + n * (ac * n.x), vec3f(-n.z * sa, ca, n.x * sa) + n * (ac * n.y), vec3f(n.y * sa, -n.x * sa, ca) + n * (ac * n.z));
    var z = q;
    var j = mat3x3f(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    var r = length(z);
    var cur = r * log(max(r * r, 1e-12)) * 0.25 / sqrt(3.0);
    var prev = cur;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 100.0) {
            break;
        }
        let rot = z * ca + cross(n, z) * sa + n * (dot(n, z) * ac);
        let h = n + z * 2.0;
        let d = 4.0 - rot * 2.0;
        let f = mat3x3f(vec3f(d.x, 0.0, 0.0) - h * jr[0], vec3f(0.0, d.y, 0.0) - h * jr[1], vec3f(0.0, 0.0, d.z) - h * jr[2]) * sc;
        j = f * j + cmm;
        z = (z * 4.0 + bb - h * rot) * sc + add;
        r = length(z);
        prev = cur;
        let r2 = max(dot(z, z), 1e-12);
        cur = sqrt(r2) * log(r2) * 0.25 / max(sqrt(dot(j[0], j[0]) + dot(j[1], j[1]) + dot(j[2], j[2])), 1e-20);
        trap = min(trap, r);
    }
    let lr = log(max(r, 1e-12));
    var t = clamp((lr - 7.313220) / (4.605170 - 7.313220), 0.0, 1.0);
    t = t * t * (3.0 - t - t);
    var e = clamp((lr - 2.302585) / 2.302585, 0.0, 1.0);
    e = e * e * (3.0 - e - e);
    let de = select(prev + (cur - prev) * t, max(cur * e, 0.0), r < 100.0);
    return vec2f(de / grow, clamp(log(max(trap, 1e-6)) * 0.5, 0.0, 1.0));
}
"
        )
    };
}

const COLLATZ: &str = concat!(
    collatz_wgsl!("collatz_core"),
    "
fn collatz(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    return collatz_core(p, -1.0, 0.0, vec3f(1.0), vec3f(1.0), 0.25, vec3f(0.0), iters, 4.4, vec3f(2.69));
}
"
);

const COLLATZ_MOD: &str = concat!(
    collatz_wgsl!("collatz_mod_core"),
    "
fn collatz_mod(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    return collatz_mod_core(p, a.x, c.w, a.yzw, b.xyz, b.w, c.xyz, iters, 6.2, vec3f(3.92));
}
"
);

/// A number and its gradient with respect to the point (forward-mode dual
/// numbers): the Newton map's Jacobian without writing it out.
#[derive(Clone, Copy)]
struct Dual {
    v: F8,
    g: V8,
}

impl Dual {
    fn add(self, b: Dual) -> Dual {
        Dual { v: self.v + b.v, g: self.g + b.g }
    }
    fn sub(self, b: Dual) -> Dual {
        Dual { v: self.v - b.v, g: self.g - b.g }
    }
    fn mul(self, b: Dual) -> Dual {
        Dual { v: self.v * b.v, g: self.g * b.v + b.g * self.v }
    }
    fn scale(self, k: f32) -> Dual {
        Dual { v: self.v * k, g: self.g * k }
    }
    fn plus(self, k: f32) -> Dual {
        Dual { v: self.v + k, g: self.g }
    }
    /// 1 / self, self > 0 (kept from 0)
    fn recip(self) -> Dual {
        let i = s(1.0) / self.v.max(s(1e-20));
        Dual { v: i, g: self.g * -(i * i) }
    }
    fn sqrt(self) -> Dual {
        let r = self.v.max(s(1e-20)).sqrt();
        Dual { v: r, g: self.g * (s(0.5) / r) }
    }
    fn sel(m: M8, a: Dual, b: Dual) -> Dual {
        Dual { v: F8::select(m, a.v, b.v), g: V8::select(m, a.g, b.g) }
    }
}

// Near the root 1 the map is z ← q² + O(1) (q the quaternion x + yi + zj):
// it grows by r² in every direction. 30 keeps Newton's step from cancelling
// away f32's digits.
const NT_BAIL: f32 = 30.0;
const NT_LN_BAIL: f32 = 3.401_197;
/// (where the next band lands: 0.7 · 30²)
const NT_LN_HI: f32 = 6.445_72;
const NT_CENTRE: f32 = -0.655;

/// gannjondal's Newton fractal for z³ = 1 in the triplex algebra
/// (Mandelbulber's newton_pow3): z is carried as 1 / (w − 1), w Newton's
/// iterate, so the basin of the root 1 escapes and the other two roots'
/// basins make the solid.
fn newton(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (o, l) = (F8::zero(), F8::one());
    let off = [k[0], k[1], k[2]];
    let p = V8::new(p.x + NT_CENTRE, p.y, p.z);
    let mut x = Dual { v: p.x, g: V8::new(l, o, o) };
    let mut y = Dual { v: p.y, g: V8::new(o, l, o) };
    let mut z = Dual { v: p.z, g: V8::new(o, o, l) };
    let bail2 = s(NT_BAIL * NT_BAIL);
    let mut r2 = p.length_sq();
    let mut cur = de_norm(p, s(3.0));
    let mut prev = cur;
    let mut trap = s(1e9);
    let mut act: M8 = r2.lt(bail2);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        // into Newton's plane: w = conj(z) / |z|² + 1
        let inv = x.mul(x).add(y.mul(y)).add(z.mul(z)).recip();
        let wx = x.mul(inv).plus(1.0);
        let wy = y.mul(inv).scale(-1.0);
        let wz = z.mul(inv).scale(-1.0);
        // w ← 2w/3 + 1 / (3w²), the triplex square's conjugate over |w|⁴
        let (tx, ty, tz) = (wx.mul(wx), wy.mul(wy), wz.mul(wz));
        let iw = tx.add(ty).add(tz).recip();
        let q = iw.mul(iw).scale(1.0 / 3.0);
        let rxy = tx.add(ty);
        let h1 = tz.mul(rxy.recip()).scale(-1.0).plus(1.0);
        let mx = h1.mul(tx.sub(ty)).mul(q);
        let my = h1.mul(wx).mul(wy).mul(q).scale(-2.0);
        let mz = wz.mul(rxy.sqrt()).mul(q).scale(2.0);
        let nx = wx.scale(2.0 / 3.0).add(mx).plus(-1.0);
        let ny = wy.scale(2.0 / 3.0).add(my);
        let nz = wz.scale(2.0 / 3.0).sub(mz);
        // and back: z = conj(w − 1) / |w − 1|² + offset
        let inv = nx.mul(nx).add(ny.mul(ny)).add(nz.mul(nz)).recip();
        let zx = nx.mul(inv).plus(off[0]);
        let zy = ny.mul(inv).scale(-1.0).plus(off[1]);
        let zz = nz.mul(inv).scale(-1.0).plus(off[2]);
        x = Dual::sel(act, zx, x);
        y = Dual::sel(act, zy, y);
        z = Dual::sel(act, zz, z);
        r2 = F8::select(act, z.v.mul_add(z.v, y.v.mul_add(y.v, x.v * x.v)), r2);
        let n2 = x.g.length_sq() + y.g.length_sq() + z.g.length_sq();
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, de_norm(V8::new(x.v, y.v, z.v), n2), cur);
        trap = F8::select(act, trap.min(r2), trap);
        act &= r2.lt(bail2);
    }
    (finish(prev, cur, r2.sqrt(), NT_LN_BAIL, NT_LN_HI, act), trap.sqrt().saturate())
}

const NEWTON: &str = r#"
fn newton_de(z: vec3f, n2: f32) -> f32 {
    let r2 = max(dot(z, z), 1e-12);
    return sqrt(r2) * log(r2) * 0.25 / max(sqrt(n2), 1e-20);
}

fn newton_finish(prev: f32, cur: f32, r: f32, lb: f32, lh: f32, inside: bool) -> f32 {
    var t = clamp((log(max(r, 1e-12)) - lh) / (lb - lh), 0.0, 1.0);
    t = t * t * (3.0 - t - t);
    var e = clamp((log(max(r, 1e-12)) - lb * 0.5) / (lb * 0.5), 0.0, 1.0);
    e = e * e * (3.0 - e - e);
    return select(prev + (cur - prev) * t, max(cur * e, 0.0), inside);
}

// dual numbers: (value, gradient)
fn newton_mul(a: vec4f, b: vec4f) -> vec4f {
    return vec4f(a.x * b.x, a.yzw * b.x + b.yzw * a.x);
}

fn newton_recip(a: vec4f) -> vec4f {
    let i = 1.0 / max(a.x, 1e-20);
    return vec4f(i, a.yzw * -(i * i));
}

fn newton_sqrt(a: vec4f) -> vec4f {
    let r = sqrt(max(a.x, 1e-20));
    return vec4f(r, a.yzw * (0.5 / r));
}

fn newton(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let off = a.xyz;
    let one = vec4f(1.0, 0.0, 0.0, 0.0);
    let q = p + vec3f(-0.655, 0.0, 0.0);
    var x = vec4f(q.x, 1.0, 0.0, 0.0);
    var y = vec4f(q.y, 0.0, 1.0, 0.0);
    var z = vec4f(q.z, 0.0, 0.0, 1.0);
    var r2 = dot(q, q);
    var cur = newton_de(q, 3.0);
    var prev = cur;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r2 >= 900.0) {
            break;
        }
        let inv = newton_recip(newton_mul(x, x) + newton_mul(y, y) + newton_mul(z, z));
        let wx = newton_mul(x, inv) + one;
        let wy = newton_mul(y, inv) * -1.0;
        let wz = newton_mul(z, inv) * -1.0;
        let tx = newton_mul(wx, wx);
        let ty = newton_mul(wy, wy);
        let tz = newton_mul(wz, wz);
        let iw = newton_recip(tx + ty + tz);
        let iq = newton_mul(iw, iw) * (1.0 / 3.0);
        let rxy = tx + ty;
        let h1 = newton_mul(tz, newton_recip(rxy)) * -1.0 + one;
        let mx = newton_mul(newton_mul(h1, tx - ty), iq);
        let my = newton_mul(newton_mul(newton_mul(h1, wx), wy), iq) * -2.0;
        let mz = newton_mul(newton_mul(wz, newton_sqrt(rxy)), iq) * 2.0;
        let nx = wx * (2.0 / 3.0) + mx - one;
        let ny = wy * (2.0 / 3.0) + my;
        let nz = wz * (2.0 / 3.0) - mz;
        let inv2 = newton_recip(newton_mul(nx, nx) + newton_mul(ny, ny) + newton_mul(nz, nz));
        x = newton_mul(nx, inv2) + vec4f(off.x, 0.0, 0.0, 0.0);
        y = newton_mul(ny, inv2) * -1.0 + vec4f(off.y, 0.0, 0.0, 0.0);
        z = newton_mul(nz, inv2) * -1.0 + vec4f(off.z, 0.0, 0.0, 0.0);
        r2 = z.x * z.x + y.x * y.x + x.x * x.x;
        prev = cur;
        cur = newton_de(vec3f(x.x, y.x, z.x), dot(x.yzw, x.yzw) + dot(y.yzw, y.yzw) + dot(z.yzw, z.yzw));
        trap = min(trap, r2);
    }
    return vec2f(newton_finish(prev, cur, sqrt(r2), 3.401197, 6.445720, r2 < 900.0), clamp(sqrt(trap), 0.0, 1.0));
}
"#;

