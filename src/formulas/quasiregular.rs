//! Julia sets of a quasiregular power map: Mayer's three-dimensional
//! analogue of z^d, built on Zorich's map, plus a constant.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[MAYER];

const MAYER: Formula = Formula {
    name: "mayer_julia",
    category: "Fractals: originals",
    desc: "Julia sets of z² + c in three dimensions done the quasiregular way: a degree-two power map of space built on Zorich's map, whose iterates keep bounded distortion (V. Mayer's uniformly quasiregular power maps, 1997-98; A. Fletcher's degree-two one), plus a constant. Spiky frost-fern crystals along the x axis, where it is the real map |x|^√2 + c; curled dragons off it. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param {
            name: "multiplier",
            default: &[1.0, 1.0],
            min: -3.0,
            max: 3.0,
            whole: true,
            doc: "The Gaussian integer a + bi the power map multiplies Zorich's square grid by (-3 to 3 each): the degree is a² + b², the radial power √(a² + b²). [1, 1] is degree two, the analogue of z² + c; [2, 0] is degree 4, [2, 1] 5, [3, 0] 9. -m is the same map as m, and anything of degree under 2 is taken as [1, 1].",
        },
        Param::num("c", &[-0.75, 0.0, 0.0], -1.5, 1.5, "The constant added after every step (-1.5 to 1.5 each). Along the x axis the map is the real |x|^√2 + c (degree two): from -0.4 to -1.5 grow spiky fern crystals, thinner the further out; sideways components (0.1 to 0.3) curl them into dragons; most other values give a furry ball or a dust of grains."),
    ],
    iterations: (14, 40, "Steps of the map: more give finer fronds, and Julia sets near the edge of connectedness need more."),
    bound: 3.7,
    lod: None,
    step: 0.6,
    example: r#"{"type":"mayer_julia","scale":0.55,"rotation":[0,0,90]}"#,
    prep: Some(mayer_prep),
    cpu: mayer_julia,
    wgsl: MAYER_JULIA,
    wgsl_fn: "mayer_julia",
};

/// The escape radius. (Once |y| > 3.7, |y|^√2 - |c| > |y| for |c| up to
/// 1.5√3: it escapes, so the set lies within the bound.)
const BAIL: f32 = 100.0;

/// The multiplier (a unit or 0 becomes 1 + i); c turned into the map's frame
/// (k[2..5]); k[5]: the radial power |m|; k[6]: ln of where an orbit that
/// escapes from just under the bailout lands next.
fn mayer_prep(k: &mut [f32; K]) {
    if k[0] * k[0] + k[1] * k[1] < 2.0 {
        k[0] = 1.0;
        k[1] = 1.0;
    }
    let sp = (k[0] * k[0] + k[1] * k[1]).sqrt();
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let (cx, cy, cz) = (k[2], k[3], k[4]);
    k[2] = (cx + cz) * h;
    k[3] = (cx - cz) * h;
    k[4] = cy;
    k[5] = sp;
    k[6] = sp * BAIL.ln() - 0.05;
    // the critical value c's escape rate: 0 when the Julia set is connected
    let (m, c) = ([k[0] as f64, k[1] as f64, sp as f64], [k[2] as f64, k[3] as f64, k[4] as f64]);
    let mut y = c;
    let mut g = 0.0;
    for n in 1..200 {
        let r = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt();
        if r > 1e10 {
            g = r.ln() / m[2].powi(n - 1);
            break;
        }
        let q = mayer_p64(y, m);
        y = [q[0] + c[0], q[1] + c[1], q[2] + c[2]];
    }
    k[7] = (1.0 / (1.0 + DUST * (sp * sp - 1.0) as f64 * g)) as f32;
}

/// How much the estimate is shrunk for a disconnected Julia set (a dust):
/// by 1 + DUST (degree - 1) G(c), G(c) the escape rate of the critical
/// value c (the image of the critical point 0).
/// There the potential has critical points (the preimages of 0) between the
/// grains, near which it is far from linear and its estimate overshoots;
/// the steepness check at the parameters' corners sets the constant.
const DUST: f64 = 0.4;

