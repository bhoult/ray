//! NURBS surfaces, traced exactly without a mesh. A surface is cut at its
//! knots into rational Bézier patches, raised to bicubic (one evaluator
//! serves every degree), and each patch is split until its pieces are nearly
//! flat. A rational patch with positive weights stays inside the hull of its
//! control points, so the pieces' control-point boxes bound them; those boxes
//! feed a small BVH, and a ray that reaches a piece solves for its hit by
//! Newton's method on (u, v) from the piece's centre (render/nurbs.rs, and
//! lib.wgsl's twin). Everything stays in the object's own frame: rays come
//! in through its transform, as they do for boxes.

use super::json::ShapeDef;
use super::{Bvh, BvhNode};
use crate::math::{Aabb, Vec3};

/// The highest degree a surface may have, in either direction.
pub const MAX_DEGREE: usize = 3;
/// Pieces per surface at most.
pub const MAX_PIECES: usize = 1 << 15;
/// A piece is flat enough when no control point strays from the straight
/// lines through its corners by more than this fraction of its size.
const FLAT: f64 = 0.02;
/// Splits per patch at most (u and v together).
const MAX_DEPTH: u32 = 12;

/// A bicubic rational Bézier patch: homogeneous control points (x w, y w,
/// z w, w), index 4 i + j with i along u.
pub type Patch = [[f32; 4]; 16];

/// A part of a patch: its square of (u, v), within the patch's 0..1, and
/// the box around its control points.
#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub patch: u32,
    pub u0: f32,
    pub u1: f32,
    pub v0: f32,
    pub v1: f32,
    pub aabb: Aabb,
}

pub struct Nurbs {
    pub patches: Vec<Patch>,
    /// in the BVH's leaf order: a leaf's `index` and `count` are a run of these
    pub pieces: Vec<Piece>,
    pub nodes: Vec<BvhNode>,
    pub aabb: Aabb,
    /// closed: it has an inside (signed distance, refraction)
    pub solid: bool,
    /// 1, or -1 when Su x Sv points inward (normals are turned outward)
    pub orient: f32,
    /// the bounding box's diagonal (tolerances scale with it)
    pub size: f32,
    /// its signed distance on a coarse grid, built the first time it is a
    /// leaf of a distance program (render/nurbs.rs grid)
    pub grid: std::sync::OnceLock<Grid>,
}

/// A surface's signed distance sampled on an n x n x n grid over its box
/// (and a margin): `values[(z * n + y) * n + x]` at lo + cell * (x, y, z).
/// Trilinear between samples it is within `slack` (half the cell's diagonal,
/// and a little) of the true distance.
pub struct Grid {
    pub n: usize,
    pub lo: Vec3,
    pub cell: Vec3,
    pub slack: f32,
    pub values: Vec<f32>,
}

type H = [f64; 4];
type Net = [H; 16];

fn mix(a: H, b: H, t: f64) -> H {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t, a[3] + (b[3] - a[3]) * t]
}

fn point(h: H) -> [f64; 3] {
    [h[0] / h[3], h[1] / h[3], h[2] / h[3]]
}

fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Clamped knots spaced evenly: `n` control points of degree `p`.
pub fn uniform_knots(n: usize, p: usize) -> Vec<f64> {
    let spans = n - p;
    (0..n + p + 1).map(|i| (i.saturating_sub(p).min(spans)) as f64 / spans as f64).collect()
}

