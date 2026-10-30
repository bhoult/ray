//! Scalar maths for scene setup and per-object constants: `Vec3`, `Mat3`
//! (rotations) and `Aabb`. Per-pixel maths is 8-wide, in simd.rs.

use core::ops::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    pub const ONE: Vec3 = Vec3 { x: 1.0, y: 1.0, z: 1.0 };
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };

    #[inline(always)]
    pub const fn new(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3 { x, y, z }
    }
    #[inline(always)]
    pub const fn splat(v: f32) -> Vec3 {
        Vec3 { x: v, y: v, z: v }
    }
    pub fn from_array(a: [f32; 3]) -> Vec3 {
        Vec3::new(a[0], a[1], a[2])
    }
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
    #[inline(always)]
    pub fn dot(self, b: Vec3) -> f32 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    #[inline(always)]
    pub fn cross(self, b: Vec3) -> Vec3 {
        Vec3::new(self.y * b.z - self.z * b.y, self.z * b.x - self.x * b.z, self.x * b.y - self.y * b.x)
    }
    #[inline(always)]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }
    #[inline(always)]
    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }
    pub fn normalize(self) -> Vec3 {
        let l = self.length();
        if l > 0.0 { self * (1.0 / l) } else { Vec3::Y }
    }
    pub fn min(self, b: Vec3) -> Vec3 {
        Vec3::new(self.x.min(b.x), self.y.min(b.y), self.z.min(b.z))
    }
    pub fn max(self, b: Vec3) -> Vec3 {
        Vec3::new(self.x.max(b.x), self.y.max(b.y), self.z.max(b.z))
    }
    pub fn abs(self) -> Vec3 {
        Vec3::new(self.x.abs(), self.y.abs(), self.z.abs())
    }
    pub fn max_comp(self) -> f32 {
        self.x.max(self.y).max(self.z)
    }
    pub fn hadamard(self, b: Vec3) -> Vec3 {
        Vec3::new(self.x * b.x, self.y * b.y, self.z * b.z)
    }
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
    pub fn get(self, axis: usize) -> f32 {
        match axis {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn add(self, b: Vec3) -> Vec3 {
        Vec3::new(self.x + b.x, self.y + b.y, self.z + b.z)
    }
}
impl Sub for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn sub(self, b: Vec3) -> Vec3 {
        Vec3::new(self.x - b.x, self.y - b.y, self.z - b.z)
    }
}
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn div(self, s: f32) -> Vec3 {
        self * (1.0 / s)
    }
}
impl Neg for Vec3 {
    type Output = Vec3;
    #[inline(always)]
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for Vec3 {
    fn add_assign(&mut self, b: Vec3) {
        *self = *self + b;
    }
}

/// A 3x3 matrix, rows `m[0..3]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat3 {
    pub m: [Vec3; 3],
}

impl Mat3 {
    pub const IDENTITY: Mat3 = Mat3 { m: [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)] };

    /// Rotation by Euler angles in degrees, applied x first, then y, then z.
    pub fn euler_deg(a: [f32; 3]) -> Mat3 {
        let [x, y, z] = a.map(f32::to_radians);
        let (sx, cx) = x.sin_cos();
        let (sy, cy) = y.sin_cos();
        let (sz, cz) = z.sin_cos();
        let rx = Mat3 { m: [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, cx, -sx), Vec3::new(0.0, sx, cx)] };
        let ry = Mat3 { m: [Vec3::new(cy, 0.0, sy), Vec3::new(0.0, 1.0, 0.0), Vec3::new(-sy, 0.0, cy)] };
        let rz = Mat3 { m: [Vec3::new(cz, -sz, 0.0), Vec3::new(sz, cz, 0.0), Vec3::new(0.0, 0.0, 1.0)] };
        rz.mul(ry).mul(rx)
    }

    /// The Euler angles (degrees) of a rotation: the inverse of `euler_deg`.
    pub fn to_euler_deg(self) -> [f32; 3] {
        let r = self.m;
        let sb = (-r[2].x).clamp(-1.0, 1.0);
        let b = sb.asin();
        let (a, c) = if sb.abs() < 0.9999 {
            (r[2].y.atan2(r[2].z), r[1].x.atan2(r[0].x))
        } else {
            ((-r[1].z).atan2(r[1].y), 0.0)
        };
        [a.to_degrees(), b.to_degrees(), c.to_degrees()]
    }

    pub fn transpose(self) -> Mat3 {
        let m = self.m;
        Mat3 {
            m: [Vec3::new(m[0].x, m[1].x, m[2].x), Vec3::new(m[0].y, m[1].y, m[2].y), Vec3::new(m[0].z, m[1].z, m[2].z)],
        }
    }

    pub fn mul(self, b: Mat3) -> Mat3 {
        let t = b.transpose();
        Mat3 { m: self.m.map(|r| Vec3::new(r.dot(t.m[0]), r.dot(t.m[1]), r.dot(t.m[2]))) }
    }

    pub fn scale(self, s: f32) -> Mat3 {
        Mat3 { m: self.m.map(|r| r * s) }
    }

    #[inline(always)]
    pub fn apply(self, v: Vec3) -> Vec3 {
        Vec3::new(self.m[0].dot(v), self.m[1].dot(v), self.m[2].dot(v))
    }

    pub fn is_identity(self) -> bool {
        self == Mat3::IDENTITY
    }

    pub fn to_array(self) -> [f32; 9] {
        let m = self.m;
        [m[0].x, m[0].y, m[0].z, m[1].x, m[1].y, m[1].z, m[2].x, m[2].y, m[2].z]
    }
}

