//! Alexander's horned sphere, drawn to any depth: a ring cut open, two
//! smaller rings hung from its cut ends and clasped through each other, each
//! of them cut open in turn. (This group's brief was the octonions; their
//! 3D slices came out either as the quaternion sets already in the catalog
//! or as smooth blobs, so the construction here is a different one.)

use super::{Formula, K, Param};
use crate::simd::{F8, V8};
use std::f32::consts::PI;

pub const FORMULAS: &[Formula] = &[HORNED];

const HORNED: Formula = Formula {
    name: "horned_sphere",
    category: "Fractals: originals",
    desc: "Alexander's horned sphere (J. W. Alexander, 1924), the wild sphere of topology: a ball grows two horns that curl toward each other, each horn forks into two that clasp the other pair's, and so on at every scale, so a loop around a horn can never be slipped off. Drawn here to any depth from tapered torus arcs, with an exact distance: a form made for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::num("shrink", &[0.45], 0.3, 0.5, "How much smaller each generation of horns is (0.3 to 0.5): small values leave the clasps sparse and easy to follow, large ones pack them into a dense knot."),
        Param::num("clasp", &[1.35], 1.2, 1.6, "How deep each pair of horns reaches into the other (1.2 to 1.6): it sets the gap between a ring's tips, as a multiple of the next ring's size. 1.5 puts each ring's gap at the center of the ring it clasps."),
        Param::num("turn", &[45.0], 0.0, 90.0, "Degrees between a ring's plane and its first child's (0 to 90); the two children are always square to each other. 45 is symmetric; at 0 and 90 one child lies in its parent's plane and overlaps the parent's tip for a stretch."),
        Param::num("tube", &[0.28], 0.05, 0.35, "The horns' thickness where they leave their ring's back, as a share of the ring's radius (0.05 to 0.35)."),
        Param::num("taper", &[1.0], 0.0, 1.0, "How much the horns narrow toward their tips (0 to 1): 1 narrows each to exactly the thickness of the horns it forks into, so they run on unbroken; 0 keeps every ring a plain tube."),
        Param::num("blend", &[0.5], 0.0, 1.0, "How smoothly each horn forks (0 to 1): the fillet where a tip meets the ring it splits into, as a share of that ring's thickness; 0 leaves a sharp crease."),
        Param::num("ball", &[0.7], 0.0, 0.9, "The radius of the ball the first two horns grow from, as a share of the first ring's radius (0 to 0.9); 0 leaves the first ring bare, a torus with a slice cut out."),
    ],
    iterations: (8, 14, "Generations of horns: each forks every tip into a clasped pair `shrink` times smaller (more are added close up)."),
    bound: 2.0,
    lod: Some((2.2, 14)),
    step: 0.9,
    example: r#"{"type":"horned_sphere","scale":0.6,"rotation":[0,-30,0]}"#,
    prep: Some(horned_prep),
    cpu: horned_sphere,
    wgsl: HORNED_SPHERE,
    wgsl_fn: "horned_sphere",
};

/// Derived values, packed for both kernels:
/// k[0] s, k[1] the tips' tube radius, k[2] cos turn, k[3] tube, k[4] the
/// half gap g, k[5] blend, k[6] ball, k[7] sin turn, k[8] sin g, k[9] cos g,
/// k[10] the radius about a ring's centre that holds it and all its
/// descendants (in its own units), k[11] the taper's rate per radian.
fn horned_prep(k: &mut [f32; K]) {
    let (s, clasp, turn, tube, taper) = (k[0], k[1], k[2].to_radians(), k[3], k[4]);
    let sg = (clasp * s).min(0.95);
    let g = sg.asin();
    let cg = (1.0 - sg * sg).sqrt();
    // a child ring's centre is (cos g, ±(sin g - s), 0): its descendants lie
    // within s R of it, so R = |centre| + s R holds them all
    let c = (cg * cg + (sg - s) * (sg - s)).sqrt();
    k[1] = tube * s.powf(taper);
    k[2] = turn.cos();
    k[4] = g;
    k[7] = turn.sin();
    k[8] = sg;
    k[9] = cg;
    k[10] = (1.0 + tube).max(c / (1.0 - s)) + 1e-3;
    k[11] = taper * s.ln() / (PI - g);
}

