//! A new formula, made for this catalog: a cusped hyperbolic honeycomb in
//! the upper half-space model, whose every horosphere is a mirror.
//!
//! The four regular honeycombs of hyperbolic space with ideal vertices and
//! finite cells, {3,4,4}, {3,3,6}, {4,3,6} and {5,3,6}, are drawn in the
//! upper half-space model turned over, so that the sphere at infinity is a
//! ceiling: their edges are ribs that fan out into vaults, ever finer
//! toward the ceiling, and every ideal vertex is a cusp where infinitely
//! many ribs meet. Each cusp gets a horosphere, drawn as the bars of the
//! cusp's Euclidean lattice (a cage tangent to the ceiling), and each
//! horosphere is a mirror: the inside of every cage is the whole space
//! turned inside out, with cages of its own. The honeycomb's reflection
//! group and the one horospherical mirror generate the Coxeter group
//! [∞, p, q, r], which acts on the 3-sphere (the boundary of hyperbolic
//! 4-space) as Indra's cells' group does.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, V8};

pub const FORMULAS: &[Formula] = &[CUSP];

const CUSP: Formula = Formula {
    name: "cusp_vaults",
    category: "Fractals: originals",
    desc: "Gothic vaults from a hyperbolic honeycomb with ideal vertices ({3,4,4} by default), in the upper half-space model turned over so that infinity is the ceiling: ribs fan out into ever finer tracery, and at every cusp, where the ribs meet, hangs a cage of the cusp's own lattice. Each cage is a mirror (a horosphere): inside it is the whole hall turned inside out, cages and all. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("honeycomb", 0.0, 0.0, 3.0, "The honeycomb (all four have ideal vertices): 0 {3,4,4} (ideal octahedra, square cusps: groin vaults), 1 {3,3,6} (tetrahedra, triangular cusps), 2 {4,3,6} (cubes), 3 {5,3,6} (dodecahedra, the lowest and finest vaults)."),
        Param::num("horosphere", &[1.15], 1.0, 2.0, "The depth of the mirror horosphere below the ceiling, over the vaults' depth (1 to 2): at 1 the cages are as big as they can be and kiss; larger leaves smaller cages and a taller hall."),
        Param::num("ribs", &[0.05], 0.0, 0.15, "The radius of the ribs (the honeycomb's edges) where they stand as columns, in units of the column spacing (0 to 0.15); they taper toward the cusps."),
        Param::num("bars", &[0.03], 0.0, 0.15, "The radius of the cages' bars (the cusp's lattice on its horosphere) at the mirror's own height (0 to 0.15); 0 leaves the mirrors invisible, but still mirrors."),
    ],
    iterations: (3, 6, "Mirror levels: 1 is the bare honeycomb with its cages; each more puts the turned-over world inside every cage of the last."),
    bound: f32::INFINITY,
    lod: None,
    step: 0.7,
    example: r#"{"type":"intersection","scale":0.5,"children":[{"type":"box","size":[4.5,2.34,4.5]},{"type":"cusp_vaults","position":[0,1.15,0]}]}"#,
    prep: Some(cusp_prep),
    cpu: cusp_vaults,
    wgsl: CUSP_VAULTS,
    wgsl_fn: "cusp_vaults",
};

/// The cell's face p and whether the cusp is triangular, by honeycomb.
const CUSP_CELLS: [(f32, bool); 4] = [(3.0, false), (3.0, true), (4.0, true), (5.0, true)];

/// The mirror sphere's radius r about a lattice point (it meets the cusp's
/// mirror through the edge's midpoint at pi / p: r = 1 / (2 cos(pi / p))),
/// the horosphere's depth H, and the cusp's shape.
fn cusp_prep(k: &mut [f32; K]) {
    let (p, tri) = CUSP_CELLS[(k[0] as usize).min(3)];
    let r = 0.5 / (std::f32::consts::PI / p).cos();
    k[4] = r;
    k[5] = k[1] * r;
    k[6] = if tri { 1.0 } else { 0.0 };
}

