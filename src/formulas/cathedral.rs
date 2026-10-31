//! The Möbius cathedral: a Gothic nave whose side chapels each hold the
//! whole cathedral inverted in the chapel's circle. The inversion acts on
//! the floor plan and is lifted to space by scaling heights with its own
//! local scale, L(w, y) = (g(w), |g'(w)| y), a new fold: it keeps the floor
//! flat and the walls upright at every level, and since lifting respects
//! composition, a group of plan maps (here the nave's mirrors and the
//! chapel's inversion) becomes a group acting on space. The lift is not
//! conformal (it shears), and its exact Lipschitz bound is carried through
//! the folds.

use super::util::s;
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};

pub const FORMULAS: &[Formula] = &[CATHEDRAL];

const CATHEDRAL: Formula = Formula {
    name: "cathedral",
    category: "Fractals: originals",
    desc: "A Gothic cathedral (nave, aisles, a tribune gallery over them, rib and groin vaults, a tiled floor) whose side chapels each hold the whole cathedral turned inside out: the chapel is a circle in the plan, and inside it lies the cathedral inverted in that circle, its nave bent into a crescent and smaller, with chapels of its own, and so on inward. The plan's inversion is lifted to space with heights scaled by its own local scale, so every level keeps a flat floor and upright walls. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("bays", 7.0, 4.0, 16.0, "Arcade bays from one chapel to the next along the nave (4 to 16): a passage leaves the aisle for a chapel every this many bays, on both sides. More bays leave room for bigger chapels (a chapel's radius is held under half the spacing)."),
        Param::num("chapel", &[3.0], 1.5, 7.5, "The chapels' radius, in nave half widths (1.5 to 7.5, held under half the chapel spacing). Bigger chapels hold bigger, less curved cathedrals."),
        Param::num("passage", &[0.6], 0.0, 4.0, "The passage from the aisle's outer wall to the chapel (0 to 4): the farther away the chapel, the smaller the cathedral inside it, roughly (radius / distance)² the size."),
        Param::num("pointed", &[1.6], 1.0, 2.5, "The arches' shape, each arc's radius over the half span (1 to 2.5): 1 round (Romanesque) arches and barrel vaults, 2 the equilateral Gothic arch, 2.5 sharp lancets. Every arch and vault takes it."),
        Param::num("height", &[1.0], 0.6, 1.6, "The stories' height (0.6 to 1.6 times): the piers, the tribune and the clerestory stretch and the arches keep their shape. Taller naves lean more where they curve inside the chapels."),
    ],
    iterations: (6, 16, "Chapels followed inward: each level is the cathedral inverted into every chapel of the level before, about a third the size at the default chapel and passage (and smaller toward each chapel's center). Linear cost."),
    bound: f32::INFINITY,
    lod: None,
    step: 0.8,
    example: r#"{"type":"intersection","scale":0.125,"children":[{"type":"union","children":[{"type":"box","size":[19,1.05,7],"position":[0,0.275,0]},{"type":"box","size":[4.6,2.55,7],"position":[0,1.025,0]}]},{"type":"cathedral"}]}"#,
    prep: Some(cathedral_prep),
    cpu: cathedral,
    wgsl: CATHEDRAL_WGSL,
    wgsl_fn: "cathedral",
};

// the plan, in the shape's units (the nave's half width is 1)
const BAY: f32 = 1.0; // arcade bay
const T: f32 = 0.12; // arcade wall's half thickness
const SP: f32 = 0.37; // arcade arches' half span (piers 0.26 wide)
const WI: f32 = 1.0 - T; // the nave's wall face
const AC: f32 = 1.0 + T + AH; // the aisle's middle
const AH: f32 = 0.4; // the aisle's half width
const WO: f32 = 0.3; // outer wall
const TS: f32 = 0.16; // tribune openings' half span
const TZ: f32 = 0.21; // and their middles (two a bay)
const RIB: f32 = 0.045; // ribs' half width
const RD: f32 = 0.06; // ribs' depth
const SH: f32 = 0.07; // wall shafts' radius