/// Polynomial smooth minimum (k = 0: min).
#[inline(always)]
fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// The distance, in its own units, from `q` to a ring of radius 1 about the
/// origin in the xy plane, cut open across the +x axis by the half angle g,
/// whose tube is `tube` thick at its back (-x) and narrows by e^(rate (π -
/// θ)) to `tip` at its cut ends, where it is capped round.
#[inline(always)]
fn ring(q: [f32; 3], k: &[f32; K]) -> f32 {
    let (tip, tube, g, sg, cg, rate) = (k[1], k[3], k[4], k[8], k[9], k[11]);
    let ay = q[1].abs();
    let th = ay.atan2(q[0]);
    if th >= g {
        let rxy = (q[0] * q[0] + ay * ay).sqrt();
        let (u, v) = (rxy - 1.0, q[2]);
        (u * u + v * v).sqrt() - tube * (rate * (PI - th)).exp()
    } else {
        let (u, v, w) = (q[0] - cg, ay - sg, q[2]);
        (u * u + v * v + w * w).sqrt() - tip
    }
}

/// A lower bound, in its own units, on the distance from `q` to a ring and
/// all its descendants: the ring lies in the torus of its thickest tube, and
/// its descendants within `off + s R` of the middle of its cut, (cos g, 0,
/// 0), where its two children's centres are `off` = sin g - s to either
/// side.
#[inline(always)]
fn subtree(q: [f32; 3], k: &[f32; K], off: f32, rb: f32) -> f32 {
    let (s, tube, cg) = (k[0], k[3], k[9]);
    let rxy = (q[0] * q[0] + q[1] * q[1]).sqrt() - 1.0;
    let torus = (rxy * rxy + q[2] * q[2]).sqrt() - tube;
    let u = q[0] - cg;
    let front = (u * u + q[1] * q[1] + q[2] * q[2]).sqrt() - (off + s * rb);
    torus.min(front)
}

/// The deepest the search goes (iterations at most 14, and a node and its
/// two children on the stack at each level).
const STACK: usize = 16;

