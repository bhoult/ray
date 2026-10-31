//! A formula from Clifford analysis, the function theory of geometric
//! algebra: escape-time iteration of a monogenic polynomial. A vector field f
//! on space is monogenic when the vector derivative ∇ = Σ eᵢ∂ᵢ of
//! Cl(3,0) kills it, ∇f = ∇·f + ∇∧f = 0: the Cauchy–Riemann equations of
//! three dimensions (in the plane the same equation picks out the conjugates
//! of holomorphic functions). The polynomial ones are exactly the gradients
//! of harmonic polynomials, so z ← ±∇h(z) + c is the 3D counterpart of
//! z ← z̄ⁿ + c. Choosing h with a polyhedron's symmetry gives the map that
//! symmetry, with no pole and no spherical coordinates.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[DIRAC];

const DIRAC: Formula = Formula {
    name: "dirac_frost",
    category: "Fractals: originals",
    desc: "Frost crystals grown by the Cauchy–Riemann equations of three dimensions: z ← ∇h(z) + c with h a harmonic polynomial with a polyhedron's symmetry, so that the map is monogenic (the Dirac operator of Clifford analysis kills it), the 3D counterpart of a holomorphic polynomial. The cube's quartic grows feathered spires around dished faces; the tetrahedral sextics grow urchins and icosahedral cages. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("quintic", 0.0, 0.0, 1.0, "0: the cubic map, the gradient of the cube's quartic harmonic x⁴+y⁴+z⁴−3(x²y²+y²z²+z²x²). 1: a quintic, the gradient of the sextic harmonic that mix picks."),
        Param::num("mix", &[1.0], -1.4, 1.4, "Which sextic harmonic, for the quintic: 0 the cube's (a long-spined star), 1 the icosahedron's (a cage of pentagonal hollows), between and beyond them ones with a tetrahedron's symmetry (negative values are mirror images). Toward ±1.66 the map stops growing along the cube's diagonals and spikes run off to infinity, so the range stops at 1.4."),
        Param::whole("flip", 1.0, 0.0, 1.0, "1 negates the gradient, z ← −∇h(z) + c. The maps are odd, so this is not a turn or a mirror: 1 grows the frost (cubic) and the cages and stars (quintic), 0 long-spined stars (cubic) and urchins (quintic)."),
        Param::whole("julia", 0.0, 0.0, 1.0, "0 adds the point itself at each step (the Mandelbrot type); 1 adds the constant c, drawing one of the map's Julia sets."),
        Param::num("c", &[0.5, 0.0, 0.0], -1.0, 1.0, "The Julia constant, with julia 1. On a symmetry axis it keeps some of the symmetry; near the origin the set is one spiky body, further out it twists and breaks apart."),
    ],
    iterations: (12, 40, "Escape-time iterations: more = finer feathers on the spires, at a 3×3 matrix product each."),
    bound: 2.5,
    lod: None,
    step: 0.5,
    example: r#"{"type":"dirac_frost","scale":0.6,"rotation":[20,25,0]}"#,
    prep: Some(dirac_prep),
    cpu: dirac_frost,
    wgsl: DIRAC_FROST,
    wgsl_fn: "dirac_frost",
};

/// The two sextic harmonics with a tetrahedron's (rotational) symmetry, as
/// RMS over the unit sphere: the cube's x⁶+y⁶+z⁶ − 7.5 Σx⁴y² + 90 x²y²z²
/// (√(8/13)) and the cube-odd (x²−y²)(y²−z²)(z²−x²) (√(32/15015)). They are
/// orthogonal, so every mix cos t, sin t of the two at unit RMS has unit RMS,
/// and its gradient RMS √78 (for a harmonic of degree ℓ the gradient's mean
/// square on the sphere is ℓ(2ℓ+1) times the function's), which normalises
/// the quintic.
const RMS_CUBE6: f64 = 0.784_464_540_552_736;
const RMS_TETRA6: f64 = 0.046_164_944_830_489;
const RMS_GRAD6: f64 = 8.831_760_866_327_848;
/// The mixing angle of the icosahedron's sextic: tan t = √(21/11).
const ICO: f64 = 0.944_293_120_196_166;

