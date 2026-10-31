//! Triply periodic minimal surfaces made hierarchical: the labyrinth sponge,
//! a sheet around a minimal surface whose walls are drilled through by the
//! labyrinths of a smaller one, whose walls are drilled by a smaller one
//! still.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, V8};
use std::f32::consts::TAU;

pub const FORMULAS: &[Formula] = &[LABYRINTH];

const LABYRINTH: Formula = Formula {
    name: "labyrinth_sponge",
    category: "Fractals: originals",
    desc: "Labyrinths within labyrinths: the sheet around a triply periodic minimal surface, whose walls are drilled through by both labyrinths of a minimal surface a few times smaller, whose walls are drilled by smaller ones again, the levels alternating between two surfaces (by default a gyroid whose walls are a lattice of Schwarz P rooms, whose walls are pierced by gyroids again). Every tunnel opens into the larger ones, so all of it can be walked. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("surface", 0.0, 0.0, 3.0, "The minimal surface of the largest level and every second one after it: 0 the gyroid, 1 Schwarz's D (diamond), 2 Schwarz's P (primitive), 3 Neovius's."),
        Param::whole("inner", 2.0, 0.0, 3.0, "The minimal surface of the second level and every second one after it (as surface): the walls of one are made of the other's labyrinths."),
        Param::num("wall", &[0.35], 0.15, 0.95, "The largest level's wall: the sheet's thickness as a share of the most it can have before its corridors close (0.15 to 0.95). Low values leave wide corridors and thin walls."),
        Param::num("tunnels", &[0.85], 0.4, 0.97, "The smaller levels' walls, as wall (0.4 to 0.97): high values drill thin tunnels into the walls and keep them solid; low ones hollow them into lace."),
        Param::whole("ratio", 5.0, 2.0, 6.0, "How many times smaller each level is than the last (2 to 6). (Detail close up assumes 5.)"),
        Param::num("round", &[0.2], 0.0, 1.0, "How much the rim of each tunnel is rounded where it opens into the larger level's wall (0 to 1): 0 cuts it sharp."),
    ],
    iterations: (2, 8, "Smaller levels, each `ratio` times finer, drilled into the walls of the last (more are added close up)."),
    bound: f32::INFINITY,
    lod: Some((5.0, 8)),
    step: 1.0,
    example: r#"{"type":"intersection","scale":0.18,"rotation":[0,20,0],"children":[{"type":"box","size":[11,11,11]},{"type":"labyrinth_sponge"}]}"#,
    prep: Some(labyrinth_prep),
    cpu: labyrinth_sponge,
    wgsl: LABYRINTH_SPONGE,
    wgsl_fn: "labyrinth_sponge",
};

/// Per surface: the level its labyrinths pinch off at (the saddle value of
/// its level-set function, where a sheet |f| <= w closes its channels).
const SADDLE: [f32; 4] = [std::f32::consts::SQRT_2, 1.0, 1.0, 0.75];

/// The walls' shares into level-set values.
fn labyrinth_prep(k: &mut [f32; K]) {
    let (t0, t1) = (k[0] as usize, k[1] as usize);
    let (wall, tunnels) = (k[2], k[3]);
    k[2] = wall * SADDLE[t0];
    k[3] = tunnels * SADDLE[t0];
    k[6] = tunnels * SADDLE[t1];
}

/// Per surface (scalars): its frequency (P and Neovius have one large room
/// a period, against the narrower channels of the gyroid and D, so they run
/// at twice the frequency), the
/// bound on its second derivative along any line and on its gradient (both
/// from its waves: f = Σ a cos(k·q + φ) has |∂²f/∂u²| <= Σ |a| (k·u)² and
/// |∂f/∂u| <= Σ |a| |k·u|), and its largest value.
#[inline(always)]
fn consts(t: u32) -> (f32, f32, f32, f32) {
    match t {
        // six waves of 1/2 along (1, ±1, 0) and its turns
        0 => (1.0, 2.0, 2.449_49, 1.5),
        // four waves of 1/2 along (±1, ±1, 1)
        1 => (1.0, 2.0, 2.0, std::f32::consts::SQRT_2),
        2 => (2.0, 1.0, 1.732_051, 3.0),
        // three of 3 along the axes, four of 1 along the diagonals
        _ => (2.0, 7.0, 9.2, 13.0),
    }
}

/// x mod 2π, in [0, 2π).
#[inline(always)]
fn wrap(x: F8) -> F8 {
    (x * (1.0 / TAU)).floor().neg_mul_add(s(TAU), x)
}

