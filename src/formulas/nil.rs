//! A fractal of Thurston's Nil geometry: a self-similar set in the
//! Heisenberg group, built of its own left translations and dilations, drawn
//! in exponential coordinates (where the group's maps are Euclidean shears).

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[NIL_STAIR];

const NIL_STAIR: Formula = Formula {
    name: "nil_stair",
    category: "Fractals: originals",
    desc: "A square tower of spiral stairs in Thurston's Nil geometry (the Heisenberg group): every step, and every stone of the central post, is a smaller copy of the whole tower, squashed by the group's dilation (about a third as wide, a ninth as high) and sheared along its contact planes, so the copies flatten from blocks to louvres to tiles. The steps lie level in Nil, side by side; they climb in our space only because a level loop in Nil does not close, rising by the area it encloses. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("turns", 4.0, 1.0, 4.0, "Turns of the stair around the post (1 to 4), eight steps a turn."),
        Param::num("twist", &[1.0], 0.0, 3.0, "The Heisenberg twist, as a share of the stair's climb (0 to 3): 0 is flat Euclidean space (level treads, plain stairs); 1 lays the steps out level in Nil, so the stair climbs by the group's holonomy alone and each tread tilts along its contact plane by as much as the stair climbs; more twists the treads and steepens the copies' shear."),
        Param::num("height", &[2.5], 1.0, 4.0, "The tower's height over its width (1 to 4). Each level of copies is about a third as wide and a ninth as high, so tall towers have steps like blocks and flat treads further down; low ones go to thin plates sooner."),
        Param::whole("post", 9.0, 0.0, 9.0, "Copies of the tower stacked into the central post (0 to 9): 0 leaves an open stairwell."),
        Param::num("gap", &[0.1], 0.02, 0.4, "The space between neighbouring copies, as a share of a copy's cell (0.02 to 0.4): each copy is (1 - gap) / 3 of its parent across and the square of that high."),
        Param::num("thickness", &[1.0], 0.1, 1.0, "The finest copies' thickness, as a share of a full copy's height (0.1 to 1): below 1 the last level is drawn as thin plates."),
    ],
    iterations: (4, 6, "Levels of copies: each makes every step and post stone a tower of its own, about a third as wide and a ninth as high. Close up none are added by themselves (each level's height is nine times finer, so they would be sub-pixel sheets), and six reach plates thinner than f32 holds well."),
    bound: 1.0,
    lod: None,
    step: 1.0,
    example: r#"{"type":"nil_stair","scale":1.0}"#,
    prep: Some(nil_prep),
    cpu: nil_stair,
    wgsl: NIL_STAIR_WGSL,
    wgsl_fn: "nil_stair",
};

/// The ring of 3×3 columns round a cell's centre (half width 1), in order
/// counterclockwise from +x (the centre column is the post).
const COLS: [(f32, f32); 8] = {
    const T: f32 = 2.0 / 3.0;
    [(T, 0.0), (T, T), (0.0, T), (-T, T), (-T, 0.0), (-T, -T), (0.0, -T), (T, -T)]
};

/// The derived constants, from (turns, twist, height, post, gap, thickness):
/// k[1] becomes the group's twist κ, k[4] the copies' ratio λ; then the ring
/// steps' climb, the post's spacing, the normalising scale, λ², 1/λ, 1/λ².
fn nil_prep(k: &mut [f32; K]) {
    let (turns, tau, h, post, gap) = (k[0], k[1], k[2], k[3], k[4]);
    let lam = (1.0 - gap) / 3.0;
    let room = h * (1.0 - lam * lam);
    // the climb per step: the 8 turns·8 - 1 steps span the height less a
    // copy's half height and its shear (at a corner, 2κλ/3, κ = 9τs/2)
    let st = room / ((8.0 * turns - 1.0) * 0.5 + 3.0 * lam * tau);
    k[1] = 4.5 * tau * st;
    k[4] = lam;
    k[6] = st;
    k[7] = if post > 1.5 { 2.0 * room / (post - 1.0) } else { 0.0 };
    k[8] = h.max(1.0);
    k[9] = lam * lam;
    k[10] = 1.0 / lam;
    k[11] = 1.0 / (lam * lam);
}

