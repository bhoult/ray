//! Rigid-body physics for drag, drop and throw, without meshes: each moving
//! object is a cloud of points sampled on its own distance surface.
//!
//! - Against the fixed scene (planes, terrain, resting objects) a body's
//!   points query the scene's distance field (render::trace::dist_query,
//!   leaving out every moving body).
//! - Between two moving bodies, each one's points are evaluated in the other's
//!   own distance program (or NURBS surface) at its exact current pose.
//!
//! A resting object that a body touches wakes up and joins the simulation
//! (collisions move everything that is not locked); a body that has been
//! still for half a second goes back to sleep in the scene. Contacts merge per
//! surface into one point each and are resolved with sequential impulses,
//! Coulomb friction, restitution, friction against spinning in place, and
//! positional correction. A carried body moves with the mouse and is
//! infinitely heavy: it shoves what it touches.

use crate::math::{Aabb, Mat3, Vec3};
use crate::render::trace::{dist_query, sdf_normal};
use crate::render::vm::{Regs, eval};
use std::collections::HashMap;
use crate::render::shade::water_surface;
use crate::render::{MAX_RIPPLES, Ripple};
use crate::scene::Scene;
use crate::scene::json::{MatRef, NodeDef};
use crate::scene::nurbs::Nurbs;
use crate::scene::sdf::{Compiler, MaterialSource, Program};
use std::sync::Arc;
use crate::settings::Settings;
use crate::simd::{F8, M8, V8};

/// (the scene's gravity: Scene::gravity)
/// Default density (kg per cubic metre of bounding box, half filled).
pub const DENSITY: f32 = 300.0;
/// Simulation step (seconds).
const DT: f32 = 1.0 / 240.0;
/// The shortest step: fast bodies take smaller ones, so that none moves more
/// than `TRAVEL` of its radius per step (a thrown object does not pass
/// through a wall), down to this.
const DT_MIN: f32 = DT / 32.0;
const TRAVEL: f32 = 0.3;
/// Steps per frame at most (beyond that the simulation runs slow).
const MAX_STEPS: u32 = 400;
/// Penetration allowed before positions are corrected (metres).
const SLOP: f32 = 0.002;
/// Distance at which a point counts as touching (metres).
const CONTACT: f32 = 0.004;
/// Seconds of holding the right button for a full-strength throw.
pub const CHARGE_TIME: f32 = 1.2;
/// kg per cubic metre: a body lighter than water floats.
const WATER_DENSITY: f32 = 1000.0;
/// Damping ratio of a floating body's bobbing (the waves it makes carry the
/// energy off): it settles within a few rises.
const BOB_DAMPING: f32 = 0.15;

/// The water a body floats in this step.
pub struct Water<'a> {
    pub level: f32,
    /// how far above `level` the waves and ripples can reach
    pub reach: f32,
    pub time: f32,
    pub wave_time: f32,
    pub height: f32,
    pub ripples: &'a [Ripple],
    /// the scene's gravity (buoyancy is the weight of the water pushed aside)
    pub gravity: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Quat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quat {
    pub fn from_mat(m: Mat3) -> Quat {
        let r = m.m;
        let tr = r[0].x + r[1].y + r[2].z;
        let q = if tr > 0.0 {
            let s = (tr + 1.0).sqrt() * 2.0;
            Quat { w: 0.25 * s, x: (r[2].y - r[1].z) / s, y: (r[0].z - r[2].x) / s, z: (r[1].x - r[0].y) / s }
        } else if r[0].x > r[1].y && r[0].x > r[2].z {
            let s = (1.0 + r[0].x - r[1].y - r[2].z).sqrt() * 2.0;
            Quat { w: (r[2].y - r[1].z) / s, x: 0.25 * s, y: (r[0].y + r[1].x) / s, z: (r[0].z + r[2].x) / s }
        } else if r[1].y > r[2].z {
            let s = (1.0 + r[1].y - r[0].x - r[2].z).sqrt() * 2.0;
            Quat { w: (r[0].z - r[2].x) / s, x: (r[0].y + r[1].x) / s, y: 0.25 * s, z: (r[1].z + r[2].y) / s }
        } else {
            let s = (1.0 + r[2].z - r[0].x - r[1].y).sqrt() * 2.0;
            Quat { w: (r[1].x - r[0].y) / s, x: (r[0].z + r[2].x) / s, y: (r[1].z + r[2].y) / s, z: 0.25 * s }
        };
        q.normalize()
    }

    pub fn normalize(self) -> Quat {
        let l = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt().max(1e-12);
        Quat { w: self.w / l, x: self.x / l, y: self.y / l, z: self.z / l }
    }

    pub fn to_mat(self) -> Mat3 {
        let Quat { w, x, y, z } = self;
        Mat3 {
            m: [
                Vec3::new(1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y)),
                Vec3::new(2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x)),
                Vec3::new(2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y)),
            ],
        }
    }

    /// Turns by angular velocity `w` for `dt` seconds.
    pub fn integrate(self, w: Vec3, dt: f32) -> Quat {
        let h = 0.5 * dt;
        let (qw, qx, qy, qz) = (self.w, self.x, self.y, self.z);
        Quat {
            w: qw + h * (-w.x * qx - w.y * qy - w.z * qz),
            x: qx + h * (w.x * qw + w.y * qz - w.z * qy),
            y: qy + h * (w.y * qw + w.z * qx - w.x * qz),
            z: qz + h * (w.z * qw + w.x * qy - w.y * qx),
        }
        .normalize()
    }
}

/// Euler angles in degrees (x, then y, then z: R = Rz Ry Rx, as
/// Mat3::euler_deg) of a rotation.
pub fn euler_deg(m: Mat3) -> [f32; 3] {
    m.to_euler_deg()
}

struct NoMaterials;

impl MaterialSource for NoMaterials {
    fn resolve(&mut self, _: &MatRef) -> Result<u16, String> {
        Ok(0)
    }
}

/// The density (kg per cubic metre) of a node's material, if it gives one:
/// its own material or the scene material it names; for a node without one,
/// the first in its tree.
pub fn density_of(doc: &serde_json::Value, node: &serde_json::Value) -> Option<f32> {
    use serde_json::Value;
    let m = match node.get("material") {
        Some(Value::String(name)) => doc.get("materials").and_then(|ms| ms.get(name.as_str())),
        Some(m @ Value::Object(_)) => Some(m),
        _ => None,
    };
    match m {
        Some(m) => m.get("density").and_then(Value::as_f64).map(|d| d as f32),
        None => super::doc::children(node).into_iter().find_map(|c| density_of(doc, c)),
    }
}

