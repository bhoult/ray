//! Exact fractals built of cubes. Three cut a cube into 3×3×3 and keep some
//! of the 27, level after level, as the Menger sponge does (render/vm.rs
//! menger: one periodic fold covers every cell of a level, and the solid is
//! the cube minus each level's cut); the Jerusalem cube has two sizes of
//! cube, so it is an IFS that follows the nearer one.

use super::util::{len2, s, sd_box};
use super::{Formula, K, Param};
use crate::simd::{F8, V8};

const THIRD: f32 = 1.0 / 3.0;

/// The Jerusalem cube's ratio k = √2 - 1 (corner cubes; edge cubes are k²)
/// and its inverses, as the WGSL has them.
const JK: f32 = 0.41421356;
const JK2: f32 = 0.17157288;
const JI: f32 = 2.414_213_6;
const JI2: f32 = 5.828_427;

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "mosely",
        category: "Fractals: geometric",
        desc: "The Mosely snowflake (Jeannine Mosely's): a cube cut into 27 with the eight corner cubes taken away, again and again, leaving 19 of 27. Seen along a diagonal its outline is a six-pointed snowflake; the light variant also hollows out each centre.",
        params: &[Param::whole("light", 0.0, 0.0, 1.0, "0: the heavier snowflake, 19 of 27 cubes kept. 1: the lighter one, which also takes away each centre cube (18 kept): hollow inside, and from the second level on it can be seen through along its four long diagonals.")],
        iterations: (4, 12, "Levels of cutting as seen from afar (1 to 12): each takes the corners off cubes a third the size, at one fold per level. Close up more are added by themselves, as many as the pixels can show (up to 12)."),
        bound: 1.0,
        lod: Some((3.0, 12)),
        step: 1.0,
        example: r#"{"type":"mosely","iterations":4,"rotation":[45,35.26,30],"scale":0.6}"#,
        prep: None,
        cpu: mosely,
        wgsl: MOSELY,
        wgsl_fn: "mosely",
    },
    Formula {
        name: "vicsek",
        category: "Fractals: geometric",
        desc: "The Vicsek fractal in 3D (after Tamás Vicsek, 1983): of a cube's 27 parts only the centre and the six face cubes are kept, a 3D cross, and so on in every arm: a branching tree of crosses with no loops.",
        params: &[],
        iterations: (4, 12, "Levels of crosses as seen from afar (1 to 12): each makes the arms out of crosses a third the size, at one fold per level. Close up more are added by themselves, as many as the pixels can show (up to 12)."),
        bound: 1.0,
        lod: Some((3.0, 12)),
        step: 1.0,
        example: r#"{"type":"vicsek","iterations":4,"scale":0.8}"#,
        prep: None,
        cpu: vicsek,
        wgsl: VICSEK,
        wgsl_fn: "vicsek",
    },
    Formula {
        name: "cantor_dust",
        category: "Fractals: geometric",
        desc: "Cantor dust in 3D (Georg Cantor's middle-thirds set on every axis): a cube's eight corner cubes are kept and the rest thrown away, over and over, until it is a cloud of ever smaller cubes, none touching.",
        params: &[],
        iterations: (4, 12, "Levels of splitting as seen from afar (1 to 12): each cube becomes eight a third the size, at one fold per level. Close up more are added by themselves, as many as the pixels can show (up to 12)."),
        bound: 1.0,
        lod: Some((3.0, 12)),
        step: 1.0,
        example: r#"{"type":"cantor_dust","iterations":4,"rotation":[30,31,16],"scale":0.6}"#,
        prep: None,
        cpu: cantor_dust,
        wgsl: CANTOR_DUST,
        wgsl_fn: "cantor_dust",
    },
    Formula {
        name: "jerusalem",
        category: "Fractals: geometric",
        desc: "The Jerusalem cube (Eric Baird, 2011): a cube with a Greek cross drilled through every face, leaving eight corner cubes (ratio √2 - 1) and twelve smaller edge cubes (its square) between them, each drilled the same way. No grid of thirds: its proportions are irrational.",
        params: &[],
        iterations: (5, 14, "Levels of drilling as seen from afar (1 to 14): each drills crosses into cubes about 2.4 times smaller, one fold and two box distances per level. Close up more are added by themselves, as many as the pixels can show (up to 14)."),
        bound: 1.0,
        lod: Some((2.414_213_6, 14)),
        step: 1.0,
        example: r#"{"type":"jerusalem","iterations":5,"rotation":[30,31,16],"scale":0.6}"#,
        prep: None,
        cpu: jerusalem,
        wgsl: JERUSALEM,
        wgsl_fn: "jerusalem",
    },
];