/// Packs: period, chapel radius (at most what fits), the chapel center's x,
/// pointedness, height; the arches' rise per half span; the tribune's floor,
/// its vault's springing, the nave vault's springing.
fn cathedral_prep(k: &mut [f32; K]) {
    let period = k[0] * BAY;
    let rad = k[1].min(0.5 * period - 0.15);
    let (rho, h) = (k[3], k[4]);
    let rr = (2.0 * rho - 1.0).sqrt();
    let hg = 1.05 * h + AH * rr + 0.12;
    let ht = hg + 0.5 * h;
    let hn = ht + AH * rr + 0.3 * h;
    k[0] = period;
    k[1] = rad;
    k[2] = 1.0 + T + 2.0 * AH + WO + k[2] + rad;
    k[5] = rr;
    k[6] = hg;
    k[7] = ht;
    k[8] = hn;
}

/// Pointed arch region of half span `a` springing at v = 0, arcs of radius
/// rho·a centered (rho - 1)·a across: signed distance (negative inside).
#[inline(always)]
fn arch(u: F8, v: F8, a: f32, rho: f32) -> F8 {
    let u = u.abs();
    let off = (rho - 1.0) * a;
    let arc = (u + off).mul_add(u + off, v * v).sqrt() - rho * a;
    F8::select(v.lt(F8::zero()), u - a, arc)
}

/// A rib under a vault: the band |zp| < RIB within RD of the vault.
#[inline(always)]
fn rib(zp: F8, vault: F8) -> F8 {
    zp.abs().max(-(vault + RD)) - RIB
}

/// The cell's open space (negative inside) and whether the nearest surface
/// is a floor, x >= 0, z in [0, period / 2]. Each opening reaches well into
/// the spaces on either side of it, so that inside the open space some part
/// is always deep (a union's distance is the least of its parts', which
/// would otherwise vanish where two parts meet).
#[inline(always)]
fn cell(x: F8, y: F8, z: F8, k: &[f32; K]) -> (F8, M8) {
    let (rho, h, rr, hg, ht, hn, cx) = (k[3], k[4], k[5], k[6], k[7], k[8], k[2]);
    let (ha, hs) = (0.95 * h, 1.05 * h);
    // the arcade bay: z folded to [0, BAY / 2] about an arch's middle
    let zb = (z - (z * (1.0 / BAY)).round() * BAY).abs();
    let zp = zb - s(0.5 * BAY); // from the pier
    // nave: barrel with groin lunettes over each bay, transverse ribs, and
    // the shafts that carry them
    let vault = arch(x, y - hn, WI, rho);
    let nave = vault.min(arch(zb, y - hn, SP, rho).max(x - WI));
    let shaft = (x - WI).mul_add(x - WI, zp * zp).sqrt() - SH;
    let nave = nave.max(-rib(zp, vault).min(shaft.max(y - hn)));
    // blind lancets in the clerestory
    let hc = ht + AH * rr;
    let nave = nave.min(arch(zb, y - hc, 0.13, rho).max(x - (WI + 0.06)).max(s(hc - 0.25 * h) - y));
    // arcade openings through the wall between nave and aisle (reaching
    // into both)
    let arc = arch(zb, y - ha, SP, rho).max((x - 1.0).abs() - (T + AH));
    // aisle and tribune above it: groin vaulted, ribbed
    let ax = x - AC;
    let av = arch(ax, y - hs, AH, rho);
    let aisle = av.min(arch(zb, y - hs, SP, rho).max(ax.abs() - AH)).max(-rib(zp, av));
    let tv = arch(ax, y - ht, AH, rho);
    let trib = tv.min(arch(zb, y - ht, SP, rho).max(ax.abs() - AH)).max(-rib(zp, tv));
    // the tribune's openings onto the nave, two a bay
    let topen = arch(zb - TZ, y - (hg + 0.28 * h), TS, rho).max((x - 1.0).abs() - (T + 0.3));
    let upper = trib.min(topen);
    let trib = upper.max(s(hg) - y);
    // the passage to the chapel, on the axis z = 0 (from the aisle's middle)
    let pass = arch(z, y - ha, SP, rho).max(x - cx).max(s(AC) - x);
    let v = nave.min(arc).min(aisle).min(trib).min(pass);
    let floor = (-y).ge(v) | (s(hg) - y).ge(upper) & (y - hg).abs().lt(s(0.03));
    (v.max(-y), floor)
}