/// A body for the document's top-level object `top`, if it can move.
pub fn body_of(doc: &serde_json::Value, top: usize) -> Option<Body> {
    let node = doc.get("objects")?.get(top)?;
    let def = serde_json::from_value::<NodeDef>(node.clone()).ok()?;
    Body::new(top, &def, density_of(doc, node))
}

/// The mass the document's top-level object `top` has without a `mass` of
/// its own (from its material's density and volume, or its size), and
/// whether it comes from the density.
pub fn natural_mass(doc: &serde_json::Value, top: usize) -> Option<(f32, bool)> {
    let node = doc.get("objects")?.get(top)?;
    let mut def = serde_json::from_value::<NodeDef>(node.clone()).ok()?;
    def.mass = None;
    let density = density_of(doc, node);
    Body::new(top, &def, density).map(|b| (b.mass, density.is_some()))
}

/// The automatic mass of an object whose bounding box is `size`.
pub fn auto_mass(size: Vec3) -> f32 {
    let s = size.max(Vec3::splat(0.05));
    (s.x * s.y * s.z * 0.5 * DENSITY).max(0.05)
}

/// A moving object.
pub struct Body {
    /// `objects[top]` of the document, and its index in the compiled scene
    pub top: usize,
    pub obj: Option<u32>,
    /// centre of mass (world), orientation, velocity, angular velocity
    pub x: Vec3,
    pub q: Quat,
    pub v: Vec3,
    pub w: Vec3,
    /// centre of mass in the node's own frame (relative to its position)
    com: Vec3,
    pub mass: f32,
    inv_mass: f32,
    /// inverse inertia about the centre of mass, body frame (diagonal)
    inv_inertia: Vec3,
    /// surface points relative to the centre of mass, body frame
    points: Vec<Vec3>,
    /// the shape in the node's own frame
    form: Form,
    pub friction: f32,
    pub bounce: f32,
    /// falls (off: it floats where it is let go, slowing to a stop)
    pub gravity: bool,
    /// seconds at rest, and since it was last moved by hand
    pub rest: f32,
    pub age: f32,
    /// kinematic: carried by the mouse
    pub held: bool,
    /// bounding radius around the centre of mass
    pub radius: f32,
    /// the speed of the impact that woke it, during the step it was woken in
    /// (what it rests against wakes too, so a hit can move a whole stack)
    jolt: f32,
    /// points filling its inside (relative to the centre of mass, body frame)
    /// and the size of the grid cell each stands for: the water they displace
    interior: Vec<Vec3>,
    cell: Vec3,
    /// cubic metres of it under water in the last step
    pub wet: f32,
    /// when it last set off a ripple
    last_ripple: f32,
}

/// A body's shape in its own frame: a distance program, or a NURBS surface
/// at a scale.
enum Form {
    Sdf(Program),
    Nurbs { surf: Arc<Nurbs>, s: f32 },
}

impl Form {
    fn dist(&self, p: V8, regs: &mut Regs) -> F8 {
        match self {
            Form::Sdf(prog) => eval(prog, p, regs, false).0,
            Form::Nurbs { surf, s } => {
                let k = 1.0 / s;
                crate::render::nurbs::dist8(surf, p * k, surf.size * 2.0 + 1.0, M8::all_true(), false) * *s
            }
        }
    }

    /// The unit gradient of the distance (tetrahedral differences).
    fn normal(&self, p: V8, regs: &mut Regs, eps: F8) -> V8 {
        match self {
            Form::Sdf(prog) => sdf_normal(prog, regs, p, p, eps),
            Form::Nurbs { .. } => {
                const K: [[f32; 3]; 4] = [[1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, 1.0]];
                let mut g = V8::zero();
                for k in K {
                    let q = V8::new(F8::splat(k[0]).mul_add(eps, p.x), F8::splat(k[1]).mul_add(eps, p.y), F8::splat(k[2]).mul_add(eps, p.z));
                    let d = self.dist(q, regs);
                    g = V8::new(F8::splat(k[0]).mul_add(d, g.x), F8::splat(k[1]).mul_add(d, g.y), F8::splat(k[2]).mul_add(d, g.z));
                }
                g * g.length_sq().max(F8::splat(1.0e-30)).rsqrt()
            }
        }
    }
}

impl Body {
    /// A body for top-level node `node` (`objects[top]`), of the material
    /// `density` (kg per cubic metre) if it gives one; None for shapes that
    /// cannot move (unbounded, or nothing to sample).
    pub fn new(top: usize, node: &NodeDef, density: Option<f32>) -> Option<Body> {
        let mut local = node.clone();
        local.position = None;
        local.rotation = None;
        let (form, bound) = match Nurbs::from_shape(&local.shape) {
            Ok(Some(surf)) => {
                let s = local.scale.unwrap_or(1.0);
                let bound = Aabb::new(surf.aabb.min * s, surf.aabb.max * s);
                (Form::Nurbs { surf: Arc::new(surf), s }, bound)
            }
            Ok(None) => {
                let mut mats = NoMaterials;
                let mut c = Compiler::new(&mut mats);
                let (_, bound) = c.node(&local, 0, 0).ok()?;
                (Form::Sdf(c.finish()), bound)
            }
            Err(_) => return None,
        };
        if !bound.is_finite() || bound.is_empty() {
            return None;
        }
        let points = sample_surface(&form, bound);
        let (interior, cell) = sample_interior(&form, bound, &points);
        if points.len() < 4 {
            return None;
        }
        let com = points.iter().fold(Vec3::ZERO, |a, &p| a + p) / points.len() as f32;
        let size = bound.size().max(Vec3::splat(0.05));
        // its own mass, else its material's density times its volume, else from its size
        let volume = interior.len() as f32 * cell.x * cell.y * cell.z;
        let mass = node.mass.filter(|m| *m > 0.0).unwrap_or_else(|| match density {
            Some(d) if d > 0.0 => (d * volume).max(0.01),
            _ => auto_mass(size),
        });
        let i = |a: f32, b: f32| mass / 12.0 * (a * a + b * b);
        let inertia = Vec3::new(i(size.y, size.z), i(size.x, size.z), i(size.x, size.y));
        let rot = node.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
        let pos = node.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
        let points: Vec<Vec3> = points.iter().map(|&p| p - com).collect();
        let radius = points.iter().map(|p| p.length()).fold(0.0, f32::max);
        Some(Body {
            top,
            obj: None,
            x: pos + rot.apply(com),
            q: Quat::from_mat(rot),
            v: Vec3::ZERO,
            w: Vec3::ZERO,
            com,
            mass,
            inv_mass: 1.0 / mass,
            inv_inertia: Vec3::new(1.0 / inertia.x, 1.0 / inertia.y, 1.0 / inertia.z),
            points,
            form,
            friction: node.friction.unwrap_or(0.5),
            bounce: node.bounce.unwrap_or(0.15),
            gravity: node.gravity.unwrap_or(true),
            rest: 0.0,
            age: 0.0,
            held: false,
            radius,
            jolt: 0.0,
            interior: interior.iter().map(|&p| p - com).collect(),
            cell,
            wet: 0.0,
            last_ripple: f32::NEG_INFINITY,
        })
    }