/// A minimal surface's level-set function at q (its own radians, times its
/// frequency m): its value and gradient.
#[inline(always)]
fn surface(t: u32, q: V8, m: f32) -> (F8, F8, F8, F8) {
    let (x, y, z) = (q.x * m, q.y * m, q.z * m);
    let (sx, cx, sy, cy, sz, cz) = (x.sin(), x.cos(), y.sin(), y.cos(), z.sin(), z.cos());
    match t {
        // the gyroid: sin x cos y + sin y cos z + sin z cos x
        0 => {
            let f = sz.mul_add(cx, sy.mul_add(cz, sx * cy));
            (f, cx.mul_add(cy, -(sz * sx)), cy.mul_add(cz, -(sx * sy)), cz.mul_add(cx, -(sy * sz)))
        }
        // Schwarz D: sin x cos(y - z) + cos x sin(y + z)
        1 => {
            let a = cy.mul_add(cz, sy * sz);
            let b = sy.mul_add(cz, cy * sz);
            let c = sy.mul_add(cz, -(cy * sz));
            let d = cy.mul_add(cz, -(sy * sz));
            (sx.mul_add(a, cx * b), cx.mul_add(a, -(sx * b)), cx.mul_add(d, -(sx * c)), cx.mul_add(d, sx * c))
        }
        // Schwarz P: cos x + cos y + cos z
        2 => (cx + cy + cz, -sx, -sy, -sz),
        // Neovius: 3 (cos x + cos y + cos z) + 4 cos x cos y cos z
        _ => {
            let f = (cx + cy + cz).mul_add(s(3.0), cx * cy * cz * 4.0);
            let gx = -sx * (cy * cz).mul_add(s(4.0), s(3.0));
            let gy = -sy * (cx * cz).mul_add(s(4.0), s(3.0));
            let gz = -sz * (cx * cy).mul_add(s(4.0), s(3.0));
            (f, gx, gy, gz)
        }
    }
}

/// The most level `i` (its surface t, wall w, scale sc) can add to the
/// estimate: its largest distance outside its sheet, plus its rounding.
#[inline(always)]
fn reach(t: u32, w: f32, sc: f32, round: f32) -> f32 {
    let (m, hb, lb, top) = consts(t);
    let h = (top - w).max(0.0);
    ((2.0 * h / hb).sqrt().max(h / lb) + round) / (sc * m)
}

/// The labyrinth sponge's distance estimate and orbit trap (which level's
/// wall the surface is).
///
/// Level i is the sheet |f(m r^i p)| <= w about a minimal surface's
/// level-set function f (r the ratio, m the surface's frequency); the shape
/// is all of them at once (their intersection), levels alternating between
/// two surfaces. Each level's estimate comes from its
/// value, its gradient g and its second-derivative bound H: |f| stays past w
/// for |v| < 2h / (g + √(g² + 2Hh)), h = | |f| - w |, since |f(q + v)| >=
/// |f(q)| - g|v| - H|v|²/2 (and within h / L, L its gradient bound). The
/// levels are joined by a smooth maximum (its gradient is a blend of theirs,
/// so the slope stays at most 1) with a radius of `round` of the finer
/// level's scale. Points in the angle-reduced radians of each level (q mod
/// 2π, scaled by the whole ratio) keep sin and cos exact at any depth.
fn labyrinth_sponge(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (t_even, t_odd) = (k[0] as u32, k[1] as u32);
    let (w0, w_even, w_odd) = (k[2], k[3], k[6]);
    let (ratio, round) = (k[4], k[5]);
    let mut q = V8::new(wrap(p.x), wrap(p.y), wrap(p.z));
    let mut sc = 1.0f32;
    let mut best = s(-1e9);
    let mut level = F8::zero();
    for i in 0..=iters {
        if i > 0 {
            q = V8::new(wrap(q.x * ratio), wrap(q.y * ratio), wrap(q.z * ratio));
            sc *= ratio;
        }
        let odd = i % 2 == 1;
        let t = if odd { t_odd } else { t_even };
        let w = if i == 0 { w0 } else if odd { w_odd } else { w_even };
        let (m, hb, lb, _) = consts(t);
        let (f, gx, gy, gz) = surface(t, q, m);
        let h = f.abs() - w;
        let a = h.abs();
        let g = gz.mul_add(gz, gy.mul_add(gy, gx * gx)).sqrt();
        let r = (a * 2.0) / (g + (g * g + a * (2.0 * hb)).sqrt()).max(s(1e-12));
        let r = r.max(a * (1.0 / lb));
        let d = r.copysign(h) * (1.0 / (sc * m));
        level = F8::select(d.gt(best), s(i as f32), level);
        // the smooth maximum, of radius kb
        let kb = round / (sc * m);
        let e = (s(kb) - (best - d).abs()).max(F8::zero());
        best = best.max(d) + e * e * (0.25 / kb.max(1e-12));
        // done when no finer level can reach this far (each parity's reach
        // shrinks level by level, so the next two bound them all)
        let (t1, t2) = if odd { (t_even, t_odd) } else { (t_odd, t_even) };
        let (w1, w2) = if odd { (w_even, w_odd) } else { (w_odd, w_even) };
        let far = reach(t1, w1, sc * ratio, round).max(reach(t2, w2, sc * ratio * ratio, round));
        if best.gt(s(far)).all() {
            break;
        }
    }
    // (four levels' colours, then round again)
    let cyc = level - (level * 0.25).floor() * 4.0;
    (best, (cyc * (1.0 / 3.0)).saturate())
}

