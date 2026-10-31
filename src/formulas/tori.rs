//! Spheres and tori: fractals built on tori and on packed, inverted
//! spheres.

use super::util::{len2, s};
use super::{Formula, K, Param};
use crate::simd::{F8, M8, V8};
use std::f32::consts::TAU;

pub const FORMULAS: &[Formula] = &[
    Formula {
        name: "msltoe_donut",
        category: "Fractals: spheres and tori",
        desc: "A torus with smaller tori looped around its tube like chain links, each looped with smaller ones in turn, so the ring sprouts bushy clusters of links. msltoe's escape-time donut (Fractal Forums, 2016); ported from Mandelbulber 2.",
        params: &[
            Param::num("thickness", &[0.1], 0.01, 0.3, "The tube radius of every torus, as a share of its ring radius (0.01 to 0.3): wire-thin rings to fat doughnuts that crowd their links."),
            Param::num("factor", &[3.0], 2.0, 5.0, "How many times smaller each generation of tori is (2 to 5): low values tangle the links into dense clusters, high ones leave them sparse. (Detail close up assumes about 3.)"),
            Param { name: "number", default: &[9.0], min: 3.0, max: 12.0, whole: true, doc: "How many tori loop around each torus (3 to 12)." },
        ],
        iterations: (5, 12, "Generations of tori: each adds `number` times as many, `factor` times smaller (more are added close up)."),
        bound: 2.0,
        lod: Some((3.0, 12)),
        step: 1.0,
        example: r#"{"type":"msltoe_donut","scale":0.75,"rotation":[30,0,0]}"#,
        prep: Some(donut_prep),
        cpu: msltoe_donut,
        wgsl: MSLTOE_DONUT,
        wgsl_fn: "msltoe_donut",
    },
    Formula {
        name: "mandeltorus",
        category: "Fractals: spheres and tori",
        desc: "A Mandelbulb wrapped around a ring: the bulb's power map measured from a circle instead of a point, so its lobes and ridges grow on a doughnut. Aexion's Mandeltorus (Fractal Forums, 2014); ported from Mandelbulber 2.",
        params: &[
            Param::num("power", &[18.0], 4.0, 24.0, "The power around the ring (4 to 24): how many lobes run around the doughnut."),
            Param::num("power2", &[18.0], 4.0, 24.0, "The power across the tube (4 to 24): how many ridges wrap the tube's cross-section; low powers make it a lumpy blob."),
            Param::num("ring", &[1.5], 1.0, 2.0, "The radius of the circle the map is measured from (1 to 2): larger opens the hole."),
            Param::whole("mode", 1.0, 1.0, 2.0, "1 measures the angle across the tube from the centre (a fluted doughnut); 2 from the ring itself (stacked, wavy rings)."),
        ],
        iterations: (10, 20, "Escape-time iterations: more = finer lobes, at two pows, three atan2 and four sin or cos per point each."),
        bound: 3.4,
        lod: None,
        step: 0.9,
        example: r#"{"type":"mandeltorus","scale":0.45,"rotation":[25,0,0]}"#,
        prep: None,
        cpu: mandeltorus,
        wgsl: MANDELTORUS,
        wgsl_fn: "mandeltorus",
    },
    Formula {
        name: "spheretree",
        category: "Fractals: spheres and tori",
        desc: "A dome of spheres within spheres: every ball is lined inside with a hexagonal layer of smaller balls, and they with smaller still, so it grows like a cauliflower. TGlad's sphere tree (Fractal Forums, 2020); ported from Mandelbulber 2.",
        params: &[
            Param::num("twist", &[30.0], 0.0, 60.0, "How far each generation's hexagonal layer is turned against its parent's (0 to 60 degrees; TGlad's 30): it moves where the small balls sit, a subtle change."),
            Param::num("lattice", &[2.0], 1.5, 2.5, "The density of each hexagonal layer (1.5 to 2.5): 2 packs the balls so they touch; lower leaves fewer, bigger balls, higher crowds in more, smaller ones."),
        ],
        iterations: (4, 16, "Generations of balls: each lines the last with smaller ones, about five times finer (more are added close up)."),
        bound: 0.62,
        lod: Some((5.0, 16)),
        step: 1.0,
        example: r#"{"type":"spheretree","scale":1.8}"#,
        prep: Some(spheretree_prep),
        cpu: spheretree,
        wgsl: SPHERETREE,
        wgsl_fn: "spheretree",
    },
    Formula {
        name: "sphere_cluster",
        category: "Fractals: spheres and tori",
        desc: "A hollow ball of sphere clusters with icosahedral symmetry, spiked like a snowflake: each cluster crowds toward the centre in ever smaller copies, and each of its balls is a cluster in turn. TGlad's sphere clusters (Fractal Forums, 2023); ported from Mandelbulber 2, with a steadier distance estimate.",
        params: &[
            Param::num("pack", &[0.8], 0.5, 0.95, "How much of its gap each sub-cluster fills (0.5 to 0.95): low values leave airy, separate clusters; high ones swell them until they touch."),
            Param::whole("dodecahedral", 0.0, 0.0, 1.0, "0 hangs the sub-clusters over the 20 faces of an icosahedron; 1 over the 12 faces of a dodecahedron (fewer, bigger spikes)."),
            Param::whole("alternate", 0.0, 0.0, 1.0, "1 switches between the two arrangements at every generation."),
        ],
        iterations: (4, 8, "Generations: each turns the balls of the last into clusters, at about thirty mirror folds each (more are added close up)."),
        bound: 1.0,
        lod: Some((3.0, 8)),
        step: 0.7,
        example: r#"{"type":"sphere_cluster","scale":1.4}"#,
        prep: None,
        cpu: sphere_cluster,
        wgsl: SPHERE_CLUSTER,
        wgsl_fn: "sphere_cluster",
    },
];

