//! A recursive space frame: Buckminster Fuller's octet truss, each of whose
//! struts carries a cage of the same truss several times finer (a girder
//! with the strut as its core), whose struts carry cages of their own, and
//! so on down. Made for this catalog.
//!
//! Every level is the same lattice: the face-centred cubic lattice with its
//! nearest-neighbour edges (the octet truss), each level `refine` times finer
//! than the last. With a whole refinement the coarse nodes are fine nodes and
//! every coarse strut is a row of fine struts, so the fine lattice sits on the
//! coarse one with no frame to choose, and the cages of struts that meet at a
//! node share that node's fine lattice: joints resolve themselves.

use super::{Formula, K, Param};
use crate::simd::{F8, V8};

pub const FORMULAS: &[Formula] = &[TRUSS];

/// A finer node is in a coarser strut's cage when it lies within this many
/// finer strut lengths of it: the nodes on the strut, and the four rows of
/// finer nodes around it at √3/2 of a finer length (the next rows are a
/// whole length out). The cage is the finer struts between them, less the
/// row along the strut itself (hidden in it).
const RHO: f32 = 0.95;
/// The least distance from a node's Voronoi cell (the rhombic dodecahedron)
/// to a strut not ending at that node: 1/√6 node units (√3/6 of a strut).
const VORONOI_GAP: f32 = 0.408_248_3;

const TRUSS: Formula = Formula {
    name: "octet_truss",
    category: "Fractals: originals",
    desc: "A space frame of space frames: Buckminster Fuller's octet truss, every strut of which is stiffened by a cage of the same truss several times finer, every strut of that cage by a finer cage, and so on down. The finer lattice is the coarse one refined a whole number of times, so the cages of struts that meet at a node share that node's lattice and join cleanly. Invented for ray by Claude (Anthropic), 2026.",
    params: &[
        Param::whole("cells", 2.0, 1.0, 4.0, "Cubic cells of the coarse truss along each side of the frame (1 to 4): 1 is an octahedron with a tetrahedron at each corner of the cube, 2 the block pictured, 4 a frame of 64 cells in the same space."),
        Param::whole("refine", 8.0, 8.0, 12.0, "How many times finer each level's truss is (8 to 12): each strut is that many bays of its cage long, and the cage is about 1.7 / refine of its length across, so 8 gives stout girders and 12 slender ones. (From 8 up, the coarser struts a strut's cage depends on are always at one node, which is what the estimate looks up.)"),
        Param::num("thickness", &[0.04], 0.01, 0.12, "Each strut's radius as a share of its length (0.01 to 0.12), at every level. Thin struts leave the cages airy; thick ones fill them in."),
    ],
    iterations: (2, 6, "Levels: 1 the plain octet truss, each one more a cage around every strut of the last, refine times finer (close up, more are added)."),
    bound: 50.5,
    lod: Some((9.0, 6)),
    step: 1.0,
    example: r#"{"type":"octet_truss","scale":0.017,"rotation":[-20,30,0]}"#,
    prep: Some(truss_prep),
    cpu: octet_truss,
    wgsl: OCTET_TRUSS,
    wgsl_fn: "octet_truss",
};

/// The frame's half size, cages and all, in the shape's units. The shape
/// is this large so that its struts are about as thick as the detail `lod`
/// expects of them: a level is added when the struts it adds become about a
/// pixel thick, not before (a thinner level would only haze the last).
const HALF: f32 = 50.0;