/// The signed distance to the square |x|, |y| <= r.
#[inline(always)]
fn box2(x: F8, y: F8, r: f32) -> F8 {
    let (dx, dy) = (x.abs() - r, y.abs() - r);
    len2(dx.max(F8::zero()), dy.max(F8::zero())) + dx.max(dy).min(F8::zero())
}

/// The Nil staircase's distance estimate and orbit trap (where along the
/// stair, and along the post, the point's copies sit).
///
/// Nil is the Heisenberg group in exponential coordinates, with the product
/// (x, y, z)·(a, b, c) = (x + a, y + b, z + c + κ(xb - ya)/2) and the
/// dilations δ(x, y, z) = (λx, λy, λ²z). A cell is the box |x|, |y| <= 1,
/// |z| <= H; its copies are g·δ(cell) on a 3×3 grid: the eight ring columns
/// hold `turns` steps each, the k-th step (counterclockwise) at height
/// s(k - (8 turns - 1)/2), and the centre column the post. Left translation
/// by g shears each copy along the contact plane at its centre, normal (κb/2,
/// -κa/2, 1). With twist 1 the steps' heights are those of the level
/// (horizontal) path around the ring: Nil's holonomy, κ times the area.
///
/// The estimate is a branch and bound over the copies. Each copy's box is a
/// parallelepiped in space, its distance the larger of its square's
/// (scaled by the frame's μ) and its tilted slab's ((|z| - λ²H) μ²/N, N the
/// slab normal's length): both exact for their slab, so 1-Lipschitz, and a
/// lower bound for the box. Every level takes the least box distance of the
/// copies it does not enter (a column's stack: its nearest and next copy by
/// rounding) and enters the nearest; the boxes nest (each copy's box lies in
/// its parent's), so the estimate is the least of those and of the larger of
/// the entered boxes' and the last level's box. The copies are kept apart by
/// `gap`, so the bounds never meet at a shared face (where marching would
/// creep). The bounds are not monotone (a tilted child slab, divided by a
/// longer normal, can bound less than its parent's), so the siblings are
/// clamped from below by the entered boxes' bound and the last box by it
/// too, which keeps the estimate continuous where the entered copy changes;
/// N is the longest of a level's columns' (one root a level), which keeps it
/// 1-Lipschitz. A level whose nearest copy is no nearer than the siblings
/// already are stops there: nothing below it would change the result.
fn nil_stair(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let turns = k[0] as u32;
    let (kap, h, np, lam) = (k[1], k[2], k[3] as u32, k[4]);
    let (thick, st, sig, kn) = (k[5], k[6], k[7], k[8]);
    let (lam2, ilam, ilam2) = (k[9], k[10], k[11]);
    let hk = 0.5 * kap;
    let k4 = 0.25 * kap * kap;
    let hc = lam2 * h;
    let ring0 = -st * (8 * turns - 1) as f32 * 0.5;
    let post0 = -sig * (np.max(1) - 1) as f32 * 0.5;
    let big = s(1e9);
    // Nil's (x, y, z), z up
    let (mut qx, mut qy, mut qz) = (p.x * kn, -p.z * kn, p.y * kn);
    let (mut mu, mut ax, mut ay) = (F8::one(), F8::zero(), F8::zero());
    let mut best = big;
    let mut path = box2(qx, qy, 1.0).max(qz.abs() - h);
    let mut trap = F8::zero();
    let mut weight = 0.7f32;
    let mut act = M8::all_true();
    for _ in 0..iters {
        if act.none() {
            break;
        }
        // the slabs' normals are at most this long over the frame's copies
        // (their centres within 2√2/3 of its own)
        let r = len2(ax, ay) + mu * RING_R;
        let zf = mu * mu / (r * r * k4 + 1.0).sqrt();
        let mut pk = Pick { n1: big, n2: big, a: F8::zero(), b: F8::zero(), c: F8::zero(), code: F8::zero() };
        let at = Frame { qx, qy, qz, mu, zf, lam, hk, hc };
        let (per, last, c1) = (8.0 * st, (turns - 1) as f32, 1.0 / turns as f32);
        for (m, &(a, b)) in COLS.iter().enumerate() {
            at.column(&mut pk, a, b, ring0 + st * m as f32, per, last, (m as f32 + 0.5) / (8 * turns) as f32, c1);
        }
        if np > 0 {
            at.column(&mut pk, 0.0, 0.0, post0, sig, (np - 1) as f32, 0.5 / np as f32, 1.0 / np as f32);
        }
        // (the siblings lie in every box entered so far: path bounds them too)
        best = F8::select(act, best.min(pk.n2.max(path)), best);
        path = F8::select(act, path.max(pk.n1), path);
        trap = F8::select(act, pk.code.mul_add(s(weight), trap), trap);
        weight *= 0.3;
        // nothing inside the nearest copy can be nearer than the siblings
        // already are
        act = act.and_not(pk.n1.ge(best));
        let sh = (qy * pk.a - qx * pk.b) * hk;
        let (nx, ny, nz) = ((qx - pk.a) * ilam, (qy - pk.b) * ilam, (qz - pk.c - sh) * ilam2);
        qx = F8::select(act, nx, qx);
        qy = F8::select(act, ny, qy);
        qz = F8::select(act, nz, qz);
        ax = F8::select(act, mu.mul_add(pk.a, ax), ax);
        ay = F8::select(act, mu.mul_add(pk.b, ay), ay);
        mu = F8::select(act, mu * lam, mu);
    }
    let zf = mu * mu / (ay.mul_add(ay, ax * ax) * k4 + 1.0).sqrt();
    let leaf = (box2(qx, qy, 1.0) * mu).max((qz.abs() - thick * h) * zf);
    let d = F8::select(act, best.min(path.max(leaf)), best.min(path));
    (d * (1.0 / kn), trap.saturate())
}