/// k[0..3] = thickness, factor, number; adds the sector angle (3), its
/// inverse (4) and how far from a torus's ring line the finer generations
/// reach, in its own units (5).
fn donut_prep(k: &mut [f32; K]) {
    let (r, f, n) = (k[0], k[1], k[2]);
    k[3] = TAU / n;
    k[4] = n / TAU;
    // a generation's own torus and all finer ones lie within (1 + reach) / f
    // of the ring line it hangs on
    k[5] = (1.0 / (f - 1.0)).max((1.0 + r) / f);
}

/// msltoe's donut as the union it draws: a torus of ring radius 1 about the
/// vertical; the point turned into its nearest of `number` sectors, the
/// sector's ring point moved to the centre, its cross-section plane turned
/// to be the next torus's ring plane, scaled by `factor`; repeat. Distance:
/// the nearest torus of any generation, scaled back (a generation stops
/// once no finer one can be nearer). Trap: the nearest torus's generation.
fn msltoe_donut(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (r, f, sect, inv_sect, reach) = (k[0], k[1], k[3], k[4], k[5]);
    // (Mandelbulber's frame: the ring about z, ours y)
    let (mut x, mut y, mut z) = (p.x, p.z, p.y);
    let mut inv_s = 1.0f32;
    let mut d = s(1e9);
    let mut lvl = F8::zero();
    let mut act = M8::all_true();
    for n in 0..iters {
        let ring = len2(len2(x, y) - 1.0, z);
        let dt = (ring - r) * inv_s;
        let near = act & dt.lt(d);
        d = F8::select(near, dt, d);
        lvl = F8::select(near, s(n as f32), lvl);
        act &= ((ring - reach) * inv_s).lt(d);
        if act.none() {
            break;
        }
        let a = (y.atan2(x) * inv_sect + 0.5).floor() * sect;
        let (c, sn) = (a.cos(), a.sin());
        let x1 = c * x + sn * y;
        let y1 = c * y - sn * x;
        x = (x1 - 1.0) * f;
        y = z * f;
        z = y1 * f;
        inv_s /= f;
    }
    (d, (lvl * 0.3).min(F8::one()))
}

const MSLTOE_DONUT: &str = r#"
fn msltoe_donut(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let r = a.x;
    let f = a.y;
    let sect = a.w;
    let inv_sect = b.x;
    let reach = b.y;
    var q = p.xzy;
    var inv_s = 1.0;
    var d = 1e9;
    var lvl = 0.0;
    for (var n = 0u; n < iters; n++) {
        let ring = length(vec2f(length(q.xy) - 1.0, q.z));
        let dt = (ring - r) * inv_s;
        if (dt < d) {
            d = dt;
            lvl = f32(n);
        }
        if ((ring - reach) * inv_s >= d) {
            break;
        }
        let an = floor(atan2(q.y, q.x) * inv_sect + 0.5) * sect;
        let cs = cos(an);
        let sn = sin(an);
        let x1 = cs * q.x + sn * q.y;
        let y1 = cs * q.y - sn * q.x;
        q = vec3f((x1 - 1.0) * f, q.z * f, y1 * f);
        inv_s = inv_s / f;
    }
    return vec2f(d, min(lvl * 0.3, 1.0));
}
"#;

