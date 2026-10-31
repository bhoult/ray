// The renderer on the GPU: a per-pixel port of the CPU renderer (render/*.rs),
// written to compute the same picture: the same hashes, noise, jitter and
// random streams, the same marches, materials, lights and integrators. Each
// frame traces the rows of the bands it is given and returns, per pixel, the
// sum of this frame's samples (RGB9E5) and the object the camera ray hit; the
// CPU accumulates, post-processes and shows them with its own rows.
//
// The distance programs of the scene's objects are generated per structure
// (gpu/codegen.rs) and appended below; their numbers live in the scene buffer.

struct Frame {
    cam_pos: vec3f, kx: f32,
    cam_f: vec3f, bx: f32,
    cam_r: vec3f, ky: f32,
    cam_u: vec3f, by: f32,
    sun_dir: vec3f, fog: f32,
    sun_col: vec3f, clouds: f32,
    amb_col: vec3f, ambient: f32,
    sky_zen: vec3f, cone: f32,
    sky_hor: vec3f, max_dist: f32,
    fog_col: vec3f, ao_radius: f32,
    water_col: vec3f, water_level: f32,
    mode: u32, bounces: u32, max_steps: u32, flags: u32,
    shadow_hardness: f32, spp: u32, sample_base: u32, width: u32,
    n_objects: u32, off_objects: u32, n_nodes: u32, off_nodes: u32,
    off_items: u32, n_unbounded: u32, off_unbounded: u32, off_mats: u32,
    n_lights: u32, off_lights: u32, n_emitters: u32, off_emitters: u32,
    n_terrains: u32, off_terrains: u32, rows: u32, daylight: f32,
    time: f32, wave_time: f32, n_ripples: u32, wave_height: f32,
    /// (x, z, start time, height)
    ripples: array<vec4f, 32>,
    /// the sun's disk and glow (scene/mod.rs Env), the moon
    glow_dir: vec3f, night: f32,
    glow_col: vec3f, moon: f32,
    moon_dir: vec3f, headlight: f32,
    /// the volume clouds' headers in S (VOL words each)
    n_volumes: u32, off_volumes: u32, pad_v0: u32, pad_v1: u32,
    /// the headlight's lamp from the camera (render/shade.rs lamp) and the
    /// metres it reaches
    lamp: vec3f, lamp_reach: f32,
    /// what f32 cannot hold of the camera's position (camera.rs Camera::fine)
    cam_fine: vec3f, pad_cf: f32,
}

@group(0) @binding(0) var<uniform> F: Frame;
@group(0) @binding(1) var<storage, read> S: array<f32>;
@group(0) @binding(2) var<storage, read> T: array<f32>;
@group(0) @binding(3) var<storage, read> B: array<u32>;
@group(0) @binding(4) var<storage, read_write> O: array<vec2<u32>>;
/// S again, four words at a time: the distance programs' numbers and the
/// NURBS patches start on a multiple of 4 words (gpu/codegen.rs), so one
/// vec4 load fetches what four scalar ones did
@group(0) @binding(5) var<storage, read> S4: array<vec4f>;

// flags
const F_SHADOWS: u32 = 1u;
const F_SOFT: u32 = 2u;
const F_AO: u32 = 4u;
const F_LIGHTS: u32 = 8u;
const F_LIGHT_SHADOWS: u32 = 16u;
const F_FOG: u32 = 32u;
const F_CLOUDS: u32 = 64u;
const F_WATER: u32 = 128u;
const F_STARS: u32 = 256u;
const F_SKY_OFF: u32 = 512u;
const F_NO_DETAIL: u32 = 1024u;
const F_LAMP_SHADOWS: u32 = 2048u;

// modes
const M_DIRECT: u32 = 0u;
const M_PATH: u32 = 1u;
const M_NORMALS: u32 = 2u;
const M_ALBEDO: u32 = 3u;
const M_DEPTH: u32 = 4u;
const M_STEPS: u32 = 5u;
const M_COST: u32 = 6u;

// object kinds
const K_SPHERE: u32 = 0u;
const K_BOX: u32 = 1u;
const K_PLANE: u32 = 2u;
const K_SDF: u32 = 3u;
const K_TERRAIN: u32 = 4u;
const K_NURBS: u32 = 5u;

// patterns
const TX_NONE: u32 = 0u;
const TX_TERRAIN: u32 = 4u;
const TX_WATER: u32 = 5u;
const TX_ORBIT: u32 = 18u;

const OBJ: u32 = 48u;
const MAT: u32 = 30u;
const TER: u32 = 32u;

const BIG: f32 = 1.0e9;
// f32's resolution relative to a coordinate's size (render/trace.rs REL)
const REL: f32 = 4.76837158e-7;

/// The finest a march resolves `m` metres from where its positions are
/// measured from (render/trace.rs grain).
fn grain(m: f32) -> f32 {
    return m * REL + 1.0e-12;
}

/// The log of the threshold (world metres) the fractals resolve detail to,
/// set before each distance evaluation: they iterate more close up (render/vm.rs
/// Regs.ln_thr; above 1e30: as set)
var<private> LN_THR: f32 = 3.0e38;

/// The anchor table of the object being evaluated (render/mod.rs anchors,
/// render/vm.rs Regs.table): its first word in S, 0 for none; set with
/// LN_THR. And the points' offset from the frame's anchor, the camera
/// (Regs.dp), set before each evaluation.
var<private> ANC: u32 = 0u;
var<private> DP: vec3f = vec3f(0.0);

/// This sample's factor on the marches' hit threshold (render/mod.rs thr_jitter).
var<private> THR_K: f32 = 1.0;

fn thr_jitter(smp: u32, pix: u32) -> f32 {
    if (smp == 0u) {
        return 1.0;
    }
    return unit(lowbias((pix * 0x2c1b3c6du) ^ ((smp * 0x297a2d39u) ^ 0x68e31da4u))) * 1.5 + 0.25;
}

/// A fractal's iterations: its own count, or more close up (scene/sdf.rs Lod).
fn more_iters(iters: u32, k: f32, per: f32, cap: u32) -> u32 {
    if (LN_THR > 1e30 || on(F_NO_DETAIL)) {
        return iters;
    }
    return max(iters, u32(clamp(ceil(k - per * LN_THR), 0.0, f32(cap))));
}
const AMBIENT: f32 = 2.0;
const PI: f32 = 3.14159265;
const TAU: f32 = 6.28318531;
const WATER_ABSORB: vec3f = vec3f(1.0, 0.40, 0.28);

fn on(f: u32) -> bool {
    return (F.flags & f) != 0u;
}

fn v3(i: u32) -> vec3f {
    return vec3f(S[i], S[i + 1u], S[i + 2u]);
}

fn u(i: u32) -> u32 {
    return bitcast<u32>(S[i]);
}

// ---- hashes and noise (noise.rs, render/shade.rs) -------------------------

const NK1: u32 = 0x8DA6B343u;
const NK2: u32 = 0xD8163841u;
const NK3: u32 = 0x2C1B3C6Du;
const NK4: u32 = 0xCB1AB31Fu;

fn mix32(h: u32) -> u32 {
    var x = h ^ (h >> 15u);
    x = x * NK3;
    return x ^ (x >> 13u);
}

fn unit(h: u32) -> f32 {
    return bitcast<f32>((h >> 9u) | 0x3F800000u) - 1.0;
}

fn lowbias(v: u32) -> u32 {
    var x = v ^ (v >> 16u);
    x = x * 0x7feb352du;
    x = x ^ (x >> 15u);
    x = x * 0x846ca68bu;
    return x ^ (x >> 16u);
}

fn fi(x: f32) -> u32 {
    return bitcast<u32>(i32(x));
}

/// Value noise with derivatives: (n, dn/dx, dn/dy).
fn noised(x: f32, y: f32, seed: u32) -> vec3f {
    let fx0 = floor(x);
    let fy0 = floor(y);
    let fx = x - fx0;
    let fy = y - fy0;
    let x0 = fi(fx0) * NK1;
    let y0 = fi(fy0) * NK2;
    let x1 = x0 + NK1;
    let y1 = y0 + NK2;
    let a = unit(mix32(x0 ^ y0 ^ seed));
    let b = unit(mix32(x1 ^ y0 ^ seed));
    let c = unit(mix32(x0 ^ y1 ^ seed));
    let d = unit(mix32(x1 ^ y1 ^ seed));
    let ux = fx * fx * (3.0 - (fx + fx));
    let dux = (1.0 - fx) * fx * 6.0;
    let uy = fy * fy * (3.0 - (fy + fy));
    let duy = (1.0 - fy) * fy * 6.0;
    let k1 = b - a;
    let k2 = c - a;
    let k4 = (d - b) - k2;
    let n = k4 * (ux * uy) + (k2 * uy + (k1 * ux + a));
    return vec3f(n, (k4 * uy + k1) * dux, (k4 * ux + k2) * duy);
}

fn fade(t: f32) -> f32 {
    return ((t * 6.0 - 15.0) * t + 10.0) * t * t * t;
}

fn grad3(hh: u32, x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let h = mix32(hh ^ seed) & 15u;
    let uu = select(y, x, h < 8u);
    let xz = h == 12u || h == 14u;
    let vv = select(select(z, x, xz), y, h < 4u);
    return select(uu, -uu, (h & 1u) != 0u) + select(vv, -vv, (h & 2u) != 0u);
}

fn perlin3(p: vec3f, seed: u32) -> f32 {
    let f0 = floor(p);
    let f = p - f0;
    let x0 = fi(f0.x) * NK1;
    let y0 = fi(f0.y) * NK2;
    let z0 = fi(f0.z) * NK4;
    var c: array<f32, 8>;
    for (var i = 0u; i < 8u; i++) {
        let ox = i & 1u;
        let oy = (i >> 1u) & 1u;
        let oz = (i >> 2u) & 1u;
        let h = select(x0, x0 + NK1, ox == 1u) ^ select(y0, y0 + NK2, oy == 1u) ^ select(z0, z0 + NK4, oz == 1u);
        c[i] = grad3(h, f.x - f32(ox), f.y - f32(oy), f.z - f32(oz), seed);
    }
    let uu = fade(f.x);
    let vv = fade(f.y);
    let ww = fade(f.z);
    let x00 = mix(c[0], c[1], uu);
    let x10 = mix(c[2], c[3], uu);
    let x01 = mix(c[4], c[5], uu);
    let x11 = mix(c[6], c[7], uu);
    return mix(mix(x00, x10, vv), mix(x01, x11, vv), ww);
}

fn fbm3(p: vec3f, octaves: u32, seed: u32) -> f32 {
    var sum = 0.0;
    var amp = 0.5;
    var q = p;
    for (var i = 0u; i < octaves; i++) {
        sum += perlin3(q, seed) * amp;
        q = vec3f(q.x * 1.6 + q.z * 1.2, q.y * 2.0, q.z * 1.6 - q.x * 1.2);
        amp *= 0.5;
    }
    return sum;
}

// ---- random numbers and jitter (render/mod.rs, shade.rs Rng8) --------------

var<private> rng_state: u32;

fn rng_init(pixel: u32, smp: u32) {
    rng_state = lowbias(pixel * 0x9E3779B1u + smp * 0x85EBCA77u);
}

fn rnd() -> f32 {
    rng_state = rng_state + 0x9E3779B9u;
    return unit(lowbias(rng_state));
}

fn jitter(smp: u32, pix: u32) -> vec2f {
    if (smp == 0u) {
        return vec2f(0.0);
    }
    let k = f32(smp);
    let a = fract(0.5 + k * 0.7548777);
    let b = fract(0.5 + k * 0.5698403);
    let h = lowbias(pix);
    return vec2f(fract(unit(h) + a) - 0.5, fract(unit(lowbias(h ^ 0x68e31da4u)) + b) - 0.5);
}

// ---- distance functions (render/vm.rs) -------------------------------------

struct DT {
    d: f32,
    t: u32,
    /// the orbit trap of the nearest leaf (fractals; 0 for other shapes)
    o: f32,
}

fn rep3(p: vec3f, per: vec3f, lim: vec3f) -> vec3f {
    var q = p;
    for (var a = 0; a < 3; a++) {
        if (per[a] != 0.0) {
            var c = round(q[a] / per[a]);
            if (lim[a] >= 0.0) {
                c = clamp(c, -lim[a], lim[a]);
            }
            q[a] = q[a] - per[a] * c;
        }
    }
    return q;
}

fn mirror3(p: vec3f, axes: u32) -> vec3f {
    return vec3f(select(p.x, abs(p.x), (axes & 1u) != 0u), select(p.y, abs(p.y), (axes & 2u) != 0u), select(p.z, abs(p.z), (axes & 4u) != 0u));
}

fn twist(p: vec3f, k: f32) -> vec3f {
    let a = p.y * k;
    let s = sin(a);
    let c = cos(a);
    return vec3f(c * p.x - s * p.z, p.y, s * p.x + c * p.z);
}

fn bend(p: vec3f, k: f32) -> vec3f {
    let a = p.x * k;
    let s = sin(a);
    let c = cos(a);
    return vec3f(c * p.x - s * p.y, s * p.x + c * p.y, p.z);
}

fn sd_box(p: vec3f, b: vec3f, r: f32) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn sd_torus(p: vec3f, big: f32, small: f32) -> f32 {
    let qx = length(p.xz) - big;
    return length(vec2f(qx, p.y)) - small;
}

fn sd_cylinder(p: vec3f, r: f32, h: f32) -> f32 {
    let dx = length(p.xz) - r;
    let dy = abs(p.y) - h;
    return min(max(dx, dy), 0.0) + length(vec2f(max(dx, 0.0), max(dy, 0.0)));
}

fn sd_cone(p: vec3f, r1: f32, r2: f32, h: f32) -> f32 {
    let qx = length(p.xz);
    let qy = p.y;
    let k2x = r2 - r1;
    let k2y = 2.0 * h;
    let rr = select(r2, r1, qy < 0.0);
    let cax = qx - min(qx, rr);
    let cay = abs(qy) - h;
    let tt = clamp(((r2 - qx) * k2x + (h - qy) * k2y) * (1.0 / (k2x * k2x + k2y * k2y)), 0.0, 1.0);
    let cbx = (qx - r2) + tt * k2x;
    let cby = (qy - h) + tt * k2y;
    let s = select(1.0, -1.0, cbx < 0.0 && cay < 0.0);
    return s * sqrt(min(cax * cax + cay * cay, cbx * cbx + cby * cby));
}

fn sd_capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) * (1.0 / max(dot(ba, ba), 1e-12)), 0.0, 1.0);
    return length(pa - ba * h) - r;
}

fn sd_ellipsoid(p: vec3f, r: vec3f) -> f32 {
    let s = p * (1.0 / r);
    let s2 = s * (1.0 / r);
    let k0 = length(s);
    let k1 = max(length(s2), 1e-12);
    return k0 * (k0 - 1.0) / k1;
}

fn sd_gyroid(p: vec3f, f: f32, t: f32) -> f32 {
    let q = p * f;
    let g = sin(q.z) * cos(q.x) + (sin(q.y) * cos(q.z) + sin(q.x) * cos(q.y));
    return (abs(g) * (1.0 / f) - t * 0.5) * 0.41;
}

/// (distance, orbit trap) of the Menger sponge (render/vm.rs menger).
fn menger(p: vec3f, iters: u32, norm: u32) -> vec2f {
    let q = abs(p) - vec3f(1.0);
    let inside = min(max(q.x, max(q.y, q.z)), 0.0);
    var d = length(max(q, vec3f(0.0))) + inside;
    var level = 0.0;
    var s = 1.0;
    for (var i = 0u; i < min(iters, 12u); i++) {
        let half_s = 0.5 * s;
        let inv3s = 1.0 / (3.0 * s);
        s *= 3.0;
        let ax = fract(p.x * half_s);
        let ay = fract(p.y * half_s);
        let az = fract(p.z * half_s);
        let a = abs(1.0 - abs(ax + ax - 1.0) * 3.0);
        let b = abs(1.0 - abs(ay + ay - 1.0) * 3.0);
        let c = abs(1.0 - abs(az + az - 1.0) * 3.0);
        let cr = min(min(max(a, b), max(b, c)), max(c, a));
        let hole = (cr - 1.0) * inv3s;
        level = select(level, f32(i + 1u), hole > d);
        d = max(d, hole);
    }
    return vec2f(d, level * (1.0 / f32(max(norm, 1u))));
}