    /// The node's position and rotation (Euler degrees) for the document.
    pub fn placement(&self) -> ([f32; 3], [f32; 3]) {
        let r = self.q.to_mat();
        let pos = self.x - r.apply(self.com);
        (pos.to_array(), euler_deg(r))
    }

    /// Moves the centre of mass to `x` (carried by the mouse); the velocity
    /// follows the motion, so letting go keeps it.
    pub fn hold_at(&mut self, x: Vec3, dt: f32) {
        if dt > 0.0 {
            let v = (x - self.x) / dt;
            self.v = self.v * 0.5 + v * 0.5;
            let s = self.v.length();
            if s > 25.0 {
                self.v = self.v * (25.0 / s);
            }
        }
        self.x = x;
        self.w = Vec3::ZERO;
        self.rest = 0.0;
        self.age = 0.0;
    }

    /// Turns the body by `angle` radians about the world axis `axis`, about
    /// its centre of mass.
    pub fn rotate(&mut self, axis: Vec3, angle: f32) {
        let a = axis.normalize();
        let (s, c) = (angle * 0.5).sin_cos();
        let d = Quat { w: c, x: a.x * s, y: a.y * s, z: a.z * s };
        let q = self.q;
        self.q = Quat {
            w: d.w * q.w - d.x * q.x - d.y * q.y - d.z * q.z,
            x: d.w * q.x + d.x * q.w + d.y * q.z - d.z * q.y,
            y: d.w * q.y - d.x * q.z + d.y * q.w + d.z * q.x,
            z: d.w * q.z + d.x * q.y - d.y * q.x + d.z * q.w,
        }
        .normalize();
    }

    /// The gravity gun's shot, hitting it at world point `at` along unit
    /// `dir`: its centre sent off at `speed`, and spun by where it was hit
    /// (a quarter of what a shove there would: a full shot spun things
    /// wildly).
    pub fn punt(&mut self, at: Vec3, dir: Vec3, speed: f32) {
        self.held = false;
        let r = self.q.to_mat();
        self.v += dir * speed;
        self.w += self.inv_inertia_world(r, (at - self.x).cross(dir * (speed * self.mass))) * 0.25;
        self.age = 0.0;
        self.rest = 0.0;
    }

    /// Lets go with a throw: `charge` 0..1 along unit `dir`. Heavier bodies fly
    /// slower (300 m/s at full charge for 5 kg and less).
    pub fn throw(&mut self, dir: Vec3, charge: f32) {
        let speed = throw_speed(self.mass, charge);
        self.held = false;
        self.v = self.v * 0.3 + dir * speed;
        self.age = 0.0;
        self.rest = 0.0;
    }

    pub fn asleep(&self) -> bool {
        !self.held && (self.rest > 0.5 || self.age > 20.0 || self.x.y < -500.0)
    }

    /// In the water: buoyancy, the weight of the water each part under the
    /// surface displaces, pushing at that part along the surface's normal
    /// there (so a floating body rights itself, bobs, and rides and rocks
    /// on the waves); the water it drags along (added mass); and drag.
    /// Returns the height of a ripple to set off at `time`, if it moves
    /// through the surface (a carried body too, without the forces).
    fn float(&mut self, w: &Water, h: f32, time: f32) -> Option<f32> {
        let was_wet = self.wet > 0.0;
        self.wet = 0.0;
        if self.x.y - self.radius > w.level + w.reach {
            return None;
        }
        let r = self.q.to_mat();
        let vol = self.cell.x * self.cell.y * self.cell.z;
        // a part counts gradually as the surface passes it, so the lift is smooth
        let depth = vol.cbrt();
        let (mut force, mut torque, mut wet) = (Vec3::ZERO, Vec3::ZERO, 0.0);
        let mut crossing = false;
        // the parts in the surface: their volume over the layer's depth is the
        // area the body cuts the water in
        let mut layer = 0.0;
        for chunk in self.interior.chunks(8) {
            let levers: Vec<Vec3> = chunk.iter().map(|&p| r.apply(p)).collect();
            let mut a = [[0.0f32; 8]; 2];
            for (i, l) in levers.iter().enumerate() {
                (a[0][i], a[1][i]) = (self.x.x + l.x, self.x.z + l.z);
            }
            let (sy, sx, sz) = water_surface(w.time, w.wave_time, w.ripples, w.height, F8::from_array(a[0]), F8::from_array(a[1]));
            let (sy, sx, sz) = (sy.to_array(), sx.to_array(), sz.to_array());
            for (i, lever) in levers.iter().enumerate() {
                let under = ((w.level + sy[i] - self.x.y - lever.y) / depth + 0.5).clamp(0.0, 1.0);
                if under <= 0.0 {
                    continue;
                }
                if under < 1.0 {
                    crossing = true;
                    layer += vol;
                }
                let normal = Vec3::new(-sx[i], 1.0, -sz[i]).normalize();
                let push = normal * (WATER_DENSITY * w.gravity * vol * under);
                force += push;
                torque += lever.cross(push);
                wet += vol * under;
            }
        }
        // a ripple where it moves through the surface: bigger for faster and
        // bigger bodies, twice as big where it first breaks the surface
        let mut ripple = None;
        let stir = self.v.length() + self.w.length() * self.radius * 0.5;
        if crossing && stir > 0.15 && time - self.last_ripple > 0.15 {
            self.last_ripple = time;
            let splash = if was_wet { 1.0 } else { 2.0 };
            ripple = Some((stir * self.radius * 0.012 * splash).clamp(0.002, 0.06));
        }
        if wet <= 0.0 || self.held {
            self.wet = if self.held { wet } else { 0.0 };
            return ripple;
        }
        self.wet = wet;
        let total = self.interior.len() as f32 * vol;
        // the water it pushes aside moves with it (added mass: half the water
        // displaced, as for a ball), so a light body is not flung out faster
        // than the water lets it. Gravity was applied to its own mass alone:
        // the added mass's share comes back here
        let added = 0.5 * WATER_DENSITY * wet;
        let m = self.mass + added;
        self.v += (force + Vec3::Y * (added * w.gravity)) * (h / m);
        self.w += self.inv_inertia_world(r, torque) * (h * self.mass / m);
        // drag: quadratic in speed over the wetted cross-section (Cd 0.5);
        // while it lies in the surface, also damping of its bobbing: a share
        // of critical for the stiffness its waterline area gives it, fading
        // for fast motion (a ball let go deep still leaps out). Implicit
        let area = 1.2 * wet.powf(2.0 / 3.0);
        let stiffness = WATER_DENSITY * w.gravity * layer / depth;
        let bob = 2.0 * BOB_DAMPING * (stiffness * m).sqrt() / (1.0 + 2.0 * self.v.y.abs());
        let c = 0.25 * WATER_DENSITY * area * self.v.length() + bob;
        self.v = self.v * (1.0 / (1.0 + c * h / m));
        self.w = self.w * (1.0 / (1.0 + h * (c / m + 2.0 * wet / total.max(1e-6))));
        ripple
    }

