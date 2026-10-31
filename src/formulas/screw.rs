//! A sponge folded by a chiral space group: P4₁32, the cubic group whose
//! four-fold axes are all right-handed 4₁ screws (β-manganese's group; its
//! mirror image is P4₃32). Every folding fractal before it folds by mirrors;
//! a screw has no mirror and no fixed point, so this one is reduced by the
//! group's translations and rotations to the cell of one screw axis instead,
//! the screw itself being a symmetry of what stands there, and scaled by 5,
//! the least ratio whose similarity maps the group into itself.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

use std::f32::consts::TAU;

pub const FORMULAS: &[Formula] = &[SCREW];

const SCREW: Formula = Formula {
    name: "screw_sponge",
    category: "Fractals: originals",
    desc: "A sponge of spiral ramps, all of one hand, at every scale: shafts along the right-handed 4₁ screw axes of the chiral cubic space group P4₁32, running three ways, each with a ramp winding around a pillar, drilled level after level through the solid at a fifth of the size (the ratio at which the group maps into itself). Open, only the pillars and ramps stand: a lattice of screws. Folded by a screw group, not by mirrors. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::num("radius", &[0.12], 0.04, 0.2, "Each shaft's radius, in unit cells (0.04 to 0.2). Screw axes of different directions pass a quarter cell apart: up to 0.125 the shafts stay apart; past it they break into each other through windows and the corridors join up."),
        Param::num("pillar", &[0.25], 0.05, 0.8, "The pillar at each shaft's center, as a share of its radius (0.05 to 0.8): the ramp winds around it."),
        Param::num("ramp", &[0.25], 0.0, 0.9, "The ramp's thickness, as a share of the height it climbs in one turn (0 to 0.9): 0 leaves bare shafts around the pillars; near 0.9 the ramps close the shafts but for a thin spiral slot."),
        Param::whole("turns", 5.0, -7.0, 9.0, "Turns of each ramp per unit cell (-7 to 9). The 4₁ screw (a quarter turn and a quarter cell) carries a ramp onto itself only if it turns 1, 5 or 9 times (right-handed) or -3 or -7 times (left-handed, against the axes' hand); other values go to the nearest of these. 1 makes steep corkscrews, 5 walkable ramps."),
        Param::num("phase", &[0.0], 0.0, 180.0, "Where the ramps start around their shafts, in degrees (0 to 180). The group pairs the shafts so that this turns half of them one way and half the other, which moves the ramps of neighboring shafts against each other."),
        Param::whole("open", 0.0, 0.0, 1.0, "0: the sponge, its shafts drilled through solid (spiral corridors). 1: only what stands in the biggest shafts, their pillars and ramps (drilled by the smaller levels), the solid between taken away: a lattice of screws."),
    ],
    iterations: (2, 6, "Levels of shafts as seen from afar (1 to 6): each drills shafts five times smaller through what the last left. Close up more are added by themselves, as many as the pixels can show (up to 6)."),
    bound: f32::INFINITY,
    lod: Some((5.0, 6)),
    step: 1.0,
    example: r#"{"type":"intersection","scale":0.9,"children":[{"type":"box","size":[2.25,2.25,2.25]},{"type":"screw_sponge","open":1}]}"#,
    prep: Some(screw_prep),
    cpu: screw_sponge,
    wgsl: SCREW_SPONGE,
    wgsl_fn: "screw_sponge",
};

/// The ratio from level to level: the least similarity ratio s with
/// s G s⁻¹ ⊂ G (s ≡ 1 mod 4: the 4₁ screws' quarter-cell shifts must stay
/// quarter-cell shifts; -3 also works, but as a point reflection it would
/// mirror every other level).
const RATIO: f32 = 5.0;

/// In place: the pillar's radius, the ramp's half thickness in turns, its
/// turns per cell (snapped to 1 mod 4) and the phase in turns.
fn screw_prep(k: &mut [f32; K]) {
    k[1] *= k[0];
    k[2] *= 0.5;
    k[3] = 4.0 * ((k[3] - 1.0) * 0.25).round() + 1.0;
    k[4] /= 360.0;
}

/// floor(x + 0.5): the same rounding on both devices.
#[inline(always)]
fn nearest(x: F8) -> F8 {
    (x + 0.5).floor()
}

/// The distance from (x, y) to the nearest screw axis along z: what
/// `shaft` starts with, without the ramp (both of its distances are at
/// least this less the radius, so a direction this far is skipped).
#[inline(always)]
fn axis_distance(x: F8, y: F8) -> F8 {
    let (ux, uy) = (x - 0.75, y - 0.5);
    let (a, b) = (ux + uy, ux - uy);
    let (a, b) = (a - nearest(a), b - nearest(b));
    len2((a + b) * 0.5, (a - b) * 0.5)
}