/// The power map P(y) = |y|^s H(y / |y|) for one point, in f64 (the map's
/// frame; m = [a, b, s]).
fn mayer_p64(y: [f64; 3], m: [f64; 3]) -> [f64; 3] {
    let r = (y[0] * y[0] + y[1] * y[1] + y[2] * y[2]).sqrt();
    if r < 1e-200 {
        return [0.0; 3];
    }
    let den = y[2].abs() + y[0].abs().max(y[1].abs());
    let (a0, b) = (y[0] / den, y[1] / den - 1.0);
    let a = if y[2] < 0.0 { 1.0 - a0 } else { a0 - 1.0 };
    let fold = |x: f64| {
        let k = ((x + 1.0) * 0.5).floor();
        let w = x - 2.0 * k;
        let odd = k.rem_euclid(2.0) > 0.5;
        (if odd { -w } else { w }, odd)
    };
    let (u, fu) = fold(m[0] * a - m[1] * b + 1.0);
    let (w, fw) = fold(m[1] * a + m[0] * b + 1.0);
    let v3 = 1.0 - u.abs().max(w.abs());
    let f = r.powf(m[2]) / (u * u + w * w + v3 * v3).sqrt();
    [u * f, w * f, if fu != fw { -v3 * f } else { v3 * f }]
}

/// Within this radius of the critical point 0, where P flattens out (its
/// stretch goes as |y|^(s - 1)), the gradient is carried as if it stretched
/// as at this radius: the estimate there assumes the potential is linear,
/// and near a critical point it is not.
/// (The WGSL repeats it as 0.6.)
const FLOOR: f32 = 0.6;

/// A dual number: a value and its gradient in space.
#[derive(Clone, Copy)]
struct Du {
    v: F8,
    g: V8,
}

impl Du {
    #[inline(always)]
    fn add(self, b: Du) -> Du {
        Du { v: self.v + b.v, g: self.g + b.g }
    }
    #[inline(always)]
    fn mul(self, b: Du) -> Du {
        Du { v: self.v * b.v, g: self.g * b.v + b.g * self.v }
    }
    /// self / b for b > 0 (kept from 0)
    #[inline(always)]
    fn div(self, b: Du) -> Du {
        let i = F8::one() / b.v.max(s(1e-12));
        let q = self.v * i;
        Du { v: q, g: (self.g - b.g * q) * i }
    }
    #[inline(always)]
    fn scale(self, k: f32) -> Du {
        Du { v: self.v * k, g: self.g * k }
    }
    #[inline(always)]
    fn plus(self, k: f32) -> Du {
        Du { v: self.v + k, g: self.g }
    }
    #[inline(always)]
    fn neg_if(self, m: M8) -> Du {
        Du { v: F8::select(m, -self.v, self.v), g: V8::select(m, -self.g, self.g) }
    }
    #[inline(always)]
    fn abs(self) -> Du {
        self.neg_if(self.v.lt(F8::zero()))
    }
    #[inline(always)]
    fn sel(m: M8, a: Du, b: Du) -> Du {
        Du { v: F8::select(m, a.v, b.v), g: V8::select(m, a.g, b.g) }
    }
}

/// Folds a coordinate into Zorich's beam [-1, 1] (period 4, odd beams
/// mirrored), and whether it was mirrored.
#[inline(always)]
fn mayer_fold(u: Du) -> (Du, M8) {
    let k = ((u.v + 1.0) * 0.5).floor();
    let w = Du { v: u.v - k * 2.0, g: u.g };
    let odd = (k * 0.5).fract().gt(s(0.25));
    (w.neg_if(odd), odd)
}

/// r ln r / |∇r| from r² and its gradient: half the escape rate (the
/// potential) over its gradient.
#[inline(always)]
fn mayer_de(r2: Du) -> F8 {
    let v = r2.v.max(s(1e-12));
    v * v.ln() * 0.5 / r2.g.length().max(s(1e-20))
}