/// Aexion's Mandeltorus: the bulb's power map with its radius measured
/// from the ring of radius `ring` about the vertical (rhrad), the angle
/// about the vertical times `power` and the angle across the tube times
/// `power2`, plus the point. Distance: the log estimate, from the escaped
/// point's own distance to the ring and a derivative that grows as the
/// map's output over rhrad times the larger power. (Mandelbulber's grows by
/// rhrad^power (power - 1) and takes |z|; both shrink the estimate at every
/// further iteration, so it jumps where the escape count changes and draws
/// shells.) Trap: the nearest the orbit comes to the ring's plane.
fn mandeltorus(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (p1, p2, ring, mode2) = (k[0], k[1], k[2], k[3] > 1.5);
    let (e1, e2) = (s(p1), s(p2));
    let pmax = p1.max(p2);
    let bail = s(10.0);
    let mut z = p;
    let mut dr = F8::one();
    let mut r = z.length();
    let mut trap = s(1e9);
    let mut act: M8 = r.lt(bail);
    for _ in 0..iters {
        if act.none() {
            break;
        }
        let rh = len2(z.x, z.z);
        let phi = z.z.atan2(z.x);
        // (cos phi, sin phi) times the ring radius
        let inv = s(ring) / rh.max(s(1e-12));
        let (px, pz) = (z.x - z.x * inv, z.z - z.z * inv);
        let rad = (px * px + pz * pz + z.y * z.y).sqrt().max(s(1e-6));
        let rh1 = rad.powf(e2);
        let rh2 = rad.powf(e1);
        let (st, ny) = if mode2 {
            let tg = len2(px, pz).atan2(z.y) * p2;
            ((tg.cos() + ring) * rh2, tg.sin() * rh1)
        } else {
            let tp = rh.atan2(z.y) * p2;
            (tp.sin() * rh2, tp.cos() * rh1)
        };
        let pp = phi * p1;
        // (the output's length scaled first: rhrad^24 squared overflows)
        let m = st.abs().max(ny.abs()).max(s(1e-30));
        let ndr = len2(st / m, ny / m) * m / rad * pmax * dr + 1.0;
        let nz = V8::new(st * pp.cos() + p.x, ny + p.y, st * pp.sin() + p.z);
        trap = F8::select(act, trap.min(nz.y.abs()), trap);
        z = V8::select(act, nz, z);
        dr = F8::select(act, ndr, dr);
        r = F8::select(act, z.length(), r);
        act &= r.lt(bail);
    }
    // the escaped point's distance to the ring, scaled first as above
    let m = z.x.abs().max(z.y.abs()).max(z.z.abs()).max(s(1e-30));
    let (x, y, zz) = (z.x / m, z.y / m, z.z / m);
    let rad = (len2(len2(x, zz) - s(ring) / m, y) * m).max(s(1e-6));
    let d = rad.ln() * rad * 0.5 / dr.max(s(1e-12));
    // (an orbit that never escaped is inside, as Mandelbulber's maxiter: near
    // the ring the map shrinks, so such orbits can settle well away from 0)
    (F8::select(r.lt(bail), d.min(F8::zero()), d), trap.saturate())
}