fn check_knots(k: &[f64], n: usize, p: usize, dir: &str) -> Result<(), String> {
    if k.len() != n + p + 1 {
        return Err(format!("knots_{dir} needs {} values ({n} points of degree {p}, plus degree + 1), not {}", n + p + 1, k.len()));
    }
    if k.iter().any(|v| !v.is_finite()) || k.windows(2).any(|w| w[1] < w[0]) {
        return Err(format!("knots_{dir} must be numbers that never decrease"));
    }
    if k[p] >= k[n] {
        return Err(format!("knots_{dir} span nothing"));
    }
    if k[..=p].iter().any(|&v| v != k[0]) || k[n..].iter().any(|&v| v != k[n + p]) || k[p + 1] == k[p] || k[n - 1] == k[n] {
        return Err(format!("knots_{dir} must be clamped: the first and last values repeated exactly degree + 1 ({}) times", p + 1));
    }
    let mut run = 1;
    for i in p + 1..n {
        run = if k[i] == k[i - 1] { run + 1 } else { 1 };
        if run > p {
            return Err(format!("knots_{dir}: an inside knot repeats more than degree ({p}) times, which breaks the surface"));
        }
    }
    Ok(())
}

/// The Bézier segments of a NURBS curve (homogeneous points), by knot
/// insertion (The NURBS Book, algorithm A5.6).
fn decompose(p: usize, u: &[f64], pts: &[H]) -> Vec<Vec<H>> {
    let m = u.len() - 1;
    let (mut a, mut b) = (p, p + 1);
    let mut segs = vec![pts[..=p].to_vec()];
    while b < m {
        let i = b;
        while b < m && u[b + 1] == u[b] {
            b += 1;
        }
        let mult = b - i + 1;
        let mut next = vec![[0.0; 4]; p + 1];
        if mult < p {
            let numer = u[b] - u[a];
            let mut alphas = [0.0; MAX_DEGREE];
            for j in (mult + 1..=p).rev() {
                alphas[j - mult - 1] = numer / (u[a + j] - u[a]);
            }
            let r = p - mult;
            let cur = segs.last_mut().unwrap();
            for j in 1..=r {
                let s = mult + j;
                for k in (s..=p).rev() {
                    cur[k] = mix(cur[k - 1], cur[k], alphas[k - s]);
                }
                if b < m {
                    next[r - j] = cur[p];
                }
            }
        }
        if b < m {
            for k in p - mult..=p {
                next[k] = pts[b - p + k];
            }
            segs.push(next);
            a = b;
            b += 1;
        }
    }
    segs
}

/// A Bézier curve raised to cubic.
fn to_cubic(mut c: Vec<H>) -> [H; 4] {
    while c.len() < 4 {
        let n = c.len();
        c = (0..=n).map(|i| if i == 0 { c[0] } else if i == n { c[n - 1] } else { mix(c[i], c[i - 1], i as f64 / n as f64) }).collect();
    }
    [c[0], c[1], c[2], c[3]]
}

/// The bicubic patches of a NURBS surface: `net[i][j]` is (x, y, z, weight),
/// i along u.
fn patches(p: usize, q: usize, net: &[Vec<H>], ku: &[f64], kv: &[f64]) -> Vec<Net> {
    let (nu, nv) = (net.len(), net[0].len());
    let h: Vec<Vec<H>> = net.iter().map(|row| row.iter().map(|c| [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]]).collect()).collect();
    // along u for every j, then along v for every point of each u segment
    let by_j: Vec<Vec<Vec<H>>> = (0..nv).map(|j| decompose(p, ku, &(0..nu).map(|i| h[i][j]).collect::<Vec<_>>())).collect();
    let mut out = Vec::new();
    for su in 0..by_j[0].len() {
        let cols: Vec<Vec<Vec<H>>> = (0..=p).map(|k| decompose(q, kv, &(0..nv).map(|j| by_j[j][su][k]).collect::<Vec<_>>())).collect();
        for sv in 0..cols[0].len() {
            // cubic along v for each k, then along u for each of the four
            let rows: Vec<[H; 4]> = (0..=p).map(|k| to_cubic(cols[k][sv].clone())).collect();
            let mut n = [[0.0; 4]; 16];
            for j in 0..4 {
                let c = to_cubic((0..=p).map(|k| rows[k][j]).collect());
                for i in 0..4 {
                    n[i * 4 + j] = c[i];
                }
            }
            out.push(n);
        }
    }
    out
}