/// The signed distance to the hole of the nearest shaft along z (its 4₁
/// axes are at (3/4, 1/2) + the lattice of (1/2, ±1/2)), in cell units:
/// the shaft less its pillar and ramp; and to what stands in the shaft, the
/// pillar and the ramp (cut to the shaft).
///
/// The fold: the axis lattice is a square one turned 45°, so rounding in
/// the turned coordinates finds the nearest axis. Its two classes, (3/4, 1/2)
/// + Z² and (1/4, 0) + Z², are swapped by the group's two-fold rotations
/// (about lines along (1, -1, 0) at height 3/8), which reverse the axis: a
/// right-handed ramp stays right-handed but its phase changes sign. Along the
/// axis no fold is needed: the ramp, θ = 2π n z + 2π phase, is carried onto
/// itself by the 4₁ screw (a quarter turn and a quarter cell) when n ≡ 1
/// mod 4, so the screw is a symmetry of the motif, and the helical phase
/// g = n z - θ / 2π (mod 1) already measures the point in the screw's frame.
///
/// The ramp is |g| ≤ w. g's gradient is sqrt(n² + 1 / (2π ρ)²), steeper
/// toward the axis, so |g| - w over it is no bound by itself; along any path
/// of length r from the point ρ stays above ρ - r (and above the pillar's
/// radius, which the path would cross first), so with r₁ the estimate at the
/// point's own ρ and r₂ the estimate at ρ - r₁, r₂ ≤ the fixed point r* =
/// G / |∇g|(ρ - r*): a true lower bound on the distance to the ramp.
#[inline(always)]
fn shaft(x: F8, y: F8, z: F8, k: &[f32; K]) -> (F8, F8) {
    let (radius, c, w, n, ph) = (k[0], k[1], k[2], k[3], k[4]);
    let (ux, uy) = (x - 0.75, y - 0.5);
    let (a, b) = (ux + uy, ux - uy);
    let (ia, ib) = (nearest(a), nearest(b));
    let (a, b) = (a - ia, b - ib);
    let (lx, ly) = ((a + b) * 0.5, (a - b) * 0.5);
    // the axis' class: its phase's sign
    let odd = ia + ib;
    let odd = odd - (odd * 0.5).floor() * 2.0;
    let phc = (odd * -2.0 + 1.0) * ph;
    let rho = len2(lx, ly);
    let th = ly.atan2(lx);
    let g = z.mul_add(s(n), th.mul_add(s(-1.0 / TAU), phc));
    let g = (g - nearest(g)).abs() - w;
    let tn = TAU * n;
    let rho1 = rho.max(s(c));
    let r1 = g * TAU * rho1 / (rho1 * tn).mul_add(rho1 * tn, F8::one()).sqrt();
    let rho2 = (rho - r1.abs()).max(s(c));
    let r2 = g * TAU * rho2 / (rho2 * tn).mul_add(rho2 * tn, F8::one()).sqrt();
    // (not each other's negatives: both are cut to the shaft, so outside it
    // both are positive)
    ((rho - radius).max(-r2).max(s(c) - rho), r2.min(rho - c).max(rho - radius))
}