/// The Julia set's distance estimate and orbit trap (the nearest the orbit
/// comes to the real axis in its first three steps: a colour per frond).
///
/// Zorich's map Z(x) = e^x3 h(x1, x2) takes the beam over the square
/// [-1, 1]² onto the upper half-space (h: the square radially onto the upper
/// hemisphere, through the pyramid (x1, x2, 1 - max|xi|)), extended to all
/// of space by reflections. The power map is P(Z(x)) = Z(A x), A multiplying
/// (x1, x2) by m = a + bi about the corner (1, 1) and x3 by |m|: since A
/// carries Z's symmetry group into itself, P is well defined. So P(y) =
/// |y|^|m| H(y / |y|), H a degree-|m|² map of the sphere (a pillowcase
/// Lattès map), and the point iterates y -> P(y) + c. Z's inverse is cheap:
/// the pyramid point is (y1, y2) / (|y3| + max(|y1|, |y2|)), and below the
/// equator it lies in the neighbouring beam, across x1 = 1.
///
/// The frame: the corner (1, 1)'s ray, which P fixes, is x (where P is
/// |x|^|m|), and Z's axis is y. The estimate is ½ r ln r / |∇r| with ∇r
/// carried by dual numbers through the exact map, blended across the
/// bailout, with the stretch floored near the critical point (FLOOR) and
/// the whole shrunk for a dust (DUST); lanes whose gradient grows past 1e15
/// stop (they are in the set).
fn mayer_julia(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (ma, mb, sp, ln_hi) = (k[0], k[1], k[5], k[6]);
    let h = std::f32::consts::FRAC_1_SQRT_2;
    let (o, hh) = (F8::zero(), s(h));
    let mut y1 = Du { v: (p.x + p.z) * h, g: V8::new(hh, o, hh) };
    let mut y2 = Du { v: (p.x - p.z) * h, g: V8::new(hh, o, -hh) };
    let mut y3 = Du { v: p.y, g: V8::new(o, F8::one(), o) };
    let mut r2 = y1.mul(y1).add(y2.mul(y2)).add(y3.mul(y3));
    let mut cur = mayer_de(r2);
    let mut prev = cur;
    let mut trap = s(1e9);
    let bail2 = s(BAIL * BAIL);
    let mut act: M8 = r2.v.lt(bail2);
    for step in 0..iters {
        if act.none() {
            break;
        }
        // back through Zorich's map: the point (a, b) of the square, then
        // times m about the corner (1, 1)
        let (a1, a2, a3) = (y1.abs(), y2.abs(), y3.abs());
        let den = a3.add(Du::sel(a1.v.gt(a2.v), a1, a2));
        let a0 = y1.div(den);
        let b = y2.div(den).plus(-1.0);
        let a = Du::sel(y3.v.lt(o), a0.scale(-1.0).plus(1.0), a0.plus(-1.0));
        let (u, fu) = mayer_fold(a.scale(ma).add(b.scale(-mb)).plus(1.0));
        let (w, fw) = mayer_fold(a.scale(mb).add(b.scale(ma)).plus(1.0));
        // and forward: onto the hemisphere, and |y|^s out
        let (au, aw) = (u.abs(), w.abs());
        let v3 = Du::sel(au.v.gt(aw.v), au, aw).scale(-1.0).plus(1.0);
        let n2 = u.mul(u).add(w.mul(w)).add(v3.mul(v3));
        let nv = n2.v.sqrt();
        let n = Du { v: nv, g: n2.g * (s(0.5) / nv) };
        let r2v = r2.v.max(s(1e-30));
        let rsv = r2v.powf(s(sp * 0.5));
        let rs = Du { v: rsv, g: r2.g * (rsv * (sp * 0.5) / r2v) };
        // (near 0 the map's stretch is floored, see FLOOR)
        let r = r2v.sqrt();
        let kf = F8::select(r.lt(s(FLOOR)), (s(FLOOR) / r.max(s(1e-6))).powf(s(sp - 1.0)), F8::one());
        let q = |d: Du, c: f32| {
            let m = d.div(n).mul(rs);
            Du { v: m.v + c, g: m.g * kf }
        };
        let q1 = q(u, k[2]);
        let q2 = q(w, k[3]);
        let q3 = q(v3.neg_if(fu ^ fw), k[4]);
        y1 = Du::sel(act, q1, y1);
        y2 = Du::sel(act, q2, y2);
        y3 = Du::sel(act, q3, y3);
        r2 = Du::sel(act, y1.mul(y1).add(y2.mul(y2)).add(y3.mul(y3)), r2);
        prev = F8::select(act, cur, prev);
        cur = F8::select(act, mayer_de(r2), cur);
        // (the real axis is y1 = y2, y3 = 0)
        let ax = y1.v - y2.v;
        let off = ax.mul_add(ax * 0.5, y3.v * y3.v);
        if step < 3 {
            trap = F8::select(act, trap.min(off), trap);
        }
        act &= r2.v.lt(bail2) & r2.g.length_sq().lt(s(1e30));
    }
    let r = r2.v.sqrt();
    let lr = r.max(s(1e-12)).ln();
    let (lb, inside) = (BAIL.ln(), r2.v.lt(bail2));
    let t = F8::smoothstep(ln_hi, lb, lr);
    // (inside 0, eased in from the bailout's root up to meet the lanes that
    // escape at the last step)
    let ease = F8::smoothstep(lb * 0.5, lb, lr);
    let d = F8::select(inside, (cur * ease).max(o), (cur - prev).mul_add(t, prev));
    (d * k[7], ((trap.sqrt().sqrt() - 0.45) * 1.8).saturate())
}