/// The sextic's coefficients of Σx⁶, the cyclic Σx⁴y² (x⁴y², y⁴z², z⁴x²),
/// the cyclic Σx⁴z² and x²y²z², from the mix, divided by the gradient's RMS.
fn dirac_prep(k: &mut [f32; K]) {
    let t = k[1] as f64 * ICO;
    let (co, si) = (t.cos() / RMS_CUBE6 / RMS_GRAD6, t.sin() / RMS_TETRA6 / RMS_GRAD6);
    k[8] = co as f32;
    k[9] = (-7.5 * co - si) as f32;
    k[10] = (-7.5 * co + si) as f32;
    k[11] = (90.0 * co) as f32;
}

/// A 3×3 matrix of lanes, row-major.
type M3 = [F8; 9];

/// a b
#[inline(always)]
fn mm(a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|n| {
        let (i, j) = (n / 3, n % 3);
        a[i * 3 + 2].mul_add(b[6 + j], a[i * 3 + 1].mul_add(b[3 + j], a[i * 3] * b[j]))
    })
}

#[inline(always)]
fn sel(m: M8, a: &M3, b: &M3) -> M3 {
    std::array::from_fn(|n| F8::select(m, a[n], b[n]))
}

/// The sextic's x derivative (y's and z's by cycling the arguments, since
/// the sextic is unchanged by cycling x, y, z).
#[inline(always)]
fn sx_g(x: F8, y: F8, z: F8, q: &[f32; 4]) -> F8 {
    let (x2, y2, z2) = (x * x, y * y, z * z);
    let t = x2 * x2 * (6.0 * q[0]) + x2 * y2 * (4.0 * q[1]) + z2 * z2 * (2.0 * q[1]) + x2 * z2 * (4.0 * q[2]) + y2 * y2 * (2.0 * q[2]) + y2 * z2 * (2.0 * q[3]);
    x * t
}

/// Its second x derivative.
#[inline(always)]
fn sx_hd(x: F8, y: F8, z: F8, q: &[f32; 4]) -> F8 {
    let (x2, y2, z2) = (x * x, y * y, z * z);
    x2 * x2 * (30.0 * q[0]) + x2 * y2 * (12.0 * q[1]) + z2 * z2 * (2.0 * q[1]) + x2 * z2 * (12.0 * q[2]) + y2 * y2 * (2.0 * q[2]) + y2 * z2 * (2.0 * q[3])
}

/// Its x-y derivative.
#[inline(always)]
fn sx_ho(x: F8, y: F8, z: F8, q: &[f32; 4]) -> F8 {
    let (x2, y2, z2) = (x * x, y * y, z * z);
    x * y * (x2 * (8.0 * q[1]) + y2 * (8.0 * q[2]) + z2 * (4.0 * q[3]))
}

const BAIL: f32 = 6.0;
/// ln of where the next band lands from the bailout: |∇h| on the unit
/// sphere is at least 1/4 for the cubic (on the cube's edge axes, against 1
/// on its face axes) and 0.11 for the quintics (at mix 0), less |c|.
const LN_HI3: f32 = 3.9;
const LN_HI5: f32 = 6.5;
/// The most a Julia set's estimate may be within JULIA_CORE of the origin
/// (beyond, the limit rises at half the rate of the distance): an orbit that
/// passes near the map's critical point, the origin, where the Hessian
/// vanishes, loses most of its Jacobian there, and the estimate overshoots
/// around such points. A Julia set whose critical point escapes falls apart
/// and has them all through its gaps. (The Mandelbrot type adds I at every
/// step, which keeps the Jacobian from collapsing.)
const JULIA_CAP: f32 = 0.1;
const JULIA_CORE: f32 = 1.5;