const LABYRINTH_SPONGE: &str = r#"
fn labyrinth_sponge_wrap(x: vec3f) -> vec3f {
    return x - floor(x * (1.0 / 6.28318531)) * 6.28318531;
}

/// Surface t's frequency, second-derivative bound, gradient bound and largest value.
fn labyrinth_sponge_consts(t: u32) -> vec4f {
    if (t == 0u) {
        return vec4f(1.0, 2.0, 2.44949, 1.5);
    }
    if (t == 1u) {
        return vec4f(1.0, 2.0, 2.0, 1.41421356);
    }
    if (t == 2u) {
        return vec4f(2.0, 1.0, 1.732051, 3.0);
    }
    return vec4f(2.0, 7.0, 9.2, 13.0);
}

/// (value, gradient) of surface t at q (times its frequency m).
fn labyrinth_sponge_surface(t: u32, q0: vec3f, m: f32) -> vec4f {
    let q = q0 * m;
    let sn = sin(q);
    let cs = cos(q);
    let sx = sn.x;
    let sy = sn.y;
    let sz = sn.z;
    let cx = cs.x;
    let cy = cs.y;
    let cz = cs.z;
    if (t == 0u) {
        let f = sz * cx + (sy * cz + sx * cy);
        return vec4f(f, cx * cy - sz * sx, cy * cz - sx * sy, cz * cx - sy * sz);
    }
    if (t == 1u) {
        let a = cy * cz + sy * sz;
        let b = sy * cz + cy * sz;
        let c = sy * cz - cy * sz;
        let d = cy * cz - sy * sz;
        return vec4f(sx * a + cx * b, cx * a - sx * b, cx * d - sx * c, cx * d + sx * c);
    }
    if (t == 2u) {
        return vec4f(cx + cy + cz, -sx, -sy, -sz);
    }
    let f = (cx + cy + cz) * 3.0 + cx * cy * cz * 4.0;
    return vec4f(f, -sx * (cy * cz * 4.0 + 3.0), -sy * (cx * cz * 4.0 + 3.0), -sz * (cx * cy * 4.0 + 3.0));
}

fn labyrinth_sponge_reach(t: u32, w: f32, sc: f32, round: f32) -> f32 {
    let k = labyrinth_sponge_consts(t);
    let h = max(k.w - w, 0.0);
    return (max(sqrt(2.0 * h / k.y), h / k.z) + round) / (sc * k.x);
}