/// Fold rounds at most (a round is the cusp's fold and, below the mirror
/// sphere, one inversion): points that need more are within a hair of a
/// ceiling.
const CUSP_FOLDS: u32 = 32;
const SQ3: f32 = 1.732_050_8;

/// (u, v) folded into the cusp group's triangle, with corners at a lattice
/// point (the origin), an edge's midpoint (1/2, 0) and a cell's centre:
/// (1/2, 1/2) on the square lattice, (1/2, 1/(2√3)) on the triangular one.
#[inline(always)]
fn cusp_fold(u: F8, v: F8, tri: bool) -> (F8, F8) {
    if tri {
        // the nearest point of the triangular lattice, from its two
        // rectangular sublattices (1, √3) and (1, √3) + (1/2, √3/2)
        let (a1, b1) = (u - (u + 0.5).floor(), v - (v * (1.0 / SQ3) + 0.5).floor() * SQ3);
        let (a2, b2) = (u - 0.5 - u.floor(), v - SQ3 * 0.5 - (v * (1.0 / SQ3)).floor() * SQ3);
        let first = b1.mul_add(b1, a1 * a1).le(b2.mul_add(b2, a2 * a2));
        let (mut x, mut y) = (F8::select(first, a1, a2).abs(), F8::select(first, b1, b2).abs());
        // then the mirrors at 60 and 30 degrees
        let d = (x * 0.866_025_4 - y * 0.5).min(F8::zero()) * 2.0;
        x = d.neg_mul_add(s(0.866_025_4), x);
        y = d.mul_add(s(0.5), y);
        let d = (x * 0.5 - y * 0.866_025_4).min(F8::zero()) * 2.0;
        x = d.neg_mul_add(s(0.5), x);
        y = d.mul_add(s(0.866_025_4), y);
        (x, y)
    } else {
        let a = (u - (u + 0.5).floor()).abs();
        let b = (v - (v + 0.5).floor()).abs();
        (a.max(b), a.min(b))
    }
}

/// What is drawn, in the folded frame: the rib (the honeycomb's edge, the
/// vertical line over the lattice point) and the bar (the lattice line
/// v = 0 on the horosphere at depth H).
#[inline(always)]
fn cusp_walls(u: F8, v: F8, h: F8, ribs: f32, bars: f32, hz: f32) -> F8 {
    let rib = v.mul_add(v, u * u).sqrt() - ribs;
    let dh = h - hz;
    let bar = dh.mul_add(dh, v * v).sqrt() - bars;
    rib.min(bar)
}