const MAYER_JULIA: &str = r#"
// dual numbers: (value, gradient)
fn mayer_julia_mul(a: vec4f, b: vec4f) -> vec4f {
    return vec4f(a.x * b.x, a.yzw * b.x + b.yzw * a.x);
}

fn mayer_julia_div(a: vec4f, b: vec4f) -> vec4f {
    let i = 1.0 / max(b.x, 1e-12);
    let q = a.x * i;
    return vec4f(q, (a.yzw - b.yzw * q) * i);
}

fn mayer_julia_abs(a: vec4f) -> vec4f {
    return select(a, -a, a.x < 0.0);
}

fn mayer_julia_fold(u: vec4f) -> vec4f {
    let k = floor((u.x + 1.0) * 0.5);
    let w = vec4f(u.x - k * 2.0, u.yzw);
    return select(w, -w, fract(k * 0.5) > 0.25);
}

fn mayer_julia_de(r2: vec4f) -> f32 {
    let v = max(r2.x, 1e-12);
    return v * log(v) * 0.5 / max(length(r2.yzw), 1e-20);
}

fn mayer_julia(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let ma = a.x;
    let mb = a.y;
    let sp = b.y;
    let ln_hi = b.z;
    let lb = log(100.0);
    let one = vec4f(1.0, 0.0, 0.0, 0.0);
    let h = 0.70710678;
    var y1 = vec4f((p.x + p.z) * h, h, 0.0, h);
    var y2 = vec4f((p.x - p.z) * h, h, 0.0, -h);
    var y3 = vec4f(p.y, 0.0, 1.0, 0.0);
    var r2 = mayer_julia_mul(y1, y1) + mayer_julia_mul(y2, y2) + mayer_julia_mul(y3, y3);
    var cur = mayer_julia_de(r2);
    var prev = cur;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r2.x >= 10000.0 || dot(r2.yzw, r2.yzw) >= 1e30) {
            break;
        }
        let a1 = mayer_julia_abs(y1);
        let a2 = mayer_julia_abs(y2);
        let den = mayer_julia_abs(y3) + select(a2, a1, a1.x > a2.x);
        let a0 = mayer_julia_div(y1, den);
        let bb = mayer_julia_div(y2, den) - one;
        let aa = select(a0 - one, one - a0, y3.x < 0.0);
        let x1 = aa * ma - bb * mb + one;
        let x2 = aa * mb + bb * ma + one;
        let fu = fract(floor((x1.x + 1.0) * 0.5) * 0.5) > 0.25;
        let fw = fract(floor((x2.x + 1.0) * 0.5) * 0.5) > 0.25;
        let u = mayer_julia_fold(x1);
        let w = mayer_julia_fold(x2);
        let au = mayer_julia_abs(u);
        let aw = mayer_julia_abs(w);
        let v3 = one - select(aw, au, au.x > aw.x);
        let n2 = mayer_julia_mul(u, u) + mayer_julia_mul(w, w) + mayer_julia_mul(v3, v3);
        let nv = sqrt(n2.x);
        let n = vec4f(nv, n2.yzw * (0.5 / nv));
        let r2v = max(r2.x, 1e-30);
        let rsv = exp2(log2(r2v) * (sp * 0.5));
        let rs = vec4f(rsv, r2.yzw * (rsv * (sp * 0.5) / r2v));
        let rr = sqrt(r2v);
        let kf = select(1.0, exp2(log2(0.6 / max(rr, 1e-6)) * (sp - 1.0)), rr < 0.6);
        let kv = vec4f(1.0, kf, kf, kf);
        y1 = mayer_julia_mul(mayer_julia_div(u, n), rs) * kv + vec4f(a.z, 0.0, 0.0, 0.0);
        y2 = mayer_julia_mul(mayer_julia_div(w, n), rs) * kv + vec4f(a.w, 0.0, 0.0, 0.0);
        y3 = mayer_julia_mul(mayer_julia_div(select(v3, -v3, fu != fw), n), rs) * kv + vec4f(b.x, 0.0, 0.0, 0.0);
        r2 = mayer_julia_mul(y1, y1) + mayer_julia_mul(y2, y2) + mayer_julia_mul(y3, y3);
        prev = cur;
        cur = mayer_julia_de(r2);
        let ax = y1.x - y2.x;
        trap = select(trap, min(trap, ax * (ax * 0.5) + y3.x * y3.x), i < 3u);
    }
    let lr = log(max(sqrt(r2.x), 1e-12));
    // (smoothstep by hand: its edges run downward here)
    var t = clamp((lr - ln_hi) / (lb - ln_hi), 0.0, 1.0);
    t = t * t * (3.0 - t - t);
    var ease = clamp((lr - lb * 0.5) / (lb * 0.5), 0.0, 1.0);
    ease = ease * ease * (3.0 - ease - ease);
    let d = select(prev + (cur - prev) * t, max(cur * ease, 0.0), r2.x < 10000.0);
    return vec2f(d * b.w, clamp((sqrt(sqrt(trap)) - 0.45) * 1.8, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, get, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), at the default and the most iterations: the
    /// estimate, times the step, changes at most 1.25 times as fast as the
    /// point moves at the 99th percentile; and the distances are finite.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[(2, -0.5), (3, 0.0), (4, 0.0)],
            &[(2, -1.0), (3, 0.0), (4, 0.0)],
            &[(2, -1.5), (3, 0.0), (4, 0.0)],
            &[(2, 0.25), (3, 0.0), (4, 0.0)],
            &[(2, -0.6), (3, 0.2), (4, 0.15)],
            &[(2, -0.3), (3, 0.35), (4, 0.2)],
            &[(2, 1.5), (3, 1.5), (4, 1.5)],
            &[(2, -1.5), (3, -1.5), (4, 1.5)],
            &[(2, 1.5), (3, -1.5), (4, -1.5)],
            &[(2, 0.0), (3, 1.5), (4, 0.0)],
            &[(2, 0.0), (3, 0.0), (4, 0.0)],
            &[(0, 1.0), (1, -1.0)],
            &[(0, 2.0), (1, 0.0), (2, -0.5)],
            &[(0, 2.0), (1, 1.0), (2, 0.3), (3, 0.3)],
            &[(0, 3.0), (1, 0.0), (2, -0.4)],
            &[(0, 3.0), (1, 3.0), (2, 0.2), (3, -0.2)],
            &[(0, -3.0), (1, 3.0), (2, -1.5), (3, 1.5), (4, -1.5)],
            &[(0, 3.0), (1, 3.0), (2, 1.5), (3, 1.5), (4, 1.5)],
            &[(0, 2.0), (1, 2.0), (2, 1.5), (3, -1.5), (4, 1.5)],
            &[(0, 3.0), (1, 0.0), (2, -1.5), (3, -1.5), (4, -1.5)],
            &[(0, 0.0), (1, 0.0)],
        ];
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let id = find("mayer_julia").unwrap();
        let f = get(id);
        let mut bad = Vec::new();
        for &set in corners {
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            for iterations in [f.iterations.0, f.iterations.1] {
                let node = FormulaNode { id, values, iterations };
                let k = node.pack();
                let mut slopes = Vec::new();
                for near in [1.0, 0.3] {
                    let reach = f.bound * 1.2 * near;
                    let e = 1e-3 * reach;
                    for _ in 0..1000 {
                        let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                        let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                        let (d, t) = (f.cpu)(p, &k, node.iters());
                        let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                        for i in 0..8 {
                            let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                            assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?} x{iterations}: {a}, trap {tr} at {:?}", p.lane(i));
                            if a > 10.0 * e && b.is_finite() {
                                slopes.push((b - a).abs() / e * f.step);
                            }
                        }
                    }
                }
                slopes.sort_by(f32::total_cmp);
                let p99 = slopes.get(slopes.len() * 99 / 100).copied().unwrap_or(0.0);
                if p99 >= 1.25 {
                    bad.push(format!("{set:?} x{iterations}: p99 {p99:.2}"));
                }
            }
        }
        assert!(bad.is_empty(), "changes too fast: {bad:?}");
    }
}