fn labyrinth_sponge(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let t_even = u32(a.x);
    let t_odd = u32(a.y);
    let w0 = a.z;
    let w_even = a.w;
    let ratio = b.x;
    let round = b.y;
    let w_odd = b.z;
    var q = labyrinth_sponge_wrap(p);
    var sc = 1.0;
    var best = -1e9;
    var level = 0.0;
    for (var i = 0u; i <= iters; i++) {
        if (i > 0u) {
            q = labyrinth_sponge_wrap(q * ratio);
            sc *= ratio;
        }
        let odd = (i & 1u) == 1u;
        let t = select(t_even, t_odd, odd);
        let w = select(select(w_even, w_odd, odd), w0, i == 0u);
        let k = labyrinth_sponge_consts(t);
        let m = k.x;
        let e = labyrinth_sponge_surface(t, q, m);
        let h = abs(e.x) - w;
        let ah = abs(h);
        let g = length(e.yzw);
        var r = (ah * 2.0) / max(g + sqrt(g * g + ah * (2.0 * k.y)), 1e-12);
        r = max(r, ah * (1.0 / k.z));
        let d = select(r, -r, h < 0.0) * (1.0 / (sc * m));
        level = select(level, f32(i), d > best);
        let kb = round / (sc * m);
        let ex = max(kb - abs(best - d), 0.0);
        best = max(best, d) + ex * ex * (0.25 / max(kb, 1e-12));
        let t1 = select(t_odd, t_even, odd);
        let t2 = select(t_even, t_odd, odd);
        let w1 = select(w_odd, w_even, odd);
        let w2 = select(w_even, w_odd, odd);
        let far = max(labyrinth_sponge_reach(t1, w1, sc * ratio, round), labyrinth_sponge_reach(t2, w2, sc * ratio * ratio, round));
        if (best > far) {
            break;
        }
    }
    let cyc = level - floor(level * 0.25) * 4.0;
    return vec2f(best, clamp(cyc * (1.0 / 3.0), 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), close up and further out.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(0, 0.0), (1, 0.0), (2, 0.15), (3, 0.4), (4, 2.0)],
            &[(0, 1.0), (1, 2.0), (2, 0.95), (3, 0.97), (4, 5.0), (5, 1.0)],
            &[(0, 2.0), (1, 3.0), (2, 0.15), (3, 0.97), (4, 4.0)],
            &[(0, 3.0), (1, 0.0), (2, 0.95), (3, 0.4), (4, 3.0), (5, 0.0)],
            &[(0, 3.0), (1, 3.0), (2, 0.5), (3, 0.8), (4, 2.0), (5, 0.6)],
            &[(0, 2.0), (1, 1.0), (2, 0.5), (3, 0.6), (4, 3.0)],
        ];
        let id = find("labyrinth_sponge").unwrap();
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
            for iterations in [1, 3, 6] {
                let node = FormulaNode { id, values, iterations };
                let k = node.pack();
                let mut slopes = Vec::new();
                for reach in [6.0, 1.0, 0.1] {
                    let e = 1e-3 * reach;
                    for _ in 0..800 {
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
                }
                slopes.sort_by(f32::total_cmp);
                let p99 = slopes.get(slopes.len() * 99 / 100).copied().unwrap_or(0.0);
                let top = slopes.last().copied().unwrap_or(0.0);
                eprintln!("{set:?} ({iterations} levels): p99 slope {p99:.3}, steepest {top:.3}");
                assert!(p99 < 1.1 && top < 1.6, "{set:?} ({iterations} levels): p99 slope {p99:.3}, steepest {top:.3}");
            }
        }
    }

    /// The estimate never reaches past the surface: no point within
    /// `step` times the estimate of a point outside is inside.
    #[test]
    fn estimates_are_conservative() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(0, 0.0), (1, 0.0), (2, 0.15), (3, 0.4), (4, 2.0)],
            &[(0, 1.0), (1, 2.0), (2, 0.95), (3, 0.97), (4, 6.0), (5, 1.0)],
            &[(0, 2.0), (1, 3.0), (2, 0.15), (3, 0.97), (4, 4.0)],
            &[(0, 3.0), (1, 0.0), (2, 0.95), (3, 0.4), (4, 3.0), (5, 0.0)],
            &[(0, 3.0), (1, 3.0), (2, 0.5), (3, 0.8), (4, 2.0), (5, 0.6)],
        ];
        let id = find("labyrinth_sponge").unwrap();
        let f = super::super::get(id);
        let mut seed = 0x1234_5679u32;
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
            let node = FormulaNode { id, values, iterations: 3 };
            let k = node.pack();
            let (mut worst, mut tried) = (0.0f32, 0);
            for reach in [6.0, 0.5] {
                for _ in 0..400 {
                    let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                    let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                    let (d, _) = (f.cpu)(p, &k, node.iters());
                    // along a random direction per lane, at shares of the estimate
                    let u: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd()));
                    let n = V8::new(F8::from_array(u[0]), F8::from_array(u[1]), F8::from_array(u[2])).normalize();
                    for j in 1..=16 {
                        let t = d.max(F8::zero()) * (f.step * j as f32 / 16.0);
                        let (dq, _) = (f.cpu)(n.mul_add(t, p), &k, node.iters());
                        for i in 0..8 {
                            if d.lane(i) > 1e-4 {
                                tried += 1;
                                // (how far inside, as a share of the step taken)
                                worst = worst.max(-dq.lane(i) / d.lane(i));
                            }
                        }
                    }
                }
            }
            eprintln!("{set:?}: {tried} probes, deepest {worst:.4} of a step inside");
            assert!(worst < 0.02, "{set:?}: a probe within the estimate was {worst:.3} of it inside");
        }
    }
}