/// De Casteljau halves of a cubic.
fn halve(c: [H; 4]) -> ([H; 4], [H; 4]) {
    let p01 = mix(c[0], c[1], 0.5);
    let p12 = mix(c[1], c[2], 0.5);
    let p23 = mix(c[2], c[3], 0.5);
    let p012 = mix(p01, p12, 0.5);
    let p123 = mix(p12, p23, 0.5);
    let mid = mix(p012, p123, 0.5);
    ([c[0], p01, p012, mid], [mid, p123, p23, c[3]])
}

fn split(n: &Net, along_u: bool) -> (Net, Net) {
    let (mut a, mut b) = ([[0.0; 4]; 16], [[0.0; 4]; 16]);
    for k in 0..4 {
        let at = |i: usize| if along_u { i * 4 + k } else { k * 4 + i };
        let (l, r) = halve([n[at(0)], n[at(1)], n[at(2)], n[at(3)]]);
        for i in 0..4 {
            a[at(i)] = l[i];
            b[at(i)] = r[i];
        }
    }
    (a, b)
}

fn net_box(n: &Net) -> Aabb {
    n.iter().fold(Aabb::EMPTY, |b, &h| {
        let p = point(h);
        let v = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
        b.union(Aabb::new(v, v))
    })
}

/// How far the control points stray from the straight lines through the
/// ends of their rows along u and along v.
fn bend(n: &Net) -> (f64, f64) {
    let (mut du, mut dv) = (0.0f64, 0.0f64);
    for k in 0..4 {
        for (along_u, d) in [(true, &mut du), (false, &mut dv)] {
            let at = |i: usize| point(if along_u { n[i * 4 + k] } else { n[k * 4 + i] });
            let (a, b) = (at(0), at(3));
            for i in 1..3 {
                let t = i as f64 / 3.0;
                let lin = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
                *d = d.max(dist3(at(i), lin));
            }
        }
    }
    (du, dv)
}

fn subdivide(patch: u32, n: &Net, (u0, u1, v0, v1): (f64, f64, f64, f64), depth: u32, out: &mut Vec<Piece>) -> Result<(), String> {
    let bx = net_box(n);
    let size = bx.size().length() as f64;
    let (du, dv) = bend(n);
    if depth >= MAX_DEPTH || du.max(dv) <= FLAT * size {
        if out.len() >= MAX_PIECES {
            return Err(format!("the surface needs more than {MAX_PIECES} pieces; give it fewer control points"));
        }
        out.push(Piece { patch, u0: u0 as f32, u1: u1 as f32, v0: v0 as f32, v1: v1 as f32, aabb: bx });
        return Ok(());
    }
    let along_u = du >= dv;
    let (a, b) = split(n, along_u);
    if along_u {
        let m = (u0 + u1) * 0.5;
        subdivide(patch, &a, (u0, m, v0, v1), depth + 1, out)?;
        subdivide(patch, &b, (m, u1, v0, v1), depth + 1, out)
    } else {
        let m = (v0 + v1) * 0.5;
        subdivide(patch, &a, (u0, u1, v0, m), depth + 1, out)?;
        subdivide(patch, &b, (u0, u1, m, v1), depth + 1, out)
    }
}