/// Dirac frost: the distance estimate and the orbit trap (the least, along
/// the orbit, of the harmonic at the iterate's direction: which of the
/// polyhedron's cones the orbit came nearest).
///
/// z ← σ∇h(z) + c (σ = ±1, c the point or the Julia constant). The map is
/// not conformal (Liouville: no polynomial is, in 3D), but its Jacobian is
/// σ Hess h, symmetric and traceless (that is the Dirac equation), so the
/// orbit's whole Jacobian J ← σ Hess h(z) J + I is carried along and the
/// estimate is 0.5 r ln r / ‖J‖_F (Frobenius bounds the stretch), mixed with
/// the previous iterate's across the bailout so the escape bands meet.
fn dirac_frost(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let quintic = k[0] > 0.5;
    let sg = if k[2] > 0.5 { -1.0 } else { 1.0 };
    let julia = k[3] > 0.5;
    let c = if julia { V8::new(s(k[4]), s(k[5]), s(k[6])) } else { p };
    let add = if julia { 0.0 } else { 1.0 };
    let q = [k[8], k[9], k[10], k[11]];
    let (ln_hi, tscale) = if quintic { (LN_HI5, 0.3) } else { (LN_HI3, 0.5) };
    let ln_bail = BAIL.ln();
    let (o, l) = (F8::zero(), F8::one());
    let est = |z: V8, j: &M3| {
        let n2 = j.iter().fold(F8::zero(), |a, &v| v.mul_add(v, a));
        let r2 = z.length_sq().max(s(1e-12));
        r2.sqrt() * r2.ln() * 0.25 / n2.sqrt().max(s(1e-20))
    };
    let mut z = p;
    let mut j: M3 = [l, o, o, o, l, o, o, o, l];
    let mut r = z.length();
    let mut cur = est(z, &j);
    let mut prev = cur;
    let mut trap = s(1.0);
    let mut act: M8 = r.lt(s(BAIL));
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let (x, y, w) = (z.x, z.y, z.z);
        let (g, h): (V8, M3) = if quintic {
            let g = V8::new(sx_g(x, y, w, &q), sx_g(y, w, x, &q), sx_g(w, x, y, &q));
            let (hxy, hyz, hzx) = (sx_ho(x, y, w, &q), sx_ho(y, w, x, &q), sx_ho(w, x, y, &q));
            (g, [sx_hd(x, y, w, &q), hxy, hzx, hxy, sx_hd(y, w, x, &q), hyz, hzx, hyz, sx_hd(w, x, y, &q)])
        } else {
            // ∇h / 4 and its Jacobian, h the cube's quartic
            let (x2, y2, w2) = (x * x, y * y, w * w);
            let g = V8::new(x * (x2 - (y2 + w2) * 1.5), y * (y2 - (w2 + x2) * 1.5), w * (w2 - (x2 + y2) * 1.5));
            let (hxy, hyz, hzx) = (x * y * -3.0, y * w * -3.0, w * x * -3.0);
            (g, [x2 * 3.0 - (y2 + w2) * 1.5, hxy, hzx, hxy, y2 * 3.0 - (w2 + x2) * 1.5, hyz, hzx, hyz, w2 * 3.0 - (x2 + y2) * 1.5])
        };
        // the harmonic at the iterate's direction (Euler: z·∇h = deg h · h)
        let r2 = (r * r).max(s(1e-20));
        let rr = if quintic { r2 * r2 * r2 } else { r2 * r2 };
        let tv = (g.dot(z) / rr).mul_add(s(sg * tscale), s(0.5));
        trap = F8::select(act, trap.min(tv), trap);
        let mut nj = mm(&h, &j);
        for v in nj.iter_mut() {
            *v *= sg;
        }
        nj[0] += add;
        nj[4] += add;
        nj[8] += add;
        z = V8::select(act, g * sg + c, z);
        j = sel(act, &nj, &j);
        r = F8::select(act, z.length(), r);
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, est(z, &j), cur);
        act &= r.lt(s(BAIL));
    }
    // (escaped: the last two iterates' estimates mixed by how far past the
    // bailout it went; inside: 0, eased in from the bailout's root up)
    let lr = r.max(s(1e-12)).ln();
    let t = F8::smoothstep(ln_hi, ln_bail, lr);
    let ease = F8::smoothstep(ln_bail * 0.5, ln_bail, lr);
    let d = F8::select(act, (cur * ease).max(F8::zero()), (cur - prev).mul_add(t, prev));
    let d = if julia { d.min((p.length() - JULIA_CORE).max(F8::zero()).mul_add(s(0.5), s(JULIA_CAP))) } else { d };
    (d, trap.saturate())
}