const MANDELTORUS: &str = r#"
fn mandeltorus(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let p1 = a.x;
    let p2 = a.y;
    let ring = a.z;
    let mode2 = a.w > 1.5;
    let pmax = max(p1, p2);
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 10.0) {
            break;
        }
        let rh = length(z.xz);
        let phi = atan2(z.z, z.x);
        let inv = ring / max(rh, 1e-12);
        let px = z.x - z.x * inv;
        let pz = z.z - z.z * inv;
        let rad = max(sqrt(px * px + pz * pz + z.y * z.y), 1e-6);
        let rh1 = pow(rad, p2);
        let rh2 = pow(rad, p1);
        var st: f32;
        var ny: f32;
        if (mode2) {
            let tg = atan2(length(vec2f(px, pz)), z.y) * p2;
            st = (cos(tg) + ring) * rh2;
            ny = sin(tg) * rh1;
        } else {
            let tp = atan2(rh, z.y) * p2;
            st = sin(tp) * rh2;
            ny = cos(tp) * rh1;
        }
        let pp = phi * p1;
        let m = max(max(abs(st), abs(ny)), 1e-30);
        dr = length(vec2f(st / m, ny / m)) * m / rad * pmax * dr + 1.0;
        z = vec3f(st * cos(pp), ny, st * sin(pp)) + p;
        trap = min(trap, abs(z.y));
        r = length(z);
    }
    let m = max(max(abs(z.x), abs(z.y)), max(abs(z.z), 1e-30));
    let zs = z / m;
    let rad = max(length(vec2f(length(zs.xz) - ring / m, zs.y)) * m, 1e-6);
    var d = log(rad) * rad * 0.5 / max(dr, 1e-12);
    if (r < 10.0) {
        d = min(d, 0.0);
    }
    return vec2f(d, clamp(trap, 0.0, 1.0));
}
"#;

/// k[0..2] = twist (degrees), lattice; adds the twist's cos (2) and sin (3)
/// and the lattice's stretch of the estimate (4).
fn spheretree_prep(k: &mut [f32; K]) {
    let a = k[0].to_radians();
    k[2] = a.cos();
    k[3] = a.sin();
    // (the lattice scales across but not up: beyond 2 it stretches)
    k[4] = (k[1] * 0.5).max(1.0);
}

const SQRT3: f32 = 1.732_050_8;
/// The spheretree's estimate is the distance in its frame over 2.22 times
/// the frame's scale (Mandelbulber's safety factor for the inversions).
const TREE_FUDGE: f32 = 2.22;

/// TGlad's sphere tree, in Mandelbulber's frame (z up, the unit-diameter
/// base sphere on the origin, all of it within the sphere of radius 0.6
/// about (0, 0, 0.6)): a point inside the base sphere is inverted about
/// the origin, which turns the sphere into the plane z = 1, and stretched
/// and flipped below it, then folded into one cell of a triangular lattice
/// whose unit spheres stand for the base sphere's children; low down and
/// outside it, only inverted. Repeat. Its breaks (inside the solid inner
/// sphere, or above and outside everything) jump the estimate between
/// generations, so the estimate is also held to each generation's base
/// sphere and inner sphere where it carries on. Trap: the generation.
fn spheretree(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let (lat, ca, sa, stretch) = (k[1], k[2], k[3], k[4]);
    let mut flips = F8::zero();
    let t = 0.6f32;
    // (ours y up, the middle of its reach at the origin)
    let (mut x, mut y, mut z) = (p.x, -p.z, p.y + t);
    let inner = SQRT3 / (1.0 + SQRT3);
    let (ih, ib) = (inner * 0.5, inner * inner * 0.25);
    let mut dd = F8::one();
    let mut act = M8::all_true();
    let mut dmin = s(1e9);
    for n in 0..iters {
        let max_h = if n == 0 { -100.0 } else { 0.4 };
        let w = F8::one() / (dd * TREE_FUDGE);
        let xy2 = x * x + y * y;
        let (zi, zh, zt) = (z - ih, z - 0.5, z - t);
        let (ri2, rh2, rt2) = (xy2 + zi * zi, xy2 + zh * zh, xy2 + zt * zt);
        let inside = ri2.lt(s(ib));
        let out = z.gt(s(max_h)) & rt2.gt(s(t * t));
        dmin = F8::select(act, dmin.min((ri2.sqrt() - ih) * w), dmin);
        let cont = act & !inside & !out;
        let base = (rh2.sqrt() - 0.5) * w;
        let gone = ((rt2.sqrt() - t) * w).min((z - max_h) * w);
        dmin = F8::select(cont, dmin.min(base.max(-gone)), dmin);
        act = cont;
        if act.none() {
            break;
        }
        let plain = z.lt(s(max_h)) & rh2.gt(s(0.25));
        let inv = F8::one() / (xy2 + z * z).max(s(1e-12));
        let (ix, iy, iz) = (x * inv, y * inv, z * inv);
        // outside the sphere low down: inverted; else inverted onto the plane z = 1, flipped and stretched below it, and twisted
        let (sx, sy) = (ix * SQRT3, iy * SQRT3);
        let sz = (1.0 - iz) * SQRT3 + 1.0;
        let (sx, sy) = (sx * ca + sy * sa, sy * ca - sx * sa);
        let nx = F8::select(plain, ix, sx);
        let ny = F8::select(plain, iy, sy);
        let nz = F8::select(plain, iz, sz);
        let ndd = F8::select(plain, dd * inv, dd * inv * SQRT3) * stretch;
        // into the central hexagon of the triangular lattice, then its kite
        let u = (nx * 0.5 - ny * (0.5 * SQRT3)) * (lat / SQRT3);
        let v = (nx * 0.5 + ny * (0.5 * SQRT3)) * (lat / SQRT3);
        let (u, v) = (u.fract(), v.fract());
        let flip = (u + v).gt(F8::one());
        let (u, v) = (F8::select(flip, 1.0 - u, u), F8::select(flip, 1.0 - v, v));
        let (kx, ky) = ((u + v) * (0.5 * SQRT3), (v - u) * 0.5);
        let (t1x, t1y) = (0.5 * SQRT3, -0.5);
        let l0 = kx * kx + ky * ky;
        let l1 = (kx - t1x) * (kx - t1x) + (ky - t1y) * (ky - t1y);
        let l2 = (kx - t1x) * (kx - t1x) + (ky + t1y) * (ky + t1y);
        let m1 = l1.lt(l0) & l1.lt(l2);
        let m2 = l2.lt(l0) & l2.lt(l1) & !m1;
        let d1 = (kx * t1x + ky * t1y) * 2.0 - 1.0;
        let d2 = (ky * t1y - kx * t1x) * 2.0 + 1.0;
        let kx = F8::select(m1, kx - d1 * t1x, F8::select(m2, kx + d2 * t1x, kx));
        let ky = F8::select(m1, ky - d1 * t1y, F8::select(m2, ky - d2 * t1y, ky));
        x = F8::select(act, kx, x);
        y = F8::select(act, ky, y);
        z = F8::select(act, nz, z);
        dd = F8::select(act, ndd, dd);
        flips = F8::select(act & (flip | m1 | m2), flips + 1.0, flips);
    }
    let zh = z - 0.5;
    let d = ((x * x + y * y + zh * zh).sqrt() - 0.5) / (dd * TREE_FUDGE);
    (d.min(dmin), (flips * 0.25).fract())
}