/// k: cells, refine, thickness, then node units per shape unit at level 0,
/// 1 / refine, a strut's radius and the reach of its cages (both in node
/// units of its own level), a cage's radius in its strut's node units, the
/// lattice's shift (odd cells put a node, not a hole, at the block's
/// corners) and the shape units per node unit at level 0.
fn truss_prep(k: &mut [f32; K]) {
    let (n, g, t) = (k[0], k[1], k[2]);
    let sq2 = std::f32::consts::SQRT_2;
    let rod = t * sq2;
    // a strut's family (its cages, theirs...) lies within RHO h (1 + 1/g +
    // ...) of its axis, plus the finest struts' radius
    let reach = rod.max(RHO * sq2 / (g - 1.0) + rod / g);
    // the node unit at level 0: the nodes' block and the cages around it
    // fill the frame
    let u0 = HALF / (n + reach);
    k[3] = 1.0 / u0;
    k[4] = 1.0 / g;
    k[5] = rod;
    k[6] = reach;
    k[7] = RHO * sq2 / g;
    k[8] = n % 2.0;
    k[9] = u0;
}

/// The strut direction j (0..12), in node units (two ±1 entries): j / 4
/// picks the pair of axes (xy, xz, yz), bit 1 the first one's sign, bit 0
/// the second's. j ^ 3 is the opposite direction.
#[inline(always)]
fn dir(j: u32) -> [f32; 3] {
    let s1 = if j & 2 != 0 { -1.0 } else { 1.0 };
    let s2 = if j & 1 != 0 { -1.0 } else { 1.0 };
    match j >> 2 {
        0 => [s1, s2, 0.0],
        1 => [s1, 0.0, s2],
        _ => [0.0, s1, s2],
    }
}

/// The nearest node of the face-centred cubic lattice (integer points of
/// even sum) to q. `bias` settles ties (midpoints of struts often sit
/// exactly between nodes) the same way on the CPU and the GPU.
#[inline(always)]
fn fcc(q: [f32; 3], bias: f32) -> [f32; 3] {
    let h = 0.5 + bias;
    let mut r = [(q[0] + h).floor(), (q[1] + h).floor(), (q[2] + h).floor()];
    let sum = r[0] + r[1] + r[2];
    if sum - 2.0 * (sum * 0.5).floor() > 0.5 {
        let e = [q[0] - r[0], q[1] - r[1], q[2] - r[2]];
        let a = [e[0].abs() + 3e-5, e[1].abs() + 2e-5, e[2].abs() + 1e-5];
        let i = if a[0] >= a[1] && a[0] >= a[2] {
            0
        } else if a[1] >= a[2] {
            1
        } else {
            2
        };
        r[i] += if e[i] >= 0.0 { 1.0 } else { -1.0 };
    }
    r
}

/// Of the twelve struts at a node, the one nearest in angle to the point w
/// (relative to the node): along the two axes w leans on most, with w's
/// signs.
#[inline(always)]
fn dir_of(w: [f32; 3]) -> u32 {
    let a = [w[0].abs() + 3e-5, w[1].abs() + 2e-5, w[2].abs() + 1e-5];
    let (pair, s, t) = if a[2] <= a[0] && a[2] <= a[1] {
        (0, w[0], w[1])
    } else if a[1] <= a[0] {
        (1, w[0], w[2])
    } else {
        (2, w[1], w[2])
    };
    pair * 4 + if s < 0.0 { 2 } else { 0 } + if t < 0.0 { 1 } else { 0 }
}

/// The flags of the struts at the coarser node cp (relative to the coarser
/// level's node): `prev` there; at a neighbour of it, only the strut back to
/// it is known; the others are taken as kept.
#[inline(always)]
fn flags_at(cp: [f32; 3], prev: u32) -> u32 {
    let ones = cp[0].abs() + cp[1].abs() + cp[2].abs();
    if ones == 0.0 {
        prev
    } else if ones == 2.0 && cp[0].abs().max(cp[1].abs()).max(cp[2].abs()) == 1.0 {
        let k = dir_of([-cp[0], -cp[1], -cp[2]]);
        (0xfff & !(1 << k)) | ((prev >> (k ^ 3) & 1) << k)
    } else {
        0xfff
    }
}