const DIRAC_FROST: &str = r#"
fn dirac_frost_g(x: f32, y: f32, z: f32, q: vec4f) -> f32 {
    let x2 = x * x;
    let y2 = y * y;
    let z2 = z * z;
    return x * (x2 * x2 * (6.0 * q.x) + x2 * y2 * (4.0 * q.y) + z2 * z2 * (2.0 * q.y) + x2 * z2 * (4.0 * q.z) + y2 * y2 * (2.0 * q.z) + y2 * z2 * (2.0 * q.w));
}

fn dirac_frost_hd(x: f32, y: f32, z: f32, q: vec4f) -> f32 {
    let x2 = x * x;
    let y2 = y * y;
    let z2 = z * z;
    return x2 * x2 * (30.0 * q.x) + x2 * y2 * (12.0 * q.y) + z2 * z2 * (2.0 * q.y) + x2 * z2 * (12.0 * q.z) + y2 * y2 * (2.0 * q.z) + y2 * z2 * (2.0 * q.w);
}

fn dirac_frost_ho(x: f32, y: f32, z: f32, q: vec4f) -> f32 {
    return x * y * (x * x * (8.0 * q.y) + y * y * (8.0 * q.z) + z * z * (4.0 * q.w));
}

fn dirac_frost_est(z: vec3f, j: mat3x3f) -> f32 {
    let r2 = max(dot(z, z), 1e-12);
    let n2 = dot(j[0], j[0]) + dot(j[1], j[1]) + dot(j[2], j[2]);
    return sqrt(r2) * log(r2) * 0.25 / max(sqrt(n2), 1e-20);
}