const SPHERETREE: &str = r#"
fn spheretree(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let lat = a.y;
    let ca = a.z;
    let sa = a.w;
    let stretch = b.x;
    let t = 0.6;
    let s3 = 1.7320508;
    var q = vec3f(p.x, -p.z, p.y + t);
    let inner = s3 / (1.0 + s3);
    let ih = inner * 0.5;
    let ib = inner * inner * 0.25;
    var dd = 1.0;
    var dmin = 1e9;
    var flips = 0.0;
    for (var n = 0u; n < iters; n++) {
        var max_h = 0.4;
        if (n == 0u) {
            max_h = -100.0;
        }
        let w = 1.0 / (dd * 2.22);
        let xy2 = q.x * q.x + q.y * q.y;
        let zi = q.z - ih;
        let zh = q.z - 0.5;
        let zt = q.z - t;
        let ri2 = xy2 + zi * zi;
        let rh2 = xy2 + zh * zh;
        let rt2 = xy2 + zt * zt;
        dmin = min(dmin, (sqrt(ri2) - ih) * w);
        if (ri2 < ib || (q.z > max_h && rt2 > t * t)) {
            break;
        }
        let base = (sqrt(rh2) - 0.5) * w;
        let gone = min((sqrt(rt2) - t) * w, (q.z - max_h) * w);
        dmin = min(dmin, max(base, -gone));
        let inv = 1.0 / max(xy2 + q.z * q.z, 1e-12);
        var nw = q * inv;
        if (q.z < max_h && rh2 > 0.25) {
            dd = dd * inv * stretch;
        } else {
            dd = dd * inv * s3 * stretch;
            let sx = nw.x * s3;
            let sy = nw.y * s3;
            nw = vec3f(sx * ca + sy * sa, sy * ca - sx * sa, (1.0 - nw.z) * s3 + 1.0);
        }
        var u = (nw.x * 0.5 - nw.y * (0.5 * s3)) * (lat / s3);
        var v = (nw.x * 0.5 + nw.y * (0.5 * s3)) * (lat / s3);
        u = u - floor(u);
        v = v - floor(v);
        var flipped = false;
        if (u + v > 1.0) {
            u = 1.0 - u;
            v = 1.0 - v;
            flipped = true;
        }
        var kx = (u + v) * (0.5 * s3);
        var ky = (v - u) * 0.5;
        let t1x = 0.5 * s3;
        let t1y = -0.5;
        let l0 = kx * kx + ky * ky;
        let l1 = (kx - t1x) * (kx - t1x) + (ky - t1y) * (ky - t1y);
        let l2 = (kx - t1x) * (kx - t1x) + (ky + t1y) * (ky + t1y);
        if (l1 < l0 && l1 < l2) {
            let d1 = (kx * t1x + ky * t1y) * 2.0 - 1.0;
            kx = kx - d1 * t1x;
            ky = ky - d1 * t1y;
            flipped = true;
        } else if (l2 < l0 && l2 < l1) {
            let d2 = (ky * t1y - kx * t1x) * 2.0 + 1.0;
            kx = kx + d2 * t1x;
            ky = ky - d2 * t1y;
            flipped = true;
        }
        if (flipped) {
            flips = flips + 1.0;
        }
        q = vec3f(kx, ky, nw.z);
    }
    let zh = q - vec3f(0.0, 0.0, 0.5);
    let d = (length(zh) - 0.5) / (dd * 2.22);
    return vec2f(min(d, dmin), fract(flips * 0.25));
}
"#;