/// The point of a patch at (u, v) and its tangents along u and v.
pub fn eval(p: &Patch, u: f32, v: f32) -> (Vec3, Vec3, Vec3) {
    let bern = |t: f32| {
        let s = 1.0 - t;
        ([s * s * s, 3.0 * t * s * s, 3.0 * t * t * s, t * t * t], [-3.0 * s * s, 3.0 * s * s - 6.0 * t * s, 6.0 * t * s - 3.0 * t * t, 3.0 * t * t])
    };
    let ((bu, du), (bv, dv)) = (bern(u), bern(v));
    let (mut h, mut hu, mut hv) = ([0.0f32; 4], [0.0f32; 4], [0.0f32; 4]);
    for i in 0..4 {
        for j in 0..4 {
            let c = p[i * 4 + j];
            for k in 0..4 {
                h[k] += bu[i] * bv[j] * c[k];
                hu[k] += du[i] * bv[j] * c[k];
                hv[k] += bu[i] * dv[j] * c[k];
            }
        }
    }
    let s = Vec3::new(h[0], h[1], h[2]) / h[3];
    let su = (Vec3::new(hu[0], hu[1], hu[2]) - s * hu[3]) / h[3];
    let sv = (Vec3::new(hv[0], hv[1], hv[2]) - s * hv[3]) / h[3];
    (s, su, sv)
}

impl std::fmt::Debug for Nurbs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Nurbs({} patches, {} pieces, solid {})", self.patches.len(), self.pieces.len(), self.solid)
    }
}

impl Nurbs {
    /// A surface of degree `p` along u and `q` along v through the control
    /// net `net[i][j]` = (x, y, z, weight), with knots (default: clamped
    /// and even).
    pub fn new(p: usize, q: usize, net: &[Vec<H>], ku: Option<&[f64]>, kv: Option<&[f64]>, solid: bool) -> Result<Nurbs, String> {
        for (d, dir) in [(p, "u"), (q, "v")] {
            if !(1..=MAX_DEGREE).contains(&d) {
                return Err(format!("degree along {dir} must be 1, 2 or 3, not {d}"));
            }
        }
        let nu = net.len();
        let nv = net.first().map_or(0, |r| r.len());
        if nu < p + 1 || nv < q + 1 {
            return Err(format!("a surface of degree {p} x {q} needs at least {} x {} control points, not {nu} x {nv}", p + 1, q + 1));
        }
        if net.iter().any(|r| r.len() != nv) {
            return Err("every row of points needs the same number of points".into());
        }
        if net.iter().flatten().any(|c| c.iter().any(|v| !v.is_finite())) {
            return Err("control points must be finite numbers".into());
        }
        if net.iter().flatten().any(|c| c[3] <= 0.0) {
            return Err("weights must be above 0".into());
        }
        let ku = ku.map(|k| k.to_vec()).unwrap_or_else(|| uniform_knots(nu, p));
        let kv = kv.map(|k| k.to_vec()).unwrap_or_else(|| uniform_knots(nv, q));
        check_knots(&ku, nu, p, "u")?;
        check_knots(&kv, nv, q, "v")?;

        let nets = patches(p, q, net, &ku, &kv);
        let mut found = Vec::new();
        for (i, n) in nets.iter().enumerate() {
            subdivide(i as u32, n, (0.0, 1.0, 0.0, 1.0), 0, &mut found)?;
        }
        let boxes: Vec<Aabb> = found.iter().map(|f| f.aabb).collect();
        let bvh = Bvh::build(&boxes, (0..found.len() as u32).collect());
        let pieces = bvh.items.iter().map(|&i| found[i as usize]).collect();
        let patches: Vec<Patch> = nets.iter().map(|n| n.map(|h| h.map(|v| v as f32))).collect();
        let aabb = boxes.iter().fold(Aabb::EMPTY, |a, &b| a.union(b));
        // turn normals outward: the divergence theorem's volume is positive then
        let mut volume = 0.0f64;
        for p in &patches {
            const N: usize = 8;
            for a in 0..N {
                for b in 0..N {
                    let (s, su, sv) = eval(p, (a as f32 + 0.5) / N as f32, (b as f32 + 0.5) / N as f32);
                    volume += s.dot(su.cross(sv)) as f64;
                }
            }
        }
        Ok(Nurbs {
            patches,
            pieces,
            nodes: bvh.nodes,
            aabb,
            solid,
            orient: if volume < 0.0 { -1.0 } else { 1.0 },
            size: aabb.size().length().max(1e-6),
            grid: std::sync::OnceLock::new(),
        })
    }