/// The cube [-1, 1]³ cut level after level: at level i its 3^i cells a side
/// (centred on multiples of 2/3^i), `cut` takes a cell's |local coordinates|
/// (0 at its centre, 1 at its faces) and gives the signed distance, in cell
/// units, to what is cut from it (positive outside the cut). The solid is the
/// cube minus every cut, as menger; the trap is the level whose cut the
/// surface is a wall of (the box: 0), six levels to 1 (deeper ones close up
/// go past: most of what shows is the deepest levels' walls).
#[inline(always)]
fn cells(p: V8, iters: u32, cut: impl Fn(F8, F8, F8) -> F8) -> (F8, F8) {
    let mut d = sd_box(p, [1.0; 3]);
    let mut level = F8::zero();
    let mut sc = 1.0f32;
    for i in 0..iters.min(12) {
        let half_s = 0.5 * sc;
        let h = 1.0 / sc;
        sc *= 3.0;
        // 1 - |mod(c s, 2) - 1|: the triangle wave that is 0 at the cell centres
        let fold = |c: F8| {
            let a = c * half_s;
            let a = a - a.floor();
            F8::one() - (a + a - 1.0).abs()
        };
        let hole = -cut(fold(p.x), fold(p.y), fold(p.z)) * h;
        level = F8::select(hole.gt(d), s((i + 1) as f32), level);
        d = d.max(hole);
    }
    (d, (level * (1.0 / 6.0)).saturate())
}

/// The signed distance to a cell's eight corner cells (all of |u| > 1/3),
/// which meet the next cells' corners: cubes 4/3 across at the cell corners.
#[inline(always)]
fn corners(x: F8, y: F8, z: F8) -> F8 {
    let t = s(THIRD);
    V8::new(t - x, t - y, t - z).max_s(F8::zero()).length() - (x.min(y).min(z) - THIRD).max(F8::zero())
}

/// Cut: the corners, and with `light` the centre cell too.
fn mosely(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let light = k[0] > 0.5;
    cells(p, iters, |x, y, z| {
        let c = corners(x, y, z);
        if light { c.min(sd_box(V8::new(x, y, z), [THIRD; 3])) } else { c }
    })
}

/// Cut: wherever two or more of |u| pass 1/3 (the edge and corner cells),
/// prisms along the cell edges.
fn vicsek(p: V8, _k: &[f32; K], iters: u32) -> (F8, F8) {
    cells(p, iters, |x, y, z| {
        let hi = x.max(y).max(z);
        let mid = x.min(y).max(x.max(y).min(z));
        len2((s(THIRD) - hi).max(F8::zero()), (s(THIRD) - mid).max(F8::zero())) - (mid - THIRD).max(F8::zero())
    })
}

/// Cut: all but the corners, so the cut's distance is minus theirs (exact
/// both ways).
fn cantor_dust(p: V8, _k: &[f32; K], iters: u32) -> (F8, F8) {
    cells(p, iters, |x, y, z| -corners(x, y, z))
}