/// One level's fold minus 1 on one axis, from the anchor (menger_a): table
/// entry `o`, offset `c` along the axis.
fn anc_fold(c: f32, o: u32, half_s: f32) -> f32 {
    let f = fract(c * half_s + S[o + 4u]);
    let far = abs(1.0 - abs(f + f - 1.0) * 3.0) - 1.0;
    return select(far, c * S[o + 1u] + S[o], c > S[o + 2u] && c < S[o + 3u]);
}

/// menger from the frame's anchor (render/vm.rs menger_anchored): `at` is
/// its entry in the object's anchor table; without one, plain menger.
fn menger_a(p: vec3f, iters: u32, norm: u32, at: u32) -> vec2f {
    if (ANC == 0u) {
        return menger(p, iters, norm);
    }
    let t = ANC + at;
    let dc = vec3f(dot(DP, vec3f(S[t], S[t + 1u], S[t + 2u])), dot(DP, vec3f(S[t + 3u], S[t + 4u], S[t + 5u])), dot(DP, vec3f(S[t + 6u], S[t + 7u], S[t + 8u])));
    let e = vec3f(S[t + 9u], S[t + 10u], S[t + 11u]);
    let sg = vec3f(S[t + 12u], S[t + 13u], S[t + 14u]);
    let q = select(abs(p) - vec3f(1.0), dc * sg + e, dc * sg + e + vec3f(1.0) > vec3f(0.0));
    let inside = min(max(q.x, max(q.y, q.z)), 0.0);
    var d = length(max(q, vec3f(0.0))) + inside;
    var level = 0.0;
    var s = 1.0;
    for (var i = 0u; i < min(iters, 20u); i++) {
        let half_s = 0.5 * s;
        let inv3s = 1.0 / (3.0 * s);
        s *= 3.0;
        let o = t + 15u + i * 15u;
        let ga = anc_fold(dc.x, o, half_s);
        let gb = anc_fold(dc.y, o + 5u, half_s);
        let gc = anc_fold(dc.z, o + 10u, half_s);
        let cr = min(min(max(ga, gb), max(gb, gc)), max(gc, ga));
        let hole = cr * inv3s;
        level = select(level, f32(i + 1u), hole > d);
        d = max(d, hole);
    }
    return vec2f(d, level * (1.0 / f32(max(norm, 1u))));
}

fn mandelbulb(p: vec3f, power: f32, iters: u32) -> vec2f {
    var z = p;
    var dr = 1.0;
    var r = length(z);
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        if (r >= 2.0) {
            break;
        }
        let rs = max(r, 1e-6);
        let theta = atan2(length(z.xz), z.y) * power;
        let phi = atan2(z.z, z.x) * power;
        let rp1 = pow(rs, power - 1.0);
        dr = rp1 * dr * power + 1.0;
        let zr = rp1 * rs;
        let st = sin(theta);
        z = vec3f(zr * st * cos(phi), zr * cos(theta), zr * st * sin(phi)) + p;
        r = length(z);
        trap = min(trap, r);
    }
    let rr = max(r, 1e-6);
    return vec2f(log(rr) * rr * 0.5 / dr, clamp(trap * 0.5, 0.0, 1.0));
}

fn mandelbox(p: vec3f, scale: f32, min_r: f32, iters: u32) -> vec2f {
    let min2 = min_r * min_r;
    var z = p;
    var dr = 1.0;
    var trap = 1e9;
    for (var i = 0u; i < iters; i++) {
        z = clamp(z, vec3f(-1.0), vec3f(1.0)) * 2.0 - z;
        let r2 = dot(z, z);
        trap = min(trap, r2);
        var k = 1.0;
        if (r2 < min2) {
            k = 1.0 / min2;
        } else if (r2 < 1.0) {
            k = 1.0 / max(r2, 1e-12);
        }
        z = z * k;
        dr *= k;
        z = z * scale + p;
        dr = dr * abs(scale) + 1.0;
    }
    return vec2f(length(z) / abs(dr), clamp(sqrt(trap), 0.0, 1.0));
}

fn julia(p: vec3f, c: vec4f, iters: u32) -> vec2f {
    var z = vec4f(p, 0.0);
    var md2 = 1.0;
    var mz2 = dot(z, z);
    var trap = mz2;
    for (var i = 0u; i < iters; i++) {
        if (mz2 >= 16.0) {
            break;
        }
        md2 = md2 * mz2 * 4.0;
        z = vec4f(z.x * z.x - z.y * z.y - z.z * z.z - z.w * z.w + c.x, 2.0 * z.x * z.y + c.y, 2.0 * z.x * z.z + c.z, 2.0 * z.x * z.w + c.w);
        mz2 = dot(z, z);
        trap = min(trap, mz2);
    }
    let m = max(mz2, 1e-12);
    return vec2f(sqrt(m / md2) * log(m) * 0.25, clamp(sqrt(trap), 0.0, 1.0));
}

fn sierpinski(p: vec3f, iters: u32, norm: u32) -> vec2f {
    var z = p;
    var folds = 0.0;
    for (var i = 0u; i < iters; i++) {
        if (z.x + z.y < 0.0) {
            z = vec3f(-z.y, -z.x, z.z);
            folds += 1.0;
        }
        if (z.x + z.z < 0.0) {
            z = vec3f(-z.z, z.y, -z.x);
            folds += 1.0;
        }
        if (z.y + z.z < 0.0) {
            z = vec3f(z.x, -z.z, -z.y);
            folds += 1.0;
        }
        z = z * 2.0 - vec3f(1.0);
    }
    let t = max(abs(z.x + z.y) - z.z, abs(z.x - z.y) + z.z) - 1.0;
    return vec2f(t * 0.57735026 * pow(0.5, f32(iters)), folds * (1.0 / f32(3u * max(norm, 1u))));
}

/// (distance, orbit trap) of iq's Apollonian packing (render/vm.rs).
fn apollonian(p: vec3f, fold: f32, iters: u32) -> vec2f {
    var z = p;
    var scale = 1.0;
    var orb = 1e9;
    for (var i = 0u; i < iters; i++) {
        let h = z * 0.5 + vec3f(0.5);
        z = (h - floor(h)) * 2.0 - vec3f(1.0);
        let r2 = max(dot(z, z), 1e-12);
        orb = min(orb, r2);
        let k = fold / r2;
        z = z * k;
        scale *= k;
    }
    return vec2f(abs(z.y) * 0.25 / scale, clamp(orb * 6.0, 0.0, 1.0));
}

