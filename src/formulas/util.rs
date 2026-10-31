//! Helpers the kernels share (CPU side; a WGSL kernel carries its own).

use crate::simd::{F8, V8};

/// sqrt(x² + y²).
#[inline(always)]
pub fn len2(x: F8, y: F8) -> F8 {
    y.mul_add(y, x * x).sqrt()
}

/// The signed distance to the box of half sizes `b` about the origin.
#[inline(always)]
pub fn sd_box(p: V8, b: [f32; 3]) -> F8 {
    let q = V8::new(p.x.abs() - b[0], p.y.abs() - b[1], p.z.abs() - b[2]);
    q.max_s(F8::zero()).length() + q.max_comp().min(F8::zero())
}

/// `v` splatted.
#[inline(always)]
pub fn s(v: f32) -> F8 {
    F8::splat(v)
}