/// One point's distance and trap: a depth-first search of the binary tree
/// of rings, nearer child first, skipping any subtree whose bounding sphere
/// is no nearer than what has been found.
///
/// A ring is the unit ring above (front +x, plane xy). Its cut ends are T± =
/// (cos g, ±sin g, 0); on each hangs a child ring `s` times smaller whose
/// back is at the tip and whose cut faces the other tip: centre T± ∓ s y,
/// front direction ∓y, and plane turned by `turn` from the parent's (the two
/// children square to each other). With sin g = clasp · s the two children
/// overlap like chain links, each one's cut end inside the other's hole, so
/// they clasp. Each child is mapped to the unit ring by a similarity, so the
/// distance found there is exact up to its scale, and a subtree lies within
/// R s of its centre (k[10]), which bounds it until the search reaches it.
/// Each ring is smooth-unioned with its parent only, so the fork at a tip is
/// filleted and the result does not depend on the order of the search; the
/// root also carries the ball. A subtree is skipped when its bound is a
/// fillet beyond the best so far: a ring there can lower nothing. Trap: the
/// generation of the nearest ring.
fn horned_lane(p: [f32; 3], k: &[f32; K], iters: u32) -> (f32, f32) {
    let (s, cp, tube, blend, ball, sp, sg, cg, rb) = (k[0], k[2], k[3], k[5], k[6], k[7], k[8], k[9], k[10]);
    let off = sg - s;
    // (x, y, z, scale) and (generation, bound, the parent ring's distance)
    let mut qs = [[0.0f32; 4]; STACK];
    let mut ls = [[0.0f32; 3]; STACK];
    // (the shape's frame: front up, the ring facing the viewer)
    qs[0] = [p[1], p[0], -p[2], 1.0];
    ls[0] = [0.0, -1e9, 1e9];
    let mut n = 1usize;
    let mut best = 1e9f32;
    let mut plain = 1e9f32;
    let mut level = 0.0f32;
    while n > 0 {
        n -= 1;
        let [x, y, z, sc] = qs[n];
        let [lv, bd, dp] = ls[n];
        let kf = blend * tube * sc;
        if bd >= best + kf {
            continue;
        }
        let mut d = ring([x, y, z], k) * sc;
        if lv == 0.0 && ball > 0.0 {
            let (u, v, w) = (x + 1.0, y, z);
            d = smin(d, (u * u + v * v + w * w).sqrt() - ball, blend * ball * 0.5);
        }
        if d < plain {
            plain = d;
            level = lv;
        }
        best = best.min(smin(dp, d, kf));
        if lv + 1.0 >= iters as f32 {
            continue;
        }
        // the children, in their own units
        let inv = 1.0 / s;
        let (dx, dy, dz) = (x - cg, y - off, z);
        let a = [-dy * inv, (sp * dx + cp * dz) * inv, (sp * dz - cp * dx) * inv];
        let dy = y + off;
        let b = [dy * inv, (cp * dx - sp * dz) * inv, (-sp * dx - cp * dz) * inv];
        let (sc2, lv2) = (sc * s, lv + 1.0);
        let (ba, bb) = (subtree(a, k, off, rb) * sc2, subtree(b, k, off, rb) * sc2);
        let (near, bn, far, bf) = if ba < bb { (a, ba, b, bb) } else { (b, bb, a, ba) };
        let reach = best + blend * tube * sc2;
        if bf < reach {
            qs[n] = [far[0], far[1], far[2], sc2];
            ls[n] = [lv2, bf, d];
            n += 1;
        }
        if bn < reach {
            qs[n] = [near[0], near[1], near[2], sc2];
            ls[n] = [lv2, bn, d];
            n += 1;
        }
    }
    (best, (level * (1.0 / 6.0)).clamp(0.0, 1.0))
}

/// Alexander's horned sphere: each of the eight points on its own (the
/// search is a tree walk whose path differs from point to point).
fn horned_sphere(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let mut d = [0.0f32; 8];
    let mut t = [0.0f32; 8];
    for i in 0..8 {
        (d[i], t[i]) = horned_lane([p.x.lane(i), p.y.lane(i), p.z.lane(i)], k, iters);
    }
    (F8::from_array(d), F8::from_array(t))
}