/// The sphere cluster's mirror normals: the icosahedron's three (applied
/// three times over) and the dodecahedron's five (twice over); for each, a
/// face sphere's radius and centre (orthogonal to the unit sphere) and the
/// radius of the hole the face spheres leave about the centre.
const SC_ICO: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [-0.309_017, 0.809_017, 0.5], [-0.309_017, -0.809_017, 0.5]];
const SC_DOD: [[f32; 3]; 5] =
    [[0.809_017, 0.5, -0.309_017], [0.309_017, 0.809_017, 0.5], [0.0, 0.0, 1.0], [0.309_017, -0.809_017, 0.5], [0.809_017, -0.5, -0.309_017]];

struct ScPoly {
    normals: &'static [[f32; 3]],
    passes: usize,
    r: f32,
    c: [f32; 3],
    hole: f32,
}

const SC_POLYS: [ScPoly; 2] = [
    ScPoly { normals: &SC_ICO, passes: 3, r: 1.018_751_8, c: [0.509_375_9, 0.0, 1.333_563_4], hole: 0.408_782_86 },
    ScPoly { normals: &SC_DOD, passes: 2, r: 1.111_785_9, c: [1.272_019_7, 0.0, 0.786_151_4], hole: 0.383_562_84 },
];

/// Face-sphere reductions per generation.
const SC_REDUCE: u32 = 2;

/// TGlad's sphere clusters, with Mandelbulber's construction but a steady
/// estimate: the point inverted about the centre (radius² the hole), which
/// turns the cluster inside out; folded into the polyhedron's fundamental
/// cell and inverted in the face sphere there while inside it, a few times
/// (the group the face spheres generate tiles space with copies of the
/// hole); a point never inverted is in the hole itself, which is empty,
/// so it is inverted once to the nearest copy. That copy of the hole holds
/// the next generation, `pack` times its size; the last generation is a
/// ball. Mandelbulber recurses only from inside the hole and measures to a
/// ball from outside it, which jumps there; always recursing does not.
/// Every inversion about the centre also adds 1/|p| (in the point's own
/// units) to the inverse of the estimate: a sphere inversion can shrink a
/// distance by at most that, so far from the centre the estimate does not
/// outgrow the distance. Trap: how many face inversions it took beyond one
/// per generation (the small spheres crowding the limit).
fn sphere_cluster(p: V8, k: &[f32; K], iters: u32) -> (F8, F8) {
    let pack = k[0];
    let mut dod = k[1] > 0.5;
    let alt = k[2] > 0.5;
    let mut z = p;
    let mut lam = F8::one();
    let mut harm = F8::zero();
    let mut extra = F8::zero();
    let mut hole = SC_POLYS[0].hole;
    for lev in 0..iters {
        let g = &SC_POLYS[dod as usize];
        hole = g.hole;
        let r2 = z.length_sq().max(s(1e-30));
        harm += lam / r2.sqrt();
        let sc = s(hole) / r2;
        z = z * sc;
        lam *= sc;
        let (c, rr) = (V8::new(s(g.c[0]), s(g.c[1]), s(g.c[2])), g.r * g.r);
        let mut count = F8::zero();
        for it in 0..=SC_REDUCE {
            for _ in 0..g.passes {
                for n in g.normals {
                    let t = (z.z.mul_add(s(n[2]), z.y.mul_add(s(n[1]), z.x * n[0]))).min(F8::zero()) * 2.0;
                    z = V8::new(z.x - t * n[0], z.y - t * n[1], z.z - t * n[2]);
                }
            }
            let q = z - c;
            let q2 = q.length_sq().max(s(1e-30));
            let go = if it < SC_REDUCE { q2.lt(s(rr)) } else { count.eq(F8::zero()) };
            let sc = s(rr) / q2;
            z = V8::select(go, q * sc + c, z);
            lam = F8::select(go, lam * sc, lam);
            count += F8::one().and(go);
        }
        extra += count - 1.0;
        if lev + 1 < iters {
            let m = 1.0 / (hole * pack);
            z = z * m;
            lam *= m;
            dod ^= alt;
        }
    }
    let dd = z.length() - hole * pack;
    let d = F8::select(dd.gt(F8::zero()), dd / (lam + harm * dd.max(F8::zero())), dd / lam);
    (d, F8::one() - (extra * -0.514_573_2).exp2())
}

