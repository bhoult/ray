//! Volume clouds: a body of scattering droplets rather than a surface, as
//! Blender makes them with a Principled Volume on a domain mesh. The domain
//! here is a smooth union of balls (puffs); its density fades in from the
//! puffs' surface over `softness` and is eroded by turbulence noise, so the
//! edges break into wisps and the core stays dense. The renderers march
//! through it (render/volume.rs, and lib.wgsl's twin).

use super::json::BallDef;
use crate::math::{Aabb, Mat3, Vec3};

/// Light-to-extinction scale: `density` 1 takes out this much light per metre
/// at the core.
pub const EXTINCTION: f32 = 0.6;

pub struct Volume {
    pub name: String,
    /// the entry of the file's `objects` this came from
    pub doc_index: usize,
    /// world bounds (puffs, blend and softness, rotated and scaled)
    pub aabb: Aabb,
    /// world to local: q = to_local (p - pos), the rotation's inverse with
    /// the scale and stretch divided out
    pub pos: Vec3,
    pub to_local: Mat3,
    /// world metres per local one: on average (the light's march), and the
    /// least along any axis (safe steps through the puffs)
    pub scale: f32,
    pub min_scale: f32,
    /// local centre and radius of each puff
    pub puffs: Vec<[f32; 4]>,
    pub blend: f32,
    pub softness: f32,
    /// local height of the flat base (cumulus are flat underneath)
    pub base: f32,
    /// extinction per metre at full density
    pub sigma: f32,
    pub turbulence: f32,
    /// the inside carved into wisps by a finer noise, 0..1
    pub wisps: f32,
    pub frequency: f32,
    pub detail: u32,
    pub seed: u32,
    /// the droplets' colour (single-scattering albedo)
    pub color: Vec3,
}

/// A cumulus: a wide flat-bottomed heap, about 13 m across and 7 m tall.
pub fn default_puffs() -> Vec<BallDef> {
    [
        ([0.0, 0.0, 0.0], 3.0),
        ([3.2, -0.6, 0.5], 2.4),
        ([-3.0, -0.7, -0.4], 2.3),
        ([1.2, 1.6, -0.6], 2.2),
        ([-1.0, 1.2, 1.4], 1.9),
        ([5.4, -1.1, -0.2], 1.6),
        ([-5.2, -1.2, 0.6], 1.4),
        ([0.3, 2.7, 0.2], 1.5),
    ]
    .iter()
    .map(|&(center, radius)| BallDef { center, radius })
    .collect()
}

/// The most turbulence a cloud takes (torn, ragged clouds).
pub const MAX_TURBULENCE: f32 = 4.0;

/// The most puffs a cloud can have (the GPU's table is sized for it).
pub const MAX_PUFFS: usize = 24;

#[allow(clippy::too_many_arguments)]
pub fn build(
    name: &str,
    doc_index: usize,
    puffs: &[BallDef],
    blend: f32,
    density: f32,
    turbulence: f32,
    wisps: f32,
    frequency: f32,
    detail: u32,
    softness: f32,
    seed: u32,
    color: [f32; 3],
    position: Option<[f32; 3]>,
    rotation: Option<[f32; 3]>,
    scale: Option<f32>,
    stretch: [f32; 3],
) -> Result<Volume, String> {
    if !stretch.iter().all(|&v| v > 0.0) {
        return Err("stretch must be positive".into());
    }
    if puffs.is_empty() || puffs.len() > MAX_PUFFS {
        return Err(format!("a cloud needs 1 to {MAX_PUFFS} puffs"));
    }
    let s = scale.unwrap_or(1.0);
    if !(s > 0.0) {
        return Err("scale must be positive".into());
    }
    let rot = rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
    let pos = Vec3::from_array(position.unwrap_or([0.0; 3]));
    let (blend, softness) = (blend.max(0.0), softness.max(0.05));
    let mut local = Aabb::new(Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for b in puffs {
        let c = Vec3::from_array(b.center);
        // the blend's bulge, and the billows turbulence can push out
        let r = b.radius.max(0.01) + blend * 0.25 + turbulence.clamp(0.0, MAX_TURBULENCE) * crate::render::volume::BILLOW * 0.5;
        local = local.union(Aabb::new(c - Vec3::splat(r), c + Vec3::splat(r)));
    }
    let base = puffs.iter().map(|b| b.center[1]).fold(f32::INFINITY, f32::min) - 0.3;
    let st = Vec3::from_array(stretch);
    let local = Aabb::new(local.min.hadamard(st), local.max.hadamard(st));
    let inv = rot.transpose();
    let to_local = Mat3 { m: [inv.m[0] * (1.0 / (s * st.x)), inv.m[1] * (1.0 / (s * st.y)), inv.m[2] * (1.0 / (s * st.z))] };
    Ok(Volume {
        name: name.to_string(),
        doc_index,
        aabb: local.transform(rot, s, pos),
        pos,
        to_local,
        scale: s * (st.x * st.y * st.z).cbrt(),
        min_scale: s * st.x.min(st.y).min(st.z),
        puffs: puffs.iter().map(|b| [b.center[0], b.center[1], b.center[2], b.radius.max(0.01)]).collect(),
        blend,
        softness,
        base,
        sigma: density.max(0.0) * EXTINCTION,
        turbulence: turbulence.clamp(0.0, MAX_TURBULENCE),
        wisps: wisps.clamp(0.0, 1.0),
        frequency: frequency.max(0.01),
        detail: detail.clamp(1, 8),
        seed,
        color: Vec3::from_array(color),
    })
}