    /// The surface for a `nurbs` or `revolve` shape, in its own frame.
    pub fn from_shape(shape: &ShapeDef) -> Result<Option<Nurbs>, String> {
        let f64s = |k: &Option<Vec<f32>>| k.as_ref().map(|k| k.iter().map(|&v| v as f64).collect::<Vec<f64>>());
        match shape {
            ShapeDef::Nurbs { degree, points, knots_u, knots_v, solid } => {
                let net = points
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| match c[..] {
                                [x, y, z] => Ok([x as f64, y as f64, z as f64, 1.0]),
                                [x, y, z, w] => Ok([x as f64, y as f64, z as f64, w as f64]),
                                _ => Err("each control point is [x, y, z] or [x, y, z, weight]".to_string()),
                            })
                            .collect::<Result<Vec<H>, String>>()
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let (ku, kv) = (f64s(knots_u), f64s(knots_v));
                Nurbs::new(degree[0] as usize, degree[1] as usize, &net, ku.as_deref(), kv.as_deref(), *solid).map(Some)
            }
            ShapeDef::Revolve { profile, degree, knots, angle, solid } => {
                let prof = profile
                    .iter()
                    .map(|c| match c[..] {
                        [r, y] => Ok([r as f64, y as f64, 1.0]),
                        [r, y, w] => Ok([r as f64, y as f64, w as f64]),
                        _ => Err("each profile point is [radius, height] or [radius, height, weight]".to_string()),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let (net, ku) = revolve_net(&prof, *angle as f64)?;
                let kv = f64s(knots);
                let full = *angle >= 359.999;
                // closed: both ends on the axis, or the profile a loop
                let scale = prof.iter().fold(1e-9f64, |m, p| m.max(p[0].abs()).max(p[1].abs()));
                let (a, b) = (prof[0], prof[prof.len() - 1]);
                let on_axis = |p: [f64; 3]| p[0].abs() <= 1e-4 * scale;
                let looped = (a[0] - b[0]).abs() + (a[1] - b[1]).abs() <= 1e-4 * scale;
                let solid = solid.unwrap_or(full && (looped || (on_axis(a) && on_axis(b))));
                Nurbs::new(2, *degree as usize, &net, Some(&ku), kv.as_deref(), solid).map(Some)
            }
            _ => Ok(None),
        }
    }
}

/// A profile `[radius, height, weight]` turned `angle` degrees about the y
/// axis: the net (u around, v along the profile) and the u knots of the
/// exact rational circle (The NURBS Book, algorithm A7.1).
fn revolve_net(prof: &[[f64; 3]], angle: f64) -> Result<(Vec<Vec<H>>, Vec<f64>), String> {
    if prof.len() < 2 {
        return Err("a profile needs at least 2 points".into());
    }
    if !(angle > 0.0 && angle <= 360.0) {
        return Err("angle must be above 0 and at most 360 degrees".into());
    }
    let arcs = (angle / 90.0).ceil().clamp(1.0, 4.0) as usize;
    let step = angle.to_radians() / arcs as f64;
    let w1 = (step * 0.5).cos();
    // the circle: (cos, sin, weight) at the arcs' ends and their tangents' crossings
    let mut ring = vec![(1.0, 0.0, 1.0)];
    for k in 1..=arcs {
        let a = step * k as f64;
        let mid = a - step * 0.5;
        ring.push((mid.cos() / w1, mid.sin() / w1, w1));
        ring.push((a.cos(), a.sin(), 1.0));
    }
    let mut ku = vec![0.0; 3];
    for k in 1..arcs {
        let t = k as f64 / arcs as f64;
        ku.extend([t, t]);
    }
    ku.extend([1.0; 3]);
    let net = ring.iter().map(|&(c, s, w)| prof.iter().map(|&[r, y, pw]| [r * c, y, -r * s, w * pw]).collect()).collect();
    Ok((net, ku))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The point of a NURBS curve by de Boor (homogeneous), for checking.
    fn de_boor(p: usize, k: &[f64], pts: &[H], t: f64) -> [f64; 3] {
        let n = pts.len() - 1;
        let mut span = p;
        while span < n && t >= k[span + 1] {
            span += 1;
        }
        let mut d: Vec<H> = (0..=p).map(|j| pts[span - p + j]).collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let a = (t - k[i]) / (k[i + p + 1 - r] - k[i]);
                d[j] = mix(d[j - 1], d[j], a);
            }
        }
        point(d[p])
    }

    #[test]
    fn decomposed_curve_matches_de_boor() {
        let pts: Vec<H> = [[0.0, 0.0, 0.0, 1.0], [1.0, 2.0, 0.0, 2.0], [3.0, 2.0, 1.0, 0.5], [4.0, 0.0, 0.0, 1.0], [5.0, 1.0, 2.0, 1.5], [6.0, 0.0, 0.0, 1.0]]
            .iter()
            .map(|c| [c[0] * c[3], c[1] * c[3], c[2] * c[3], c[3]])
            .collect();
        for p in 1..=3 {
            let k = uniform_knots(pts.len(), p);
            let segs = decompose(p, &k, &pts);
            assert_eq!(segs.len(), pts.len() - p);
            for (s, seg) in segs.iter().enumerate() {
                let c = to_cubic(seg.clone());
                for step in 0..=4 {
                    let t = step as f64 / 4.0;
                    // the Bézier segment by de Casteljau
                    let mut b = c.to_vec();
                    for r in 1..4 {
                        for i in 0..4 - r {
                            b[i] = mix(b[i], b[i + 1], t);
                        }
                    }
                    let (a0, a1) = (k[p + s], k[p + s + 1]);
                    let want = de_boor(p, &k, &pts, a0 + (a1 - a0) * t);
                    assert!(dist3(point(b[0]), want) < 1e-9, "degree {p} segment {s} t {t}");
                }
            }
        }
    }

    #[test]
    fn revolved_circle_is_round() {
        // a sphere of radius 2: a half circle of 2 quarter arcs revolved
        let h = std::f64::consts::FRAC_1_SQRT_2;
        let prof = [[0.0, -2.0, 1.0], [2.0, -2.0, h], [2.0, 0.0, 1.0], [2.0, 2.0, h], [0.0, 2.0, 1.0]];
        let (net, ku) = revolve_net(&prof, 360.0).unwrap();
        let kv = [0.0, 0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.0];
        let s = Nurbs::new(2, 2, &net, Some(&ku), Some(&kv), true).unwrap();
        assert_eq!(s.patches.len(), 8);
        for p in &s.patches {
            for a in 0..=6 {
                for b in 0..=6 {
                    let (q, su, sv) = eval(p, a as f32 / 6.0, b as f32 / 6.0);
                    assert!((q.length() - 2.0).abs() < 1e-5, "{q:?}");
                    let n = su.cross(sv) * s.orient;
                    if n.length() > 1e-4 {
                        assert!(n.normalize().dot(q.normalize()) > 0.999, "normal {n:?} at {q:?}");
                    }
                }
            }
        }
        assert!(s.solid);
        assert!((s.aabb.max.y - 2.0).abs() < 1e-5 && (s.aabb.min.x + 2.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_bad_input() {
        let net = vec![vec![[0.0, 0.0, 0.0, 1.0]; 4]; 4];
        assert!(Nurbs::new(4, 3, &net, None, None, false).is_err());
        assert!(Nurbs::new(3, 3, &net[..3], None, None, false).is_err());
        let mut bad = net.clone();
        bad[1][1][3] = 0.0;
        assert!(Nurbs::new(3, 3, &bad, None, None, false).is_err());
        assert!(Nurbs::new(3, 3, &net, Some(&[0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0]), None, false).is_err());
    }
}