const SPHERE_CLUSTER: &str = r#"
fn sphere_cluster_fold(p: vec3f, n: vec3f) -> vec3f {
    let t = min(dot(p, n), 0.0) * 2.0;
    return p - n * t;
}

fn sphere_cluster(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f {
    let pack = a.x;
    var dod = a.y > 0.5;
    let alt = a.z > 0.5;
    var z = p;
    var lam = 1.0;
    var harm = 0.0;
    var extra = 0.0;
    var hole = 0.40878286;
    for (var lev = 0u; lev < iters; lev++) {
        var cc = vec3f(0.5093759, 0.0, 1.3335634);
        var rr = 1.0187518 * 1.0187518;
        hole = 0.40878286;
        if (dod) {
            cc = vec3f(1.2720197, 0.0, 0.7861514);
            rr = 1.1117859 * 1.1117859;
            hole = 0.38356284;
        }
        let r2 = max(dot(z, z), 1e-30);
        harm = harm + lam / sqrt(r2);
        let sc = hole / r2;
        z = z * sc;
        lam = lam * sc;
        var count = 0.0;
        for (var it = 0u; it <= 2u; it++) {
            if (dod) {
                for (var k = 0u; k < 2u; k++) {
                    z = sphere_cluster_fold(z, vec3f(0.809017, 0.5, -0.309017));
                    z = sphere_cluster_fold(z, vec3f(0.309017, 0.809017, 0.5));
                    z = sphere_cluster_fold(z, vec3f(0.0, 0.0, 1.0));
                    z = sphere_cluster_fold(z, vec3f(0.309017, -0.809017, 0.5));
                    z = sphere_cluster_fold(z, vec3f(0.809017, -0.5, -0.309017));
                }
            } else {
                for (var k = 0u; k < 3u; k++) {
                    z = sphere_cluster_fold(z, vec3f(1.0, 0.0, 0.0));
                    z = sphere_cluster_fold(z, vec3f(-0.309017, 0.809017, 0.5));
                    z = sphere_cluster_fold(z, vec3f(-0.309017, -0.809017, 0.5));
                }
            }
            let q = z - cc;
            let q2 = max(dot(q, q), 1e-30);
            var go = q2 < rr;
            if (it == 2u) {
                go = count == 0.0;
            }
            if (go) {
                let s = rr / q2;
                z = q * s + cc;
                lam = lam * s;
                count = count + 1.0;
            }
        }
        extra = extra + count - 1.0;
        if (lev + 1u < iters) {
            let m = 1.0 / (hole * pack);
            z = z * m;
            lam = lam * m;
            dod = dod != alt;
        }
    }
    let dd = length(z) - hole * pack;
    var d = dd / lam;
    if (dd > 0.0) {
        d = dd / (lam + harm * dd);
    }
    return vec2f(d, 1.0 - exp2(extra * -0.5145732));
}
"#;