/// The four struts along ±a ±b, as bits (+,+), (+,-), (-,+), (-,-): which
/// of their lines pass within lim of the point ahead of the node (r2 its
/// squared distance from the node).
#[inline(always)]
fn geo4(a: f32, b: f32, r2: f32, lim2: f32) -> u32 {
    let (t0, t1) = (a + b, a - b);
    let (n0, n1) = (r2 - 0.5 * t0 * t0 <= lim2, r2 - 0.5 * t1 * t1 <= lim2);
    (t0 > 0.0 && n0) as u32 | ((t1 > 0.0 && n1) as u32) << 1 | ((t1 < 0.0 && n1) as u32) << 2 | ((t0 < 0.0 && n0) as u32) << 3
}

/// Which of the struts at a node pass within lim of the point e (relative
/// to the node), as bits.
#[inline(always)]
fn geo(e: [f32; 3], lim2: f32) -> u32 {
    let r2 = e[0] * e[0] + e[1] * e[1] + e[2] * e[2];
    if r2 <= lim2 {
        0xfff
    } else {
        geo4(e[0], e[1], r2, lim2) | geo4(e[0], e[2], r2, lim2) << 4 | geo4(e[1], e[2], r2, lim2) << 8
    }
}

/// A finer node x (in finer node units, about the coarser level's node
/// scaled up): whether it is within lim of a kept strut at the coarser node
/// nearest it, that node's flags, and x's offset from it (in coarser units).
#[inline(always)]
fn node_in(x: [f32; 3], invg: f32, lim2: f32, prev: u32) -> (bool, u32, [f32; 3]) {
    let y = [x[0] * invg, x[1] * invg, x[2] * invg];
    let cp = fcc(y, 1e-4);
    let e = [y[0] - cp[0], y[1] - cp[1], y[2] - cp[2]];
    let flags = flags_at(cp, prev);
    (geo(e, lim2) & flags != 0, flags, e)
}