const HORNED_SPHERE: &str = r#"
fn horned_sphere_smin(a: f32, b: f32, k: f32) -> f32 {
    if (k <= 0.0) {
        return min(a, b);
    }
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

fn horned_sphere_ring(q: vec3f, a: vec4f, b: vec4f, c: vec4f) -> f32 {
    let ay = abs(q.y);
    let th = atan2(ay, q.x);
    if (th >= b.x) {
        let rxy = sqrt(q.x * q.x + ay * ay);
        let u = rxy - 1.0;
        return sqrt(u * u + q.z * q.z) - a.w * exp(c.w * (3.14159265 - th));
    }
    let u = q.x - c.y;
    let v = ay - c.x;
    return sqrt(u * u + v * v + q.z * q.z) - a.y;
}

fn horned_sphere_subtree(q: vec3f, s: f32, tube: f32, cg: f32, off: f32, rb: f32) -> f32 {
    let rxy = sqrt(q.x * q.x + q.y * q.y) - 1.0;
    let torus = sqrt(rxy * rxy + q.z * q.z) - tube;
    let u = q.x - cg;
    let front = sqrt(u * u + q.y * q.y + q.z * q.z) - (off + s * rb);
    return min(torus, front);
}

fn horned_sphere(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let s = a.x;
    let cp = a.z;
    let tube = a.w;
    let blend = b.y;
    let ball = b.z;
    let sp = b.w;
    let sg = c.x;
    let cg = c.y;
    let rb = c.z;
    let off = sg - s;
    // (x, y, z, scale) and (bound, the parent ring's distance): the
    // generation is the scale's log (a smaller stack runs faster)
    var qs: array<vec4f, 16>;
    var ls: array<vec2f, 16>;
    let ln_s = log(s);
    let leaf = exp(ln_s * (f32(iters) - 1.5));
    qs[0] = vec4f(p.y, p.x, -p.z, 1.0);
    ls[0] = vec2f(-1e9, 1e9);
    var n = 1u;
    var best = 1e9;
    var plain = 1e9;
    var level = 0.0;
    while (n > 0u) {
        n -= 1u;
        let x = qs[n].x;
        let y = qs[n].y;
        let z = qs[n].z;
        let sc = qs[n].w;
        let bd = ls[n].x;
        let dp = ls[n].y;
        let kf = blend * tube * sc;
        if (bd >= best + kf) {
            continue;
        }
        var d = horned_sphere_ring(vec3f(x, y, z), a, b, c) * sc;
        if (sc == 1.0 && ball > 0.0) {
            let u = x + 1.0;
            d = horned_sphere_smin(d, sqrt(u * u + y * y + z * z) - ball, blend * ball * 0.5);
        }
        if (d < plain) {
            plain = d;
            level = round(log(sc) / ln_s);
        }
        best = min(best, horned_sphere_smin(dp, d, kf));
        if (sc < leaf) {
            continue;
        }
        let inv = 1.0 / s;
        let dx = x - cg;
        let dz = z;
        let dya = y - off;
        let ca = vec3f(-dya * inv, (sp * dx + cp * dz) * inv, (sp * dz - cp * dx) * inv);
        let dyb = y + off;
        let cb = vec3f(dyb * inv, (cp * dx - sp * dz) * inv, (-sp * dx - cp * dz) * inv);
        let sc2 = sc * s;
        let ba = horned_sphere_subtree(ca, s, tube, cg, off, rb) * sc2;
        let bb = horned_sphere_subtree(cb, s, tube, cg, off, rb) * sc2;
        let up = ba < bb;
        let near = select(cb, ca, up);
        let bn = select(bb, ba, up);
        let far = select(ca, cb, up);
        let bf = select(ba, bb, up);
        let reach = best + blend * tube * sc2;
        if (bf < reach) {
            qs[n] = vec4f(far, sc2);
            ls[n] = vec2f(bf, d);
            n += 1u;
        }
        if (bn < reach) {
            qs[n] = vec4f(near, sc2);
            ls[n] = vec2f(bn, d);
            n += 1u;
        }
    }
    return vec2f(best, clamp(level * (1.0 / 6.0), 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults): the estimate, times the step, changes at most 1.25
    /// times as fast as the point moves, in all but a hundredth of the
    /// points, and never more than 1.6 times.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(0, 0.3), (1, 1.2), (3, 0.05)],
            &[(0, 0.5), (1, 1.6), (3, 0.35)],
            &[(0, 0.5), (1, 1.2), (2, 0.0), (4, 0.0)],
            &[(0, 0.3), (1, 1.6), (2, 90.0), (5, 1.0)],
            &[(3, 0.35), (4, 0.0), (5, 0.0), (6, 0.0)],
            &[(3, 0.35), (5, 1.0), (6, 0.9)],
            &[(0, 0.45), (2, 20.0), (4, 0.5), (6, 0.3)],
        ];
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let id = find("horned_sphere").unwrap();
        let f = super::super::get(id);
        for &set in corners {
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            for iterations in [f.iterations.0, f.iterations.1] {
                let node = FormulaNode { id, values, iterations };
                let k = node.pack();
                let mut slopes = Vec::new();
                for near in [1.0, 0.3, 0.1] {
                    // (the deeper clasps sit near the top of the first ring)
                    let (reach, cy) = (f.bound * 1.2 * near, if near < 1.0 { 0.8 } else { 0.0 });
                    let e = 1e-3 * reach;
                    for _ in 0..600 {
                        let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                        let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]) + cy, F8::from_array(pts[2]));
                        let (d, t) = (f.cpu)(p, &k, node.iters());
                        let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                        for i in 0..8 {
                            let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                            assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                            if a > 10.0 * e && b.is_finite() {
                                let steep = (b - a).abs() / e * f.step;
                                assert!(steep < 1.6, "{set:?} ({iterations}): changes {steep:.2} times as fast as the point at {:?}", p.lane(i));
                                slopes.push(steep);
                            }
                        }
                    }
                }
                slopes.sort_by(f32::total_cmp);
                let p99 = slopes[slopes.len() * 99 / 100];
                assert!(p99 < 1.25, "{set:?} ({iterations}): a hundredth of the estimate changes {p99:.2} times as fast as the point");
            }
        }
    }

    /// The two children of a ring clasp: each one's cut end lies inside the
    /// other's hole (in the plane of its ring, within its radius), and no
    /// two rings that are not parent and child come within the sum of their
    /// tubes, at the defaults.
    #[test]
    fn the_children_clasp_without_touching() {
        let id = find("horned_sphere").unwrap();
        let f = super::super::get(id);
        let k = FormulaNode { id, values: f.defaults(), iterations: 4 }.pack();
        let (s, cp, tube, sp, sg, cg) = (k[0], k[2], k[3], k[7], k[8], k[9]);
        // child a: centre (cg, sg - s, 0), front -y, in-plane (sp, 0, cp)
        let ca = [cg, sg - s, 0.0];
        let cb = [cg, s - sg, 0.0];
        let front_b = [cb[0], cb[1] + s, 0.0];
        let rel = [front_b[0] - ca[0], front_b[1] - ca[1], front_b[2] - ca[2]];
        let normal_a = [-cp, 0.0, sp];
        let off_plane = rel[0] * normal_a[0] + rel[1] * normal_a[1] + rel[2] * normal_a[2];
        let r = (rel.iter().map(|v| v * v).sum::<f32>() - off_plane * off_plane).sqrt();
        assert!(off_plane.abs() < 1e-5 && r < s * (1.0 - tube), "b's front is {r} from a's centre, a's hole is {}", s * (1.0 - tube));
        // the two children's centre lines, sampled, with their tubes
        let ring_pts = |c: [f32; 3], fr: [f32; 3], w: [f32; 3]| -> Vec<([f32; 3], f32)> {
            let g = k[4];
            (0..200)
                .map(|i| {
                    let th = g + (2.0 * PI - 2.0 * g) * i as f32 / 199.0;
                    let r = tube * (k[11] * (th - PI).abs()).exp() * s;
                    ([0, 1, 2].map(|j| c[j] + s * (th.cos() * fr[j] + th.sin() * w[j])), r)
                })
                .collect()
        };
        let a = ring_pts(ca, [0.0, -1.0, 0.0], [sp, 0.0, cp]);
        let b = ring_pts(cb, [0.0, 1.0, 0.0], [cp, 0.0, -sp]);
        let gap = a
            .iter()
            .flat_map(|(p, rp)| b.iter().map(move |(q, rq)| (0..3).map(|j| (p[j] - q[j]).powi(2)).sum::<f32>().sqrt() - rp - rq))
            .fold(f32::INFINITY, f32::min);
        assert!(gap > 0.0, "the two children touch: {gap}");
    }
}