fn cathedral(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (period, rad, cx) = (k[0], k[1], k[2]);
    let r2 = rad * rad;
    let (mut x, mut y, mut z) = (p.x, p.y, p.z);
    let mut kk = F8::one();
    // y0 times the gradient of log|G'| (G the plan map so far), in the current frame
    let (mut ax, mut az) = (F8::zero(), F8::zero());
    let mut level = F8::zero();
    // how far the last inversion was from its rim (0 on it, 1 a third of the
    // radius in): a chapel ahead of the final point is then a new one
    let mut ahead = F8::one();
    let mut act: M8 = M8::all_true();
    for _ in 0..iters {
        ax = F8::select(x.lt(F8::zero()), -ax, ax);
        x = x.abs();
        z = z - (z * (1.0 / period)).round() * period;
        az = F8::select(z.lt(F8::zero()), -az, az);
        z = z.abs();
        let dx = x - cx;
        let q2 = dx.mul_add(dx, z * z).max(s(1e-8));
        act &= q2.lt(s(r2));
        if act.none() {
            break;
        }
        let m = s(r2) / q2;
        let q = q2.sqrt();
        let iq = q.recip();
        let (nx, nz) = (dx * iq, z * iq);
        // the shear: the composite's a = y0 grad log|G'|, carried in the current
        // frame (an inversion reflects it in the radius and adds 2 y / q along it)
        let f = (y * iq - nz.mul_add(az, nx * ax)) * 2.0;
        ax = F8::select(act, f.mul_add(nx, ax), ax);
        az = F8::select(act, f.mul_add(nz, az), az);
        ahead = F8::select(act, ((s(rad) - q) * (3.0 / rad)).saturate(), ahead);
        x = F8::select(act, dx.mul_add(m, s(cx)), x);
        z = F8::select(act, z * m, z);
        y = F8::select(act, y * m, y);
        kk = F8::select(act, kk * m, kk);
        level = F8::select(act, level + 1.0, level);
    }
    // the last fold; a point still in a chapel when the iterations run out
    // is taken as solid (the chapel as deep as it goes)
    x = x.abs();
    z = z - (z * (1.0 / period)).round() * period;
    z = z.abs();
    let dx = x - cx;
    let q = dx.mul_add(dx, z * z).sqrt();
    let (mut void, floor) = cell(x, y, z, k);
    void = F8::select(act & q.lt(s(rad)), void.max(s(rad) - q), void);
    // a chapel ahead: past its rim the inversion's shear, y / rad there, will
    // divide the estimate, so it is at most the way to the rim plus that
    // (continuous across the rim; not for the rim just come through, where
    // the inside's own shear already counts it)
    let df = -void;
    let sr = y.max(F8::zero()) * (1.0 / rad);
    let lose = F8::one() - (sr + sr.mul_add(sr, F8::one()).sqrt()).recip();
    let df = df.min((q - rad).max(F8::zero()) + df * ahead.neg_mul_add(lose, F8::one()));
    let sh = az.mul_add(az, ax * ax).sqrt() * 0.5;
    let shear = sh + (sh.mul_add(sh, F8::one())).sqrt();
    let d = df / (kk * shear);
    // trap: stone by height, the floor in tiles, each chapel deeper bluer
    let tile = ((x * 2.0).round() + (z * 2.0).round()) * 0.5;
    let tile = (tile - tile.floor()) * 2.0;
    let stone = (y * (1.0 / (k[8] + 1.0))).saturate() * 0.3;
    let base = F8::select(floor, tile.mul_add(s(0.3), s(0.55)), stone);
    (d, level.mul_add(s(0.12), base).saturate())
}

const CATHEDRAL_WGSL: &str = r#"
const cathedral_bay: f32 = 1.0;
const cathedral_t: f32 = 0.12;
const cathedral_sp: f32 = 0.37;
const cathedral_wi: f32 = 0.88;
const cathedral_ac: f32 = 1.52;
const cathedral_ah: f32 = 0.4;
const cathedral_ts: f32 = 0.16;
const cathedral_tz: f32 = 0.21;
const cathedral_rib: f32 = 0.045;
const cathedral_rd: f32 = 0.06;
const cathedral_sh: f32 = 0.07;