    fn im(&self) -> f32 {
        if self.held { 0.0 } else { self.inv_mass }
    }

    /// R I^-1 R^T v (zero while carried: it does not yield)
    fn inv_inertia_world(&self, r: Mat3, v: Vec3) -> Vec3 {
        if self.held {
            return Vec3::ZERO;
        }
        let b = r.transpose().apply(v);
        r.apply(Vec3::new(b.x * self.inv_inertia.x, b.y * self.inv_inertia.y, b.z * self.inv_inertia.z))
    }

    fn apply(&mut self, r: Mat3, lever: Vec3, impulse: Vec3) {
        if self.held {
            return;
        }
        self.v += impulse * self.inv_mass;
        self.w += self.inv_inertia_world(r, lever.cross(impulse));
    }

    /// The effective inverse mass along `n` at lever arm `lever`.
    fn k(&self, r: Mat3, lever: Vec3, n: Vec3) -> f32 {
        self.im() + self.inv_inertia_world(r, lever.cross(n)).cross(lever).dot(n)
    }

    fn velocity_at(&self, lever: Vec3) -> Vec3 {
        self.v + self.w.cross(lever)
    }

    /// World points to the node frame of the program.
    fn to_program(&self, r: Mat3, p: V8) -> V8 {
        let d = p - self.x;
        let rt = r.transpose();
        let m = &rt.m;
        V8::new(d.dot3(m[0]) + self.com.x, d.dot3(m[1]) + self.com.y, d.dot3(m[2]) + self.com.z)
    }
}

/// Game mode's gravity gun: seconds to full charge, the speed (m/s) a full
/// shot sends a body of GUN_MASS kg or less off at, and its reach (m, for a
/// person: more for a bigger walker).
pub const GUN_CHARGE: f32 = 1.5;
pub const GUN_SPEED: f32 = 40.0;
pub const GUN_MASS: f32 = 20.0;
pub const GUN_REACH: f32 = 30.0;

/// The speed (m/s) the gravity gun sends a body of `mass` kg off at, at
/// `charge` 0..1: a tap nudges it, held to full GUN_SPEED (heavier bodies
/// slower).
pub fn punt_speed(mass: f32, charge: f32) -> f32 {
    GUN_SPEED * (0.1 + 0.9 * charge.clamp(0.0, 1.0)) * (GUN_MASS / mass.max(1e-3)).powf(0.4).min(1.0)
}

/// Speed (m/s) of a throw of a body of `mass` kg at `charge` 0..1: 300 m/s at
/// full charge for 5 kg or less, slower for heavier bodies.
pub fn throw_speed(mass: f32, charge: f32) -> f32 {
    300.0 * charge.clamp(0.0, 1.0) * (5.0 / mass.max(1e-3)).powf(0.3).min(1.0)
}

/// A raw contact: body `a` touches body `b` (or the fixed scene) at world
/// point `p`; `n` pushes `a` away; friction and bounce of the pair.
#[derive(Clone, Copy)]
struct Touch {
    a: usize,
    b: Option<usize>,
    p: Vec3,
    n: Vec3,
    depth: f32,
    mu: f32,
    e: f32,
}

/// A merged contact: one of up to four points spread over the touching
/// points of one surface, and the radius of that patch (for friction
/// against spinning in place).
struct Contact {
    t: Touch,
    patch: f32,
    /// the patch's deepest point: it carries the push out of penetration
    push: bool,
}

#[derive(Default)]
pub struct World {
    pub bodies: Vec<Body>,
    /// the water's clock (seconds; set before each step), the clock its
    /// waves drift by (0: still), whether they are moving, and the ripples
    /// spreading on it
    pub time: f32,
    pub wave_time: f32,
    pub waves_moving: bool,
    pub ripples: Vec<Ripple>,
    /// the compiled objects of each group (by its index in the document): a
    /// group moves as one body, none of whose members is fixed scene to it
    parts: HashMap<usize, Vec<u32>>,
}

impl World {
    /// Adds a ripple; when there are as many as the renderer takes, it
    /// replaces the faintest.
    fn ripple(&mut self, r: Ripple) {
        let now = self.time;
        self.ripples.retain(|q| q.alive(now));
        if self.ripples.len() >= MAX_RIPPLES {
            let faint = (0..self.ripples.len())
                .min_by(|&a, &b| {
                    let (qa, qb) = (&self.ripples[a], &self.ripples[b]);
                    qa.height(now - qa.t0).total_cmp(&qb.height(now - qb.t0))
                })
                .unwrap();
            self.ripples.swap_remove(faint);
        }
        self.ripples.push(r);
    }

    pub fn find(&self, top: usize) -> Option<usize> {
        self.bodies.iter().position(|b| b.top == top)
    }

    /// Advances every body by `dt` seconds. `wake(object)` turns a resting
    /// scene object that got hit into a body (None: it stays put).
    pub fn step(&mut self, scene: &Scene, s: &Settings, dt: f32, wake: &mut dyn FnMut(u32) -> Option<Body>) {
        if self.bodies.is_empty() {
            return;
        }
        let mut left = dt.min(0.05);
        self.parts.clear();
        for (i, o) in scene.objects.iter().enumerate() {
            if !o.path.is_empty() {
                self.parts.entry(o.doc_index).or_default().push(i as u32);
            }
        }
        crate::render::query(scene, s, |ctx, regs| {
            let mut steps = 0;
            while left > 1e-6 && steps < MAX_STEPS {
                let fastest = self
                    .bodies
                    .iter()
                    .filter(|b| !b.held)
                    .map(|b| TRAVEL * b.radius.max(0.05) / b.v.length().max(1e-6))
                    .fold(DT, f32::min);
                let h = left.min(fastest.max(DT_MIN));
                left -= h;
                steps += 1;
                self.substep(ctx, regs, h, wake);
            }
        });
        for b in &mut self.bodies {
            if b.held {
                continue;
            }
            b.age += dt;
            if b.wet > 0.0 && self.waves_moving {
                // riding moving waves: it never comes to rest
                (b.age, b.rest) = (0.0, 0.0);
            } else if b.v.length() < if b.wet > 0.0 { 0.03 } else { 0.08 } && b.w.length() < if b.wet > 0.0 { 0.1 } else { 0.25 } {
                // (afloat, it must be stiller: a slow bob is not rest)
                b.rest += dt;
            } else {
                b.rest = 0.0;
            }
        }
    }