/// (distance, orbit trap) of the kaleidoscopic IFS (render/vm.rs kifs).
fn kifs(p: vec3f, s: f32, off: vec3f, r0: vec3f, r1: vec3f, r2: vec3f, iters: u32) -> vec2f {
    var z = p;
    var scale = 1.0;
    var orb = 1e9;
    let k = s - 1.0;
    for (var i = 0u; i < iters; i++) {
        z = abs(z);
        if (z.x < z.y) {
            z = vec3f(z.y, z.x, z.z);
        }
        if (z.x < z.z) {
            z = vec3f(z.z, z.y, z.x);
        }
        if (z.y < z.z) {
            z = vec3f(z.x, z.z, z.y);
        }
        z = vec3f(dot(r0, z), dot(r1, z), dot(r2, z));
        z = z * s - off * k;
        if (z.z < -0.5 * off.z * k) {
            z.z += off.z * k;
        }
        scale *= s;
        orb = min(orb, dot(z, z) * (1.0 / (scale * scale)));
    }
    let q = abs(z) - vec3f(1.0);
    let boxd = length(max(q, vec3f(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0);
    return vec2f(boxd * (1.0 / scale), clamp(sqrt(orb) * 1.5, 0.0, 1.0));
}

/// (distance, orbit trap) of the pseudo-Kleinian (render/vm.rs kleinian).
fn kleinian(p: vec3f, cbox: vec3f, size: f32, iters: u32) -> vec2f {
    var z = p;
    var total = 1.0;
    var orb = 1e9;
    for (var i = 0u; i < iters; i++) {
        z = clamp(z, -cbox, cbox) * 2.0 - z;
        let r2 = max(dot(z, z), 1e-12);
        orb = min(orb, r2);
        let k = max(size / r2, 1.0);
        z = z * k;
        total *= k;
    }
    let rxy = length(z.xy);
    let d = max(rxy - 0.92784, abs(rxy * z.z) / max(length(z), 1e-12));
    return vec2f(d / total, clamp(sqrt(orb), 0.0, 1.0));
}

// ---- generated: prog_dist(k, base, p), prog_tag(k, base, p) -> DT, HAS_NURBS
//GENERATED

// ---- rays and objects (render/trace.rs) ------------------------------------

struct Ray {
    o: vec3f,
    /// the origin's offset from the frame's anchor, the camera (render/trace.rs Ray.od)
    od: vec3f,
    d: vec3f,
    inv: vec3f,
    inside: i32,
    /// how near a hit on an exact shape may be (render/trace.rs Ray.near)
    near: f32,
}

fn make_ray(o: vec3f, od: vec3f, d: vec3f, inside: i32) -> Ray {
    let inv = vec3f(1.0) / (sign_nz(d) * max(abs(d), vec3f(1e-12)));
    return Ray(o, od, d, inv, inside, grain(length(o)) * 0.25);
}

fn sign_nz(v: vec3f) -> vec3f {
    return select(vec3f(1.0), vec3f(-1.0), v < vec3f(0.0));
}

struct Hit {
    t: f32,
    obj: i32,
    steps: u32,
}

fn slab(r: Ray, bmin: vec3f, bmax: vec3f) -> vec2f {
    let t1 = (bmin - r.o) * r.inv;
    let t2 = (bmax - r.o) * r.inv;
    let tn = min(t1, t2);
    let tf = max(t1, t2);
    return vec2f(max(tn.x, max(tn.y, tn.z)), min(tf.x, min(tf.y, tf.z)));
}

fn obj_base(i: u32) -> u32 {
    return F.off_objects + i * OBJ;
}

fn obj_kind(i: u32) -> u32 {
    return u(obj_base(i));
}

fn obj_mat(i: u32) -> u32 {
    return u(obj_base(i) + 1u);
}

fn obj_flags(i: u32) -> u32 {
    return u(obj_base(i) + 2u);
}

fn obj_bmin(i: u32) -> vec3f {
    return v3(obj_base(i) + 8u);
}

fn obj_bmax(i: u32) -> vec3f {
    return v3(obj_base(i) + 12u);
}

/// The rows of a 3x3 matrix stored at `b` (3 vec4, xyz used).
fn row(b: u32, k: u32) -> vec3f {
    return v3(b + k * 4u);
}

fn box_local(i: u32, v: vec3f) -> vec3f {
    let b = obj_base(i) + 24u;
    return vec3f(dot(v, row(b, 0u)), dot(v, row(b, 1u)), dot(v, row(b, 2u)));
}

/// Where a distance program's object stands (words 16..18): its program
/// takes points less this (scene/mod.rs Object.org).
fn obj_org(i: u32) -> vec3f {
    return v3(obj_base(i) + 16u);
}

/// The detail its fractals resolve this frame (word 19, written each frame:
/// render/mod.rs fractal_detail), to set LN_THR before evaluating it.
fn obj_lod(i: u32) -> f32 {
    return S[obj_base(i) + 19u];
}

/// Object `i`'s anchor table this frame (its first word after the header
/// that says whether it has one now), 0 for none (render/mod.rs Ctx::prepare).
fn obj_anc(i: u32) -> u32 {
    let t = u(obj_base(i) + 20u);
    return select(0u, t + 1u, t != 0u && S[t] != 0.0);
}

/// Readies the evaluation of object `i`: its detail and anchor table.
fn prepare(i: u32) {
    LN_THR = obj_lod(i);
    ANC = obj_anc(i);
}

/// Every leaf of object `i` is anchored and it has a table now: its floor is
/// that of the offset from the camera (sdf::Program::anchored).
fn obj_fine(i: u32) -> bool {
    return ANC != 0u && u(obj_base(i) + 21u) != 0u;
}

/// An anchored slice's distance (render/vm.rs Op::Slice): the anchor's plus
/// the offset's along its normal.
fn anc_slice(at: u32) -> f32 {
    let t = ANC + at;
    return dot(DP, vec3f(S[t], S[t + 1u], S[t + 2u])) + S[t + 3u];
}

fn sdf_of(i: u32, p: vec3f) -> f32 {
    let b = obj_base(i);
    return prog_dist(u(b + 3u), u(b + 4u), p);
}

fn sdf_tag_of(i: u32, p: vec3f) -> DT {
    let b = obj_base(i);
    return prog_tag(u(b + 3u), u(b + 4u), p);
}

// ---- NURBS surfaces (scene/nurbs.rs, render/nurbs.rs) -----------------------
// Object words 16..19: the centre and scale, 20..23: the patches, pieces and
// nodes in S and the flags (1 solid, 2 normals reversed), 24..35: the rotation
// rows (world to own frame), header word 7: the surface's size.
// A patch is 16 homogeneous control points; a piece 12 words (patch, u0, u1,
// v0, v1, box min, box max); a node 8 (as the scene's BVH nodes).

const NB_ITER: u32 = 10u;
const NB_PIECE: u32 = 12u;

struct PatchPoint {
    s: vec3f,
    su: vec3f,
    sv: vec3f,
}

fn nb_bern(t: f32) -> vec4f {
    let s = 1.0 - t;
    return vec4f(s * s * s, 3.0 * t * s * s, 3.0 * t * t * s, t * t * t);
}

fn nb_dbern(t: f32) -> vec4f {
    let s = 1.0 - t;
    return vec4f(-3.0 * s * s, 3.0 * s * s - 6.0 * t * s, 6.0 * t * s - 3.0 * t * t, 3.0 * t * t);
}

fn nb_eval(pb: u32, u_: f32, v_: f32) -> PatchPoint {
    let bu = nb_bern(u_);
    let du = nb_dbern(u_);
    let bv = nb_bern(v_);
    let dv = nb_dbern(v_);
    var h = vec4f(0.0);
    var hu = vec4f(0.0);
    var hv = vec4f(0.0);
    for (var i = 0u; i < 4u; i++) {
        var r = vec4f(0.0);
        var rv = vec4f(0.0);
        for (var j = 0u; j < 4u; j++) {
            let c = S4[(pb >> 2u) + i * 4u + j];
            r += bv[j] * c;
            rv += dv[j] * c;
        }
        h += bu[i] * r;
        hu += du[i] * r;
        hv += bu[i] * rv;
    }
    let iw = 1.0 / h.w;
    let p = h.xyz * iw;
    return PatchPoint(p, (hu.xyz - p * hu.w) * iw, (hv.xyz - p * hv.w) * iw);
}

/// The outward unit normal; at a pole, the one a little toward the piece's centre.
fn nb_normal(pb: u32, cu: f32, cv: f32, u_: f32, v_: f32, su: vec3f, sv: vec3f, orient: f32) -> vec3f {
    var n = cross(su, sv);
    let span = dot(su, su) + dot(sv, sv);
    if (dot(n, n) <= span * span * 1e-6) {
        let e = nb_eval(pb, (cu - u_) * 0.02 + u_, (cv - v_) * 0.02 + v_);
        n = cross(e.su, e.sv);
    }
    return n * (inverseSqrt(max(dot(n, n), 1e-30)) * orient);
}

/// Newton's method on (u, v) for where the patch meets the ray given as two
/// planes: (u, v, 1 when it converged, the iterations it took).
fn nb_newton(pb: u32, u0: f32, v0: f32, n1: vec3f, n2: vec3f, d1: f32, d2: f32, eps: f32) -> vec4f {
    var u_ = u0;
    var v_ = v0;
    for (var k = 0u; k < NB_ITER; k++) {
        let e = nb_eval(pb, u_, v_);
        let f1 = dot(e.s, n1) + d1;
        let f2 = dot(e.s, n2) + d2;
        if (abs(f1) + abs(f2) < eps) {
            return vec4f(u_, v_, 1.0, f32(k + 1u));
        }
        let j11 = dot(e.su, n1);
        let j12 = dot(e.sv, n1);
        let j21 = dot(e.su, n2);
        let j22 = dot(e.sv, n2);
        let det = j11 * j22 - j12 * j21;
        if (abs(det) <= 1e-20) {
            return vec4f(u_, v_, 0.0, f32(k + 1u));
        }
        let inv = 1.0 / det;
        u_ = u_ - (j22 * f1 - j12 * f2) * inv;
        v_ = v_ - (j11 * f2 - j21 * f1) * inv;
        if (u_ < -0.5 || u_ > 1.5 || v_ < -0.5 || v_ > 1.5) {
            return vec4f(u_, v_, 0.0, f32(k + 1u));
        }
    }
    return vec4f(u_, v_, 0.0, f32(NB_ITER));
}

fn nb_inside(x: f32, lo: f32, hi: f32, m: f32) -> bool {
    return x >= max(lo - m, -1e-4) && x <= min(hi + m, 1.0 + 1e-4);
}

struct NbHit {
    t: f32,
    /// outward unit normal, in the object's frame
    n: vec3f,
    /// Newton iterations (the march-steps view's measure)
    steps: u32,
}

/// The nearest hit on NURBS object `i` between `near` and `limit` (world
/// metres; t = BIG on a miss); `any` takes the first one found (shadows).
fn nb_hit(i: u32, r: Ray, limit: f32, near: f32, want_normal: bool, any: bool) -> NbHit {
    let b = obj_base(i);
    let sc = S[b + 19u];
    let k = 1.0 / sc;
    let o = box_local(i, r.o - v3(b + 16u)) * k;
    let d = box_local(i, r.d);
    let lr = make_ray(o, o, d, -1);
    let off_p = u(b + 20u);
    let off_pc = u(b + 21u);
    let off_n = u(b + 22u);
    let orient = select(1.0, -1.0, (u(b + 23u) & 2u) != 0u);
    let xm = abs(d.x) > abs(d.y) && abs(d.x) > abs(d.z);
    let n1 = normalize(select(vec3f(0.0, d.z, -d.y), vec3f(d.y, -d.x, 0.0), xm));
    let n2 = cross(n1, d);
    let d1 = -dot(n1, o);
    let d2 = -dot(n2, o);
    let eps = 2e-5 * max(S[b + 7u], 1.0);
    let nl = near * k;
    var best = limit * k;
    var found = BIG;
    var nrm = vec3f(0.0);
    var steps = 0u;
    var stack: array<u32, 48>;
    var sp = 1u;
    stack[0] = 0u;
    while (sp > 0u) {
        sp -= 1u;
        let nb = off_n + stack[sp] * 8u;
        let sl = slab(lr, v3(nb), v3(nb + 4u));
        if (!(sl.x <= sl.y && sl.y >= nl && sl.x < best)) {
            continue;
        }
        let count = u(nb + 3u);
        let index = u(nb + 7u);
        if (count == 0u) {
            if (sp + 2u <= 48u) {
                stack[sp] = index + 1u;
                stack[sp + 1u] = index;
                sp += 2u;
            }
            continue;
        }
        for (var q = 0u; q < count; q++) {
            let pc = off_pc + (index + q) * NB_PIECE;
            // only the pieces the ray crosses before its nearest hit so far (render/nurbs.rs)
            let ps = slab(lr, v3(pc + 5u), v3(pc + 8u));
            if (!(ps.x <= ps.y && ps.y >= nl && ps.x < best)) {
                continue;
            }
            let pb = off_p + u(pc) * 64u;
            let u0 = S[pc + 1u];
            let u1 = S[pc + 2u];
            let v0 = S[pc + 3u];
            let v1 = S[pc + 4u];
            let w = u1 - u0;
            let h = v1 - v0;
            // from the centre; a grazing ray may cross the piece twice, so it
            // looks for a nearer root from four more starts (render/nurbs.rs)
            var graze = false;
            for (var st = 0u; st < 5u; st++) {
                if (st > 0u && !graze) {
                    break;
                }
                // (0.5, 0.5), then (0.25 | 0.75, 0.25 | 0.75): no indexed array (local memory)
                let fu = select(select(0.25, 0.75, ((st - 1u) & 1u) != 0u), 0.5, st == 0u);
                let fv = select(select(0.25, 0.75, st >= 3u), 0.5, st == 0u);
                let r0 = nb_newton(pb, u0 + w * fu, v0 + h * fv, n1, n2, d1, d2, eps);
                steps += u32(r0.w);
                if (r0.z < 0.5) {
                    continue;
                }
                let e = nb_eval(pb, r0.x, r0.y);
                let t = dot(e.s - o, d);
                let here = nb_inside(r0.x, u0, u1, w * 0.5) && nb_inside(r0.y, v0, v1, h * 0.5) && t > nl;
                if (here && t < best) {
                    if (any) {
                        return NbHit(t * sc, nrm, steps);
                    }
                    best = t;
                    found = t;
                    if (want_normal) {
                        nrm = nb_normal(pb, u0 + w * 0.5, v0 + h * 0.5, r0.x, r0.y, e.su, e.sv, orient);
                    }
                }
                if (st == 0u) {
                    let nn = cross(e.su, e.sv);
                    let c = dot(nn, d);
                    graze = here && c * c < dot(nn, nn) * 0.15;
                }
            }
        }
    }
    return NbHit(select(BIG, found * sc, found < BIG * 0.5), nrm, steps);
}

fn nb_box_dist(p: vec3f, bmin: vec3f, bmax: vec3f) -> f32 {
    return length(max(max(bmin - p, p - bmax), vec3f(0.0)));
}

/// Whether to refine inside a box nearer than `best`: a small far one gives
/// its own distance instead (render/nurbs.rs FAR; only AO asks here).
fn nb_near(q: vec3f, bmin: vec3f, bmax: vec3f, best: ptr<function, f32>, sd: ptr<function, f32>, coarse: bool) -> bool {
    let bd = nb_box_dist(q, bmin, bmax);
    if (bd >= *best) {
        return false;
    }
    if (coarse && length(bmax - bmin) < bd * 0.5) {
        *best = bd;
        *sd = bd;
        return false;
    }
    return true;
}

/// The distance from `p` to NURBS object `i`, at most `radius` (world
/// metres; signed when the surface is solid), roughly from afar (AO).
fn nb_dist(i: u32, p: vec3f, radius: f32) -> f32 {
    let b = obj_base(i);
    let sc = S[b + 19u];
    let k = 1.0 / sc;
    let q = box_local(i, p - v3(b + 16u)) * k;
    return nb_dist_at(u(b + 20u), u(b + 21u), u(b + 22u), u(b + 23u), q, radius * k, true) * sc;
}

/// A NURBS leaf of a distance program (in a boolean, under a modifier): the
/// signed distance at `q` (its own frame) to the surface whose patches,
/// pieces, nodes and distance grid begin at those words (render/nurbs.rs
/// sdf8). Outside the grid the distance to its box; in it but away from the
/// surface, the grid's less its slack; near it, exact from the pieces within
/// the grid's estimate.
fn nb_sdf(off_p: u32, off_pc: u32, off_n: u32, fl: u32, off_g: u32, q: vec3f) -> f32 {
    let lo = v3(off_g);
    let n = u(off_g + 3u);
    let cell = v3(off_g + 4u);
    let slack = S[off_g + 7u];
    let top = f32(n - 1u);
    if (nb_box_dist(q, lo, lo + cell * top) > 0.0) {
        return nb_box_dist(q, v3(off_n), v3(off_n + 4u));
    }
    let fq = clamp((q - lo) / cell, vec3f(0.0), vec3f(top - 1e-4));
    let i = vec3u(fq);
    let t = fq - vec3f(i);
    let g = off_g + 8u;
    let at = (i.z * n + i.y) * n + i.x;
    let row = n;
    let plane = n * n;
    let c00 = mix(S[g + at], S[g + at + 1u], t.x);
    let c10 = mix(S[g + at + row], S[g + at + row + 1u], t.x);
    let c01 = mix(S[g + at + plane], S[g + at + plane + 1u], t.x);
    let c11 = mix(S[g + at + plane + row], S[g + at + plane + row + 1u], t.x);
    let v = mix(mix(c00, c10, t.y), mix(c01, c11, t.y), t.z);
    if (abs(v) > 3.0 * slack) {
        return v;
    }
    return nb_dist_at(off_p, off_pc, off_n, fl, q, abs(v) + 1.05 * slack, false);
}

/// The distance at `q` (the surface's frame) to the surface at those words,
/// at most `radius`, signed when it is solid; `coarse` (AO) lets a small far
/// box stand in for what it holds (unsigned).
fn nb_dist_at(off_p: u32, off_pc: u32, off_n: u32, fl: u32, q: vec3f, radius: f32, coarse: bool) -> f32 {
    let orient = select(1.0, -1.0, (fl & 2u) != 0u);
    var best = radius;
    var sd = best;
    var stack: array<u32, 48>;
    var sp = 1u;
    stack[0] = 0u;
    while (sp > 0u) {
        sp -= 1u;
        let nb = off_n + stack[sp] * 8u;
        if (!nb_near(q, v3(nb), v3(nb + 4u), &best, &sd, coarse)) {
            continue;
        }
        let count = u(nb + 3u);
        let index = u(nb + 7u);
        if (count == 0u) {
            let lb = off_n + index * 8u;
            let rb = lb + 8u;
            let near_left = nb_box_dist(q, v3(lb), v3(lb + 4u)) <= nb_box_dist(q, v3(rb), v3(rb + 4u));
            if (sp + 2u <= 48u) {
                stack[sp] = select(index, index + 1u, near_left);
                stack[sp + 1u] = select(index + 1u, index, near_left);
                sp += 2u;
            }
            continue;
        }
        for (var c = 0u; c < count; c++) {
            let pc = off_pc + (index + c) * NB_PIECE;
            if (!nb_near(q, v3(pc + 5u), v3(pc + 8u), &best, &sd, coarse)) {
                continue;
            }
            let pb = off_p + u(pc) * 64u;
            let u0 = S[pc + 1u];
            let u1 = S[pc + 2u];
            let v0 = S[pc + 3u];
            let v1 = S[pc + 4u];
            var uu = (u0 + u1) * 0.5;
            var vv = (v0 + v1) * 0.5;
            // one Gauss-Newton step toward the nearest point (render/nurbs.rs PROJECT)
            let e0 = nb_eval(pb, uu, vv);
            let r0 = e0.s - q;
            let g1 = dot(e0.su, r0);
            let g2 = dot(e0.sv, r0);
            let aa = dot(e0.su, e0.su);
            let bb = dot(e0.su, e0.sv);
            let cc = dot(e0.sv, e0.sv);
            let det = max(aa * cc - bb * bb, 1e-30);
            uu = clamp(uu - (cc * g1 - bb * g2) / det, u0, u1);
            vv = clamp(vv - (aa * g2 - bb * g1) / det, v0, v1);
            let e = nb_eval(pb, uu, vv);
            let rr = q - e.s;
            let dd = length(rr);
            if (dd >= best) {
                continue;
            }
            best = dd;
            sd = dd;
            if ((fl & 1u) != 0u && dot(rr, nb_normal(pb, (u0 + u1) * 0.5, (v0 + v1) * 0.5, uu, vv, e.su, e.sv, orient)) < 0.0) {
                sd = -dd;
            }
        }
    }
    return sd;
}

// ---- terrain (scene/terrain.rs, render/trace.rs) ----------------------------

struct Cell {
    idx: u32,
    fx: f32,
    fz: f32,
}

fn ter_base(k: u32) -> u32 {
    return F.off_terrains + k * TER;
}

fn ter_cell(k: u32, x: f32, z: f32) -> Cell {
    let b = ter_base(k);
    let n = u(b);
    let lim = f32(n - 1u) - 0.001;
    let cx = clamp((x - S[b + 3u]) * S[b + 2u], 0.0, lim);
    let cz = clamp((z - S[b + 4u]) * S[b + 2u], 0.0, lim);
    let ix = floor(cx);
    let iz = floor(cz);
    return Cell(u32(ix) + u32(iz) * n, cx - ix, cz - iz);
}

fn ter_height(k: u32, x: f32, z: f32) -> f32 {
    let b = ter_base(k);
    let n = u(b);
    let o = u(b + 8u);
    let c = ter_cell(k, x, z);
    let a0 = T[o + c.idx];
    let a1 = T[o + c.idx + 1u];
    let top = (a1 - a0) * c.fx + a0;
    let b0 = T[o + c.idx + n];
    let b1 = T[o + c.idx + n + 1u];
    let bot = (b1 - b0) * c.fx + b0;
    return (bot - top) * c.fz + top;
}

fn unpack_n(v: f32) -> vec2f {
    let i = bitcast<i32>(v);
    return vec2f(f32((i << 16u) >> 16u), f32(i >> 16u));
}

/// Unit normal and cavity AO.
fn ter_normal(k: u32, x: f32, z: f32) -> vec4f {
    let b = ter_base(k);
    let n = u(b);
    let o = u(b + 9u);
    let c = ter_cell(k, x, z);
    let n00 = unpack_n(T[o + c.idx]);
    let n10 = unpack_n(T[o + c.idx + 1u]);
    let n01 = unpack_n(T[o + c.idx + n]);
    let n11 = unpack_n(T[o + c.idx + n + 1u]);
    let top = (n10 - n00) * c.fx + n00;
    let bot = (n11 - n01) * c.fx + n01;
    let v = ((bot - top) * c.fz + top) * (1.0 / 32767.0);
    let ny = sqrt(max(1.0 - v.x * v.x - v.y * v.y, 0.05));
    return vec4f(v.x, ny, v.y, T[u(b + 10u) + c.idx]);
}

fn march_terrain(k: u32, r: Ray, t0: f32, t_end: f32, max_steps: u32, steps: ptr<function, u32>) -> f32 {
    let b = ter_base(k);
    let cell = S[b + 1u];
    let relax = S[b + 5u];
    let step0 = 0.05 * cell;
    let step1 = 0.0025;
    var t = t0;
    var t_prev = t0;
    var h_prev = BIG;
    var result = BIG;
    var b_tp = 0.0;
    var b_hp = 0.0;
    var b_t = 0.0;
    var b_hh = 0.0;
    for (var i = 0u; i < max_steps; i++) {
        let p = r.o + r.d * t;
        let dh = p.y - ter_height(k, p.x, p.z);
        if (dh < 0.0) {
            let frac = min(h_prev / max(h_prev - dh, 1e-20), 1.0);
            result = frac * (t - t_prev) + t_prev;
            b_tp = t_prev;
            b_hp = h_prev;
            b_t = t;
            b_hh = dh;
            break;
        }
        *steps += 1u;
        if (t >= t_end) {
            break;
        }
        t_prev = t;
        h_prev = dh;
        t += max(dh * relax, t * step1 + step0);
    }
    if (result >= BIG * 0.5) {
        return result;
    }
    for (var j = 0; j < 2; j++) {
        let p = r.o + r.d * result;
        let dh = p.y - ter_height(k, p.x, p.z);
        if (dh < 0.0) {
            b_t = result;
            b_hh = dh;
        } else {
            b_tp = result;
            b_hp = dh;
        }
        let frac = min(b_hp / max(b_hp - b_hh, 1e-20), 1.0);
        result = frac * (b_t - b_tp) + b_tp;
    }
    return result;
}

fn march_sdf(i: u32, r: Ray, t0: f32, end: f32, sgn: f32, cone: f32, steps: ptr<function, u32>) -> f32 {
    let step = S[obj_base(i) + 5u];
    // (from the object's position: Object.org)
    let ro = r.o - obj_org(i);
    prepare(i);
    // (anchored: as fine as the offset from the camera)
    let g = grain(length(select(ro, r.od, obj_fine(i))));
    var t = t0;
    for (var k = 0u; k < F.max_steps; k++) {
        let thr = max(t * cone * THR_K, t * REL + g);
        if (ANC != 0u) {
            DP = r.od + r.d * t;
        }
        let d = sdf_of(i, ro + r.d * t) * sgn;
        if (d < thr) {
            return t;
        }
        *steps += 1u;
        t = d * step + t;
        if (t >= end) {
            break;
        }
    }
    return BIG;
}

/// Intersects object `i`, keeping a nearer hit.
fn object(i: u32, r: Ray, hit: ptr<function, Hit>, cone: f32) {
    let b = obj_base(i);
    let kind = u(b);
    let near = r.near;
    var t = BIG;
    var ok = false;
    if (kind == K_SPHERE) {
        let a = v3(b + 16u);
        let rad = S[b + 19u];
        let oc = r.o - a;
        let bb = dot(oc, r.d);
        let h = bb * bb - (dot(oc, oc) - rad * rad);
        let sq = sqrt(max(h, 0.0));
        let t0 = -bb - sq;
        let t1 = sq - bb;
        t = select(t1, t0, t0 > near);
        ok = h > 0.0 && t > near;
    } else if (kind == K_BOX) {
        let lo = box_local(i, r.o - v3(b + 16u));
        let ld = box_local(i, r.d);
        let lr = make_ray(lo, r.od, ld, r.inside);
        let half = v3(b + 20u);
        let s = slab(lr, -half, half);
        t = select(s.y, s.x, s.x > near);
        ok = s.x <= s.y && t > near;
    } else if (kind == K_PLANE) {
        let n = v3(b + 16u);
        let dn = dot(r.d, n);
        t = (S[b + 19u] - dot(r.o, n)) / dn;
        ok = t > near && abs(dn) > 1e-9;
    } else if (HAS_NURBS && kind == K_NURBS) {
        let nh = nb_hit(i, r, (*hit).t, 1e-5, false, false);
        t = nh.t;
        (*hit).steps += nh.steps;
        ok = t < BIG * 0.5;
    } else if (kind == K_SDF) {
        let s = slab(r, obj_bmin(i), obj_bmax(i));
        let t0 = max(s.x, 0.0);
        let end = min(s.y, (*hit).t);
        if (t0 < end) {
            let sgn = select(1.0, -1.0, r.inside == i32(i));
            var st = 0u;
            t = march_sdf(i, r, t0, end, sgn, cone, &st);
            (*hit).steps += st;
            ok = t < BIG * 0.5;
        }
    } else {
        let s = slab(r, obj_bmin(i), obj_bmax(i));
        let t0 = max(s.x, 0.0);
        let end = min(s.y, (*hit).t);
        if (t0 < end) {
            var st = 0u;
            t = march_terrain(u(b + 6u), r, t0, end, F.max_steps + 60u, &st);
            (*hit).steps += st;
            ok = t < BIG * 0.5;
        }
    }
    if (ok && t < (*hit).t) {
        (*hit).t = t;
        (*hit).obj = i32(i);
    }
}

fn node_min(n: u32) -> vec3f {
    return v3(F.off_nodes + n * 8u);
}

fn node_max(n: u32) -> vec3f {
    return v3(F.off_nodes + n * 8u + 4u);
}

fn node_count(n: u32) -> u32 {
    return u(F.off_nodes + n * 8u + 3u);
}

fn node_index(n: u32) -> u32 {
    return u(F.off_nodes + n * 8u + 7u);
}

fn reach(s: vec2f, limit: f32) -> bool {
    return s.x <= s.y && s.y >= 0.0 && s.x < limit;
}

/// The nearest hit before `tmax` (the unbounded objects, then the BVH, nearer child first).
fn intersect(r: Ray, tmax: f32, cone: f32) -> Hit {
    var hit = Hit(tmax, -1, 0u);
    for (var k = 0u; k < F.n_unbounded; k++) {
        object(u(F.off_unbounded + k), r, &hit, cone);
    }
    if (F.n_nodes == 0u) {
        return hit;
    }
    var stack: array<u32, 64>;
    var sp = 1u;
    stack[0] = 0u;
    while (sp > 0u) {
        sp -= 1u;
        let n = stack[sp];
        if (!reach(slab(r, node_min(n), node_max(n)), hit.t)) {
            continue;
        }
        let count = node_count(n);
        let index = node_index(n);
        if (count > 0u) {
            for (var k = 0u; k < count; k++) {
                object(u(F.off_items + index + k), r, &hit, cone);
            }
        } else {
            let l = index;
            let rr = index + 1u;
            let ls = slab(r, node_min(l), node_max(l));
            let rs = slab(r, node_min(rr), node_max(rr));
            let lm = reach(ls, hit.t);
            let rm = reach(rs, hit.t);
            let near_left = select(BIG, ls.x, lm) <= select(BIG, rs.x, rm);
            let first = select(rr, l, near_left);
            let second = select(l, rr, near_left);
            let fm = select(rm, lm, near_left);
            let sm = select(lm, rm, near_left);
            if (sm && sp < 64u) {
                stack[sp] = second;
                sp += 1u;
            }
            if (fm && sp < 64u) {
                stack[sp] = first;
                sp += 1u;
            }
        }
    }
    return hit;
}

/// One object's shadow contribution: updates `vis`; true when it blocks outright.
fn shadow_object(i: u32, r: Ray, limit: f32, soft: bool, k: f32, vis: ptr<function, f32>) -> bool {
    let b = obj_base(i);
    let kind = u(b);
    let near = r.near;
    if (kind == K_SPHERE) {
        let a = v3(b + 16u);
        let rad = S[b + 19u];
        let oc = r.o - a;
        let bb = dot(oc, r.d);
        let h = bb * bb - (dot(oc, oc) - rad * rad);
        let sq = sqrt(max(h, 0.0));
        let t0 = -bb - sq;
        let t1 = sq - bb;
        let t = select(t1, t0, t0 > near);
        return h > 0.0 && t > near && t < limit;
    } else if (kind == K_BOX) {
        let lo = box_local(i, r.o - v3(b + 16u));
        let ld = box_local(i, r.d);
        let half = v3(b + 20u);
        let s = slab(make_ray(lo, r.od, ld, -1), -half, half);
        let t = select(s.y, s.x, s.x > near);
        return s.x <= s.y && t > near && t < limit;
    } else if (kind == K_PLANE) {
        let n = v3(b + 16u);
        let t = (S[b + 19u] - dot(r.o, n)) / dot(r.d, n);
        return t > near && t < limit;
    } else if (HAS_NURBS && kind == K_NURBS) {
        return nb_hit(i, r, limit, 1e-4, false, true).t < BIG * 0.5;
    } else if (kind == K_SDF) {
        let s = slab(r, obj_bmin(i), obj_bmax(i));
        let ro = r.o - obj_org(i);
        prepare(i);
        let g = grain(length(select(ro, r.od, obj_fine(i))));
        var t = max(s.x, 2.0 * g);
        let end = min(s.y, limit);
        let step = S[b + 5u];
        let max_steps = min(F.max_steps, 160u);
        for (var j = 0u; j < max_steps; j++) {
            if (t >= end || *vis <= 0.02) {
                break;
            }
            if (ANC != 0u) {
                DP = r.od + r.d * t;
            }
            let dist = sdf_of(i, ro + r.d * t);
            if (soft) {
                *vis = min(*vis, dist * k / t);
            } else if (dist < t * REL + g) {
                *vis = 0.0;
            }
            t += max(dist * step, min(t, 1.0) * 2e-4 + 2.0 * g);
        }
        return false;
    } else {
        let ter = u(b + 6u);
        let cell = S[ter_base(ter) + 1u];
        let s = slab(r, obj_bmin(i), obj_bmax(i));
        var t = max(s.x, 0.05 * cell);
        let end = min(s.y, limit);
        let kk = select(1.0e4, k * 0.4, soft);
        // (steps of at least 1.5% of the way: trace.rs)
        for (var j = 0; j < 96; j++) {
            if (t >= end || *vis <= 0.02) {
                break;
            }
            let p = r.o + r.d * t;
            let dh = p.y - ter_height(ter, p.x, p.z);
            *vis = min(*vis, dh * kk / t);
            let st = max(min(max(dh * 0.6, 0.6 * cell), 16.0 * cell), t * 0.015);
            t += st;
        }
        return false;
    }
}

/// Visibility 0..1 along a ray up to `tmax`; `skip` is an object to ignore.
fn shadow(o: vec3f, od: vec3f, d: vec3f, tmax: f32, soft: bool, k: f32, skip: i32) -> f32 {
    let r = make_ray(o, od, d, -1);
    var vis = 1.0;
    for (var q = 0u; q < F.n_unbounded; q++) {
        let i = u(F.off_unbounded + q);
        if ((obj_flags(i) & 1u) == 0u || i32(i) == skip) {
            continue;
        }
        if (shadow_object(i, r, tmax, soft, k, &vis)) {
            vis = 0.0;
        }
        if (vis <= 0.02) {
            return 0.0;
        }
    }
    if (F.n_nodes > 0u) {
        var stack: array<u32, 64>;
        var sp = 1u;
        stack[0] = 0u;
        while (sp > 0u) {
            sp -= 1u;
            let n = stack[sp];
            if (!reach(slab(r, node_min(n), node_max(n)), tmax)) {
                continue;
            }
            let count = node_count(n);
            let index = node_index(n);
            if (count > 0u) {
                for (var c = 0u; c < count; c++) {
                    let i = u(F.off_items + index + c);
                    if ((obj_flags(i) & 1u) == 0u || i32(i) == skip) {
                        continue;
                    }
                    if (shadow_object(i, r, tmax, soft, k, &vis)) {
                        vis = 0.0;
                    }
                    if (vis <= 0.02) {
                        return 0.0;
                    }
                }
            } else {
                let l = index;
                let rr = index + 1u;
                let ls = slab(r, node_min(l), node_max(l));
                let rs = slab(r, node_min(rr), node_max(rr));
                let lm = reach(ls, tmax);
                let rm = reach(rs, tmax);
                let near_left = select(BIG, ls.x, lm) <= select(BIG, rs.x, rm);
                if (near_left) {
                    if (rm && sp < 64u) { stack[sp] = rr; sp += 1u; }
                    if (lm && sp < 64u) { stack[sp] = l; sp += 1u; }
                } else {
                    if (lm && sp < 64u) { stack[sp] = l; sp += 1u; }
                    if (rm && sp < 64u) { stack[sp] = rr; sp += 1u; }
                }
            }
        }
    }
    let v = clamp(vis, 0.0, 1.0);
    let sv = v * v * (3.0 - (v + v));
    // clouds on the way dim it
    if (F.n_volumes == 0u || sv <= 0.0) {
        return sv;
    }
    return sv * cloud_transmittance(o, d, tmax);
}

// ---- volume clouds (render/volume.rs)

const VOL: u32 = 36u;
const VOL_STEPS: u32 = 40u;
const VOL_SHADOW_STEPS: u32 = 14u;
const BILLOW: f32 = 2.4;
const CLOUD_FILL: f32 = 1.2;

fn vol_local(b: u32, p: vec3f) -> vec3f {
    let q = p - v3(b + 8u);
    return vec3f(dot(q, v3(b + 12u)), dot(q, v3(b + 16u)), dot(q, v3(b + 20u)));
}

/// Distance to the surface of the puffs' smooth union.
fn vol_envelope(b: u32, q: vec3f) -> f32 {
    let k = max(S[b + 28u], 1e-4);
    var d = 1e9;
    let n = u(b + 30u);
    let o = u(b + 31u);
    for (var i = 0u; i < n; i++) {
        let pb = o + i * 4u;
        let e = length(q - v3(pb)) - S[pb + 3u];
        let h = clamp((e - d) * (0.5 / k) + 0.5, 0.0, 1.0);
        d = (d - e) * h + e - h * (1.0 - h) * k;
    }
    return d;
}

fn sstep01(x: f32) -> f32 {
    let t = clamp(x, 0.0, 1.0);
    return t * t * (3.0 - t - t);
}

const ROUGHNESS: f32 = 0.7;

/// The clouds' noise (render/volume.rs cloud_noise).
fn cloud_noise(p: vec3f, octaves: u32, full: u32, seed: u32) -> f32 {
    var sum = 0.0;
    var amp = 1.0;
    var q = p;
    for (var i = 0u; i < octaves; i++) {
        sum += perlin3(q, seed) * amp;
        q = vec3f(q.x * 1.6 + q.z * 1.2, q.y * 2.0, q.z * 1.6 - q.x * 1.2);
        amp *= ROUGHNESS;
    }
    let norm = (1.0 - pow(ROUGHNESS, f32(full))) / (1.0 - ROUGHNESS);
    return sum * (0.94 / norm);
}

fn vol_density(b: u32, q: vec3f, de: f32, octaves: u32) -> f32 {
    let turb = S[b + 7u];
    let n = cloud_noise(q * S[b + 15u], octaves, u(b + 19u), u(b + 23u));
    let d = de - n * (turb * BILLOW);
    let shape = sstep01(-d * (1.0 / S[b + 27u])) * sstep01((q.y - (S[b + 29u] - 0.8)) * (1.0 / 1.4));
    let dens = shape * (1.0 - clamp(0.5 - n, 0.0, 1.0) * min(turb, 1.0) * (1.0 - shape));
    let wisps = S[b + 32u];
    if (wisps <= 0.0) {
        return dens;
    }
    let m = cloud_noise(q * (S[b + 15u] * 2.7), octaves, u(b + 19u), u(b + 23u) ^ 0x9e3779b9u) + 0.5;
    let keep = sstep01((m - wisps * 0.7) * (1.0 / (0.1 + 0.25 * (1.0 - wisps))));
    let k = sqrt(wisps);
    return dens * (keep * k + (1.0 - k));
}

fn hg(g: f32, c: f32) -> f32 {
    let x = c * (-2.0 * g) + (1.0 + g * g);
    return (1.0 - g * g) / (x * sqrt(x));
}

/// The light the clouds scatter back along `r` before `tmax` (xyz) and the
/// fraction of what is behind them they let through (w).
fn cloud_march(r: Ray, tmax: f32) -> vec4f {
    var light = vec3f(0.0);
    var trans = 1.0;
    if (F.n_volumes == 0u) {
        return vec4f(light, trans);
    }
    var ldir = F.sun_dir;
    var lcol = F.sun_col;
    if (max(F.glow_col.x, max(F.glow_col.y, F.glow_col.z)) > max(F.sun_col.x, max(F.sun_col.y, F.sun_col.z))) {
        ldir = F.glow_dir;
        lcol = F.glow_col;
    }
    let c = dot(r.d, ldir);
    let phase = hg(0.6, c) * 0.7 + hg(-0.3, c) * 0.3;
    let fill = F.amb_col * (CLOUD_FILL * AMBIENT * F.ambient);
    let jitter = rnd();
    for (var i = 0u; i < F.n_volumes; i++) {
        let b = F.off_volumes + i * VOL;
        let tt = slab(r, v3(b), v3(b + 4u));
        let t0 = max(tt.x, 0.0);
        let t1 = min(tt.y, tmax);
        if (t0 >= t1 || trans <= 0.01) {
            continue;
        }
        let dt = (t1 - t0) * (1.0 / f32(VOL_STEPS));
        var t = jitter * dt + t0;
        let sigma = S[b + 3u];
        let scale = S[b + 11u];
        let reach = S[b + 7u] * BILLOW * 0.5;
        let col = v3(b + 24u);
        var here = vec3f(0.0);
        for (var k = 0u; k < VOL_STEPS; k++) {
            let p = r.o + r.d * t;
            let q = vol_local(b, p);
            let de = vol_envelope(b, q);
            if (de < reach) {
                let s = vol_density(b, q, de, u(b + 19u)) * sigma;
                if (s > 0.0) {
                    var tau = 0.0;
                    var at = 0.3;
                    var len = 0.6;
                    for (var j = 0u; j < 4u; j++) {
                        let ql = vol_local(b, ldir * (at * scale) + p);
                        tau += vol_density(b, ql, vol_envelope(b, ql), 2u) * sigma * (len * scale);
                        at += len * 1.5;
                        len *= 2.0;
                    }
                    let direct = exp(-tau) * phase;
                    let many = exp(tau * -0.2) * 0.3;
                    let up = sstep01((q.y - S[b + 29u]) * (1.0 / 6.0)) * 0.55 + 0.45;
                    let src = (lcol * (direct + many) + fill * up) * col;
                    let a = 1.0 - exp(-(s * dt));
                    here += src * (trans * a);
                    trans *= 1.0 - a;
                }
            }
            t += dt;
            if (t >= t1 || trans <= 0.01) {
                break;
            }
        }
        light += here * fog(t0, r.d).w;
    }
    return vec4f(light, trans);
}

/// The fraction of light the clouds let through from `o` along `d` (shadow
/// rays: coarser, no light).
fn cloud_transmittance(o: vec3f, d: vec3f, tmax: f32) -> f32 {
    let r = make_ray(o, o - F.cam_pos, d, -1);
    let jitter = unit(lowbias(u32(i32(o.x * 37.0 + o.z * 91.0 + o.y * 13.0))));
    var trans = 1.0;
    for (var i = 0u; i < F.n_volumes; i++) {
        let b = F.off_volumes + i * VOL;
        let tt = slab(r, v3(b), v3(b + 4u));
        let t0 = max(tt.x, 0.0);
        let t1 = min(tt.y, tmax);
        if (t0 >= t1) {
            continue;
        }
        let dt = (t1 - t0) * (1.0 / f32(VOL_SHADOW_STEPS));
        var t = jitter * dt + t0;
        let reach = S[b + 7u] * BILLOW * 0.5;
        var tau = 0.0;
        for (var k = 0u; k < VOL_SHADOW_STEPS; k++) {
            let q = vol_local(b, o + d * t);
            let de = vol_envelope(b, q);
            if (de < reach) {
                tau += vol_density(b, q, de, 2u) * S[b + 3u] * dt;
            }
            t += dt;
            if (t >= t1) {
                break;
            }
        }
        trans *= exp(-tau);
    }
    return trans;
}

/// One object's distance at `p` (AO).
fn obj_dist(i: u32, p: vec3f, dp: vec3f, skip_terrain: bool, radius: f32) -> f32 {
    let b = obj_base(i);
    let kind = u(b);
    if (kind == K_SPHERE) {
        return length(p - v3(b + 16u)) - S[b + 19u];
    } else if (kind == K_BOX) {
        let l = abs(box_local(i, p - v3(b + 16u))) - v3(b + 20u);
        return length(max(l, vec3f(0.0))) + min(max(l.x, max(l.y, l.z)), 0.0);
    } else if (kind == K_PLANE) {
        return dot(p, v3(b + 16u)) - S[b + 19u];
    } else if (kind == K_SDF) {
        prepare(i);
        DP = dp;
        return sdf_of(i, p - obj_org(i));
    } else if (HAS_NURBS && kind == K_NURBS) {
        return nb_dist(i, p, radius);
    }
    if (skip_terrain) {
        return BIG;
    }
    let ter = u(b + 6u);
    let n = ter_normal(ter, p.x, p.z);
    return (p.y - ter_height(ter, p.x, p.z)) * n.y;
}

fn inside_grown(p: vec3f, bmin: vec3f, bmax: vec3f, r: f32) -> bool {
    return all(p >= bmin - vec3f(r)) && all(p <= bmax + vec3f(r));
}

/// The scene's distance at `p`, at most `radius` (shadow-casting objects).
fn scene_dist(p: vec3f, dp: vec3f, radius: f32, skip_terrain: bool) -> f32 {
    var best = radius;
    for (var q = 0u; q < F.n_unbounded; q++) {
        let i = u(F.off_unbounded + q);
        if ((obj_flags(i) & 1u) != 0u) {
            best = min(best, obj_dist(i, p, dp, skip_terrain, best));
        }
    }
    if (F.n_nodes == 0u) {
        return best;
    }
    var stack: array<u32, 64>;
    var sp = 1u;
    stack[0] = 0u;
    while (sp > 0u) {
        sp -= 1u;
        let n = stack[sp];
        if (!inside_grown(p, node_min(n), node_max(n), radius)) {
            continue;
        }
        let count = node_count(n);
        let index = node_index(n);
        if (count > 0u) {
            for (var c = 0u; c < count; c++) {
                let i = u(F.off_items + index + c);
                if ((obj_flags(i) & 1u) != 0u) {
                    best = min(best, obj_dist(i, p, dp, skip_terrain, best));
                }
            }
        } else if (sp + 2u <= 64u) {
            stack[sp] = index;
            stack[sp + 1u] = index + 1u;
            sp += 2u;
        }
    }
    return best;
}

fn sdf_normal(i: u32, p: vec3f, dp: vec3f, eps: f32) -> vec3f {
    var ks = array<vec3f, 4>(vec3f(1.0, -1.0, -1.0), vec3f(-1.0, -1.0, 1.0), vec3f(-1.0, 1.0, -1.0), vec3f(1.0, 1.0, 1.0));
    var g = vec3f(0.0);
    for (var k = 0; k < 4; k++) {
        if (ANC != 0u) {
            DP = dp + ks[k] * eps;
        }
        g += ks[k] * sdf_of(i, p + ks[k] * eps);
    }
    return g * inverseSqrt(max(dot(g, g), 1e-30));
}

// ---- sky and fog (render/atmos.rs) -----------------------------------------

const NIGHT_ZEN = vec3f(0.0012, 0.0016, 0.0040);
const NIGHT_HOR = vec3f(0.0035, 0.0042, 0.0075);
const BAND = vec3f(0.36, 0.62, 0.70);
const BAND_COL = vec3f(0.0040, 0.0040, 0.0060);
const STAR_GRID = 45.0;

/// The night sky: its glow, the band and two layers of stars (atmos.rs).
fn stars(d: vec3f) -> vec3f {
    let up = sqrt(max(d.y, 0.0));
    let b = dot(d, BAND);
    let band = exp2(b * b * -40.0);
    let col = mix(NIGHT_HOR, NIGHT_ZEN, up) + BAND_COL * band;
    let turned = vec3f(dot(d, vec3f(0.36, 0.48, -0.80)), dot(d, vec3f(-0.80, 0.60, 0.0)), dot(d, vec3f(0.48, 0.64, 0.60)));
    return col + star_layer(d, STAR_GRID, 0u, band) + star_layer(turned, STAR_GRID * 0.83, 0x5f356495u, band);
}

fn star_layer(d: vec3f, grid: f32, seed: u32, band: f32) -> vec3f {
    let f = floor(d * grid);
    let h = lowbias((bitcast<u32>(i32(f.x)) * 0x8DA6B343u) ^ (bitcast<u32>(i32(f.y)) * 0xD8163841u) ^ (bitcast<u32>(i32(f.z)) * 0xCB1AB31Fu) ^ seed);
    if (unit(h) >= band * 0.3 + 0.25) {
        return vec3f(0.0);
    }
    let h2 = lowbias(h ^ 1u);
    let sd = normalize(vec3f(unit(h2), unit(lowbias(h ^ 2u)), unit(lowbias(h ^ 3u))) * 0.3 + (f + vec3f(0.35)));
    let r = sd * grid - f;
    if (any(r < vec3f(0.3)) || any(r > vec3f(0.7))) {
        return vec3f(0.0);
    }
    let u = unit(lowbias(h ^ 4u));
    let u2 = u * u;
    let u4 = u2 * u2;
    let bright = (u4 * u4) * 6.0 + (u2 * 0.15 + 0.02);
    let sigma = (u4 * 0.6 + 0.62) * max(F.cone, 1e-5);
    let dd = d - sd;
    let glow = exp2(dot(dd, dd) / (sigma * sigma) * -1.442695);
    let tint = mix(vec3f(1.0, 0.92, 0.82), vec3f(0.85, 0.92, 1.0), unit(h2));
    return tint * (glow * bright);
}

fn sky(d: vec3f, disk: bool) -> vec3f {
    if (on(F_SKY_OFF)) {
        return vec3f(0.0);
    }
    if (on(F_STARS)) {
        return stars(d);
    }
    let g = sqrt(max(d.y, 0.0));
    var col = mix(F.sky_hor, F.sky_zen, g);
    let haze = clamp(d.y * -8.0, 0.0, 1.0);
    col = mix(col, F.fog_col, haze);
    var alpha = 0.0;
    if (on(F_CLOUDS) && F.clouds > 0.0) {
        let t = 1000.0 / max(d.y, 0.02);
        let uu = t * d.x * 0.0012;
        let vv = t * d.z * 0.0012;
        let n1 = noised(uu, vv, 0x5bd1e995u).x;
        let n2 = noised(uu * 2.3, vv * 2.3, 0x5bd1e995u).x;
        let density = n2 * 0.5 + n1;
        let threshold = 1.26 - F.clouds * 2.0;
        alpha = clamp((density - threshold) * 2.5, 0.0, 0.85) * clamp(d.y * 6.0, 0.0, 1.0);
        var lit = max(dot(d, F.sun_dir), 0.0);
        lit = (lit * lit) * (lit * lit);
        let cloud = (F.sky_hor * 2.0 + F.sun_col * lit) * 0.6;
        col = mix(col, cloud, alpha);
    }
    // the sun's glow and disk (where the sun is, also after it set)
    let s = max(dot(d, F.glow_dir), 0.0);
    let s2 = s * s;
    let s4 = s2 * s2;
    let s8 = s4 * s4;
    let s16 = s8 * s8;
    let s32 = s16 * s16;
    let s64 = s32 * s32;
    var glow = s8 * 0.3;
    glow = s64 * 0.8 + glow;
    let show = disk && d.y > -0.01;
    if (show) {
        let s128 = s64 * s64;
        let s256 = s128 * s128;
        let s512 = s256 * s256;
        let s1024 = s512 * s512;
        glow = (s1024 * (1.0 - alpha)) * 12.0 + glow;
    }
    col += F.glow_col * glow;
    if (F.moon > 0.0) {
        col += moon_disk(d, show, alpha);
    }
    if (F.night > 0.0) {
        col += stars(d) * ((1.0 - alpha) * F.night * clamp(d.y * 20.0 + 0.2, 0.0, 1.0));
    }
    return col;
}

/// The moon (atmos.rs moon()).
fn moon_disk(d: vec3f, show: bool, alpha: f32) -> vec3f {
    let m = dot(d, F.moon_dir);
    let inside = clamp((1.0 - (1.0 - m) * (1.0 / 9.75e-5)) * 6.0, 0.0, 1.0);
    var out = vec3f(0.0);
    if (show && inside > 0.0) {
        let n = noised((d.x - F.moon_dir.x) * 900.0, (d.y - F.moon_dir.y) * 900.0 + (d.z - F.moon_dir.z) * 900.0, 0x2c1b3c6du).x;
        out = vec3f(4.0, 3.9, 3.6) * ((1.0 - n * 0.3) * inside);
    }
    let h = max(m, 0.0);
    let h4 = (h * h) * (h * h);
    let h16 = h4 * h4 * (h4 * h4);
    let halo = h16 * h16 * 0.02;
    return (out + vec3f(halo * 0.8, halo * 0.9, halo)) * ((1.0 - alpha) * F.moon);
}

/// Fog over `dist` along `dir`: transmittance (w) and in-scattered light (xyz).
fn fog(dist: f32, dir: vec3f) -> vec4f {
    if (!on(F_FOG) || F.fog <= 0.0) {
        return vec4f(0.0, 0.0, 0.0, 1.0);
    }
    let trans = exp2(dist * -F.fog);
    var lobe = max(dot(dir, F.sun_dir), 0.0);
    lobe = lobe * lobe;
    lobe = lobe * lobe;
    lobe = lobe * lobe * 0.4;
    let haze = F.fog_col + F.sun_col * lobe;
    return vec4f(haze * (1.0 - trans), trans);
}

// ---- patterns (render/pattern.rs) ------------------------------------------

const PK1: u32 = 0x8DA6B343u;
const PK2: u32 = 0xD8163841u;
const PK3: u32 = 0xCB1AB31Fu;

/// (f1, f2, id of the nearest cell)
fn voronoi(q: vec3f) -> vec3f {
    let c = floor(q);
    let f = q - c;
    let ix = i32(c.x);
    let iy = i32(c.y);
    let iz = i32(c.z);
    var f1 = 9.0;
    var f2 = 9.0;
    var id = 0.0;
    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let h = lowbias((bitcast<u32>(ix + dx) * PK1) ^ (bitcast<u32>(iy + dy) * PK2) ^ (bitcast<u32>(iz + dz) * PK3));
                let h2 = lowbias(h ^ 0x5bd1e995u);
                let h3 = lowbias(h2 ^ 0x27d4eb2fu);
                let px = unit(h) + f32(dx) - f.x;
                let py = unit(h2) + f32(dy) - f.y;
                let pz = unit(h3) + f32(dz) - f.z;
                let d = pz * pz + (py * py + px * px);
                if (d < f1) {
                    f2 = f1;
                    id = unit(lowbias(h3));
                } else {
                    f2 = min(f2, d);
                }
                f1 = min(f1, d);
            }
        }
    }
    return vec3f(sqrt(f1), sqrt(f2), id);
}