/// The Jerusalem cube as an IFS: folded into the sorted first octant
/// (x ≥ y ≥ z ≥ 0), where every other piece is a mirror image of the corner
/// cube or the edge cube along z and no nearer, it goes into the nearer of
/// those two and keeps the other's box distance as a bound: the estimate is
/// the least of boxes that hold the fractal, so it never overshoots (where
/// two pieces touch, the one not followed shows its box face through holes
/// in the other: deep inside, rarely seen). The trap is the level whose
/// cross-shaped hole the surface is a wall of, as cells'.
fn jerusalem(p: V8, _k: &[f32; K], iters: u32) -> (F8, F8) {
    let (c1, c2) = (1.0 - JK, 1.0 - JK2);
    let mut z = p;
    let mut u = F8::one();
    let mut d = s(1e9);
    let mut best = sd_box(p, [1.0; 3]);
    let mut level = F8::zero();
    for i in 0..iters.min(14) {
        let (x, y, w) = (z.x.abs(), z.y.abs(), z.z.abs());
        let (x, y) = (x.max(y), x.min(y));
        let (x, w) = (x.max(w), x.min(w));
        let (y, w) = (y.max(w), y.min(w));
        // the hole (positive inside): no corner (w < k²) and no edge cube (y < 1 - 2k²)
        let t = (s(JK2) - w).min(s(1.0 - 2.0 * JK2) - y) * u;
        level = F8::select(t.gt(best), s((i + 1) as f32), level);
        best = best.max(t);
        let corner = sd_box(V8::new(x - c1, y - c1, w - c1), [JK; 3]);
        let edge = sd_box(V8::new(x - c2, y - c2, w), [JK2; 3]);
        let m = corner.le(edge);
        d = d.min(F8::select(m, edge, corner) * u);
        z = V8::select(m, V8::new((x - c1) * JI, (y - c1) * JI, (w - c1) * JI), V8::new((x - c2) * JI2, (y - c2) * JI2, w * JI2));
        u *= F8::select(m, s(JK), s(JK2));
    }
    d = d.min(sd_box(z, [1.0; 3]) * u);
    (d, (level * (1.0 / 5.0)).saturate())
}

const MOSELY: &str = r#"
fn mosely_fold(c: f32, half_s: f32) -> f32 {
    let a = fract(c * half_s);
    return 1.0 - abs(a + a - 1.0);
}

fn mosely_corners(u: vec3f) -> f32 {
    let t = 1.0 / 3.0;
    return length(max(vec3f(t) - u, vec3f(0.0))) - max(min(u.x, min(u.y, u.z)) - t, 0.0);
}

fn mosely(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let light = a.x > 0.5;
    let q = abs(p) - vec3f(1.0);
    var d = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    var level = 0.0;
    var sc = 1.0;
    for (var i = 0u; i < min(iters, 12u); i++) {
        let half_s = 0.5 * sc;
        let h = 1.0 / sc;
        sc *= 3.0;
        let u = vec3f(mosely_fold(p.x, half_s), mosely_fold(p.y, half_s), mosely_fold(p.z, half_s));
        var cut = mosely_corners(u);
        if (light) {
            let m = u - vec3f(1.0 / 3.0);
            cut = min(cut, length(max(m, vec3f(0.0))) + min(max(m.x, max(m.y, m.z)), 0.0));
        }
        let hole = -cut * h;
        level = select(level, f32(i + 1u), hole > d);
        d = max(d, hole);
    }
    return vec2f(d, clamp(level * (1.0 / 6.0), 0.0, 1.0));
}
"#;

const VICSEK: &str = r#"
fn vicsek_fold(c: f32, half_s: f32) -> f32 {
    let a = fract(c * half_s);
    return 1.0 - abs(a + a - 1.0);
}

fn vicsek(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let t = 1.0 / 3.0;
    let q = abs(p) - vec3f(1.0);
    var d = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    var level = 0.0;
    var sc = 1.0;
    for (var i = 0u; i < min(iters, 12u); i++) {
        let half_s = 0.5 * sc;
        let h = 1.0 / sc;
        sc *= 3.0;
        let x = vicsek_fold(p.x, half_s);
        let y = vicsek_fold(p.y, half_s);
        let z = vicsek_fold(p.z, half_s);
        let hi = max(max(x, y), z);
        let mid = max(min(x, y), min(max(x, y), z));
        let cut = length(vec2f(max(t - hi, 0.0), max(t - mid, 0.0))) - max(mid - t, 0.0);
        let hole = -cut * h;
        level = select(level, f32(i + 1u), hole > d);
        d = max(d, hole);
    }
    return vec2f(d, clamp(level * (1.0 / 6.0), 0.0, 1.0));
}
"#;