    fn substep(&mut self, ctx: &crate::render::Ctx, regs: &mut Regs, h: f32, wake: &mut dyn FnMut(u32) -> Option<Body>) {
        for b in &mut self.bodies {
            if !b.held && b.gravity {
                b.v.y -= ctx.scene.gravity * h;
            }
        }
        if let Some((level, _)) = ctx.scene.water {
            let mut made = Vec::new();
            let ripples = std::mem::take(&mut self.ripples);
            let w = Water {
                level,
                reach: 0.25 * ctx.scene.wave_height + 0.12,
                time: self.time,
                wave_time: self.wave_time,
                height: ctx.scene.wave_height,
                ripples: &ripples,
                gravity: ctx.scene.gravity,
            };
            for b in &mut self.bodies {
                if let Some(amp) = b.float(&w, h, self.time) {
                    made.push(Ripple { x: b.x.x, z: b.x.z, t0: self.time, amp });
                }
            }
            self.ripples = ripples;
            for r in made {
                self.ripple(r);
            }
        }
        self.time += h;
        let mut touches = Vec::new();
        // against the fixed scene (a carried body passes through it, but still
        // wakes what it touches). What gets hit joins in at once, in another
        // pass, so the hit moves it instead of bouncing off it as off a wall;
        // after a hard hit, so does what that rests against
        for pass in 0..4 {
            touches.clear();
            let moving: Vec<u32> = self.bodies.iter().flat_map(|b| self.parts.get(&b.top).cloned().unwrap_or_else(|| b.obj.into_iter().collect())).collect();
            // (object, impact, and whose: a group's members wake one body)
            let mut woken: Vec<(u32, f32, usize)> = Vec::new();
            for (i, b) in self.bodies.iter().enumerate() {
                let r = b.q.to_mat();
                let search = (b.v.length() * h * 2.0 + 0.05).max(CONTACT * 2.0);
                for chunk in b.points.chunks(8) {
                    let (pw, n) = world_points(b, r, chunk);
                    let (d, who) = dist_query(ctx, regs, pw, search, M8::none_set(), &moving);
                    let touching = d.lt(F8::splat(CONTACT));
                    if touching.none() {
                        continue;
                    }
                    let (dd, ww) = (d.to_array(), who.to_array());
                    let g = scene_gradient(ctx, regs, pw, search, &moving);
                    for k in 0..n {
                        if dd[k] >= CONTACT || ww[k] < 0 {
                            continue;
                        }
                        let o = &ctx.scene.objects[ww[k] as usize];
                        // only a real impact (or a push by the carried body) sets a resting
                        // object moving; resting contact does not, or stacks never sleep
                        let approach = -b.velocity_at(pw.lane(k) - b.x).dot(g.lane(k));
                        let jolt = approach.max(b.jolt * 0.8);
                        if !o.locked && (approach > 0.2 || b.jolt > 1.0) && !woken.iter().any(|w| w.2 == o.doc_index) {
                            woken.push((ww[k] as u32, jolt, o.doc_index));
                        }
                        if !b.held {
                            touches.push(Touch {
                                a: i,
                                b: None,
                                p: pw.lane(k),
                                n: g.lane(k),
                                depth: dd[k],
                                mu: (b.friction * o.friction).sqrt(),
                                e: (b.bounce + o.bounce) * 0.5,
                            });
                        }
                    }
                }
            }
            let before = self.bodies.len();
            if pass < 3 {
                for (o, jolt, top) in woken {
                    if self.bodies.iter().any(|b| b.top == top) {
                        continue;
                    }
                    if let Some(mut b) = wake(o) {
                        b.obj = Some(o);
                        b.jolt = jolt;
                        self.bodies.push(b);
                    }
                }
            }
            if self.bodies.len() == before {
                break;
            }
        }
        for b in &mut self.bodies {
            b.jolt = 0.0;
        }
        // between moving bodies: each one's points in the other's shape
        for i in 0..self.bodies.len() {
            for j in 0..self.bodies.len() {
                if i == j {
                    continue;
                }
                let (a, b) = (&self.bodies[i], &self.bodies[j]);
                if a.held && b.held {
                    continue;
                }
                if (a.x - b.x).length() > a.radius + b.radius + 0.05 {
                    continue;
                }
                let (ra, rb) = (a.q.to_mat(), b.q.to_mat());
                for chunk in a.points.chunks(8) {
                    let (pw, n) = world_points(a, ra, chunk);
                    let q = b.to_program(rb, pw);
                    let d = b.form.dist(q, regs);
                    if d.lt(F8::splat(CONTACT)).none() {
                        continue;
                    }
                    let gl = b.form.normal(q, regs, F8::splat(0.002));
                    let gw = V8::new(gl.dot3(rb.m[0]), gl.dot3(rb.m[1]), gl.dot3(rb.m[2]));
                    let dd = d.to_array();
                    for k in 0..n {
                        if dd[k] < CONTACT {
                            touches.push(Touch {
                                a: i,
                                b: Some(j),
                                p: pw.lane(k),
                                n: gw.lane(k),
                                depth: dd[k],
                                mu: (a.friction * b.friction).sqrt(),
                                e: (a.bounce + b.bounce) * 0.5,
                            });
                        }
                    }
                }
            }
        }
        let contacts = merge(&touches);
        let mut touched = vec![false; self.bodies.len()];
        for c in &contacts {
            touched[c.t.a] = true;
            if let Some(b) = c.t.b {
                touched[b] = true;
            }
        }
        for _ in 0..6 {
            for c in &contacts {
                self.solve(c);
            }
        }
        // push out of penetration (once per patch), split by inverse mass
        for c in contacts.iter().filter(|c| c.push) {
            let pen = -c.t.depth - SLOP;
            if pen <= 0.0 {
                continue;
            }
            let ia = self.bodies[c.t.a].im();
            let ib = c.t.b.map_or(0.0, |b| self.bodies[b].im());
            let sum = ia + ib;
            if sum <= 0.0 {
                continue;
            }
            let push = c.t.n * (pen * 0.6 / sum);
            self.bodies[c.t.a].x += push * ia;
            if let Some(b) = c.t.b {
                self.bodies[b].x += push * -ib;
            }
        }
        for (i, b) in self.bodies.iter_mut().enumerate() {
            if b.held {
                continue;
            }
            if touched[i] {
                // rolling resistance
                b.w = b.w * (1.0 - 1.5 * h);
            }
            b.v = b.v * (1.0 - 0.05 * h);
            b.w = b.w * (1.0 - 0.6 * h);
            if !b.gravity {
                // floating: a push moves it a little way, then it stops
                b.v = b.v * (1.0 - 3.0 * h);
                b.w = b.w * (1.0 - 3.0 * h);
            }
            b.x += b.v * h;
            b.q = b.q.integrate(b.w, h);
        }
    }