fn warped(q: vec3f, warp: f32) -> vec3f {
    let seed = 0x12345678u;
    var offs = array<vec3f, 3>(vec3f(0.0), vec3f(5.2, 1.3, 7.7), vec3f(9.1, 4.7, 2.3));
    var n = vec3f(0.0);
    for (var a = 0; a < 3; a++) {
        n[a] = fbm3(q + offs[a], 3u, seed);
    }
    return n * (warp * 0.8) + q;
}

/// The orbit pattern (render/shade.rs orbit).
fn orbit_blend(trap: f32, scale: f32, sharp: f32) -> f32 {
    let b = 0.5 - 0.5 * cos(trap * scale * PI);
    return clamp((b - 0.5) * sharp + 0.5, 0.0, 1.0);
}

/// The surface's main plane: (u, v) of `q` seen along the axis nearest the
/// normal (render/pattern.rs plane_uv).
fn plane_uv(q: vec3f, n: vec3f) -> vec2f {
    let a = abs(n);
    let top = a.y >= a.x && a.y >= a.z;
    let side = a.x >= a.z && !top;
    return vec2f(select(select(q.x, q.z, side), q.x, top), select(q.y, q.z, top));
}

fn sstep(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) * (1.0 / (b - a)), 0.0, 1.0);
    return t * t * (3.0 - t - t);
}