fn dirac_frost(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let quintic = a.x > 0.5;
    let sg = select(1.0, -1.0, a.z > 0.5);
    let julia = a.w > 0.5;
    let k = select(p, b.xyz, julia);
    let add = select(1.0, 0.0, julia);
    let eye = mat3x3f(add, 0.0, 0.0, 0.0, add, 0.0, 0.0, 0.0, add);
    let q = c;
    let ln_hi = select(3.9, 6.5, quintic);
    let tscale = select(0.5, 0.3, quintic);
    let ln_bail = log(6.0);
    var z = p;
    var j = mat3x3f(1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    var r = length(z);
    var cur = dirac_frost_est(z, j);
    var prev = cur;
    var trap = 1.0;
    for (var i = 0u; i < iters; i++) {
        if (r >= 6.0) {
            break;
        }
        let x = z.x;
        let y = z.y;
        let w = z.z;
        var g: vec3f;
        var h: mat3x3f;
        if (quintic) {
            g = vec3f(dirac_frost_g(x, y, w, q), dirac_frost_g(y, w, x, q), dirac_frost_g(w, x, y, q));
            let hxy = dirac_frost_ho(x, y, w, q);
            let hyz = dirac_frost_ho(y, w, x, q);
            let hzx = dirac_frost_ho(w, x, y, q);
            h = mat3x3f(dirac_frost_hd(x, y, w, q), hxy, hzx, hxy, dirac_frost_hd(y, w, x, q), hyz, hzx, hyz, dirac_frost_hd(w, x, y, q));
        } else {
            let x2 = x * x;
            let y2 = y * y;
            let w2 = w * w;
            g = vec3f(x * (x2 - (y2 + w2) * 1.5), y * (y2 - (w2 + x2) * 1.5), w * (w2 - (x2 + y2) * 1.5));
            let hxy = x * y * -3.0;
            let hyz = y * w * -3.0;
            let hzx = w * x * -3.0;
            h = mat3x3f(x2 * 3.0 - (y2 + w2) * 1.5, hxy, hzx, hxy, y2 * 3.0 - (w2 + x2) * 1.5, hyz, hzx, hyz, w2 * 3.0 - (x2 + y2) * 1.5);
        }
        let r2 = max(r * r, 1e-20);
        let rr = select(r2 * r2, r2 * r2 * r2, quintic);
        trap = min(trap, dot(g, z) / rr * (sg * tscale) + 0.5);
        // (H is symmetric: rows or columns, the same)
        j = (h * j) * sg + eye;
        z = g * sg + k;
        r = length(z);
        prev = cur;
        cur = dirac_frost_est(z, j);
    }
    let lr = log(max(r, 1e-12));
    var t = clamp((lr - ln_hi) / (ln_bail - ln_hi), 0.0, 1.0);
    t = t * t * (3.0 - t - t);
    var e = clamp((lr - ln_bail * 0.5) / (ln_bail * 0.5), 0.0, 1.0);
    e = e * e * (3.0 - e - e);
    let d = select(prev + (cur - prev) * t, max(cur * e, 0.0), r < 6.0);
    // (a Julia set's limit, as JULIA_CAP and JULIA_CORE)
    let dj = min(d, max(length(p) - 1.5, 0.0) * 0.5 + 0.1);
    return vec2f(select(d, dj, julia), clamp(trap, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// The steepness check of mod.rs at the corners of the parameters (it
    /// tries only the defaults): over random points near and around the
    /// shape, a hundredth of the estimate (times the step) may change at
    /// most 1.25 times as fast as the point moves, a thousandth 2 times, and
    /// every distance is finite with its trap in 0..1.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(2, 0.0)],
            &[(0, 1.0)],
            &[(0, 1.0), (2, 0.0)],
            &[(0, 1.0), (1, 0.0)],
            &[(0, 1.0), (1, 0.0), (2, 0.0)],
            &[(0, 1.0), (1, 1.4)],
            &[(0, 1.0), (1, 1.4), (2, 0.0)],
            &[(0, 1.0), (1, -1.4)],
            &[(0, 1.0), (1, -1.4), (2, 0.0)],
            &[(3, 1.0)],
            &[(3, 1.0), (2, 0.0), (4, -0.6)],
            &[(3, 1.0), (4, 0.0), (5, 0.0), (6, 0.0)],
            &[(3, 1.0), (0, 1.0), (4, 0.0), (5, 0.6), (6, 0.0)],
            &[(3, 1.0), (0, 1.0), (2, 0.0), (4, 0.3), (5, 0.3), (6, 0.3)],
            &[(3, 1.0), (4, 1.0), (5, 1.0), (6, 1.0)],
            &[(3, 1.0), (0, 1.0), (1, -1.4), (4, -1.0), (5, 1.0), (6, -1.0)],
        ];
        let id = find("dirac_frost").unwrap();
        let f = super::super::get(id);
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        for &set in corners {
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            let node = FormulaNode { id, values, iterations: f.iterations.0 };
            let k = node.pack();
            for near in [1.0, 0.3] {
                let reach = f.bound * 1.2 * near;
                let e = 1e-3 * reach;
                let mut slopes = Vec::new();
                for _ in 0..1500 {
                    let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                    let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                    let (d, t) = (f.cpu)(p, &k, node.iters());
                    let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                    for i in 0..8 {
                        let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                        assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                        if a > 10.0 * e && b.is_finite() {
                            slopes.push((b - a).abs() / e * f.step);
                        }
                    }
                }
                slopes.sort_by(f32::total_cmp);
                let n = slopes.len();
                if n < 100 {
                    continue;
                }
                let (p99, p999) = (slopes[n * 99 / 100], slopes[n * 999 / 1000]);
                assert!(p99 < 1.25 && p999 < 2.0, "{set:?} (within {reach}): a hundredth changes {p99:.2}, a thousandth {p999:.2} times as fast as the point");
            }
        }
    }
}