/// The ring columns' farthest centre from a cell's (a corner's, 2√2/3).
const RING_R: f32 = 0.942_809_04;

/// The nearest copy so far (its box distance, centre and code) and the
/// second nearest's box distance.
struct Pick {
    n1: F8,
    n2: F8,
    a: F8,
    b: F8,
    c: F8,
    code: F8,
}

/// A cell's frame: the point in it, its scale and slab factor, and the
/// constants of its copies.
struct Frame {
    qx: F8,
    qy: F8,
    qz: F8,
    mu: F8,
    zf: F8,
    lam: f32,
    hk: f32,
    hc: f32,
}

impl Frame {
    /// The column at (a, b), a stack of copies at heights base + j period
    /// (j up to `last`, sheared to the contact plane at (a, b)): its nearest
    /// and next copies' box distances into `pk`; a copy's code is j c1 + c0.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn column(&self, pk: &mut Pick, a: f32, b: f32, base: f32, period: f32, last: f32, c0: f32, c1: f32) {
        let big = s(1e9);
        let dxy = box2(self.qx - a, self.qy - b, self.lam) * self.mu;
        let zr = self.qz - (self.qy * a - self.qx * b) * self.hk - base;
        let j = (zr * (1.0 / period.max(1e-9))).round().clamp(0.0, last);
        let e1 = (zr - j * period).abs();
        let lo = F8::select(j.gt(F8::zero()), (zr - (j - 1.0) * period).abs(), big);
        let hi = F8::select(j.lt(s(last)), (zr - (j + 1.0) * period).abs(), big);
        let d1 = dxy.max((e1 - self.hc) * self.zf);
        let d2 = dxy.max((lo.min(hi) - self.hc) * self.zf);
        let nearer = d1.lt(pk.n1);
        pk.n2 = F8::select(nearer, pk.n1.min(d2), pk.n2.min(d1));
        pk.n1 = F8::select(nearer, d1, pk.n1);
        pk.a = F8::select(nearer, s(a), pk.a);
        pk.b = F8::select(nearer, s(b), pk.b);
        pk.c = F8::select(nearer, j.mul_add(s(period), s(base)), pk.c);
        pk.code = F8::select(nearer, j.mul_add(s(c1), s(c0)), pk.code);
    }
}