/// (blend t, brightness) of pattern `kind` at `q` with normal `n`.
fn pattern(kind: u32, q0: vec3f, n: vec3f, warp: f32, sharp: f32) -> vec2f {
    let seed = 0x3c6ef372u;
    // noise, cells, cracks and spots work on warped points (cracks always)
    let warps = kind == 3u || kind == 8u || kind == 9u || kind == 10u;
    let w = select(warp, warp + 0.3, kind == 9u);
    var q = q0;
    if (warps && w > 0.0) {
        q = warped(q0, w);
    }
    switch kind {
        case 1u: {
            let p = n * -0.25 + q;
            let c = floor(p.x) + floor(p.y) + floor(p.z);
            return vec2f(select(0.0, 1.0, fract(c * 0.5) > 0.25), 1.0);
        }
        case 2u: {
            var d = 1.0;
            let fx = fract(q.x);
            let fy = fract(q.y);
            let fz = fract(q.z);
            let lx = select(1.0, min(fx, 1.0 - fx), abs(n.x) < 0.7);
            let ly = select(1.0, min(fy, 1.0 - fy), abs(n.y) < 0.7);
            let lz = select(1.0, min(fz, 1.0 - fz), abs(n.z) < 0.7);
            d = min(lx, min(ly, lz));
            return vec2f(1.0 - sstep(0.02, 0.035, d * sharp), 1.0);
        }
        case 3u: {
            let f = fbm3(q, 5u, seed);
            return vec2f(clamp((f * 1.6) * sharp + 0.5, 0.0, 1.0), 1.0);
        }
        case 6u: {
            let w = fbm3(q * 0.5, 5u, seed);
            let v = sin(q.x * 1.5 + q.y * 0.5 + w * (warp * 6.0 + 2.0));
            return vec2f(pow(max(1.0 - abs(v), 1e-6), sharp * 6.0), 1.0);
        }
        case 7u: {
            let w = fbm3(q * 0.7, 3u, seed);
            let rr = length(q.xz) * 3.0 + w * (warp * 2.0 + 0.3) + q.y * 0.05;
            let f = fract(rr);
            let rings = pow(max(f, 1e-6), sharp * 3.0);
            let grain = perlin3(vec3f(q.x * 30.0, q.y * 1.5, q.z * 30.0), seed) * 0.08 + 1.0;
            return vec2f(rings, grain);
        }
        case 8u: {
            let v = voronoi(q);
            return vec2f(1.0 - sstep(0.0, 0.08, (v.y - v.x) * sharp), v.z * 0.5 + 0.75);
        }
        case 9u: {
            let v = voronoi(q);
            return vec2f(1.0 - sstep(0.0, 0.04, (v.y - v.x) * sharp), 1.0);
        }
        case 10u: {
            let v = voronoi(q);
            let rr = v.z * 0.15 + 0.2;
            let edge = 0.03 / sharp;
            let t = clamp(1.0 - ((v.x - rr + edge) / (edge + edge)), 0.0, 1.0);
            return vec2f(t * t * (3.0 - t - t), 1.0);
        }
        case 11u: {
            var w = 0.0;
            if (warp > 0.0) {
                w = fbm3(q, 3u, seed) * (warp * 4.0);
            }
            let v = sin(q.y * TAU + w);
            return vec2f(clamp(v * sharp * 2.0 + 0.5, 0.0, 1.0), 1.0);
        }
        case 12u: {
            let a = abs(n);
            let top = a.y >= a.x && a.y >= a.z;
            let side = a.x >= a.z && !top;
            let uu = select(select(q.x, q.z, side), q.x, top);
            let vv = select(q.y, q.z, top) * 2.0;
            let rw = floor(vv);
            let ub = uu * 0.5 + rw * 0.5;
            let fu = fract(ub);
            let fv = fract(vv);
            let edge = min(min(fu, 1.0 - fu) * 2.0, min(fv, 1.0 - fv));
            let mortar = 1.0 - sstep(0.04, 0.08, edge * sharp);
            let shade = unit(lowbias((fi(floor(ub)) * PK1) ^ (fi(rw) * PK2))) * 0.3 + 0.85;
            return vec2f(mortar, shade);
        }
        case 13u: {
            let uv = plane_uv(q, n);
            let rr = vec2f(1.0, 1.7320508);
            let a = uv - floor(uv / rr) * rr - rr * 0.5;
            let bq = uv - rr * 0.5;
            let b = bq - floor(bq / rr) * rr - rr * 0.5;
            let g = select(b, a, dot(a, a) < dot(b, b));
            let hex = max(abs(g.x), abs(g.x) * 0.5 + abs(g.y) * 0.8660254);
            let grout = 1.0 - sstep(0.03, 0.06, (0.5 - hex) * sharp);
            let c = round(vec2f((uv.x - g.x) * 2.0, (uv.y - g.y) * (1.0 / 0.8660254)));
            let shade = unit(lowbias((fi(c.x) * PK1) ^ (fi(c.y) * PK2))) * 0.3 + 0.85;
            return vec2f(grout, shade);
        }
        case 14u: {
            var w = sin(q.x * PI) * 0.3 + sin(q.z * (PI * 1.3)) * 0.2;
            if (warp > 0.0) {
                w = fbm3(q, 3u, seed) * (warp * 2.0) + w;
            }
            let v = sin((q.y + w) * TAU);
            return vec2f(clamp((v * sharp) * 0.5 + 0.5, 0.0, 1.0), 1.0);
        }
        case 15u: {
            var r = length(q.xz);
            if (warp > 0.0) {
                r = fbm3(q, 3u, seed) * (warp * 0.5) + r;
            }
            let v = sin(r * TAU);
            return vec2f(clamp((v * sharp) * 0.5 + 0.5, 0.0, 1.0), 1.0);
        }
        case 16u: {
            let uv = plane_uv(q, n);
            let rw = floor(uv.y);
            let uu = rw * 0.5 + uv.x;
            let fu = fract(uu) - 0.5;
            let fv = fract(uv.y);
            let d = sqrt(fu * fu + fv * fv);
            let rim = sstep(0.45, 0.72, d * sharp);
            let shade = unit(lowbias((fi(floor(uu)) * PK1) ^ (fi(rw) * PK2))) * 0.25 + 0.88;
            return vec2f(rim, shade);
        }
        case 17u: {
            let uv = plane_uv(q, n) * 2.0;
            let along_u = fract((floor(uv.x) + floor(uv.y)) * 0.5) < 0.25;
            let across = select(fract(uv.x), fract(uv.y), along_u);
            let along = select(fract(uv.y), fract(uv.x), along_u);
            let thread = sin(across * PI);
            let gap = max(1.0 - sstep(0.0, 0.35, thread * sharp), select(0.3, 0.0, along_u));
            let shade = (thread * 0.4 + 0.6) * (sin(along * PI) * 0.2 + 0.8);
            return vec2f(gap, shade);
        }
        case 19u: {
            let a = abs(n);
            let top = a.y >= a.x && a.y >= a.z;
            let side = a.x >= a.z && !top;
            let uu = select(q.x, q.z, side);
            let vv = q.y * (1.0 / 1.4);
            let inner = n * -0.25 + q;
            let ww = select(inner.z, inner.x, side);
            let cu = floor(uu);
            let cv = floor(vv);
            let fu = uu - cu;
            let fv = vv - cv;
            let inside = min(min(0.36 - abs(fu - 0.5), fv - 0.2), 0.86 - fv);
            let mullion = sstep(0.0, 0.02, (abs(fu - 0.5) - 0.012) * sharp);
            let win = select(sstep(0.0, 0.04, inside * sharp) * mullion, 0.0, top);
            let up = clamp((fv - 0.2) * (1.0 / 0.66), 0.0, 1.0) * 0.35 + 0.65;
            let iu = fi(cu) * PK1;
            let iv = fi(cv) * PK2;
            let iw = fi(floor(ww)) * PK3;
            let fl = unit(lowbias(iv ^ iw ^ 0x2545f491u));
            let h = lowbias(iu ^ iv ^ iw);
            let lit = unit(h) < fl * 0.6 + 0.15;
            let glow = unit(lowbias(h ^ 0x1b873593u)) * 0.45 + 0.55;
            return vec2f(win * select(0.0, glow * up, lit), select(win * -0.7 + 1.0, 1.0, lit));
        }
        default: {
            return vec2f(0.0, 1.0);
        }
    }
}

