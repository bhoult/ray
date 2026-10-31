//! A new formula, made for this catalog: an icosahedral quasicrystal of
//! rooms, cut from six-dimensional space by the irrational 3-plane of the
//! cut-and-project method and nested under the golden inflation τ³.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[ROOMS];

// ---- the quasicrystal rooms -------------------------------------------------

const ROOMS: Formula = Formula {
    name: "quasicrystal_rooms",
    category: "Fractals: originals",
    desc: "Rooms of an icosahedral quasicrystal, never repeating: dodecahedral rooms (or triacontahedral, or icosahedral) whose walls are pierced by rooms τ³ ≈ 4.24 times smaller, and theirs by smaller still, so every wall is a five-fold rose window. Space is lifted into the six-dimensional cubic lattice along the irrational icosahedral 3-plane (the cut-and-project method), the rooms are the cells of de Bruijn's icosahedral multigrid, and the golden inflation, an integer matrix on the 6-torus, nests them. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("rooms", 0.0, 0.0, 2.0, "The rooms' shape (0 to 2): 0 dodecahedra, whose faces are square to the six five-fold axes; 1 rhombic triacontahedra (the fifteen two-fold axes); 2 icosahedra (the ten three-fold axes). Only the central room is regular; every other one is the same 6D cell cut at another depth, so no two are alike."),
        Param::num("size", &[0.88], 0.3, 1.0, "How much of each cell the room takes (0.3 to 1): small rooms leave thick walls with few windows; near 1 the walls thin to lace. Triacontahedral and icosahedral rooms open doorways into their neighbors from about 0.55."),
        Param::num("shift", &[0.0, 0.0, 0.0], -1.0, 1.0, "Moves the slice through 6D space in the three directions square to our own (each −1 to 1). At 0 the origin is the center of a room with full icosahedral symmetry; any other value gives another patch of the same quasicrystal, with no center."),
        Param::num("taper", &[0.92], 0.7, 1.15, "Each finer level's rooms, as a share of the size above (0.7 to 1.15): below 1 the fine windows shrink and the walls stay solid; above 1 they spread into lace."),
    ],
    iterations: (4, 8, "Levels of rooms: each pierces the walls of the last with rooms τ³ ≈ 4.24 times smaller. More are added close up."),
    bound: f32::INFINITY,
    lod: Some((ROOMS_TAU3, 8)),
    step: 1.0,
    example: r#"{"type":"intersection","scale":1.0,"children":[{"type":"sphere","radius":1.1},{"type":"quasicrystal_rooms","scale":0.45,"rotation":[31.717,0,0]}]}"#,
    prep: Some(rooms_prep),
    cpu: quasicrystal_rooms,
    wgsl: QUASICRYSTAL_ROOMS,
    wgsl_fn: "quasicrystal_rooms",
};

/// τ³ = 2 + √5, the inflation from one level to the next.
const ROOMS_TAU3: f32 = 4.236_068;
/// The six five-fold axes' parts in our space (the rows of the projection,
/// (±A, B, 0) cyclically, each of length 1/√2): A = 1/√(2(2+τ)), B = τA.
const ROOMS_A: f32 = 0.371_748_03;
const ROOMS_B: f32 = 0.601_500_96;
/// The room's size at `size` 1, per shape: in units of the sum of the
/// largest one, two or three of |y_i| (whose most are 1/2, 1 and 3/2).
const ROOMS_FULL: [f32; 3] = [0.5, 0.95, 1.2];
/// The steepest that sum gets along our space (|a_i|, |a_i ± a_j|,
/// |a_i ± a_j ± a_k| at most), inverted: dividing by it makes each level's
/// estimate change no faster than the point moves.
const ROOMS_INV_LIP: [f32; 3] = [1.414_213_6, 0.831_253_9, 0.593_219_5];

/// The shape; the room's size in the sum's units; its slope's inverse; the
/// taper; and the perpendicular offset `shift` seen by the six lattice axes,
/// (γ · a*_i), the conjugate axes a*_i = (±B, −A, 0) cyclically.
fn rooms_prep(k: &mut [f32; K]) {
    let shape = (k[0] as usize).min(2);
    let (size, g, taper) = (k[1], [k[2] as f64, k[3] as f64, k[4] as f64], k[5]);
    let (a, b) = (ROOMS_A as f64, ROOMS_B as f64);
    let w = [
        b * g[0] - a * g[1],
        -b * g[0] - a * g[1],
        b * g[1] - a * g[2],
        -b * g[1] - a * g[2],
        -a * g[0] + b * g[2],
        a * g[0] + b * g[2],
    ];
    k[0] = shape as f32;
    k[1] = size * ROOMS_FULL[shape];
    k[2] = ROOMS_INV_LIP[shape];
    k[3] = taper;
    for i in 0..6 {
        k[4 + i] = w[i] as f32;
    }
    k[10] = 0.0;
    k[11] = 0.0;
}