const CANTOR_DUST: &str = r#"
fn cantor_dust_fold(c: f32, half_s: f32) -> f32 {
    let a = fract(c * half_s);
    return 1.0 - abs(a + a - 1.0);
}

fn cantor_dust(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let t = 1.0 / 3.0;
    let q = abs(p) - vec3f(1.0);
    var d = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    var level = 0.0;
    var sc = 1.0;
    for (var i = 0u; i < min(iters, 12u); i++) {
        let half_s = 0.5 * sc;
        let h = 1.0 / sc;
        sc *= 3.0;
        let u = vec3f(cantor_dust_fold(p.x, half_s), cantor_dust_fold(p.y, half_s), cantor_dust_fold(p.z, half_s));
        let corners = length(max(vec3f(t) - u, vec3f(0.0))) - max(min(u.x, min(u.y, u.z)) - t, 0.0);
        let hole = corners * h;
        level = select(level, f32(i + 1u), hole > d);
        d = max(d, hole);
    }
    return vec2f(d, clamp(level * (1.0 / 6.0), 0.0, 1.0));
}
"#;

const JERUSALEM: &str = r#"
fn jerusalem_box(q: vec3f, r: f32) -> f32 {
    let m = abs(q) - vec3f(r);
    return length(max(m, vec3f(0.0))) + min(max(m.x, max(m.y, m.z)), 0.0);
}