/// The pattern at `q` (xy) and, with `grad`, its gradient (zw.. in the
/// returned array's slots 1..3): one call site for the whole pattern code.
fn pattern_with_gradient(kind: u32, q: vec3f, n: vec3f, warp: f32, sharp: f32, grad: bool) -> array<vec3f, 2> {
    let e = 0.01;
    var offs = array<vec3f, 7>(vec3f(0.0), vec3f(e, 0.0, 0.0), vec3f(-e, 0.0, 0.0), vec3f(0.0, e, 0.0), vec3f(0.0, -e, 0.0), vec3f(0.0, 0.0, e), vec3f(0.0, 0.0, -e));
    var vals: array<f32, 7>;
    var centre = vec2f(0.0, 1.0);
    let count = select(1, 7, grad);
    for (var k = 0; k < count; k++) {
        let r = pattern(kind, q + offs[k], n, warp, sharp);
        vals[k] = r.x;
        if (k == 0) {
            centre = r;
        }
    }
    let g = vec3f(vals[1] - vals[2], vals[3] - vals[4], vals[5] - vals[6]) * (0.5 / e);
    return array<vec3f, 2>(vec3f(centre, 0.0), select(vec3f(0.0), g, grad));
}

// ---- surfaces (render/shade.rs) --------------------------------------------

struct Surf {
    p: vec3f,
    n: vec3f,
    albedo: vec3f,
    emission: vec3f,
    rough: f32,
    metal: f32,
    trans: f32,
    ior: f32,
    absorb: vec3f,
    scatter: vec3f,
    ao: f32,
    terrain: bool,
    node: u32,
    /// sunlight scattered through it (clouds)
    transl: f32,
    /// the pixel's footprint at distance-field hits (render/shade.rs Surf.foot)
    foot: f32,
    /// the point's offset from the camera, and whether it is on an anchored
    /// object (render/shade.rs Surf.dp, Surf.fine)
    dp: vec3f,
    fine: bool,
}

fn smoothk(x: f32, a: f32, k: f32) -> f32 {
    let t = clamp((x - a) * k, 0.0, 1.0);
    return t * t * (3.0 - t - t);
}

const RIPPLE_SPEED = 0.9;
const RIPPLE_FADE = 1.3;
const RIPPLE_K = 15.7;
const RIPPLE_REACH = 3.5;

/// The water surface's slope (dh/dx, dh/dz) at (x, z): two drifting noise
/// octaves and the ripples (shade.rs).
fn wave_slope(x: f32, z: f32) -> vec2f {
    let seed = 0x165667b1u;
    let tt = F.wave_time;
    let g1 = noised(x * 0.35 + tt * 0.21, z * 0.35 + tt * 0.13, seed);
    let g2 = noised(x * 1.3 - tt * 0.47, z * 1.3 + tt * 0.36, seed);
    let g3 = noised(x * 3.7 + tt * 0.9, z * 3.7 - tt * 0.7, seed);
    var s = vec2f(g3.y * 0.2 + (g2.y * 0.45 + g1.y), g3.z * 0.2 + (g2.z * 0.45 + g1.z)) * (0.12 * F.wave_height);
    for (var i = 0u; i < min(F.n_ripples, 32u); i++) {
        let r = F.ripples[i];
        let age = F.time - r.z;
        if (age < 0.0) {
            continue;
        }
        let rad = RIPPLE_SPEED * age;
        let width = 0.18 + 0.35 * age;
        let d = vec2f(x - r.x, z - r.y);
        let dist = max(length(d), 1e-4);
        let off = dist - rad;
        let u = off / width;
        // only near its ring
        if (abs(u) >= RIPPLE_REACH) {
            continue;
        }
        let a = r.w * exp(-RIPPLE_FADE * age) / sqrt(1.0 + 2.0 * rad);
        let ph = off * RIPPLE_K;
        let dh = exp2(u * u * -1.442695) * a * (u * (-2.0 / width) * cos(ph) - sin(ph) * RIPPLE_K);
        s += d * (dh / dist);
    }
    return s;
}

fn wave_normal(p: vec3f, t: f32, n: vec3f) -> vec3f {
    let s = wave_slope(p.x, p.z);
    let fade = 1.0 / (t * 0.02 + 1.0);
    let nx = -(s.x * fade);
    let nz = -(s.y * fade);
    let ny = inverseSqrt(nz * nz + nx * nx + 1.0);
    let w = vec3f(nx * ny, ny, nz * ny);
    return select(w, -w, n.y < 0.0);
}

/// fraktal256's terrain material: (albedo, detail normal). The strata and
/// the finest colour noise fade as they shrink below a few pixels
/// (shade.rs terrain_material).
fn terrain_material(k: u32, p: vec3f, rt: f32, n: vec3f, d: vec3f, cone: f32, albedo: ptr<function, vec3f>) -> vec3f {
    let b = ter_base(k);
    let seed = 0x27d4eb2fu;
    let foot = rt * cone / max(abs(dot(n, d)), 0.1);
    let g1 = noised(p.x * 0.7, p.z * 0.7, seed);
    let n1 = g1.x;
    var fade = max(1.0 - rt * 0.01, 0.0) * 0.35;
    var nx = n.x - g1.y * fade;
    var nz = n.z - g1.z * fade;
    let n2 = (noised(p.x * 2.9, p.z * 2.9, seed).x - 0.5) * clamp(1.5 - foot * 2.9 * 3.0, 0.0, 1.0) + 0.5;
    let g3 = noised(p.x * 0.06, p.z * 0.06, seed);
    let macro_n = g3.x - 0.5;
    fade = max(1.0 - rt * 0.04, 0.0) * 0.08;
    nx = nx - g3.y * fade;
    nz = nz - g3.z * fade;
    let nn = normalize(vec3f(nx, n.y, nz));
    let strata = sin(p.y * 1.3 + n1 * 3.0) * clamp(1.5 - foot * sqrt(max(1.0 - n.y * n.y, 0.0)) * (10.0 * 1.3 / 6.2831853) * 3.0, 0.0, 1.0);
    var bright = (strata * 0.12 + 0.9) + n2 * 0.3;
    bright = macro_n * 0.5 + bright;
    let rock = v3(b + 16u) * bright;
    let warm = macro_n * 0.6;
    var col = vec3f(warm * rock.x + rock.x, rock.y, rock.z - warm * rock.z);
    let flat_w = smoothk(nn.y, 0.78, 8.0);
    let high = smoothk(n1 * 10.0 + p.y, S[b + 11u], 0.1);
    let patch_w = smoothk(n1 * 0.3 + macro_n, -0.15, 6.0);
    let grass_w = flat_w * (1.0 - high) * patch_w;
    let grass = v3(b + 20u) * (n2 * 0.5 + 0.75);
    col = mix(col, grass, grass_w);
    let snow_w = smoothk(n1 * 8.0 + p.y, S[b + 12u], 0.15) * smoothk(nn.y, 0.6, 5.0);
    col = mix(col, v3(b + 24u), snow_w);
    let sand_w = 1.0 - smoothk(p.y, S[b + 13u], 1.0);
    col = mix(col, v3(b + 28u), sand_w);
    let gr = terrain_grit(p, rt * cone, nn);
    *albedo = col * gr.w;
    return gr.xyz;
}

/// Close-up detail for terrain (shade.rs terrain_grit): the bumped normal
/// (xyz) and the brightness factor (w); `foot` is metres per pixel there.
fn terrain_grit(p: vec3f, foot: f32, n: vec3f) -> vec4f {
    let f1 = clamp(1.5 - foot * 12.0, 0.0, 1.0);
    if (f1 <= 0.0) {
        return vec4f(n, 1.0);
    }
    let seed = 0x6d2b79f5u;
    let f = 4.0;
    let w = n * n;
    let ax = noised(p.z * f, p.y * f, seed);
    let ay = noised(p.x * f, p.z * f, seed);
    let az = noised(p.x * f, p.y * f, seed);
    let v = ax.x * w.x + ay.x * w.y + az.x * w.z;
    let g = vec3f(ay.y * w.y + az.y * w.z, ax.z * w.x + az.z * w.z, ax.y * w.x + ay.z * w.y);
    let nn = normalize(n - g * (f1 * 0.12));
    var grit = (v - 0.5) * 0.7 * f1 + 1.0;
    let f2 = clamp(1.5 - foot * 42.0, 0.0, 1.0);
    let f3 = clamp(1.5 - foot * 120.0, 0.0, 1.0);
    if (f2 > 0.0) {
        grit += perlin3(p * 14.0, seed) * 0.3 * f2;
    }
    if (f3 > 0.0) {
        grit += perlin3(p * 40.0, seed ^ 0x55u) * 0.25 * f3;
    }
    return vec4f(nn, max(grit, 0.2));
}

fn mat_base(m: u32) -> u32 {
    return F.off_mats + m * MAT;
}

fn local_apply(i: u32, v: vec3f) -> vec3f {
    let b = obj_base(i) + 36u;
    return vec3f(dot(v, row(b, 0u)), dot(v, row(b, 1u)), dot(v, row(b, 2u)));
}

fn local_apply_t(i: u32, v: vec3f) -> vec3f {
    let b = obj_base(i) + 36u;
    return row(b, 0u) * v.x + row(b, 1u) * v.y + row(b, 2u) * v.z;
}

fn local_t(i: u32) -> vec3f {
    let b = obj_base(i) + 36u;
    return vec3f(S[b + 3u], S[b + 7u], S[b + 11u]);
}