/// The quasicrystal rooms: the distance estimate and the orbit trap (the
/// level of the nearest room, and which of the six axes is furthest out in
/// it, as a sixth within the level).
///
/// The point x is lifted to the 6D lattice's coordinates X_i = x · a_i + γ ·
/// a*_i, the rows of an orthogonal 6×6 matrix whose first three columns span
/// our space at the icosahedral angle (cut and project). With Y = X −
/// round(X) the offset from the nearest lattice point, a level's room is
/// where the largest |Y_i| (or the sum of the two or three largest) is under
/// the size: within one 6D cube, a dodecahedral cell of de Bruijn's
/// multigrid. The next level is X ← M Y, M the integer matrix of the golden
/// inflation τ³ (τ³ along our space, −τ⁻³ across it): the same as inflating
/// x by τ³ about the origin, but with every value bounded by 3.5 at any
/// depth. The solid is what no level's room takes; each level's estimate is
/// the room's sum less the size, over its slope in our space (exact on the
/// faces, conservative off them) and over τ³ per level, and the solid's is
/// the largest of them, as in the Menger sponge.
fn quasicrystal_rooms(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let shape = k[0] as u32;
    let (mut r, inv_l, taper) = (k[1], k[2], k[3]);
    let (a, b) = (s(ROOMS_A), s(ROOMS_B));
    let mut x = [
        a.mul_add(p.x, b.mul_add(p.y, s(k[4]))),
        (-a).mul_add(p.x, b.mul_add(p.y, s(k[5]))),
        a.mul_add(p.y, b.mul_add(p.z, s(k[6]))),
        (-a).mul_add(p.y, b.mul_add(p.z, s(k[7]))),
        b.mul_add(p.x, a.mul_add(p.z, s(k[8]))),
        (-b).mul_add(p.x, a.mul_add(p.z, s(k[9]))),
    ];
    let mut sc = 1.0f32;
    let mut d = s(-1e9);
    let mut trap = F8::zero();
    for level in 0..iters {
        let y: [F8; 6] = std::array::from_fn(|i| x[i] - x[i].round());
        let ab: [F8; 6] = std::array::from_fn(|i| y[i].abs());
        // the largest |y_i| and its axis, and the sum the shape takes
        let mut m1 = ab[0];
        let mut axis = F8::zero();
        let (mut m2, mut m3) = (F8::zero(), F8::zero());
        for (i, &v) in ab.iter().enumerate().skip(1) {
            let far: M8 = v.gt(m1);
            axis = F8::select(far, s(i as f32), axis);
            m3 = m3.max(m2.min(v));
            m2 = m2.max(m1.min(v));
            m1 = m1.max(v);
        }
        let sum = match shape {
            0 => m1,
            1 => m1 + m2,
            _ => m1 + m2 + m3,
        };
        // inside the room positive (empty space), in the walls negative
        let e = (s(r) - sum) * (inv_l / sc);
        let nearer = e.gt(d);
        trap = F8::select(nearer, axis.mul_add(s(0.125), s(level as f32)), trap);
        d = d.max(e);
        // the inflation, M = 2I + (the signs of the axes' angles)
        let [y0, y1, y2, y3, y4, y5] = y;
        x = [
            y0 + y0 + y1 + y2 - y3 + y4 - y5,
            y0 + y1 + y1 + y2 - y3 - y4 + y5,
            y0 + y1 + y2 + y2 + y3 + y4 + y5,
            -y0 - y1 + y2 + y3 + y3 + y4 + y5,
            y0 - y1 + y2 + y3 + y4 + y4 - y5,
            -y0 + y1 + y2 + y3 - y4 + y5 + y5,
        ];
        sc *= ROOMS_TAU3;
        r *= taper;
    }
    (d, (trap * 0.25).saturate())
}

const QUASICRYSTAL_ROOMS: &str = r#"
const quasicrystal_rooms_a: f32 = 0.37174803;
const quasicrystal_rooms_b: f32 = 0.60150096;