const NIL_STAIR_WGSL: &str = r#"
struct nil_stair_pick {
    n1: f32,
    n2: f32,
    ab: vec2f,
    c: f32,
    code: f32,
}

fn nil_stair_box2(x: f32, y: f32, r: f32) -> f32 {
    let dx = abs(x) - r;
    let dy = abs(y) - r;
    return length(vec2f(max(dx, 0.0), max(dy, 0.0))) + min(max(dx, dy), 0.0);
}

// f: (mu, zf, lam, hk); g: (base, period, last, hc); code: (c0, c1)
fn nil_stair_col(pk: nil_stair_pick, q: vec3f, ab: vec2f, f: vec4f, g: vec4f, code: vec2f) -> nil_stair_pick {
    let dxy = nil_stair_box2(q.x - ab.x, q.y - ab.y, f.z) * f.x;
    let zr = q.z - (q.y * ab.x - q.x * ab.y) * f.w - g.x;
    let period = g.y;
    let j = clamp(round(zr * (1.0 / max(period, 1e-9))), 0.0, g.z);
    let e1 = abs(zr - j * period);
    let lo = select(1e9, abs(zr - (j - 1.0) * period), j > 0.0);
    let hi = select(1e9, abs(zr - (j + 1.0) * period), j < g.z);
    let d1 = max(dxy, (e1 - g.w) * f.y);
    let d2 = max(dxy, (min(lo, hi) - g.w) * f.y);
    var r = pk;
    if (d1 < pk.n1) {
        r.n2 = min(pk.n1, d2);
        r.n1 = d1;
        r.ab = ab;
        r.c = j * period + g.x;
        r.code = j * code.y + code.x;
    } else {
        r.n2 = min(pk.n2, d1);
    }
    return r;
}