/// The cusp vaults' distance estimate and orbit trap (the mirror level, odd
/// or even, and how deep in the honeycomb the nearest rib is).
///
/// Space is the upper half-space model upside down: depth h = -y below the
/// ceiling y = 0, with (u, v) = (x, z) along it. The honeycomb's group is
/// generated by the cusp's Euclidean triangle group (vertical mirrors: the
/// fold) and the sphere of radius r about a lattice point. Each round folds
/// the point into the triangle and inverts it in that sphere if it is
/// inside; once outside, it is in the fundamental domain, a chimney over the
/// triangle that reaches up into the cusp. There the rib is the edge over
/// the lattice point and the bar the lattice line on the horosphere h = H.
/// Past the horosphere (h > H) the point is reflected in it, h -> 2H - h,
/// and goes on at the next level: in the cusp at infinity that is a plane
/// mirror; in every other cusp it is an inversion in the cage's sphere.
/// The reflections are isometries; distances go back through the inversions
/// by their factors (first order, as for every Kleinian group). That alone
/// overshoots by half in the open rooms, where the nearest rib is often the
/// image across the mirror sphere of a column farther off, so that image is
/// measured too, through one more inversion carried strictly: an inversion
/// of radius r keeps whatever is at least d' from the image point at least
/// d' / (r²/ρ² + d'/ρ) from the point, ρ its distance from the centre (the
/// bound the `schottky` kernel uses). It is the same on both sides of the
/// sphere, so the estimate stays continuous there. A point reflected past
/// the mirrored ceiling (h <= 0) is above it, and the ceiling is a surface.
fn cusp_vaults(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (ribs, bars, r, hz, tri) = (k[2], k[3], k[4], k[5], k[6] > 0.5);
    let rr = r * r;
    let above = p.y.ge(F8::zero());
    let (mut u, mut v, mut h) = (p.x, p.z, (-p.y).max(s(1e-7)));
    let mut dd = F8::one();
    let mut best = s(1e9);
    let mut level = F8::zero();
    let mut ninv = F8::zero();
    let mut trap = F8::zero();
    let mut act = !above;
    let top = s(iters as f32 - 1.0);
    for _ in 0..CUSP_FOLDS {
        if act.none() {
            break;
        }
        let (fu, fv) = cusp_fold(u, v, tri);
        u = F8::select(act, fu, u);
        v = F8::select(act, fv, v);
        let q2 = h.mul_add(h, v.mul_add(v, u * u));
        let ins = act & q2.lt(s(rr));
        let kf = s(rr) / q2.max(s(1e-30));
        u = F8::select(ins, u * kf, u);
        v = F8::select(ins, v * kf, v);
        h = F8::select(ins, h * kf, h);
        dd = F8::select(ins, dd * kf, dd);
        ninv = F8::select(ins, ninv + 1.0, ninv);
        // in the fundamental domain: measure, then through the mirror
        let landed = act.and_not(ins);
        // what is drawn here, and its image across the mirror sphere: that
        // is often nearer, and the inversion's factor at the point
        // understates how it bends what is far, so it is carried strictly
        let w = cusp_walls(u, v, h, ribs, bars, hz) / dd;
        let kv = s(rr) / q2.max(s(1e-30));
        let wv = cusp_walls(u * kv, v * kv, h * kv, ribs, bars, hz);
        let wv = F8::select(wv.gt(F8::zero()), wv / (q2.max(s(1e-30)).rsqrt().mul_add(wv, kv) * dd), wv / (kv * dd));
        let w = w.min(wv);
        let nearer = landed & w.lt(best);
        let odd = level - (level * 0.5).floor() * 2.0;
        trap = F8::select(nearer, odd.mul_add(s(0.55), ninv.min(s(8.0)) * 0.05), trap);
        best = F8::select(landed, best.min(w), best);
        let into = landed & level.lt(top) & h.gt(s(hz));
        h = F8::select(into, s(2.0 * hz) - h, h);
        level = F8::select(into, level + 1.0, level);
        ninv = F8::select(into, F8::zero(), ninv);
        let gone = into & h.le(F8::zero());
        best = F8::select(gone, best.min(-h / dd), best);
        act = ins | into.and_not(gone);
    }
    // (still folding: as near a ceiling as the rounds reach)
    let w = cusp_walls(u, v, h, ribs, bars, hz) / dd;
    best = F8::select(act, best.min(w), best);
    (F8::select(above, -p.y, best), trap.saturate())
}

const CUSP_VAULTS: &str = r#"
fn cusp_vaults_fold(q: vec2f, tri: bool) -> vec2f {
    if (tri) {
        let sq3 = 1.7320508;
        let q1 = vec2f(q.x - floor(q.x + 0.5), q.y - floor(q.y * (1.0 / sq3) + 0.5) * sq3);
        let q2 = vec2f(q.x - 0.5 - floor(q.x), q.y - sq3 * 0.5 - floor(q.y * (1.0 / sq3)) * sq3);
        var f = abs(select(q2, q1, dot(q1, q1) <= dot(q2, q2)));
        let n60 = vec2f(0.8660254, -0.5);
        f -= n60 * (min(dot(f, n60), 0.0) * 2.0);
        let n30 = vec2f(0.5, -0.8660254);
        f -= n30 * (min(dot(f, n30), 0.0) * 2.0);
        return f;
    }
    let f = abs(q - floor(q + vec2f(0.5)));
    return vec2f(max(f.x, f.y), min(f.x, f.y));
}

fn cusp_vaults_walls(q: vec3f, ribs: f32, bars: f32, hz: f32) -> f32 {
    let rib = length(q.xy) - ribs;
    let bar = length(vec2f(q.z - hz, q.y)) - bars;
    return min(rib, bar);
}