fn surface(r: Ray, hit: Hit, cone: f32) -> Surf {
    var sf: Surf;
    let i = u32(hit.obj);
    let b = obj_base(i);
    let kind = u(b);
    var p = r.o + r.d * hit.t;
    var dp = r.od + r.d * hit.t;
    var fine = false;
    var n = vec3f(0.0, 1.0, 0.0);
    var mat = u(b + 1u);
    var node = 0u;
    var trap = 0.0;
    var ao = 1.0;
    var foot = hit.t * cone;
    var terrain = false;
    var terrain_tex = false;
    var tcol = vec3f(0.0);
    if (kind == K_SPHERE) {
        n = (p - v3(b + 16u)) * (1.0 / S[b + 19u]);
    } else if (kind == K_BOX) {
        let l = box_local(i, p - v3(b + 16u)) / v3(b + 20u);
        let a = abs(l);
        let xm = a.x >= a.y && a.x >= a.z;
        let ym = a.y >= a.z && !xm;
        let zm = !(xm || ym);
        let s = vec3f(select(0.0, sign(l.x) + select(0.0, 1.0, l.x == 0.0), xm), select(0.0, sign(l.y) + select(0.0, 1.0, l.y == 0.0), ym), select(0.0, sign(l.z) + select(0.0, 1.0, l.z == 0.0), zm));
        let rb = b + 24u;
        n = row(rb, 0u) * s.x + row(rb, 1u) * s.y + row(rb, 2u) * s.z;
    } else if (kind == K_PLANE) {
        n = v3(b + 16u);
    } else if (HAS_NURBS && kind == K_NURBS) {
        // the hit again, for its normal (an open surface shows both sides)
        let nh = nb_hit(i, r, hit.t * (1.0 + 1e-4) + 1e-6 * S[b + 19u], 1e-5, true, false);
        let rb = b + 24u;
        n = row(rb, 0u) * nh.n.x + row(rb, 1u) * nh.n.y + row(rb, 2u) * nh.n.z;
        if (dot(n, n) < 0.5) {
            n = -r.d;
        }
        if ((u(b + 23u) & 1u) == 0u && dot(n, r.d) > 0.0) {
            n = -n;
        }
    } else if (kind == K_SDF) {
        prepare(i);
        // (from the object's position, along the ray: Object.org; anchored,
        // as fine as the offset from the camera)
        let pr = (r.o - obj_org(i)) + r.d * hit.t;
        fine = obj_fine(i);
        let gr = grain(length(select(pr, dp, fine)));
        let eps = max(hit.t * (cone * 0.5), 2.0 * gr);
        foot = max(hit.t * cone, gr);
        let nn = sdf_normal(i, pr, dp, eps);
        DP = dp;
        let dt = sdf_tag_of(i, pr);
        trap = dt.o;
        p = p - nn * dt.d;
        dp = dp - nn * dt.d;
        n = nn;
        mat = dt.t & 0xffffu;
        node = dt.t >> 16u;
    } else {
        let ter = u(b + 6u);
        let tn = ter_normal(ter, p.x, p.z);
        ao = tn.w;
        terrain = true;
        foot = 1.0;
        if (u(mat_base(mat) + 21u) == TX_TERRAIN) {
            n = terrain_material(ter, p, hit.t, tn.xyz, r.d, cone, &tcol);
            terrain_tex = true;
        } else {
            n = tn.xyz;
        }
    }
    let mb = mat_base(mat);
    var albedo = v3(mb);
    var rough = S[mb + 3u];
    let albedo2 = v3(mb + 4u);
    var emission = v3(mb + 8u);
    var metal = S[mb + 11u];
    let texk = u(mb + 21u);
    // the colour's pattern, and the relief's (by default the same one: then
    // one call gives both)
    let bump = S[mb + 25u];
    let bumpk = u(mb + 28u);
    let colour = !terrain_tex && texk != TX_NONE && texk != TX_TERRAIN && texk != TX_WATER;
    let bumped = !terrain_tex && bump != 0.0 && bumpk != TX_NONE && bumpk != TX_TERRAIN && bumpk != TX_WATER && bumpk != TX_ORBIT;
    if (colour || bumped) {
        let tscale = S[mb + 22u];
        let bscale = S[mb + 29u];
        let warp = S[mb + 23u];
        let sharp = S[mb + 24u];
        let world = u(mb + 26u) == 1u;
        let lp = local_apply(i, p - local_t(i));
        let ln = normalize(local_apply(i, n));
        let at = select(lp, p, world);
        let qn = select(ln, n, world);
        let same = bumped && bumpk == texk && bscale == tscale;
        var gq = vec3f(0.0);
        if (colour) {
            var res = array<vec3f, 2>(vec3f(orbit_blend(trap, tscale, sharp), 1.0, 0.0), vec3f(0.0));
            if (texk != TX_ORBIT) {
                res = pattern_with_gradient(texk, at * tscale, qn, warp, sharp, same);
            }
            let pt = res[0].xy;
            albedo = mix(albedo, albedo2, pt.x) * pt.y;
            rough = mix(rough, S[mb + 7u], pt.x);
            metal = mix(metal, S[mb + 15u], pt.x);
            emission = mix(emission, v3(mb + 12u), pt.x);
            gq = res[1];
        }
        if (bumped) {
            if (!same) {
                gq = pattern_with_gradient(bumpk, at * bscale, qn, warp, sharp, true)[1];
            }
            let gw = select(local_apply_t(i, gq), gq, world);
            let tangential = gw - n * dot(gw, n);
            n = normalize(tangential * (bump * -0.05) + n);
        }
    }
    let water = texk == TX_WATER;
    if (water) {
        n = wave_normal(p, hit.t, n);
    }
    if (terrain_tex) {
        albedo = tcol;
    }
    sf.p = p;
    sf.n = n;
    sf.albedo = albedo;
    sf.emission = emission;
    sf.rough = rough;
    sf.metal = metal;
    sf.trans = S[mb + 19u];
    sf.transl = S[mb + 27u];
    sf.ior = S[mb + 20u];
    sf.absorb = select(v3(mb + 16u), WATER_ABSORB, water);
    sf.scatter = select(vec3f(0.0), albedo2 * F.daylight, water);
    sf.ao = ao;
    sf.terrain = terrain;
    sf.node = node;
    sf.foot = foot;
    sf.dp = dp;
    sf.fine = fine;
    return sf;
}

// ---- lighting (render/shade.rs) --------------------------------------------

fn schlick(f0: vec3f, c: f32) -> vec3f {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    let k = m2 * m2 * m;
    return (vec3f(1.0) - f0) * k + f0;
}

fn schlick1(f0: f32, c: f32) -> f32 {
    let m = clamp(1.0 - c, 0.0, 1.0);
    let m2 = m * m;
    return (1.0 - f0) * (m2 * m2 * m) + f0;
}

/// Refraction (xyz) and whether it reflects totally (w = 1).
fn refract_tir(d: vec3f, nf: vec3f, eta: f32) -> vec4f {
    let cosi = -dot(d, nf);
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
    let s = eta * cosi - sqrt(max(k, 0.0));
    return vec4f(d * eta + nf * s, select(0.0, 1.0, k < 0.0));
}

const WATER_IOR = 1.33;

/// Sunlight reaching `p`, `depth` under the water: toward the light inside
/// the water (xyz), refracted through the waves, and the metres it crossed (w).
fn under_water(sun: vec3f, p: vec3f, depth: f32) -> vec4f {
    let eta = 1.0 / WATER_IOR;
    let t0 = refract_tir(-sun, vec3f(0.0, 1.0, 0.0), eta).xyz;
    let s0 = p - t0 * (depth / max(-t0.y, 0.05));
    let sl = wave_slope(s0.x, s0.z);
    let wn = normalize(vec3f(-sl.x, 1.0, -sl.y));
    let lw = normalize(-refract_tir(-sun, wn, eta).xyz);
    return vec4f(lw, depth / max(lw.y, 0.05));
}

/// `x` and `y` along an orthonormal basis around unit `n`.
fn disk(n: vec3f, x: f32, y: f32) -> vec3f {
    let sgn = select(1.0, -1.0, n.z < 0.0);
    let a = -1.0 / (sgn + n.z);
    let b = n.x * n.y * a;
    let t1 = vec3f(1.0 + sgn * n.x * n.x * a, sgn * b, -sgn * n.x);
    let t2 = vec3f(b, sgn + n.y * n.y * a, -n.y);
    return t1 * x + t2 * y;
}

fn cosine_dir(n: vec3f, u1: f32, u2: f32) -> vec3f {
    let r = sqrt(u1);
    let phi = u2 * TAU;
    let sgn = select(1.0, -1.0, n.z < 0.0);
    let a = -1.0 / (sgn + n.z);
    let b = n.x * n.y * a;
    let t1 = vec3f(1.0 + sgn * n.x * n.x * a, sgn * b, -sgn * n.x);
    let t2 = vec3f(b, sgn + n.y * n.y * a, -n.y);
    let z = sqrt(max(1.0 - u1, 0.0));
    return t1 * (r * cos(phi)) + t2 * (r * sin(phi)) + n * z;
}

fn offset(sf: Surf, t: f32) -> f32 {
    let o = t * 2.0e-5 + min(max(sf.foot * 4.0, select(grain(length(sf.p)), 0.0, sf.fine)), 1.0e-3);
    return select(o, max(o, 0.02), sf.terrain);
}

/// Light arriving at the surface from the sun, point lights and glowing
/// objects (to be multiplied by albedo), and the sun's visibility (w). Every
/// light goes through one shadow call (the shader stays small).
fn direct_light(sf: Surf, n: vec3f, t: f32, random: bool) -> vec4f {
    let off = offset(sf, t);
    let o = n * off + sf.p;
    let od = n * off + sf.dp;
    let soft = !random && on(F_SOFT);
    var light = vec3f(0.0);
    var sun_vis = 1.0;
    // the sun under water (shade.rs): slot 0 is its refracted way down to
    // here, slot 1 its shadow on from where it entered the water
    var under = false;
    var entry = vec3f(0.0);
    var sun_air = vec3f(0.0);
    var sun_light = vec3f(0.0);
    let n_lights = select(0u, F.n_lights, on(F_LIGHTS));
    let n_emitters = select(0u, F.n_emitters, on(F_LIGHTS));
    let count = 2u + n_lights + n_emitters;
    for (var li = 0u; li < count; li++) {
        var origin = o;
        var origin_d = od;
        var dir = vec3f(0.0);
        var tmax = BIG;
        var weight = 0.0;
        var colour = vec3f(0.0);
        var shadowed = false;
        var lsoft = false;
        var k = 1.0;
        var skip = -1;
        if (li == 0u) {
            // the sun
            var sun = F.sun_dir;
            if (random) {
                sun = normalize(sun + vec3f(rnd() - 0.5, rnd() - 0.5, rnd() - 0.5) * 0.02);
            }
            sun_air = sun;
            dir = sun;
            colour = F.sun_col;
            shadowed = on(F_SHADOWS);
            lsoft = soft;
            k = F.shadow_hardness;
            if (on(F_WATER)) {
                let depth = F.water_level - sf.p.y;
                if (depth > 0.0) {
                    let uw = under_water(sun, sf.p, depth);
                    let len = uw.w;
                    entry = sf.p + uw.xyz * len;
                    let e = 0.05;
                    let curv = (wave_slope(entry.x + e, entry.z).x - wave_slope(entry.x - e, entry.z).x
                        + wave_slope(entry.x, entry.z + e).y - wave_slope(entry.x, entry.z - e).y) * (0.5 / e);
                    let spread = max(abs(len * (1.0 - 1.0 / WATER_IOR) * curv + 1.0), 0.25);
                    let r0 = (WATER_IOR - 1.0) / (WATER_IOR + 1.0);
                    let through = 1.0 - schlick1(r0 * r0, max(sun.y, 0.0));
                    colour = colour * exp2(len * -WATER_ABSORB) * (through / spread);
                    dir = uw.xyz;
                    tmax = length(entry - sf.p);
                    under = true;
                }
            }
            weight = max(dot(n, dir), 0.0);
        } else if (li == 1u) {
            if (!under || all(sun_light == vec3f(0.0))) {
                continue;
            }
            origin = entry;
            origin_d = entry - F.cam_pos;
            dir = sun_air;
            weight = 1.0;
            colour = sun_light;
            shadowed = on(F_SHADOWS);
            lsoft = soft;
            k = F.shadow_hardness;
        } else if (li <= n_lights + 1u) {
            let lb = F.off_lights + (li - 2u) * 8u;
            let range = S[lb + 3u];
            let to = v3(lb) - sf.p;
            let d2 = dot(to, to);
            if (d2 >= range * range) {
                continue;
            }
            let inv = inverseSqrt(max(d2, 1e-8));
            let lndl = max(dot(to, n) * inv, 0.0);
            let window = 1.0 - d2 / (range * range);
            weight = window * window / (d2 * 0.2 + 1.0) * lndl;
            dir = to * inv;
            tmax = sqrt(d2);
            colour = v3(lb + 4u);
            shadowed = on(F_LIGHT_SHADOWS);
        } else {
            // a glowing object: toward a random point of its disk, so that
            // shadows soften with its size; faded out toward its range
            let eb = F.off_emitters + (li - 2u - n_lights) * 12u;
            let radius = S[eb + 3u];
            let c = v3(eb) - sf.p;
            let rr = sqrt(rnd()) * radius;
            let phi = rnd() * TAU;
            let to = c + disk(c * inverseSqrt(max(dot(c, c), 1e-12)), rr * cos(phi), rr * sin(phi));
            let d2 = dot(to, to);
            let range = S[eb + 7u];
            if (d2 >= range * range) {
                continue;
            }
            let inv = inverseSqrt(max(d2, 1e-8));
            let fade = max(1.0 - d2 / (range * range), 0.0);
            weight = max(dot(to, n) * inv, 0.0) * (radius * radius / max(d2, radius * radius)) * fade * fade;
            dir = to * inv;
            tmax = sqrt(d2);
            colour = v3(eb + 4u);
            shadowed = on(F_LIGHT_SHADOWS);
            skip = i32(u(eb + 8u));
        }
        if (weight <= 0.0) {
            continue;
        }
        var vis = 1.0;
        if (shadowed) {
            vis = shadow(origin, origin_d, dir, tmax, lsoft, k, skip);
        }
        if (li == 0u) {
            sun_vis = vis;
            if (under) {
                sun_light = colour * (weight * vis);
                continue;
            }
        } else if (li == 1u) {
            sun_vis *= vis;
        }
        light += colour * (weight * vis);
    }
    return vec4f(light, sun_vis);
}

fn occlusion(sf: Surf, n: vec3f) -> f32 {
    let r = max(min(max(F.ao_radius, 0.01), sf.foot * 1000.0), 1e-12);
    var occ = 0.0;
    for (var i = 1; i <= 5; i++) {
        let h = r * f32(i) / 5.0;
        // nothing farther than h occludes this sample
        let d = scene_dist(n * h + sf.p, n * h + sf.dp, h, sf.terrain);
        occ = max(h - d, 0.0) * (0.2 / h) + occ;
    }
    return clamp(1.0 - occ * 1.2, 0.0, 1.0);
}

