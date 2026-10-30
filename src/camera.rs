//! The camera: position, yaw / pitch, vertical field of view, and the
//! per-frame image-plane constants the ray generators use (fraktal256's
//! Camera::setup).

use crate::math::Vec3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Camera {
    pub pos: Vec3,
    /// what f32 cannot hold of the position at `pos`: the camera is at
    /// `pos + fine` (deep in a fractal, or as a micrometre-tall walker, steps
    /// are smaller than f32's half a micrometre six metres out). Rendering
    /// anchors at `pos` and starts the camera's rays `fine` from it.
    pub fine: Vec3,
    /// radians: 0 looks along -z
    pub yaw: f32,
    pub pitch: f32,
    /// vertical field of view, degrees
    pub fov: f32,
    /// forward, right and up
    pub f: Vec3,
    pub r: Vec3,
    pub u: Vec3,
    /// image plane: x = col * kx + bx, y = row * ky + by (pixel centres)
    pub kx: f32,
    pub bx: f32,
    pub ky: f32,
    pub by: f32,
    /// the angle one pixel covers (radians), for level of detail
    pub pixel_angle: f32,
}

impl Camera {
    pub fn new(pos: Vec3, yaw: f32, pitch: f32, fov: f32) -> Camera {
        Camera { pos, yaw, pitch, fov, ..Default::default() }
    }

    /// The basis and image-plane constants for a `w` x `h` image.
    pub fn setup(&mut self, w: u32, h: u32) {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.f = Vec3::new(-sy * cp, sp, -cy * cp);
        self.r = Vec3::new(cy, 0.0, -sy);
        self.u = self.r.cross(self.f);
        let tan_y = (self.fov.to_radians() * 0.5).tan();
        let (w, h) = (w as f32, h as f32);
        let tan_x = w / h * tan_y;
        self.kx = 2.0 * tan_x / w;
        self.bx = self.kx * 0.5 - tan_x;
        self.ky = -2.0 * tan_y / h;
        self.by = self.ky * 0.5 + tan_y;
        self.pixel_angle = 2.0 * tan_y / h;
    }

    /// Unit direction through pixel (col, row).
    pub fn ray(&self, col: f32, row: f32) -> Vec3 {
        let v = row * self.ky + self.by;
        let px = col * self.kx + self.bx;
        (self.r * px + self.u * v + self.f).normalize()
    }

    /// Moves in the camera's frame: x right, y world up, z forward.
    pub fn fly(&mut self, local: Vec3) {
        self.translate(self.fly_dir(local));
    }

    /// Where the camera is, in f64.
    pub fn exact(&self) -> [f64; 3] {
        [0, 1, 2].map(|i| self.pos.to_array()[i] as f64 + self.fine.to_array()[i] as f64)
    }

    /// Puts the camera at `p` (f64): `pos` its f32 rounding, `fine` the rest.
    pub fn set_exact(&mut self, p: [f64; 3]) {
        self.pos = Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32);
        self.fine = Vec3::new((p[0] - self.pos.x as f64) as f32, (p[1] - self.pos.y as f64) as f32, (p[2] - self.pos.z as f64) as f32);
    }

    /// Puts the camera at `p`, exactly.
    pub fn place(&mut self, p: Vec3) {
        self.pos = p;
        self.fine = Vec3::ZERO;
    }

    /// Moves the camera by `v`, keeping what f32 cannot (`fine`): steps far
    /// below its precision add up.
    pub fn translate(&mut self, v: Vec3) {
        let e = self.exact();
        self.set_exact([e[0] + v.x as f64, e[1] + v.y as f64, e[2] + v.z as f64]);
    }

    /// `local` (right, up, forward) in world directions, as `fly` moves.
    pub fn fly_dir(&self, local: Vec3) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let f = Vec3::new(-sy * cp, sp, -cy * cp);
        let r = Vec3::new(cy, 0.0, -sy);
        r * local.x + Vec3::Y * local.y + f * local.z
    }

    pub fn look(&mut self, dyaw: f32, dpitch: f32) {
        self.yaw = (self.yaw + dyaw) % core::f32::consts::TAU;
        self.pitch = (self.pitch + dpitch).clamp(-1.55, 1.55);
    }

    /// The pose (what moving the camera changes), for "did the view change".
    pub fn pose(&self) -> [f32; 9] {
        [self.pos.x, self.pos.y, self.pos.z, self.fine.x, self.fine.y, self.fine.z, self.yaw, self.pitch, self.fov]
    }
}