/// One point: the distance estimate and the orbit trap (the level of the
/// nearest strut).
///
/// Level l's nodes are the integer points of even sum (odd for odd `cells`)
/// in node units of u0 / refine^l; a strut joins two at distance √2. Level 0
/// keeps the struts with both ends in the block. A node of level l + 1 is in
/// a cage when it lies within RHO of its strut length of a kept level-l
/// strut, and a strut of level l + 1 is kept when both its ends are in
/// cages and it does not run along a kept level-l strut (hidden in it). Each
/// level keeps the ones before: its cages surround their struts.
///
/// At each level, the point is put in that level's node units about its
/// nearest node, and the twelve struts at the node are tested. A node's
/// cage is looked up in the last level's flags, at that level's node or a
/// neighbour of it (the struts that can be within reach of the point); any
/// other strut there is taken as kept, which only lowers the estimate. The
/// struts not at the node are at least VORONOI_GAP beyond the node's Voronoi
/// cell, a bound that meets the exact distances on the cell's faces, so the
/// estimate is continuous across them. Capsule distances are exact.
///
/// Everything finer than a level lies within `reach` of its kept struts
/// (`fam` below). Well clear of that, the finer levels are skipped; nearer,
/// they are looked at, and their estimate is held between `fam` and 2m -
/// `fam` (m the margin), so the two meet where the finer levels start. Each
/// level's terms are composed as a clamp, X -> min(hi, max(lo, X)). A strut
/// whose family (or, at the last level, whose own capsule) is beyond `hi`
/// cannot change the result, so it is not tested and taken as kept.
fn truss1(p: [f32; 3], k: &[f32; K], iters: u32) -> (f32, f32) {
    let (n, inv_u0, invg, rod, reach, lim, shift, u0) = (k[0], k[3], k[4], k[5], k[6], k[7], k[8], k[9]);
    let g = k[1];
    let half = n * u0;
    let qb = [p[0].abs() - half, p[1].abs() - half, p[2].abs() - half];
    let outside = (qb[0].max(0.0).powi(2) + qb[1].max(0.0).powi(2) + qb[2].max(0.0).powi(2)).sqrt();
    let boxd = outside + qb[0].max(qb[1]).max(qb[2]).min(0.0) - reach * u0;
    if boxd > u0 {
        return (boxd, 0.0);
    }
    let mut v = [p[0] * inv_u0 + shift, p[1] * inv_u0, p[2] * inv_u0];
    let mut c = fcc(v, 0.0);
    v = [v[0] - c[0], v[1] - c[1], v[2] - c[2]];
    let mut u = u0;
    // (the levels above, as a clamp; the value the last one gives)
    let (mut lo, mut hi) = (-1e9f32, 1e9f32);
    let mut x = 1e9f32;
    let mut lvl = 0.0;
    let mut nearest = 1e9f32;
    let mut prev = 0u32;
    let edge = n + 0.5;
    let lim2 = lim * lim;
    for l in 0..iters {
        if l > 0 {
            let q = [v[0] * g, v[1] * g, v[2] * g];
            c = fcc(q, 0.0);
            v = [q[0] - c[0], q[1] - c[1], q[2] - c[2]];
            u *= invg;
        }
        let last = l + 1 == iters;
        let mut mask = 0u32;
        // (this node: in a cage of the coarser level?)
        let here = if l > 0 { node_in(c, invg, lim2, prev) } else { (true, 0, [0.0; 3]) };
        // (the point's distance from strut j is √(|v|² - t²/2) for t = v·dⱼ
        // > 0, |v| otherwise, as v is in the node's Voronoi cell: the nearest
        // kept strut has the largest t. Beyond `need`, a strut cannot matter)
        let vv = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
        let need = (hi / u + if last { rod } else { reach }).max(0.0);
        let need2 = need * need;
        let mut tk = -1.0f32;
        let interior = l == 0 && (c[0] - shift).abs().max(c[1].abs()).max(c[2].abs()) <= n - 0.5;
        for j in 0..12u32 {
            let d = dir(j);
            let t = (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]).max(0.0);
            let kept = if l == 0 {
                // both ends in the block
                interior || {
                    let (x0, x1) = (c[0] - shift, c[0] + d[0] - shift);
                    let e = [c[1] + d[1], c[2] + d[2]];
                    x0.abs().max(c[1].abs()).max(c[2].abs()) <= edge && x1.abs().max(e[0].abs()).max(e[1].abs()) <= edge
                }
            } else if vv - 0.5 * t * t >= need2 {
                true
            } else if !here.0 {
                false
            } else {
                // its other end in a cage too, and not along a kept strut of
                // the coarser level (hidden in it)
                let e = here.2;
                let along = {
                    let x = [e[1] * d[2] - e[2] * d[1], e[2] * d[0] - e[0] * d[2], e[0] * d[1] - e[1] * d[0]];
                    let k = if e[0] * d[0] + e[1] * d[1] + e[2] * d[2] >= 0.0 { j } else { j ^ 3 };
                    x[0] * x[0] + x[1] * x[1] + x[2] * x[2] < 1e-6 && here.1 >> k & 1 == 1
                };
                // (the other end: at the same coarser node when it is in that
                // node's Voronoi cell)
                let e = [e[0] + d[0] * invg, e[1] + d[1] * invg, e[2] + d[2] * invg];
                let a = [e[0].abs(), e[1].abs(), e[2].abs()];
                let other = if a[0] + a[1] + a[2] - a[0].min(a[1]).min(a[2]) < 0.999 {
                    geo(e, lim2) & here.1 != 0
                } else {
                    node_in([c[0] + d[0], c[1] + d[1], c[2] + d[2]], invg, lim2, prev).0
                };
                !along && other
            };
            if kept {
                mask |= 1 << j;
                tk = tk.max(t);
            }
        }
        let near = if tk >= 0.0 { (vv - 0.5 * tk * tk).max(0.0).sqrt() } else { 1e9 };
        let a = [v[0].abs(), v[1].abs(), v[2].abs()];
        let top2 = a[0] + a[1] + a[2] - a[0].min(a[1]).min(a[2]);
        let others = (1.0 - top2) * std::f32::consts::FRAC_1_SQRT_2 + VORONOI_GAP;
        let nb = near.min(others);
        let rods = (nb - rod) * u;
        if rods < nearest {
            nearest = rods;
            lvl = l as f32;
        }
        // (everything finer is within `reach` of these struts)
        let fam = (nb - reach) * u;
        let m = 0.25 * reach * u;
        if last || fam >= m {
            x = if last { rods } else { rods.min(fam) };
            break;
        }
        // X -> min(rods, 2m - fam, max(fam, X)), composed under the clamp so far
        let top = rods.min(2.0 * m - fam);
        let bottom = fam.min(top);
        (lo, hi) = (bottom.max(lo).min(hi), top.max(lo).min(hi));
        prev = mask;
    }
    let d = x.max(lo).min(hi);
    // (held below 2 u0 - boxd, to meet boxd where the lattice is first looked at)
    let d = d.max(boxd).min(2.0 * u0 - boxd);
    (d, (lvl * 0.5).min(1.0))
}