fn cathedral_arch(u0: f32, v: f32, a: f32, rho: f32) -> f32 {
    let u = abs(u0);
    let off = (rho - 1.0) * a;
    let arc = sqrt((u + off) * (u + off) + v * v) - rho * a;
    return select(arc, u - a, v < 0.0);
}

fn cathedral_rib_of(zp: f32, vault: f32) -> f32 {
    return max(abs(zp), -(vault + cathedral_rd)) - cathedral_rib;
}

// (open space, 1 if the nearest surface is a floor)
fn cathedral_cell(x: f32, y: f32, z: f32, a: vec4f, b: vec4f, c: vec4f) -> vec2f {
    let rho = a.w;
    let h = b.x;
    let rr = b.y;
    let hg = b.z;
    let ht = b.w;
    let hn = c.x;
    let cx = a.z;
    let ha = 0.95 * h;
    let hs = 1.05 * h;
    let zb = abs(z - round(z * (1.0 / cathedral_bay)) * cathedral_bay);
    let zp = zb - 0.5 * cathedral_bay;
    let vault = cathedral_arch(x, y - hn, cathedral_wi, rho);
    var nave = min(vault, max(cathedral_arch(zb, y - hn, cathedral_sp, rho), x - cathedral_wi));
    let shaft = sqrt((x - cathedral_wi) * (x - cathedral_wi) + zp * zp) - cathedral_sh;
    nave = max(nave, -min(cathedral_rib_of(zp, vault), max(shaft, y - hn)));
    let hc = ht + cathedral_ah * rr;
    nave = min(nave, max(max(cathedral_arch(zb, y - hc, 0.13, rho), x - (cathedral_wi + 0.06)), (hc - 0.25 * h) - y));
    let arc = max(cathedral_arch(zb, y - ha, cathedral_sp, rho), abs(x - 1.0) - (cathedral_t + cathedral_ah));
    let ax = x - cathedral_ac;
    let av = cathedral_arch(ax, y - hs, cathedral_ah, rho);
    let aisle = max(min(av, max(cathedral_arch(zb, y - hs, cathedral_sp, rho), abs(ax) - cathedral_ah)), -cathedral_rib_of(zp, av));
    let tv = cathedral_arch(ax, y - ht, cathedral_ah, rho);
    let trib = max(min(tv, max(cathedral_arch(zb, y - ht, cathedral_sp, rho), abs(ax) - cathedral_ah)), -cathedral_rib_of(zp, tv));
    let topen = max(cathedral_arch(zb - cathedral_tz, y - (hg + 0.28 * h), cathedral_ts, rho), abs(x - 1.0) - (cathedral_t + 0.3));
    let upper = min(trib, topen);
    let walk = max(max(cathedral_arch(z, y - ha, cathedral_sp, rho), x - cx), cathedral_ac - x);
    let v = min(min(min(nave, arc), min(aisle, max(upper, hg - y))), walk);
    let flr = -y >= v || (hg - y >= upper && abs(y - hg) < 0.03);
    return vec2f(max(v, -y), select(0.0, 1.0, flr));
}