    /// Normal impulse with restitution (for real impacts), Coulomb friction
    /// and friction against spinning, between a contact's two sides.
    fn solve(&mut self, c: &Contact) {
        let t = &c.t;
        let (a, mut b) = match t.b {
            Some(j) => {
                let (lo, hi) = self.bodies.split_at_mut(t.a.max(j));
                if t.a < j { (&mut lo[t.a], Some(&mut hi[0])) } else { (&mut hi[0], Some(&mut lo[j])) }
            }
            None => (&mut self.bodies[t.a], None),
        };
        let ra_m = a.q.to_mat();
        let rb_m = b.as_ref().map(|b| b.q.to_mat()).unwrap_or(Mat3::IDENTITY);
        let la = t.p - a.x;
        let lb = b.as_ref().map(|b| t.p - b.x).unwrap_or(Vec3::ZERO);
        let rel = |a: &Body, b: &Option<&mut Body>| a.velocity_at(la) - b.as_ref().map_or(Vec3::ZERO, |b| b.velocity_at(lb));
        let n = t.n;
        let vn = rel(a, &b).dot(n);
        if vn >= 0.0 {
            return;
        }
        let e = if vn < -0.8 { t.e } else { 0.0 };
        let kn = a.k(ra_m, la, n) + b.as_ref().map_or(0.0, |b| b.k(rb_m, lb, n));
        if kn <= 1e-9 {
            return;
        }
        let jn = -(1.0 + e) * vn / kn;
        a.apply(ra_m, la, n * jn);
        if let Some(b) = b.as_mut() {
            b.apply(rb_m, lb, n * -jn);
        }
        let vr = rel(a, &b);
        let vt = vr - n * vr.dot(n);
        let st = vt.length();
        if st > 1e-6 {
            let tdir = vt / st;
            let kt = a.k(ra_m, la, tdir) + b.as_ref().map_or(0.0, |b| b.k(rb_m, lb, tdir));
            let jt = (st / kt.max(1e-9)).min(t.mu * jn);
            a.apply(ra_m, la, tdir * -jt);
            if let Some(b) = b.as_mut() {
                b.apply(rb_m, lb, tdir * jt);
            }
        }
        // a touching patch resists spinning about its normal
        let spin = a.w.dot(n) - b.as_ref().map_or(0.0, |b| b.w.dot(n));
        let k = a.inv_inertia_world(ra_m, n).dot(n) + b.as_ref().map_or(0.0, |b| b.inv_inertia_world(rb_m, n).dot(n));
        if k > 1e-9 {
            let limit = (t.mu * jn * c.patch).max(0.0);
            let j = (spin / k).clamp(-limit, limit);
            let da = a.inv_inertia_world(ra_m, n * j);
            a.w = a.w - da;
            if let Some(b) = b.as_mut() {
                let db = b.inv_inertia_world(rb_m, n * j);
                b.w += db;
            }
        }
    }
}

/// Up to eight body points in world space (the rest of the octet repeats
/// the first), and how many are real.
fn world_points(b: &Body, r: Mat3, chunk: &[Vec3]) -> (V8, usize) {
    let mut a = [[0.0f32; 8]; 3];
    for i in 0..8 {
        let p = b.x + r.apply(chunk[i.min(chunk.len() - 1)]);
        a[0][i] = p.x;
        a[1][i] = p.y;
        a[2][i] = p.z;
    }
    (V8::new(F8::from_array(a[0]), F8::from_array(a[1]), F8::from_array(a[2])), chunk.len())
}

/// Merges touches per pair of sides and surface (normals within 25 degrees)
/// into up to four points spread over the touching patch: the deepest, the
/// one farthest from it, and the farthest on either side of the line between
/// those. (One point at the patch's centre held a standing box up like a pin
/// under its middle: nothing resisted a tilt, and tall things, dominoes and
/// columns, rocked until they fell.) The deepest point carries the push out
/// of penetration, once per patch.
fn merge(touches: &[Touch]) -> Vec<Contact> {
    struct Group {
        t: Touch,
        n: Vec3,
        pts: Vec<(Vec3, f32)>,
    }
    let cos_max = 25f32.to_radians().cos();
    let mut groups: Vec<Group> = Vec::new();
    for t in touches {
        match groups.iter_mut().find(|g| g.t.a == t.a && g.t.b == t.b && g.n.normalize().dot(t.n) > cos_max) {
            Some(g) => {
                g.n += t.n;
                g.pts.push((t.p, t.depth));
                g.t.mu = g.t.mu.max(t.mu);
                g.t.e = g.t.e.max(t.e);
            }
            None => groups.push(Group { t: *t, n: t.n, pts: vec![(t.p, t.depth)] }),
        }
    }
    let mut out = Vec::new();
    for g in groups {
        let n = g.n.normalize();
        let count = g.pts.len() as f32;
        let centre = g.pts.iter().fold(Vec3::ZERO, |s, p| s + p.0) / count;
        // the patch's radius (friction against spinning in place)
        let patch = (g.pts.iter().map(|p| (p.0 - centre).length_sq()).sum::<f32>() / count).sqrt();
        let pick = |key: &dyn Fn(&(Vec3, f32)) -> f32| (0..g.pts.len()).max_by(|&i, &j| key(&g.pts[i]).total_cmp(&key(&g.pts[j]))).unwrap();
        let deepest = pick(&|p| -p.1);
        let a = g.pts[deepest].0;
        let far = pick(&|p| (p.0 - a).length_sq());
        let mut chosen = vec![deepest];
        let ab = g.pts[far].0 - a;
        if ab.length() > 1e-3 {
            chosen.push(far);
            // signed distance from the line a-b, in the patch's plane
            let side = |p: &(Vec3, f32)| (p.0 - a).cross(ab).dot(n);
            for i in [pick(&side), pick(&|p| -side(p))] {
                if !chosen.contains(&i) {
                    chosen.push(i);
                }
            }
        }
        for (k, &i) in chosen.iter().enumerate() {
            let (p, depth) = g.pts[i];
            out.push(Contact { t: Touch { p, n, depth, ..g.t }, patch, push: k == 0 });
        }
    }
    out
}