/// An axis-aligned box; components may be infinite (unbounded shapes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb { min: Vec3::splat(f32::INFINITY), max: Vec3::splat(f32::NEG_INFINITY) };
    pub const INFINITE: Aabb = Aabb { min: Vec3::splat(f32::NEG_INFINITY), max: Vec3::splat(f32::INFINITY) };

    pub fn new(min: Vec3, max: Vec3) -> Aabb {
        Aabb { min, max }
    }
    pub fn cube(h: f32) -> Aabb {
        Aabb::new(Vec3::splat(-h), Vec3::splat(h))
    }
    pub fn half(h: Vec3) -> Aabb {
        Aabb::new(-h, h)
    }
    pub fn union(self, b: Aabb) -> Aabb {
        Aabb::new(self.min.min(b.min), self.max.max(b.max))
    }
    pub fn intersect(self, b: Aabb) -> Aabb {
        Aabb::new(self.min.max(b.min), self.max.min(b.max))
    }
    pub fn grow(self, r: f32) -> Aabb {
        Aabb::new(self.min - Vec3::splat(r), self.max + Vec3::splat(r))
    }
    pub fn is_finite(self) -> bool {
        self.min.is_finite() && self.max.is_finite()
    }
    pub fn is_empty(self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }
    pub fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    pub fn size(self) -> Vec3 {
        self.max - self.min
    }
    pub fn area(self) -> f32 {
        let s = self.size();
        2.0 * (s.x * s.y + s.y * s.z + s.z * s.x)
    }
    /// The box around this one after `p -> rot * (p * s) + t`. Infinite boxes
    /// stay infinite on every axis a rotation mixes them into.
    pub fn transform(self, rot: Mat3, s: f32, t: Vec3) -> Aabb {
        if self.is_empty() {
            return self;
        }
        if !self.is_finite() {
            if rot.is_identity() {
                return Aabb::new(self.min * s + t, self.max * s + t);
            }
            // a rotated half-space: keep the axes the rotation leaves alone
            let mut out = Aabb::INFINITE;
            for a in 0..3 {
                let row = rot.m[a];
                let only = (0..3).filter(|&k| row.get(k).abs() > 1e-6).collect::<Vec<_>>();
                if only.len() == 1 {
                    let k = only[0];
                    let c = row.get(k) * s;
                    let (lo, hi) = (self.min.get(k) * c, self.max.get(k) * c);
                    let (lo, hi) = (lo.min(hi), lo.max(hi));
                    set(&mut out.min, a, lo + t.get(a));
                    set(&mut out.max, a, hi + t.get(a));
                }
            }
            return out;
        }
        let mut out = Aabb::EMPTY;
        for i in 0..8 {
            let c = Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            );
            let p = rot.apply(c * s) + t;
            out.min = out.min.min(p);
            out.max = out.max.max(p);
        }
        out
    }
}

fn set(v: &mut Vec3, axis: usize, x: f32) {
    match axis {
        0 => v.x = x,
        1 => v.y = x,
        _ => v.z = x,
    }
}