fn cusp_vaults(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let ribs = a.z;
    let bars = a.w;
    let r = b.x;
    let hz = b.y;
    let tri = b.z > 0.5;
    let rr = r * r;
    if (p.y >= 0.0) {
        return vec2f(-p.y, 0.0);
    }
    var q = vec3f(p.x, p.z, max(-p.y, 1e-7));
    var dd = 1.0;
    var best = 1e9;
    var level = 0.0;
    var ninv = 0.0;
    var trap = 0.0;
    var act = true;
    let top = f32(iters) - 1.0;
    for (var i = 0u; i < 32u; i++) {
        q = vec3f(cusp_vaults_fold(q.xy, tri), q.z);
        let q2 = dot(q, q);
        if (q2 < rr) {
            let kf = rr / max(q2, 1e-30);
            q *= kf;
            dd *= kf;
            ninv += 1.0;
            continue;
        }
        let kv = rr / max(q2, 1e-30);
        let wi = cusp_vaults_walls(q * kv, ribs, bars, hz);
        let wv = select(wi / (kv * dd), wi / ((kv + wi * inverseSqrt(max(q2, 1e-30))) * dd), wi > 0.0);
        let w = min(cusp_vaults_walls(q, ribs, bars, hz) / dd, wv);
        if (w < best) {
            let odd = level - floor(level * 0.5) * 2.0;
            trap = odd * 0.55 + min(ninv, 8.0) * 0.05;
        }
        best = min(best, w);
        if (level >= top || q.z <= hz) {
            act = false;
            break;
        }
        q.z = 2.0 * hz - q.z;
        level += 1.0;
        ninv = 0.0;
        if (q.z <= 0.0) {
            best = min(best, -q.z / dd);
            act = false;
            break;
        }
    }
    if (act) {
        best = min(best, cusp_vaults_walls(q, ribs, bars, hz) / dd);
    }
    return vec2f(best, clamp(trap, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), each honeycomb, at the tangent and the deepest
    /// horosphere, the thinnest and thickest ribs and bars, and one, three
    /// and six mirror levels: a hundredth of the estimate, times the step,
    /// changes at most 1.25 times as fast as the point moves, and no point
    /// 3 times (a jump where a mirror or a level changes would show as much
    /// more).
    #[test]
    fn estimates_hold_across_the_parameters() {
        let id = find("cusp_vaults").unwrap();
        let f = super::super::get(id);
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        for honeycomb in 0..4 {
            for horosphere in [1.0, 2.0] {
                for (ribs, bars) in [(0.0, 0.0), (0.15, 0.15), (0.0, 0.15), (0.15, 0.0)] {
                    for iterations in [1, 3, 6] {
                        let mut values = f.defaults();
                        values[..4].copy_from_slice(&[honeycomb as f32, horosphere, ribs, bars]);
                        let node = FormulaNode { id, values, iterations };
                        let k = node.pack();
                        let mut slopes = Vec::new();
                        for near in [1.0, 0.3] {
                            let reach = 3.0 * near;
                            let e = 1e-3 * reach;
                            for _ in 0..300 {
                                let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                                let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                                let (d, t) = (f.cpu)(p, &k, node.iters());
                                let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                                for i in 0..8 {
                                    let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                                    let at = p.lane(i);
                                    assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{values:?} x{iterations}: {a}, trap {tr} at {at:?}");
                                    if a > 10.0 * e && b.is_finite() {
                                        let steep = (b - a).abs() / e * f.step;
                                        assert!(steep < 3.0, "{values:?} x{iterations}: changes {steep:.2} times as fast as the point at {at:?}");
                                        slopes.push(steep);
                                    }
                                }
                            }
                        }
                        slopes.sort_by(f32::total_cmp);
                        let p99 = slopes[slopes.len() * 99 / 100];
                        assert!(p99 < 1.25, "{values:?} x{iterations}: a hundredth of the estimate changes {p99:.2} times as fast as the point");
                    }
                }
            }
        }
    }
}