/// The octet truss of trusses' distance estimate and orbit trap, a lane at a
/// time (its lattice lookups branch per point).
fn octet_truss(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (x, y, z) = (p.x.to_array(), p.y.to_array(), p.z.to_array());
    let mut d = [0.0f32; 8];
    let mut t = [0.0f32; 8];
    for i in 0..8 {
        (d[i], t[i]) = truss1([x[i], y[i], z[i]], k, iters);
    }
    (F8::from_array(d), F8::from_array(t))
}

const OCTET_TRUSS: &str = r#"
fn octet_truss_dir(j: u32) -> vec3f {
    let s1 = select(1.0, -1.0, (j & 2u) != 0u);
    let s2 = select(1.0, -1.0, (j & 1u) != 0u);
    let pr = j >> 2u;
    if (pr == 0u) {
        return vec3f(s1, s2, 0.0);
    }
    if (pr == 1u) {
        return vec3f(s1, 0.0, s2);
    }
    return vec3f(0.0, s1, s2);
}

fn octet_truss_fcc(q: vec3f, bias: f32) -> vec3f {
    let r = floor(q + vec3f(0.5 + bias));
    let sum = r.x + r.y + r.z;
    let odd = sum - 2.0 * floor(sum * 0.5) > 0.5;
    let e = q - r;
    let a = abs(e) + vec3f(3e-5, 2e-5, 1e-5);
    let ix = a.x >= a.y && a.x >= a.z;
    let iy = !ix && a.y >= a.z;
    let iz = !ix && !iy;
    let sg = select(vec3f(-1.0), vec3f(1.0), e >= vec3f(0.0));
    return r + select(vec3f(0.0), sg, vec3<bool>(odd && ix, odd && iy, odd && iz));
}

fn octet_truss_dir_of(w: vec3f) -> u32 {
    let a = abs(w) + vec3f(3e-5, 2e-5, 1e-5);
    var pr = 2u;
    var s = w.y;
    var t = w.z;
    if (a.z <= a.x && a.z <= a.y) {
        pr = 0u;
        s = w.x;
        t = w.y;
    } else if (a.y <= a.x) {
        pr = 1u;
        s = w.x;
        t = w.z;
    }
    return pr * 4u + select(0u, 2u, s < 0.0) + select(0u, 1u, t < 0.0);
}

// (the four struts along +-a +-b, as bits (+,+), (+,-), (-,+), (-,-): which
// lines pass within lim of the point ahead of the node, r2 its squared
// distance from the node)
fn octet_truss_geo4(a: f32, b: f32, r2: f32, lim2: f32) -> u32 {
    let t0 = a + b;
    let t1 = a - b;
    let n0 = r2 - 0.5 * t0 * t0 <= lim2;
    let n1 = r2 - 0.5 * t1 * t1 <= lim2;
    return select(0u, 1u, t0 > 0.0 && n0) | select(0u, 2u, t1 > 0.0 && n1) | select(0u, 4u, t1 < 0.0 && n1) | select(0u, 8u, t0 < 0.0 && n0);
}