/// The screw sponge's distance estimate and orbit trap (which level's
/// shafts, and which of their three directions, the surface is a wall of).
///
/// The point is reduced by the lattice, and the shafts of the three
/// directions are found by the three-fold rotation (x, y, z) -> (z, x, y)
/// about the cube's diagonal: the shafts along x are those along z turned,
/// so they are measured at (y, z, x), and those along y at (z, x, y). Their
/// union is the level's holes, invariant under all of P4₁32. Then the point
/// is scaled by 5 about the origin: 5 G 5⁻¹ ⊂ G, as the translations' parts
/// are quarters, so the scaled holes are invariant under G too and the
/// reduction stays valid. The solid is space less every level's holes; its
/// estimate is the greatest of each level's depth into its holes (exact but
/// for the ramp's lower bound) over 5^level, as for the Menger sponge. Open,
/// the first level is its screws instead, what stands in its shafts.
fn screw_sponge(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let mut q = V8::new(p.x - nearest(p.x), p.y - nearest(p.y), p.z - nearest(p.z));
    let mut d = s(-1e9);
    let mut trap = F8::zero();
    let mut sc = 1.0f32;
    for i in 0..iters {
        // (no hole of this level or deeper is deeper than its shaft's radius
        // over its scale: once every lane is that far out, they cannot move
        // the estimate, and the march in open space stops at the first level)
        if i > 0 && d.ge(s(k[0] / sc)).all() {
            break;
        }
        // the three directions' shafts, z, then x and y (found by the
        // three-fold rotation); a direction whose nearest axis is farther
        // than the radius plus the best so far cannot do better, and is
        // skipped (when every lane can skip it)
        let open = i == 0 && k[5] > 0.5;
        let (h, a) = shaft(q.x, q.y, q.z, k);
        let mut best = if open { a } else { h };
        let mut fam = F8::zero();
        for (f, x, y, z) in [(1.0, q.y, q.z, q.x), (2.0, q.z, q.x, q.y)] {
            if (axis_distance(x, y) - k[0]).ge(best).all() {
                continue;
            }
            let (h, a) = shaft(x, y, z, k);
            let v = if open { a } else { h };
            fam = F8::select(v.lt(best), s(f), fam);
            best = best.min(v);
        }
        if open {
            // open: the first level's screws (pillars and ramps) alone, the
            // union of their three directions. (Not the sponge cut to the
            // shafts: a hole and its shaft share a wall, where the greater of
            // their two estimates is 0 with nothing there, a phantom wall.)
            trap = fam * (1.0 / 9.0);
            d = best;
        } else {
            let h = -best * (1.0 / sc);
            let deeper: M8 = h.gt(d);
            trap = F8::select(deeper, fam.mul_add(s(1.0 / 9.0), s(i as f32 / 3.0)), trap);
            d = d.max(h);
        }
        let r = q * RATIO;
        q = V8::new(r.x - nearest(r.x), r.y - nearest(r.y), r.z - nearest(r.z));
        sc *= RATIO;
    }
    (d, trap.saturate())
}

const SCREW_SPONGE: &str = r#"
fn screw_sponge_axis(x: f32, y: f32) -> f32 {
    let ux = x - 0.75;
    let uy = y - 0.5;
    let a0 = ux + uy;
    let b0 = ux - uy;
    let aa = a0 - floor(a0 + 0.5);
    let bb = b0 - floor(b0 + 0.5);
    return length(vec2f((aa + bb) * 0.5, (aa - bb) * 0.5));
}

fn screw_sponge_shaft(x: f32, y: f32, z: f32, a: vec4f, b: vec4f) -> vec2f {
    let tau = 6.2831853;
    let radius = a.x;
    let c = a.y;
    let w = a.z;
    let n = a.w;
    let ph = b.x;
    let ux = x - 0.75;
    let uy = y - 0.5;
    let a0 = ux + uy;
    let b0 = ux - uy;
    let ia = floor(a0 + 0.5);
    let ib = floor(b0 + 0.5);
    let aa = a0 - ia;
    let bb = b0 - ib;
    let lx = (aa + bb) * 0.5;
    let ly = (aa - bb) * 0.5;
    let o = ia + ib;
    let odd = o - floor(o * 0.5) * 2.0;
    let phc = (odd * -2.0 + 1.0) * ph;
    let rho = length(vec2f(lx, ly));
    let th = atan2(ly, lx);
    var g = z * n + (th * (-1.0 / tau) + phc);
    g = abs(g - floor(g + 0.5)) - w;
    let tn = tau * n;
    let rho1 = max(rho, c);
    let r1 = g * tau * rho1 / sqrt(rho1 * tn * rho1 * tn + 1.0);
    let rho2 = max(rho - abs(r1), c);
    let r2 = g * tau * rho2 / sqrt(rho2 * tn * rho2 * tn + 1.0);
    return vec2f(max(max(rho - radius, -r2), c - rho), max(min(r2, rho - c), rho - radius));
}