fn jerusalem(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let k = 0.41421356;
    let k2 = 0.17157288;
    let c1 = 1.0 - k;
    let c2 = 1.0 - k2;
    var z = p;
    var u = 1.0;
    var d = 1e9;
    var best = jerusalem_box(p, 1.0);
    var level = 0.0;
    for (var i = 0u; i < min(iters, 14u); i++) {
        var x = abs(z.x);
        var y = abs(z.y);
        var w = abs(z.z);
        let x0 = x;
        x = max(x0, y);
        y = min(x0, y);
        let x1 = x;
        x = max(x1, w);
        w = min(x1, w);
        let y1 = y;
        y = max(y1, w);
        w = min(y1, w);
        let t = min(k2 - w, (1.0 - 2.0 * k2) - y) * u;
        level = select(level, f32(i + 1u), t > best);
        best = max(best, t);
        let corner = jerusalem_box(vec3f(x - c1, y - c1, w - c1), k);
        let edge = jerusalem_box(vec3f(x - c2, y - c2, w), k2);
        if (corner <= edge) {
            d = min(d, edge * u);
            z = vec3f(x - c1, y - c1, w - c1) * 2.41421356;
            u *= k;
        } else {
            d = min(d, corner * u);
            z = vec3f((x - c2) * 5.82842712, (y - c2) * 5.82842712, w * 5.82842712);
            u *= k2;
        }
    }
    d = min(d, jerusalem_box(z, 1.0) * u);
    return vec2f(d, clamp(level * (1.0 / 5.0), 0.0, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    /// The pieces after `n` levels, (centre, half size), from the definitions: of the 3×3×3 cells (offsets each -1, 0 or 1) those `keep` takes by how many offsets are not 0; the Jerusalem cube's 8 corners and 12 edges.
    fn pieces(name: &str, n: u32) -> Vec<([f64; 3], f64)> {
        let mut v = vec![([0.0; 3], 1.0)];
        for _ in 0..n {
            let mut next = Vec::new();
            for (c, h) in v {
                if name == "jerusalem" {
                    let k = 2f64.sqrt() - 1.0;
                    for i in 0..8 {
                        let sg = [0, 1, 2].map(|a| if i >> a & 1 == 1 { 1.0 } else { -1.0 });
                        next.push(([0, 1, 2].map(|a| c[a] + sg[a] * (1.0 - k) * h), k * h));
                        for along in 0..3 {
                            if sg[along] > 0.0 {
                                next.push(([0, 1, 2].map(|a| if a == along { c[a] } else { c[a] + sg[a] * (1.0 - k * k) * h }), k * k * h));
                            }
                        }
                    }
                    continue;
                }
                for off in 0..27 {
                    let o = [off % 3, off / 3 % 3, off / 9].map(|x| x as f64 - 1.0);
                    let nz = o.iter().filter(|&&x| x != 0.0).count();
                    let keep = match name {
                        "mosely" => nz < 3,
                        "mosely_light" => nz > 0 && nz < 3,
                        "vicsek" => nz <= 1,
                        _ => nz == 3,
                    };
                    if keep {
                        next.push(([0, 1, 2].map(|a| c[a] + o[a] * 2.0 * h / 3.0), h / 3.0));
                    }
                }
            }
            v = next;
        }
        v
    }

    /// Each kernel against its definition, built piece by piece: inside the pieces it is negative, outside positive and never more than the true distance (so the march never passes through a cube), at random points and at points crowded about the corner and the faces.
    #[test]
    fn cubes_are_where_they_should_be() {
        for (name, ty, light, n) in [("mosely", "mosely", 0.0, 3), ("mosely_light", "mosely", 1.0, 3), ("vicsek", "vicsek", 0.0, 3), ("cantor_dust", "cantor_dust", 0.0, 3), ("jerusalem", "jerusalem", 0.0, 3)] {
            let f = super::super::get(super::super::find(ty).unwrap());
            let mut k = [0.0; K];
            k[0] = light;
            let ps = pieces(name, n);
            let truth = |p: [f64; 3]| {
                let mut out = f64::INFINITY;
                let mut inside = f64::INFINITY;
                for (c, h) in &ps {
                    let q = [0, 1, 2].map(|a| (p[a] - c[a]).abs() - h);
                    let o = q.iter().map(|x| x.max(0.0).powi(2)).sum::<f64>().sqrt();
                    out = out.min(o);
                    if o == 0.0 {
                        inside = inside.min(q[0].max(q[1]).max(q[2]));
                    }
                }
                if inside.is_finite() { inside } else { out }
            };
            let mut seed = 0x2545_f491u32;
            let mut rnd = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed >> 8) as f32 / (1u32 << 24) as f32
            };
            let (mut ins, mut outs, mut ratio) = (0, 0, 0.0);
            for b in 0..600 {
                let pts: [[f32; 8]; 3] = std::array::from_fn(|_| {
                    std::array::from_fn(|_| match b % 3 {
                        0 => rnd() * 2.4 - 1.2,
                        1 => 0.6 + rnd() * 0.5,
                        _ => rnd() * 2.0 - 1.0,
                    })
                });
                let mut pts = pts;
                if b % 3 == 2 {
                    // on a face, a little in or out
                    pts[2] = std::array::from_fn(|i| 1.0 + (pts[2][i] - 0.5) * 0.02);
                }
                let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                let (d, _) = (f.cpu)(p, &k, n);
                for i in 0..8 {
                    let q = [pts[0][i] as f64, pts[1][i] as f64, pts[2][i] as f64];
                    let (e, t) = (d.lane(i) as f64, truth(q));
                    if t < -1e-5 {
                        assert!(e < 0.0, "{name}: {e} at {q:?}, inside by {t}");
                        ins += 1;
                    } else if t > 1e-5 {
                        assert!(e > 0.0 && e <= t + 1e-5, "{name}: {e} at {q:?}, {t} from it");
                        outs += 1;
                        ratio += e / t;
                    }
                }
            }
            println!("{name}: {ins} inside, {outs} outside, estimate {:.2} of the distance on average", ratio / outs as f64);
        }
    }
}