fn quasicrystal_rooms(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let shape = u32(a.x);
    var r = a.y;
    let inv_l = a.z;
    let taper = a.w;
    let qa = quasicrystal_rooms_a;
    let qb = quasicrystal_rooms_b;
    var xlo = vec3f(qa * p.x + (qb * p.y + b.x), -qa * p.x + (qb * p.y + b.y), qa * p.y + (qb * p.z + b.z));
    var xhi = vec3f(-qa * p.y + (qb * p.z + b.w), qb * p.x + (qa * p.z + c.x), -qb * p.x + (qa * p.z + c.y));
    var sc = 1.0;
    var d = -1e9;
    var trap = 0.0;
    for (var level = 0u; level < iters; level++) {
        let ylo = xlo - round(xlo);
        let yhi = xhi - round(xhi);
        let alo = abs(ylo);
        let ahi = abs(yhi);
        let ab = array<f32, 6>(alo.x, alo.y, alo.z, ahi.x, ahi.y, ahi.z);
        var m1 = ab[0];
        var axis = 0.0;
        var m2 = 0.0;
        var m3 = 0.0;
        for (var i = 1u; i < 6u; i++) {
            let v = ab[i];
            axis = select(axis, f32(i), v > m1);
            m3 = max(m3, min(m2, v));
            m2 = max(m2, min(m1, v));
            m1 = max(m1, v);
        }
        var sum = m1;
        if (shape == 1u) {
            sum = m1 + m2;
        } else if (shape >= 2u) {
            sum = m1 + m2 + m3;
        }
        let e = (r - sum) * (inv_l / sc);
        trap = select(trap, axis * 0.125 + f32(level), e > d);
        d = max(d, e);
        let y0 = ylo.x;
        let y1 = ylo.y;
        let y2 = ylo.z;
        let y3 = yhi.x;
        let y4 = yhi.y;
        let y5 = yhi.z;
        xlo = vec3f(y0 + y0 + y1 + y2 - y3 + y4 - y5, y0 + y1 + y1 + y2 - y3 - y4 + y5, y0 + y1 + y2 + y2 + y3 + y4 + y5);
        xhi = vec3f(-y0 - y1 + y2 + y3 + y3 + y4 + y5, y0 - y1 + y2 + y3 + y4 + y4 - y5, -y0 + y1 + y2 + y3 - y4 + y5 + y5);
        sc *= 4.236068;
        r *= taper;
    }
    return vec2f(d, clamp(trap * 0.25, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// The lift is an isometry of our space into the lattice's (the six
    /// axes are orthonormal in 6D), and M is the τ³ inflation: integer, τ³
    /// on our space and −τ⁻³ across it.
    #[test]
    fn the_lift_and_the_inflation() {
        let (a, b) = (ROOMS_A as f64, ROOMS_B as f64);
        let par = [[a, b, 0.0], [-a, b, 0.0], [0.0, a, b], [0.0, -a, b], [b, 0.0, a], [-b, 0.0, a]];
        let perp = [[b, -a, 0.0], [-b, -a, 0.0], [0.0, b, -a], [0.0, -b, -a], [-a, 0.0, b], [a, 0.0, b]];
        let dot = |u: [f64; 3], v: [f64; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        for i in 0..6 {
            for j in 0..6 {
                let g = dot(par[i], par[j]) + dot(perp[i], perp[j]);
                assert!((g - if i == j { 1.0 } else { 0.0 }).abs() < 1e-6, "axes {i}, {j}: {g}");
            }
        }
        // M_ij = τ³ a_i·a_j − τ⁻³ a*_i·a*_j
        let t3 = ROOMS_TAU3 as f64;
        let m: [[f64; 6]; 6] = std::array::from_fn(|i| std::array::from_fn(|j| t3 * dot(par[i], par[j]) - dot(perp[i], perp[j]) / t3));
        let sign = [[2, 1, 1, -1, 1, -1], [1, 2, 1, -1, -1, 1], [1, 1, 2, 1, 1, 1], [-1, -1, 1, 2, 1, 1], [1, -1, 1, 1, 2, -1], [-1, 1, 1, 1, -1, 2]];
        for i in 0..6 {
            for j in 0..6 {
                assert!((m[i][j] - sign[i][j] as f64).abs() < 1e-5, "M[{i}][{j}] = {}", m[i][j]);
            }
        }
    }

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults), close up too, and with the most levels. The
    /// estimate is 1-Lipschitz, so it may change no faster than the point
    /// moves, less a margin for rounding (mod.rs allows 1.25).
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[(0, 0.0), (1, 0.3)],
            &[(0, 0.0), (1, 1.0)],
            &[(0, 1.0), (1, 0.3)],
            &[(0, 1.0), (1, 1.0)],
            &[(0, 2.0), (1, 0.3)],
            &[(0, 2.0), (1, 1.0)],
            &[(0, 0.0), (5, 0.7)],
            &[(0, 2.0), (5, 1.15)],
            &[(2, 1.0), (3, -1.0), (4, 0.5)],
            &[(0, 1.0), (2, -1.0), (3, -1.0), (4, -1.0), (5, 1.15)],
        ];
        let mut seed = 0x2545_f491u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let id = find("quasicrystal_rooms").unwrap();
        let f = super::super::get(id);
        for &set in corners {
            for iters in [f.iterations.0, f.iterations.1] {
                let mut values = f.defaults();
                for &(i, v) in set {
                    values[i] = v;
                }
                let node = FormulaNode { id, values, iterations: iters };
                let k = node.pack();
                for reach in [3.0f32, 0.3, 0.03] {
                    let e = 1e-3 * reach;
                    let mut slopes = Vec::new();
                    for _ in 0..1000 {
                        let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                        let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                        let (d, t) = (f.cpu)(p, &k, node.iters());
                        let (dq, _) = (f.cpu)(V8::new(p.x, p.y + e, p.z), &k, node.iters());
                        for i in 0..8 {
                            let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                            assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                            if a > 10.0 * e && b.is_finite() {
                                slopes.push((b - a).abs() / e * f.step);
                            }
                        }
                    }
                    slopes.sort_by(f32::total_cmp);
                    let steep = slopes.last().copied().unwrap_or(0.0);
                    assert!(steep < 1.05, "{set:?}, {iters} levels, within {reach}: changes {steep:.4} times as fast as the point");
                }
            }
        }
    }
}