fn screw_sponge(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    var q = p - floor(p + 0.5);
    var d = -1e9;
    var trap = 0.0;
    var sc = 1.0;
    for (var i = 0u; i < iters; i++) {
        if (i > 0u && d >= a.x / sc) {
            break;
        }
        let open = i == 0u && b.y > 0.5;
        let s0 = screw_sponge_shaft(q.x, q.y, q.z, a, b);
        var best = select(s0.x, s0.y, open);
        var fam = 0.0;
        if (screw_sponge_axis(q.y, q.z) - a.x < best) {
            let s1 = screw_sponge_shaft(q.y, q.z, q.x, a, b);
            let v = select(s1.x, s1.y, open);
            fam = select(fam, 1.0, v < best);
            best = min(best, v);
        }
        if (screw_sponge_axis(q.z, q.x) - a.x < best) {
            let s2 = screw_sponge_shaft(q.z, q.x, q.y, a, b);
            let v = select(s2.x, s2.y, open);
            fam = select(fam, 2.0, v < best);
            best = min(best, v);
        }
        if (open) {
            trap = fam * (1.0 / 9.0);
            d = best;
        } else {
            let h = -best * (1.0 / sc);
            trap = select(trap, fam * (1.0 / 9.0) + f32(i) / 3.0, h > d);
            d = max(d, h);
        }
        let r = q * 5.0;
        q = r - floor(r + 0.5);
        sc *= 5.0;
    }
    return vec2f(d, clamp(trap, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, get, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), close to a screw axis too: a hundredth of the
    /// estimate, times the step, changes at most 1.25 times as fast as the
    /// point moves, and none jumps (at most 3 times: the ramp's bound is
    /// steepest beside thin pillars).
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(5, 1.0)],
            &[(0, 0.04), (1, 0.05), (3, 1.0)],
            &[(0, 0.2), (1, 0.05), (3, 1.0)],
            &[(0, 0.2), (1, 0.05), (2, 0.9), (3, 9.0), (5, 1.0)],
            &[(0, 0.2), (1, 0.8), (2, 0.0), (4, 180.0)],
            &[(0, 0.2), (1, 0.05), (2, 0.1), (3, -7.0), (4, 90.0), (5, 1.0)],
            &[(0, 0.12), (1, 0.05), (2, 0.5), (3, -3.0), (4, 30.0)],
            &[(0, 0.04), (1, 0.8), (2, 0.9), (3, 9.0), (5, 1.0)],
        ];
        let id = find("screw_sponge").unwrap();
        let f = get(id);
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
            let mut slopes = Vec::new();
            // (everywhere, then about a screw axis, (3/4, 1/2, z), where
            // the shafts are)
            for (reach, at) in [(3.0f32, [0.0f32; 3]), (0.6, [0.0; 3]), (0.25, [0.75, 0.5, 0.0]), (0.08, [0.75, 0.5, 0.0])] {
                let e = 1e-3 * reach;
                for _ in 0..1500 {
                    let pts: [[f32; 8]; 3] = std::array::from_fn(|c| std::array::from_fn(|_| at[c] + rnd() * reach));
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
            }
            assert!(slopes.len() > 500, "{set:?}: only {} points outside", slopes.len());
            slopes.sort_by(f32::total_cmp);
            let p99 = slopes[slopes.len() * 99 / 100];
            let worst = slopes[slopes.len() - 1];
            assert!(p99 < 1.25 && worst < 3.0, "{set:?}: a hundredth of the estimate changes {p99:.2} times as fast as the point (worst {worst:.2})");
        }
    }

    /// The holes are invariant under P4₁32 (generated by two 2₁ screws, the
    /// three-fold rotation and a two-fold rotation, which together give the
    /// 4₁ screws), and under the group scaled by 5: the estimate is the same
    /// at every point of an orbit, sponge or open.
    #[test]
    fn the_holes_have_the_screw_group() {
        type Op = fn([f32; 3]) -> [f32; 3];
        let ops: [Op; 5] = [
            |[x, y, z]| [0.5 - x, -y, z + 0.5],
            |[x, y, z]| [-x, y + 0.5, 0.5 - z],
            |[x, y, z]| [z, x, y],
            |[x, y, z]| [y + 0.75, x + 0.25, 0.25 - z],
            // the 4₁ screw about (3/4, 1/2): a quarter turn and a quarter cell
            |[x, y, z]| [0.25 - y, x + 0.75, z + 0.25],
        ];
        let id = find("screw_sponge").unwrap();
        let f = get(id);
        let mut seed = 0x1234_5678u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let split = |v: [[f32; 3]; 8]| -> [[f32; 8]; 3] { std::array::from_fn(|c| std::array::from_fn(|i| v[i][c])) };
        for open in [0.0, 1.0] {
            let mut values = f.defaults();
            values[4] = 37.0;
            values[5] = open;
            let node = FormulaNode { id, values, iterations: 2 };
            let k = node.pack();
            let at = |p: [[f32; 8]; 3]| (f.cpu)(V8::new(F8::from_array(p[0]), F8::from_array(p[1]), F8::from_array(p[2])), &k, 2).0;
            for _ in 0..200 {
                let pts: [[f32; 3]; 8] = std::array::from_fn(|_| [rnd() * 2.0, rnd() * 2.0, rnd() * 2.0]);
                let d = at(split(pts));
                for op in ops {
                    let d2 = at(split(pts.map(op)));
                    for i in 0..8 {
                        // (near the surface: deep in the solid a point can
                        // lie halfway between two axes, where which one is
                        // measured is a matter of rounding)
                        if d.lane(i) > -0.1 {
                            assert!((d.lane(i) - d2.lane(i)).abs() < 2e-4, "open {open}: {:?}: {} vs {}", pts[i], d.lane(i), d2.lane(i));
                        }
                    }
                }
            }
        }
    }
}