fn octet_truss_geo(e: vec3f, lim2: f32) -> u32 {
    let r2 = dot(e, e);
    let m = octet_truss_geo4(e.x, e.y, r2, lim2) | (octet_truss_geo4(e.x, e.z, r2, lim2) << 4u) | (octet_truss_geo4(e.y, e.z, r2, lim2) << 8u);
    return select(m, 0xfffu, r2 <= lim2);
}

fn octet_truss_flags(cp: vec3f, prev: u32) -> u32 {
    let ac = abs(cp);
    let ones = ac.x + ac.y + ac.z;
    let k = octet_truss_dir_of(-cp);
    let nb = (0xfffu & ~(1u << k)) | (((prev >> (k ^ 3u)) & 1u) << k);
    return select(select(0xfffu, nb, ones == 2.0 && max(ac.x, max(ac.y, ac.z)) == 1.0), prev, ones == 0.0);
}

// (x in finer node units: whether it is within lim of a kept strut at the
// coarser node nearest it; that node's flags; x's offset from it)
struct OctetTrussNode {
    inn: bool,
    flags: u32,
    e: vec3f,
}

fn octet_truss_node(x: vec3f, invg: f32, lim2: f32, prev: u32) -> OctetTrussNode {
    let y = x * invg;
    let cp = octet_truss_fcc(y, 1e-4);
    let e = y - cp;
    let flags = octet_truss_flags(cp, prev);
    return OctetTrussNode((octet_truss_geo(e, lim2) & flags) != 0u, flags, e);
}