fn nil_stair(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let turns = a.x;
    let kap = a.y;
    let h = a.z;
    let np = a.w;
    let lam = b.x;
    let thick = b.y;
    let st = b.z;
    let sig = b.w;
    let kn = c.x;
    let lam2 = c.y;
    let ilam = c.z;
    let ilam2 = c.w;
    let hk = 0.5 * kap;
    let k4 = 0.25 * kap * kap;
    let hc = lam2 * h;
    let ring0 = -st * (8.0 * turns - 1.0) * 0.5;
    let post0 = -sig * (max(np, 1.0) - 1.0) * 0.5;
    let t3 = 2.0 / 3.0;
    let per = 8.0 * st;
    let last = turns - 1.0;
    let n8 = 1.0 / (8.0 * turns);
    let c1 = 1.0 / turns;
    var q = vec3f(p.x, -p.z, p.y) * kn;
    var mu = 1.0;
    var aw = vec2f(0.0);
    var best = 1e9;
    var path = max(nil_stair_box2(q.x, q.y, 1.0), abs(q.z) - h);
    var trap = 0.0;
    var weight = 0.7;
    var live = true;
    for (var it = 0u; it < iters; it++) {
        let r = length(aw) + mu * 0.94280904;
        let f = vec4f(mu, mu * mu / sqrt(r * r * k4 + 1.0), lam, hk);
        var pk = nil_stair_pick(1e9, 1e9, vec2f(0.0), 0.0, 0.0);
        pk = nil_stair_col(pk, q, vec2f(t3, 0.0), f, vec4f(ring0, per, last, hc), vec2f(0.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(t3, t3), f, vec4f(ring0 + st, per, last, hc), vec2f(1.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(0.0, t3), f, vec4f(ring0 + 2.0 * st, per, last, hc), vec2f(2.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(-t3, t3), f, vec4f(ring0 + 3.0 * st, per, last, hc), vec2f(3.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(-t3, 0.0), f, vec4f(ring0 + 4.0 * st, per, last, hc), vec2f(4.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(-t3, -t3), f, vec4f(ring0 + 5.0 * st, per, last, hc), vec2f(5.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(0.0, -t3), f, vec4f(ring0 + 6.0 * st, per, last, hc), vec2f(6.5 * n8, c1));
        pk = nil_stair_col(pk, q, vec2f(t3, -t3), f, vec4f(ring0 + 7.0 * st, per, last, hc), vec2f(7.5 * n8, c1));
        if (np > 0.5) {
            pk = nil_stair_col(pk, q, vec2f(0.0), f, vec4f(post0, sig, np - 1.0, hc), vec2f(0.5 / np, 1.0 / np));
        }
        best = min(best, max(pk.n2, path));
        path = max(path, pk.n1);
        trap = pk.code * weight + trap;
        weight *= 0.3;
        if (pk.n1 >= best) {
            live = false;
            break;
        }
        let sh = (q.y * pk.ab.x - q.x * pk.ab.y) * hk;
        q = vec3f((q.x - pk.ab.x) * ilam, (q.y - pk.ab.y) * ilam, (q.z - pk.c - sh) * ilam2);
        aw = aw + pk.ab * mu;
        mu = mu * lam;
    }
    var d = min(best, path);
    if (live) {
        let zf = mu * mu / sqrt(dot(aw, aw) * k4 + 1.0);
        let leaf = max(nil_stair_box2(q.x, q.y, 1.0) * mu, (abs(q.z) - thick * h) * zf);
        d = min(best, max(path, leaf));
    }
    return vec2f(d / kn, clamp(trap, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), at the default and the most iterations, along all
    /// three axes: the estimate is finite, the trap in 0..1, and it is
    /// 1-Lipschitz (every bound in it is an exact slab distance, and the
    /// clamps keep it continuous where the copy it enters changes), so it
    /// never changes more than 1.05 times as fast as the point moves.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(0, 2.0), (3, 5.0)],
            &[(3, 0.0), (4, 0.4)],
            &[(0, 1.0), (1, 0.0), (2, 1.0), (3, 0.0), (4, 0.02), (5, 0.1)],
            &[(0, 4.0), (1, 3.0), (2, 4.0), (3, 9.0), (4, 0.02), (5, 1.0)],
            &[(0, 4.0), (1, 3.0), (2, 1.0), (3, 1.0), (4, 0.4), (5, 0.1)],
            &[(0, 1.0), (1, 3.0), (2, 4.0), (3, 9.0), (4, 0.4), (5, 1.0)],
            &[(0, 1.0), (1, 3.0), (2, 1.0), (3, 0.0), (4, 0.02)],
            &[(0, 4.0), (1, 0.0), (2, 4.0), (3, 2.0), (4, 0.2)],
            &[(1, 2.0), (2, 4.0), (3, 9.0), (5, 0.3)],
        ];
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let id = find("nil_stair").unwrap();
        let f = super::super::get(id);
        for &set in corners {
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            for iters in [f.iterations.0, f.iterations.1] {
                let node = FormulaNode { id, values, iterations: iters };
                let k = node.pack();
                let mut slopes = Vec::new();
                for near in [1.0, 0.3] {
                    let reach = f.bound * 1.2 * near;
                    let e = 1e-3 * reach;
                    for _ in 0..1500 {
                        let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                        let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                        let (d, t) = (f.cpu)(p, &k, node.iters());
                        for (axis, q) in [V8::new(p.x + e, p.y, p.z), V8::new(p.x, p.y + e, p.z), V8::new(p.x, p.y, p.z + e)].into_iter().enumerate() {
                            let (dq, _) = (f.cpu)(q, &k, node.iters());
                            for i in 0..8 {
                                let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                                assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                                if a > 10.0 * e && b.is_finite() {
                                    let steep = (b - a).abs() / e * f.step;
                                    // (observed at most 1.001: f32 rounding)
                                    assert!(steep < 1.05, "{set:?} x{iters}: changes {steep:.2} times as fast as the point along axis {axis} at {:?}", p.lane(i));
                                    slopes.push(steep);
                                }
                            }
                        }
                    }
                }
                slopes.sort_by(f32::total_cmp);
                let p99 = slopes.get(slopes.len() * 99 / 100).copied().unwrap_or(0.0);
                assert!(p99 < 1.25, "{set:?} x{iters}: a hundredth changes {p99:.2} times as fast");
            }
        }
    }
}