fn sun_spec(n: vec3f, v: vec3f, rough: f32, f0: vec3f, vis: f32) -> vec3f {
    let l = F.sun_dir;
    let h = normalize(v + l);
    let nh = max(dot(n, h), 0.0);
    let ndl = max(dot(n, l), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(rough, 0.2);
    let a2 = (a * a) * (a * a);
    let dd = (nh * nh) * (a2 - 1.0) + 1.0;
    let d = a2 / (dd * dd * PI);
    return schlick(f0, vh) * F.sun_col * (d * ndl * vis * 0.25);
}

// ---- integrators (render/shade.rs) -----------------------------------------

struct Path {
    ray: Ray,
    thr: vec3f,
    alive: bool,
    absorb: vec3f,
    scatter: vec3f,
    medium: bool,
    /// bounced off a diffuse surface (path tracing): gathers capped
    diffused: bool,
}

fn new_path(o: vec3f, d: vec3f) -> Path {
    // (the camera's rays: it is cam_fine from the anchor)
    var p = Path(make_ray(o, o - F.cam_pos + F.cam_fine, d, -1), vec3f(1.0), true, vec3f(0.0), vec3f(0.0), false, false);
    if (on(F_WATER) && F.cam_pos.y < F.water_level) {
        p.absorb = WATER_ABSORB;
        p.scatter = F.water_col;
        p.medium = true;
    }
    return p;
}

/// After a hit: the sky on a miss, the medium or fog over the segment, the
/// surface's glow. False when the path ended.
fn advance(path: ptr<function, Path>, hit: Hit, col: ptr<function, vec3f>, disk: bool) -> bool {
    if (hit.obj < 0) {
        var s = sky((*path).ray.d, disk);
        if (!disk) {
            s = s * F.ambient;
        }
        // in a medium (under water) out to nowhere: its own deep colour
        if ((*path).medium) {
            s = (*path).scatter;
        }
        *col += (*path).thr * s;
        (*path).alive = false;
        return false;
    }
    let t = hit.t;
    if ((*path).medium) {
        let tr = exp2(t * -(*path).absorb);
        *col += (*path).thr * ((*path).scatter * (vec3f(1.0) - tr));
        (*path).thr = (*path).thr * tr;
    } else {
        let f = fog(t, (*path).ray.d);
        *col += (*path).thr * f.xyz;
        (*path).thr = (*path).thr * f.w;
    }
    return true;
}

fn emits(hit: Hit) -> bool {
    return (obj_flags(u32(hit.obj)) & 2u) != 0u;
}

/// Sunlight a translucent material lets through (shade.rs through()).
fn through(sf: Surf, d: vec3f, n: vec3f) -> vec3f {
    if (sf.transl <= 0.0) {
        return vec3f(0.0);
    }
    let toward = max(dot(d, F.sun_dir), 0.0);
    let t2 = toward * toward;
    let rim = 1.0 - min(abs(dot(n, -d)), 1.0);
    return F.sun_col * (((t2 * t2 * (rim + 0.6)) * 1.5 + 0.12) * sf.transl);
}

const MAX_INDIRECT: f32 = 8.0;

/// Caps what a bounce added since `was` on a path that had bounced
/// diffusely before it (shade.rs clamp_indirect).
fn clamp_indirect(col: vec3f, was: vec3f, diffused: bool) -> vec3f {
    if (!diffused) {
        return col;
    }
    let added = col - was;
    let peak = max(added.x, max(added.y, added.z));
    if (peak <= MAX_INDIRECT) {
        return col;
    }
    return was + added * (MAX_INDIRECT / peak);
}

/// The surface's normal on the side the ray comes from: flipped inside glass
/// and water, tilted to face the ray on opaque surfaces (shade.rs facing()).
fn facing(d: vec3f, sf: Surf) -> vec3f {
    let dn = dot(d, sf.n);
    if (dn < 0.0) {
        return sf.n;
    }
    if (sf.trans > 0.0) {
        return -sf.n;
    }
    return normalize(sf.n - d * (dn + 0.02));
}

fn cross_medium(path: ptr<function, Path>, entering: bool, sf: Surf, obj: i32, inside: ptr<function, i32>) {
    if (entering) {
        *inside = obj;
        (*path).absorb = sf.absorb;
        (*path).scatter = sf.scatter;
        (*path).medium = true;
    } else {
        *inside = -1;
        (*path).absorb = vec3f(0.0);
        (*path).scatter = vec3f(0.0);
        (*path).medium = false;
    }
}

struct Sample {
    col: vec3f,
    obj: i32,
    steps: u32,
    t: f32,
}

/// Every mode in one loop, so the shader has a single call site of
/// intersect, surface and direct_light (the driver inlines each call site,
/// and four copies of everything made the shader take tens of seconds to
/// build). Direct mode's extra reflection ray at glass (fraktal256's trace2,
/// `simple` on the CPU) is queued and traced on the next pass of the loop.
fn render_sample(o: vec3f, d: vec3f, cone: f32) -> Sample {
    var out = Sample(vec3f(0.0), -1, 0u, BIG);
    var path = new_path(o, d);
    var col = vec3f(0.0);
    var disk = true;
    var bounce = 0u;
    var pend = false;
    var pend_o = vec3f(0.0);
    var pend_od = vec3f(0.0);
    var pend_d = vec3f(0.0);
    var pend_w = vec3f(0.0);
    let pt = F.mode == M_PATH;
    let dbg = F.mode >= M_NORMALS;
    // what the path had gathered before this bounce, and whether it had
    // bounced diffusely by then (shade.rs clamp_indirect)
    var before = col;
    var before_diffused = false;
    for (var iter = 0u; iter < F.bounces * 2u + 4u; iter++) {
        if (pt) {
            col = clamp_indirect(col, before, before_diffused);
            before = col;
            before_diffused = path.diffused;
        }
        if (!path.alive && !pend) {
            break;
        }
        let simple_ray = pend;
        var ray = path.ray;
        if (simple_ray) {
            ray = make_ray(pend_o, pend_od, pend_d, -1);
        }
        let hit = intersect(ray, F.max_dist, cone);
        if (!simple_ray && bounce == 0u) {
            out.obj = hit.obj;
            out.steps = hit.steps;
            out.t = hit.t;
        }
        var sf: Surf;
        let need_surface = hit.obj >= 0 && !(dbg && (F.mode == M_STEPS || F.mode == M_DEPTH));
        if (need_surface) {
            sf = surface(ray, hit, cone);
        }
        if (dbg) {
            if (F.mode == M_STEPS) {
                out.col = ramp(f32(hit.steps) * (2.5 / f32(F.max_steps)));
            } else if (F.mode == M_DEPTH) {
                let v = 1.0 - log(hit.t + 1.0) * (1.0 / log(F.max_dist + 1.0));
                out.col = select(vec3f(0.0), vec3f(v), hit.obj >= 0);
            } else if (F.mode == M_COST) {
                // its price on the ramp (render/mod.rs prices: word 11), shaded
                // by how it faces the camera
                if (hit.obj >= 0) {
                    out.col = ramp(S[obj_base(u32(hit.obj)) + 11u]) * (abs(dot(sf.n, d)) * 0.65 + 0.35);
                } else {
                    out.col = sky(d, true) * 0.15;
                }
            } else if (hit.obj >= 0) {
                out.col = select(sf.albedo, sf.n * 0.5 + vec3f(0.5), F.mode == M_NORMALS);
            } else {
                out.col = select(sky(d, true) * 0.5, vec3f(0.0), F.mode == M_NORMALS);
            }
            return out;
        }
        if (simple_ray) {
            // simple shading: sun without shadow, ambient, sky reflections for shiny surfaces
            pend = false;
            var c = vec3f(0.0);
            if (hit.obj < 0) {
                c = sky(ray.d, true);
            } else {
                let nf = facing(ray.d, sf);
                let ndl = max(dot(nf, F.sun_dir), 0.0);
                let light = F.sun_col * ndl + F.amb_col * ((nf.y * 0.45 + 0.55) * sf.ao * (AMBIENT * F.ambient));
                let kd = (1.0 - sf.metal) * (1.0 - sf.trans);
                let shiny = max(sf.metal, sf.trans);
                let refl = sky(reflect(ray.d, nf), false);
                c = light * sf.albedo * kd + sf.emission + refl * mix(vec3f(1.0), sf.albedo, sf.metal) * (shiny * 0.5) + sf.scatter;
                let f = fog(hit.t, ray.d);
                c = c * f.w + f.xyz;
            }
            col += pend_w * c;
            continue;
        }
        // the clouds between the ray's origin and what it hit
        if (F.n_volumes > 0u) {
            let cm = cloud_march(path.ray, hit.t);
            col += path.thr * cm.xyz;
            path.thr *= cm.w;
            // sky seen through a cloud: not twinkled (render/mod.rs CLOUDED_SKY)
            if (bounce == 0u && hit.obj < 0 && cm.w < 0.95) {
                out.obj = -2;
            }
        }
        if (!advance(&path, hit, &col, disk)) {
            continue;
        }
        if (!pt || disk || !emits(hit)) {
            col += path.thr * sf.emission;
        }
        let v = -path.ray.d;
        let entering = dot(path.ray.d, sf.n) < 0.0;
        let nf = facing(path.ray.d, sf);
        let cosi = max(dot(v, nf), 0.0);
        let f0 = mix(vec3f(0.04), sf.albedo, sf.metal);
        let kd = (1.0 - sf.metal) * (1.0 - sf.trans);
        // the headlight (render/shade.rs headlight), fading toward its reach
        if (bounce == 0u && F.headlight > 0.0) {
            // (from the camera: F.lamp is its offset, Surf.dp)
            let to = F.lamp - sf.dp;
            let d2 = dot(to, to);
            let l = to * inverseSqrt(max(d2, 1e-12));
            let fade = max(1.0 - d2 * (1.0 / (F.lamp_reach * F.lamp_reach)), 0.0);
            var w = (sf.metal * 0.5 + kd) * max(dot(nf, l), 0.0) * (fade * fade) * (F.headlight * 2.0);
            if (on(F_LAMP_SHADOWS) && w > 0.0) {
                let dist = sqrt(d2);
                let off = offset(sf, dist);
                w *= shadow(nf * off + sf.p, nf * off + sf.dp, l, dist, false, 1.0, -1);
            }
            col += path.thr * (sf.albedo * vec3f(1.0, 0.94, 0.85) * w);
        }
        // the path tracer picks a lobe: dielectric, metal, specular coat or diffuse
        var tm = false;
        var mm = false;
        var sm = false;
        var diff = false;
        if (pt) {
            let u_t = rnd();
            let u_m = rnd();
            let u_s = rnd();
            tm = u_t < sf.trans;
            mm = !tm && u_m < sf.metal;
            let fs = schlick1(0.04, cosi) * (1.0 - sf.rough);
            sm = !tm && !mm && u_s < fs;
            diff = !tm && !mm && !sm;
            path.diffused = path.diffused || diff;
        }
        let lit = select(sf.trans < 0.999, diff, pt);
        if (lit) {
            let dl = direct_light(sf, nf, hit.t, pt);
            let thru = through(sf, path.ray.d, nf);
            if (pt) {
                col += path.thr * ((dl.xyz + thru) * sf.albedo);
            } else {
                var ao = sf.ao;
                if (on(F_AO)) {
                    ao = occlusion(sf, nf) * sf.ao;
                }
                let amb = F.amb_col * ((nf.y * 0.45 + 0.55) * ao * (AMBIENT * F.ambient));
                let diffuse = (dl.xyz + amb + thru) * sf.albedo * kd;
                let spec = sun_spec(nf, v, sf.rough, f0, dl.w);
                col += path.thr * (diffuse + spec);
                if (sf.metal > 0.0) {
                    let gloss = (1.0 - sf.rough) * (1.0 - sf.rough);
                    let env_l = sky(nf, false) * ((1.0 - gloss) * sf.metal * ao * 0.6);
                    col += path.thr * (env_l * sf.albedo);
                }
            }
        }
        if (bounce == F.bounces) {
            path.alive = false;
            continue;
        }
        let refl_d = reflect(path.ray.d, nf);
        let off = offset(sf, hit.t);
        let above = nf * off + sf.p;
        let below = nf * -off + sf.p;
        let above_d = nf * off + sf.dp;
        let below_d = nf * -off + sf.dp;
        var new_d = refl_d;
        var new_o = above;
        var new_od = above_d;
        var w = vec3f(0.0);
        var inside = path.ray.inside;
        var keep = false;
        // a rough surface scatters the ray about the mirror (or refracted)
        // direction in both modes (render/shade.rs)
        let cdir = cosine_dir(nf, rnd(), rnd());
        let r2 = sf.rough * sf.rough;
        var gdir = normalize(mix(refl_d, cdir, r2));
        if (dot(gdir, nf) <= 0.0) {
            gdir = refl_d;
        }
        if (pt) {
            new_d = select(normalize(mix(refl_d, cdir, r2)), cdir, diff);
            w = select(vec3f(1.0), sf.albedo, diff);
            if (mm) {
                w = schlick(sf.albedo, cosi);
            }
            var reflected = sm || mm;
            if (tm) {
                let eta = select(sf.ior, 1.0 / sf.ior, entering);
                let rt = refract_tir(path.ray.d, nf, eta);
                let r0 = (sf.ior - 1.0) / (sf.ior + 1.0);
                let fr = schlick1(r0 * r0, cosi);
                let mirror = rt.w > 0.5 || rnd() < fr;
                if (!mirror) {
                    new_d = normalize(mix(rt.xyz, -cdir, r2));
                    new_o = below;
                    new_od = below_d;
                    cross_medium(&path, entering, sf, hit.obj, &inside);
                }
                reflected = reflected || mirror;
            }
            keep = !(reflected && dot(new_d, nf) <= 0.0);
            disk = tm || (reflected && sf.rough < 0.25);
        } else if (sf.trans > 0.5) {
            let eta = select(sf.ior, 1.0 / sf.ior, entering);
            let rt = refract_tir(path.ray.d, nf, eta);
            let tir = rt.w > 0.5;
            let r0 = (sf.ior - 1.0) / (sf.ior + 1.0);
            let fr = select(schlick1(r0 * r0, cosi), 1.0, tir);
            if (!tir) {
                if (bounce < 2u) {
                    // the reflection, traced once with simple shading (next pass)
                    pend = true;
                    pend_o = above;
                    pend_od = above_d;
                    pend_d = gdir;
                    pend_w = path.thr * fr;
                }
                new_d = normalize(mix(rt.xyz, -cdir, r2));
                new_o = below;
                new_od = below_d;
                cross_medium(&path, entering, sf, hit.obj, &inside);
            }
            w = vec3f(select((1.0 - fr) * sf.trans, 1.0, tir));
            keep = true;
        } else {
            let gloss = (1.0 - sf.rough) * (1.0 - sf.rough);
            let ww = schlick(f0, cosi) * gloss;
            if (max(ww.x, max(ww.y, ww.z)) > 0.02) {
                w = ww;
                keep = true;
                new_d = gdir;
            }
        }
        path.thr = path.thr * w;
        if (pt && keep && bounce >= 2u) {
            let q = clamp(max(path.thr.x, max(path.thr.y, path.thr.z)), 0.05, 1.0);
            let survive = rnd() < q;
            path.thr = path.thr * (1.0 / q);
            keep = survive;
        }
        if (!pt && max(path.thr.x, max(path.thr.y, path.thr.z)) <= 0.003) {
            keep = false;
        }
        bounce += 1u;
        path.alive = keep;
        path.ray = make_ray(new_o, new_od, new_d, inside);
    }
    if (pt) {
        col = clamp_indirect(col, before, before_diffused);
    }
    out.col = col;
    return out;
}

const RAMP: array<vec3f, 8> = array<vec3f, 8>(
    vec3f(0.043, 0.059, 0.098),
    vec3f(0.051, 0.212, 0.420),
    vec3f(0.094, 0.310, 0.584),
    vec3f(0.145, 0.416, 0.749),
    vec3f(0.224, 0.529, 0.898),
    vec3f(0.427, 0.655, 0.925),
    vec3f(0.718, 0.827, 0.965),
    vec3f(0.949, 0.973, 1.000),
);

fn ramp(x: f32) -> vec3f {
    let f = clamp(x, 0.0, 1.0) * 7.0;
    let i = min(floor(f), 6.0);
    let t = f - i;
    var ramp_tab = RAMP;
    return mix(ramp_tab[u32(i)], ramp_tab[u32(i) + 1u], t);
}

// ---- output ------------------------------------------------------------

/// RGB9E5: three 9-bit mantissas with a shared 5-bit exponent.
fn pack_rgb9e5(c: vec3f) -> u32 {
    let m = clamp(c, vec3f(0.0), vec3f(65408.0));
    let mx = max(m.x, max(m.y, m.z));
    var e = max(-16, i32(floor(log2(max(mx, 1e-30))))) + 1;
    var scale = exp2(f32(9 - e));
    if (u32(floor(mx * scale + 0.5)) == 512u) {
        e += 1;
        scale *= 0.5;
    }
    let q = vec3<u32>(floor(m * scale + vec3f(0.5)));
    return (q.x & 511u) | ((q.y & 511u) << 9u) | ((q.z & 511u) << 18u) | (u32(e + 15) << 27u);
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    if (x >= F.width || gid.y >= F.rows) {
        return;
    }
    let band = B[gid.y / 16u];
    let row_i = band * 16u + gid.y % 16u;
    let pix = row_i * F.width + x;
    var sum = vec3f(0.0);
    var obj = -1;
    for (var k = 0u; k < F.spp; k++) {
        let smp = F.sample_base + k;
        let j = jitter(smp, pix);
        let vv = (j.y + f32(row_i)) * F.ky + F.by;
        let px = (f32(x) + j.x) * F.kx + F.bx;
        let d = normalize(F.cam_r * px + F.cam_u * vv + F.cam_f);
        rng_init(pix, smp);
        THR_K = thr_jitter(smp, pix);
        let s = render_sample(F.cam_pos, d, F.cone);
        // the CPU's NaN and firefly guard
        let c = select(s.col, vec3f(0.0), s.col != s.col);
        sum += clamp(c, vec3f(0.0), vec3f(256.0));
        if (k == 0u) {
            obj = s.obj;
        }
    }
    O[gid.y * F.width + x] = vec2<u32>(pack_rgb9e5(sum), bitcast<u32>(obj));
}