/// The fixed scene's distance gradient at eight points (tetrahedral differences).
fn scene_gradient(ctx: &crate::render::Ctx, regs: &mut Regs, p: V8, radius: f32, exclude: &[u32]) -> V8 {
    const K: [[f32; 3]; 4] = [[1.0, -1.0, -1.0], [-1.0, -1.0, 1.0], [-1.0, 1.0, -1.0], [1.0, 1.0, 1.0]];
    let e = 0.002;
    let mut g = V8::zero();
    for k in K {
        let q = p + Vec3::new(k[0] * e, k[1] * e, k[2] * e);
        let (d, _) = dist_query(ctx, regs, q, radius + e * 2.0, M8::none_set(), exclude);
        g = V8::new(F8::splat(k[0]).mul_add(d, g.x), F8::splat(k[1]).mul_add(d, g.y), F8::splat(k[2]).mul_add(d, g.z));
    }
    g * g.length_sq().max(F8::splat(1e-20)).rsqrt()
}

/// Up to 320 points spread over the zero surface of `form` inside `bound`:
/// random points pulled onto the surface along the gradient, kept only when
/// no earlier point is too close (so flat faces get evenly covered).
/// Points filling the inside of a shape, on a grid of up to 10 per side, and
/// the grid cell's size. A shape too thin for the grid (a thin ring) uses its
/// surface points, each standing for a share of a third of its box.
fn sample_interior(form: &Form, bound: Aabb, surface: &[Vec3]) -> (Vec<Vec3>, Vec3) {
    const N: usize = 10;
    let mut regs = Regs::new();
    let size = bound.size();
    let cell = size * (1.0 / N as f32);
    let grid: Vec<Vec3> = (0..N * N * N)
        .map(|i| {
            let (x, y, z) = (i % N, i / N % N, i / (N * N));
            bound.min + Vec3::new((x as f32 + 0.5) * cell.x, (y as f32 + 0.5) * cell.y, (z as f32 + 0.5) * cell.z)
        })
        .collect();
    let mut out = Vec::new();
    for chunk in grid.chunks(8) {
        let mut a = [[0.0f32; 8]; 3];
        for (i, p) in chunk.iter().enumerate() {
            (a[0][i], a[1][i], a[2][i]) = (p.x, p.y, p.z);
        }
        let d = form.dist(V8::new(F8::from_array(a[0]), F8::from_array(a[1]), F8::from_array(a[2])), &mut regs);
        for (i, p) in chunk.iter().enumerate() {
            if d.lane(i) < 0.0 {
                out.push(*p);
            }
        }
    }
    if out.len() < 8 && !surface.is_empty() {
        let v = size.x * size.y * size.z / 3.0 / surface.len() as f32;
        return (surface.to_vec(), Vec3::splat(v.cbrt()));
    }
    (out, cell)
}

fn sample_surface(form: &Form, bound: Aabb) -> Vec<Vec3> {
    let mut regs = Regs::new();
    let size = bound.size();
    let tol = size.max_comp() * 0.004 + 1e-4;
    let mut seed = 0x2545_F491u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed >> 8) as f32 / (1u32 << 24) as f32
    };
    let mut out: Vec<Vec3> = Vec::new();
    // points at least `spacing` apart; where that leaves too few (a long row
    // of columns), closer
    let mut spacing = size.length() * 0.03;
    for _round in 0..4 {
        for _ in 0..240 {
            if out.len() >= 320 {
                break;
            }
            let mut a = [[0.0f32; 8]; 3];
            for i in 0..8 {
                a[0][i] = bound.min.x + rnd() * size.x;
                a[1][i] = bound.min.y + rnd() * size.y;
                a[2][i] = bound.min.z + rnd() * size.z;
            }
            let mut p = V8::new(F8::from_array(a[0]), F8::from_array(a[1]), F8::from_array(a[2]));
            for _ in 0..12 {
                let d = form.dist(p, &mut regs);
                let n = form.normal(p, &mut regs, F8::splat(tol * 0.5));
                p = n.mul_add(-d, p);
            }
            let d = form.dist(p, &mut regs);
            for i in 0..8 {
                let v = p.lane(i);
                let inside = v.x >= bound.min.x - tol
                    && v.y >= bound.min.y - tol
                    && v.z >= bound.min.z - tol
                    && v.x <= bound.max.x + tol
                    && v.y <= bound.max.y + tol
                    && v.z <= bound.max.z + tol;
                if d.lane(i).abs() < tol && inside && v.is_finite() && out.iter().all(|q| (*q - v).length() > spacing) {
                    out.push(v);
                }
            }
        }
        if out.len() >= 320 {
            break;
        }
        spacing *= 0.6;
    }
    out.extend(sample_bottom(form, bound, tol, &mut regs, &out));
    out
}