fn cathedral(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let period = a.x;
    let rad = a.y;
    let cx = a.z;
    let r2 = rad * rad;
    var x = p.x;
    var y = p.y;
    var z = p.z;
    var kk = 1.0;
    var sv = vec2f(0.0);
    var level = 0.0;
    var ahead = 1.0;
    var inside = true;
    for (var i = 0u; i < iters; i++) {
        sv.x = select(sv.x, -sv.x, x < 0.0);
        x = abs(x);
        z = z - round(z * (1.0 / period)) * period;
        sv.y = select(sv.y, -sv.y, z < 0.0);
        z = abs(z);
        let dx = x - cx;
        let q2 = max(dx * dx + z * z, 1e-8);
        if (q2 >= r2) {
            inside = false;
            break;
        }
        let m = r2 / q2;
        let q = sqrt(q2);
        let iq = 1.0 / q;
        let n = vec2f(dx, z) * iq;
        let f = (y * iq - dot(n, sv)) * 2.0;
        sv = f * n + sv;
        ahead = clamp((rad - q) * (3.0 / rad), 0.0, 1.0);
        x = dx * m + cx;
        z = z * m;
        y = y * m;
        kk = kk * m;
        level = level + 1.0;
    }
    x = abs(x);
    z = abs(z - round(z * (1.0 / period)) * period);
    let dx = x - cx;
    let q = sqrt(dx * dx + z * z);
    let cl = cathedral_cell(x, y, z, a, b, c);
    var open = cl.x;
    if (inside && q < rad) {
        open = max(open, rad - q);
    }
    let sr = max(y, 0.0) * (1.0 / rad);
    let lose = 1.0 - 1.0 / (sr + sqrt(sr * sr + 1.0));
    let df = min(-open, max(q - rad, 0.0) + -open * (1.0 - ahead * lose));
    let sh = length(sv) * 0.5;
    let shear = sh + sqrt(sh * sh + 1.0);
    let d = df / (kk * shear);
    let tl = (round(x * 2.0) + round(z * 2.0)) * 0.5;
    let tile = (tl - floor(tl)) * 2.0;
    let stone = clamp(y * (1.0 / (c.x + 1.0)), 0.0, 1.0) * 0.3;
    let base = select(stone, tile * 0.3 + 0.55, cl.y > 0.5);
    return vec2f(d, clamp(level * 0.12 + base, 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, get, FormulaNode};

    /// mod.rs's steepness check, but over the nave, the aisles and a whole
    /// chapel (the default test's points reach only a sliver of one), at the
    /// defaults and the corners of the parameters: the estimate, times the
    /// step, changes at most 1.6 times as fast as the point moves, along each
    /// axis, and a hundredth of it no more than 1.25 times.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[(&[(usize, f32)], u32)] = &[
            (&[], 6),
            (&[], 16),
            (&[(0, 4.0), (1, 1.5), (2, 0.0)], 6),
            (&[(0, 16.0), (1, 7.5), (2, 4.0)], 6),
            (&[(1, 1.5), (3, 1.0), (4, 1.6)], 6),
            (&[(1, 7.5), (2, 0.0), (3, 2.5), (4, 1.6)], 16),
            (&[(0, 4.0), (1, 7.5), (3, 2.5), (4, 0.6)], 6),
            (&[(2, 4.0), (3, 1.0), (4, 0.6)], 6),
        ];
        let mut seed = 0x2545_f491u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let id = find("cathedral").unwrap();
        let f = get(id);
        for &(set, iters) in corners {
            let mut values = f.defaults();
            for &(i, v) in set {
                values[i] = v;
            }
            let node = FormulaNode { id, values, iterations: iters };
            let k = node.pack();
            let (period, rad, cx) = (k[0], k[1], k[2]);
            let top = k[8] + 2.0;
            let e = 1e-3;
            let mut slopes = Vec::new();
            for _ in 0..4000 {
                // the cell: across the nave to past the chapel, a bay's length, floor to roof
                let pts: [[f32; 8]; 3] = [
                    std::array::from_fn(|_| rnd() * (cx + rad + 0.5)),
                    std::array::from_fn(|_| rnd() * (top + 0.5) - 0.5),
                    std::array::from_fn(|_| rnd() * 0.5 * period),
                ];
                let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                let (d, t) = (f.cpu)(p, &k, node.iters());
                for axis in 0..3 {
                    let q = match axis {
                        0 => V8::new(p.x + e, p.y, p.z),
                        1 => V8::new(p.x, p.y + e, p.z),
                        _ => V8::new(p.x, p.y, p.z + e),
                    };
                    let (dq, _) = (f.cpu)(q, &k, node.iters());
                    for i in 0..8 {
                        let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                        assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                        if a > 10.0 * e && b.is_finite() {
                            let steep = (b - a).abs() / e * f.step;
                            assert!(steep < 1.6, "{set:?}: changes {steep:.2} times as fast as the point along axis {axis} at {:?} (d {a})", p.lane(i));
                            slopes.push(steep);
                        }
                    }
                }
            }
            slopes.sort_by(f32::total_cmp);
            let p99 = slopes[slopes.len() * 99 / 100];
            assert!(p99 < 1.25, "{set:?}: a hundredth changes {p99:.2} times as fast");
        }
    }
}