fn octet_truss(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let n = a.x;
    let g = a.y;
    let inv_u0 = a.w;
    let invg = b.x;
    let rod = b.y;
    let reach = b.z;
    let lim = b.w;
    let shift = c.x;
    let u0 = c.y;
    let qb = abs(p) - vec3f(n * u0);
    let boxd = length(max(qb, vec3f(0.0))) + min(max(qb.x, max(qb.y, qb.z)), 0.0) - reach * u0;
    if (boxd > u0) {
        return vec2f(boxd, 0.0);
    }
    var v = vec3f(p.x * inv_u0 + shift, p.y * inv_u0, p.z * inv_u0);
    var cn = octet_truss_fcc(v, 0.0);
    v = v - cn;
    var u = u0;
    var lo = -1e9;
    var hi = 1e9;
    var x = 1e9;
    var lvl = 0.0;
    var nearest = 1e9;
    var prev = 0u;
    let edge = n + 0.5;
    let lim2 = lim * lim;
    for (var l = 0u; l < iters; l++) {
        if (l > 0u) {
            let q = v * g;
            cn = octet_truss_fcc(q, 0.0);
            v = q - cn;
            u *= invg;
        }
        let last = l + 1u == iters;
        var mask = 0u;
        var here = OctetTrussNode(true, 0u, vec3f(0.0));
        if (l > 0u) {
            here = octet_truss_node(cn, invg, lim2, prev);
        }
        let vv = dot(v, v);
        let need = max(hi / u + select(reach, rod, last), 0.0);
        let need2 = need * need;
        var tk = -1.0;
        let interior = l == 0u && max(abs(cn.x - shift), max(abs(cn.y), abs(cn.z))) <= n - 0.5;
        for (var j = 0u; j < 12u; j++) {
            let d = octet_truss_dir(j);
            let t = max(dot(v, d), 0.0);
            var kept = true;
            if (l == 0u) {
                if (!interior) {
                    let x0 = cn.x - shift;
                    let e = cn + d;
                    let x1 = e.x - shift;
                    kept = max(abs(x0), max(abs(cn.y), abs(cn.z))) <= edge && max(abs(x1), max(abs(e.y), abs(e.z))) <= edge;
                }
            } else if (vv - 0.5 * t * t < need2) {
                kept = here.inn;
                if (kept) {
                    let xc = cross(here.e, d);
                    let k = select(j ^ 3u, j, dot(here.e, d) >= 0.0);
                    let along = dot(xc, xc) < 1e-6 && ((here.flags >> k) & 1u) == 1u;
                    let e = here.e + d * invg;
                    let ae = abs(e);
                    var other = false;
                    if (ae.x + ae.y + ae.z - min(ae.x, min(ae.y, ae.z)) < 0.999) {
                        other = (octet_truss_geo(e, lim2) & here.flags) != 0u;
                    } else {
                        other = octet_truss_node(cn + d, invg, lim2, prev).inn;
                    }
                    kept = !along && other;
                }
            }
            if (kept) {
                mask |= 1u << j;
                tk = max(tk, t);
            }
        }
        var near = 1e9;
        if (tk >= 0.0) {
            near = sqrt(max(vv - 0.5 * tk * tk, 0.0));
        }
        let av = abs(v);
        let top2 = av.x + av.y + av.z - min(av.x, min(av.y, av.z));
        let others = (1.0 - top2) * 0.70710678 + 0.4082483;
        let nb = min(near, others);
        let rods = (nb - rod) * u;
        if (rods < nearest) {
            nearest = rods;
            lvl = f32(l);
        }
        let fam = (nb - reach) * u;
        let m = 0.25 * reach * u;
        if (last || fam >= m) {
            x = select(min(rods, fam), rods, last);
            break;
        }
        let top = min(rods, 2.0 * m - fam);
        let bottom = min(fam, top);
        let nlo = min(max(bottom, lo), hi);
        hi = min(max(top, lo), hi);
        lo = nlo;
        prev = mask;
    }
    var d = min(max(x, lo), hi);
    d = min(max(d, boxd), 2.0 * u0 - boxd);
    return vec2f(d, min(lvl * 0.5, 1.0));
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::{find, FormulaNode};

    /// mod.rs's steepness check at the corners of the parameters (it tries
    /// only the defaults) and at several levels, near and far: the
    /// estimate, times the step, changes at most 1.6 times as fast as the
    /// point moves, anywhere.
    #[test]
    fn estimates_hold_across_the_parameters() {
        let corners: &[&[(usize, f32)]] = &[
            &[],
            &[(0, 1.0), (1, 8.0), (2, 0.12)],
            &[(0, 1.0), (1, 12.0), (2, 0.01)],
            &[(0, 4.0), (1, 8.0), (2, 0.01)],
            &[(0, 4.0), (1, 12.0), (2, 0.12)],
            &[(0, 3.0), (1, 10.0), (2, 0.08)],
        ];
        let id = find("octet_truss").unwrap();
        let f = crate::formulas::get(id);
        let mut seed = 0x9e37_79b9u32;
        let mut rnd = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        for set in corners {
            for iters in [1, 3, 5] {
                let mut values = f.defaults();
                for &(i, v) in *set {
                    values[i] = v;
                }
                let node = FormulaNode { id, values, iterations: iters };
                let k = node.pack();
                for near in [1.0, 0.3, 0.05] {
                    let reach = f.bound * 1.2 * near;
                    let e = 1e-3 * reach;
                    for _ in 0..1500 {
                        let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                        let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                        let (d, t) = (f.cpu)(p, &k, node.iters());
                        let (dq, _) = (f.cpu)(V8::new(p.x + e, p.y, p.z), &k, node.iters());
                        for i in 0..8 {
                            let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                            assert!(a.is_finite() && (0.0..=1.0).contains(&tr), "{set:?}: {a}, trap {tr} at {:?}", p.lane(i));
                            if a > 10.0 * e && b.is_finite() {
                                let steep = (b - a).abs() / e * f.step;
                                assert!(steep < 1.6, "{set:?} at {iters} levels: changes {steep:.2} times as fast as the point at {:?}", p.lane(i));
                            }
                        }
                    }
                }
            }
        }
    }
}