/// Points on the surfaces a body stands on: rays straight up from below,
/// on a grid of square cells over its footprint (at least two each way), to
/// the first downward-facing surface. Random points can miss a small base
/// (each column of a row), and a base without points sinks and tips over.
fn sample_bottom(form: &Form, bound: Aabb, tol: f32, regs: &mut Regs, have: &[Vec3]) -> Vec<Vec3> {
    let size = bound.size();
    let cell = (size.x * size.z / 96.0).sqrt().max(1e-4);
    let (nx, nz) = (((size.x / cell).round() as usize).clamp(2, 64), ((size.z / cell).round() as usize).clamp(2, 64));
    let starts: Vec<Vec3> = (0..nx * nz)
        .map(|i| {
            let (ix, iz) = (i % nx, i / nx);
            Vec3::new(bound.min.x + (ix as f32 + 0.5) * size.x / nx as f32, bound.min.y - tol, bound.min.z + (iz as f32 + 0.5) * size.z / nz as f32)
        })
        .collect();
    let close = (size.x / nx as f32).min(size.z / nz as f32) * 0.4;
    let mut out: Vec<Vec3> = Vec::new();
    for chunk in starts.chunks(8) {
        let mut a = [[0.0f32; 8]; 3];
        for i in 0..8 {
            let p = chunk[i.min(chunk.len() - 1)];
            (a[0][i], a[1][i], a[2][i]) = (p.x, p.y, p.z);
        }
        let o = V8::new(F8::from_array(a[0]), F8::from_array(a[1]), F8::from_array(a[2]));
        // sphere tracing up through the bound
        let mut t = F8::zero();
        let mut hit = M8::none_set();
        for _ in 0..96 {
            let p = V8::new(o.x, o.y + t, o.z);
            let d = form.dist(p, regs);
            hit |= d.lt(F8::splat(tol));
            t = F8::select(hit, t, t + d.max(F8::splat(tol * 0.5)));
            if hit.all() || t.gt(F8::splat(size.y + tol)).all() {
                break;
            }
        }
        let p = V8::new(o.x, o.y + t, o.z);
        let n = form.normal(p, regs, F8::splat(tol * 0.5));
        for i in 0..chunk.len() {
            let v = p.lane(i);
            let facing_down = n.lane(i).y < -0.5;
            if hit.lane(i) && facing_down && v.y <= bound.max.y && v.is_finite() && have.iter().chain(out.iter()).all(|q| (*q - v).length() > close) {
                out.push(v);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::TerrainCache;

    /// The scene of `objects` (environment `env`), and a world in which
    /// top-level objects `movers` are bodies (each kept off its own compiled
    /// copy, as the viewer's are: the scene stays as compiled).
    fn world(objects: serde_json::Value, env: serde_json::Value, movers: &[usize]) -> (Scene, World) {
        let doc = serde_json::json!({ "environment": env, "objects": objects });
        let scene = Scene::from_value(&doc, &mut TerrainCache::default()).unwrap();
        let mut w = World::default();
        for &m in movers {
            let mut b = body_of(&doc, m).expect("a body");
            b.obj = scene.objects.iter().position(|o| o.doc_index == m).map(|i| i as u32);
            w.bodies.push(b);
        }
        (scene, w)
    }

    /// `secs` of 60 Hz steps; as in the viewer, a body that falls asleep
    /// leaves the world where it is: each body's position then (its node's,
    /// as the viewer writes it; its centre of mass, from sampled points, can
    /// sit a little off it), or at the end, and whether it slept.
    fn run(scene: &Scene, w: &mut World, secs: f32) -> Vec<(Vec3, bool)> {
        let s = Settings::default();
        let dt = 1.0 / 60.0;
        let mut out: Vec<Option<(Vec3, bool)>> = vec![None; w.bodies.len()];
        let mut index: Vec<usize> = (0..w.bodies.len()).collect();
        for f in 0..(secs / dt).round() as usize {
            w.time = f as f32 * dt;
            w.step(scene, &s, dt, &mut |_| None);
            for k in (0..w.bodies.len()).rev() {
                if w.bodies[k].asleep() {
                    out[index[k]] = Some((Vec3::from_array(w.bodies[k].placement().0), true));
                    w.bodies.remove(k);
                    index.remove(k);
                }
            }
        }
        for (k, b) in w.bodies.iter().enumerate() {
            out[index[k]] = Some((Vec3::from_array(b.placement().0), false));
        }
        out.into_iter().map(Option::unwrap).collect()
    }

    /// A box dropped onto the floor comes to rest on it, flat, and sleeps.
    #[test]
    fn a_dropped_box_rests_on_the_floor() {
        let (scene, mut w) = world(
            serde_json::json!([{ "type": "plane" }, { "type": "box", "size": [0.5, 0.5, 0.5], "position": [0, 2, 0] }]),
            serde_json::json!({}),
            &[1],
        );
        let (x, slept) = run(&scene, &mut w, 4.0)[0];
        assert!(slept, "should come to rest");
        assert!((x.y - 0.25).abs() < 0.01, "should rest on the floor: y {}", x.y);
    }

    /// The scene's gravity: on the Moon a box falls slower.
    #[test]
    fn gravity_is_the_scenes() {
        let fall = |g: f32| {
            let (scene, mut w) = world(
                serde_json::json!([{ "type": "plane" }, { "type": "box", "size": [0.5, 0.5, 0.5], "position": [0, 5, 0] }]),
                serde_json::json!({ "gravity": g }),
                &[1],
            );
            run(&scene, &mut w, 0.5)[0].0.y
        };
        let (earth, moon) = (fall(9.81), fall(1.62));
        // (a free fall: 5 - g t^2 / 2)
        assert!((earth - (5.0 - 9.81 * 0.125)).abs() < 0.1, "earth {earth}");
        assert!((moon - (5.0 - 1.62 * 0.125)).abs() < 0.05, "moon {moon}");
    }

    /// In still water a box lighter than water floats, as deep as its density
    /// says (half its density: its centre at the surface, whichever way it
    /// turns, a cube being symmetric); a heavier one sinks to the bottom.
    #[test]
    fn floats_by_density() {
        let settle = |density: f32| {
            let (scene, mut w) = world(
                serde_json::json!([
                    { "type": "water", "waves": 0 },
                    { "type": "plane", "offset": -3 },
                    { "type": "box", "size": [0.5, 0.5, 0.5], "position": [0, 0.6, 0], "material": { "density": density } }
                ]),
                serde_json::json!({}),
                &[2],
            );
            run(&scene, &mut w, 20.0)[0].0.y
        };
        let light = settle(500.0);
        assert!(light.abs() < 0.05, "half under: centre {light}");
        let heavy = settle(2000.0);
        assert!((heavy + 2.75).abs() < 0.03, "should lie on the bottom: centre {heavy}");
    }

    /// Throws and the gravity gun's shots: harder the longer held, never
    /// past the full charge, and heavier bodies slower.
    #[test]
    fn charges_and_masses() {
        for f in [throw_speed, punt_speed] {
            assert!(f(5.0, 0.0) < f(5.0, 0.5) && f(5.0, 0.5) < f(5.0, 1.0));
            assert_eq!(f(5.0, 1.0), f(5.0, 3.0), "past the full charge");
            assert!(f(500.0, 1.0) < f(5.0, 1.0), "heavier, slower");
        }
        assert!(punt_speed(5.0, 0.0) > 0.0, "a tap still nudges");
        assert!((punt_speed(GUN_MASS, 1.0) - GUN_SPEED).abs() < 1e-3);
    }

    /// The gravity gun's shove: through the centre it sends a body straight
    /// off without spin; off centre it spins it too, the same speed ahead.
    #[test]
    fn a_shot_through_the_centre_does_not_spin() {
        let (_, mut w) = world(
            serde_json::json!([{ "type": "box", "size": [1, 1, 1], "position": [0, 5, 0] }]),
            serde_json::json!({}),
            &[0],
        );
        let dir = Vec3::new(0.0, 0.0, -1.0);
        let b = &mut w.bodies[0];
        let centre = b.x;
        b.punt(centre + Vec3::new(0.0, 0.0, 0.5), dir, 10.0);
        assert!((b.v - dir * 10.0).length() < 1e-4 && b.w.length() < 1e-5, "v {:?} w {:?}", b.v, b.w);
        b.v = Vec3::ZERO;
        b.punt(centre + Vec3::new(0.4, 0.0, 0.5), dir, 10.0);
        assert!((b.v - dir * 10.0).length() < 1e-4, "v {:?}", b.v);
        assert!(b.w.y.abs() > 0.1, "off centre it should spin about y: w {:?}", b.w);
    }
}
