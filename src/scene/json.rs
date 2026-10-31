//! The scene file format: what serde reads from JSON. `compile.rs` turns it
//! into the renderable `Scene`. Every field has a default, so a scene can be
//! as short as `{"objects": [{"type": "sphere"}]}`; see saves/*.json and the
//! README for the full reference.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SceneDef {
    /// one line on what the scene shows (the Open dialog lists it)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub camera: CameraDef,
    #[serde(default)]
    pub environment: EnvDef,
    #[serde(default)]
    pub materials: BTreeMap<String, MaterialDef>,
    #[serde(default)]
    pub lights: Vec<LightDef>,
    #[serde(default)]
    pub objects: Vec<NodeDef>,
    /// render settings this scene starts with (any subset of `Settings`)
    #[serde(default)]
    pub render: Option<serde_json::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct CameraDef {
    /// (f64: a camera deep in a fractal is placed finer than f32 can)
    pub position: [f64; 3],
    /// a point to look at; overrides yaw / pitch
    pub look_at: Option<[f32; 3]>,
    /// degrees: yaw 0 looks along -z, 90 along -x; pitch up is positive
    pub yaw: f32,
    pub pitch: f32,
    /// vertical field of view, degrees
    pub fov: f32,
    /// fly speed, metres per second
    pub speed: f32,
}

impl Default for CameraDef {
    fn default() -> Self {
        CameraDef { position: [0.0, 2.0, 8.0], look_at: None, yaw: 0.0, pitch: 0.0, fov: 60.0, speed: 5.0 }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct EnvDef {
    /// "golden" (default), "dusk", "frozen", "storm": fraktal256's palettes
    pub preset: Option<String>,
    /// towards the sun; or `sun_azimuth` / `sun_elevation` in degrees
    pub sun_direction: Option<[f32; 3]>,
    pub sun_azimuth: Option<f32>,
    pub sun_elevation: Option<f32>,
    pub sun_color: Option<[f32; 3]>,
    pub ambient: Option<[f32; 3]>,
    pub sky_zenith: Option<[f32; 3]>,
    pub sky_horizon: Option<[f32; 3]>,
    pub fog_color: Option<[f32; 3]>,
    /// fog per metre (0 = none)
    pub fog_density: Option<f32>,
    /// cloud cover 0..1 (0 = clear)
    pub clouds: Option<f32>,
    /// multiplies the sun's brightness (default 1)
    pub sun_level: Option<f32>,
    /// multiplies the sky's fill light (default 1)
    pub ambient_level: Option<f32>,
    /// "day" (default), "stars" (a night sky) or "off" (black)
    pub sky: Option<String>,
    /// the hour, 0 to 24: the sun rises at 6 in the east, is highest at 12
    /// (at `sun_azimuth`, default 180: south, toward +z; `sun_elevation`
    /// high, default 60) and sets at 18 in the west; the full moon is
    /// opposite, and lights the night. Sunrise and sunset redden the light
    /// and the sky, and the stars come out after dusk
    pub time_of_day: Option<f32>,
    /// metres per second squared, straight down (default Earth's, 9.81):
    /// what falling objects, emitted copies and game mode's walker feel
    pub gravity: Option<f32>,
}

/// A colour: `[r, g, b]` or one grey value.
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
#[serde(untagged)]
pub enum ColorDef {
    Rgb([f32; 3]),
    Grey(f32),
}

impl ColorDef {
    pub fn rgb(self) -> [f32; 3] {
        match self {
            ColorDef::Rgb(c) => c,
            ColorDef::Grey(g) => [g; 3],
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct MaterialDef {
    pub color: ColorDef,
    /// 0 = mirror, 1 = matte
    pub roughness: f32,
    pub metallic: f32,
    pub emission: ColorDef,
    /// 0 = opaque, 1 = clear dielectric (glass, water)
    pub transmission: f32,
    pub ior: f32,
    /// how strongly transmissive material absorbs the complement of `color`, per metre
    pub absorption: f32,
    /// the procedural pattern: "checker", "grid", "noise", "marble", "wood", "cells",
    /// "cracks", "spots", "stripes", "bricks", "terrain", "water"
    #[serde(alias = "pattern", skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    /// the pattern blends from `color` / `roughness` / `metallic` / `emission`
    /// to these second values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color2: Option<ColorDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roughness2: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metallic2: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emission2: Option<ColorDef>,
    /// pattern frequency (cells per unit)
    #[serde(alias = "scale")]
    pub texture_scale: f32,
    /// noise distortion of the pattern
    pub warp: f32,
    /// contrast of the pattern's blend
    pub sharpness: f32,
    /// surface relief from the pattern
    pub bump: f32,
    /// the pattern the relief follows, and its scale (default: the colour's)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bump_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bump_scale: Option<f32>,
    /// pattern coordinates: "object" (moves with the object) or "world"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space: Option<String>,
    /// kg per cubic metre: an object of it weighs this times its volume
    /// (unless it has a `mass` of its own); under water's 1000 it floats
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<f32>,
    /// metres: the light its emission gives fades to nothing at this
    /// distance (automatic: where it grows too faint to see)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub light_range: Option<f32>,
    /// 0..1: sunlight scattered through it (clouds): lit from behind, it
    /// glows, most at its rims looking toward the sun
    #[serde(skip_serializing_if = "Option::is_none")]
    pub translucency: Option<f32>,
    /// what it is or is for, in words (the library shows it; rendering ignores it)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for MaterialDef {
    fn default() -> Self {
        MaterialDef {
            color: ColorDef::Grey(0.7),
            roughness: 0.6,
            metallic: 0.0,
            emission: ColorDef::Grey(0.0),
            transmission: 0.0,
            ior: 1.5,
            absorption: 1.0,
            texture: None,
            color2: None,
            roughness2: None,
            metallic2: None,
            emission2: None,
            texture_scale: 1.0,
            warp: 0.0,
            sharpness: 1.0,
            bump: 0.0,
            bump_pattern: None,
            bump_scale: None,
            space: None,
            density: None,
            light_range: None,
            translucency: None,
            description: None,
        }
    }
}

/// A material by name (from `materials`) or inline.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum MatRef {
    Name(String),
    Inline(Box<MaterialDef>),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct LightDef {
    pub position: [f32; 3],
    pub color: ColorDef,
    pub intensity: f32,
    /// reach in metres: the light fades to zero there
    pub range: f32,
}

impl Default for LightDef {
    fn default() -> Self {
        LightDef { position: [0.0, 3.0, 0.0], color: ColorDef::Grey(1.0), intensity: 4.0, range: 10.0 }
    }
}

/// One node of a shape tree: a shape plus the fields every node may carry.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NodeDef {
    #[serde(flatten)]
    pub shape: ShapeDef,
    pub position: Option<[f32; 3]>,
    /// Euler angles in degrees, applied x, then y, then z
    pub rotation: Option<[f32; 3]>,
    /// uniform scale
    pub scale: Option<f32>,
    pub material: Option<MatRef>,
    pub name: Option<String>,
    /// top level only: casts shadows (default true; false for water and glass)
    pub shadow: Option<bool>,
    /// top level only: march step multiplier, < 1 for shapes whose distance estimate overshoots
    pub step: Option<f32>,
    /// top level only: bounding radius around `position` (for fractals whose extent is unknown)
    pub bound: Option<f32>,
    /// top level only: sliding friction when dropped (0 = ice, 1 = rubber; default 0.5)
    pub friction: Option<f32>,
    /// top level only: bounciness when dropped (0 = dead, 1 = elastic; default 0.15)
    pub bounce: Option<f32>,
    /// top level only: cannot be dragged (planes, terrain and water always are)
    pub locked: Option<bool>,
    /// top level only: mass in kg (default: from its size)
    pub mass: Option<f32>,
    /// top level only: falls when dropped (default true; lights float)
    pub gravity: Option<bool>,
    /// skip this node
    #[serde(default)]
    pub disabled: bool,
}

/// Radius of the sphere an old point light becomes.
pub const LIGHT_RADIUS: f32 = 0.12;

/// Turns old scenes' point lights into what lights are now: glowing spheres
/// that float (visible, selectable, movable, softer shadows when bigger).
/// The emission matches the old light's strength a couple of metres away.
/// Returns whether `v` changed.
pub fn migrate(v: &mut serde_json::Value) -> bool {
    use serde_json::json;
    let Some(lights) = v.as_object_mut().and_then(|o| o.remove("lights")) else { return false };
    let mut add = Vec::new();
    for (i, l) in lights.as_array().into_iter().flatten().enumerate() {
        let d: LightDef = serde_json::from_value(l.clone()).unwrap_or_default();
        let c = d.color.rgb();
        let peak = c.iter().cloned().fold(1e-6f32, f32::max);
        let k = d.intensity * 2.0 / (LIGHT_RADIUS * LIGHT_RADIUS);
        let round = |x: f32| (x as f64 * 100.0).round() / 100.0;
        add.push(json!({
            "name": format!("light {i}"),
            "type": "sphere",
            "radius": LIGHT_RADIUS,
            "position": d.position.map(round),
            "gravity": false,
            "material": { "color": c.map(|x| round(x / peak)), "emission": c.map(|x| round(x * k)), "light_range": d.range }
        }));
    }
    if let Some(o) = v.as_object_mut() {
        let objects = o.entry("objects").or_insert_with(|| json!([]));
        if let Some(a) = objects.as_array_mut() {
            a.extend(add);
        }
    }
    true
}

fn one() -> f32 {
    1.0
}
fn half() -> f32 {
    0.5
}
fn up() -> [f32; 3] {
    [0.0, 1.0, 0.0]
}
fn ones() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShapeDef {
    // ---- primitives
    Sphere {
        #[serde(default = "one")]
        radius: f32,
    },
    Box {
        /// full size (not half extents)
        #[serde(default = "ones")]
        size: [f32; 3],
        #[serde(default)]
        rounding: f32,
    },
    Plane {
        #[serde(default = "up")]
        normal: [f32; 3],
        #[serde(default)]
        offset: f32,
    },
    Torus {
        #[serde(default = "one")]
        radius: f32,
        #[serde(default = "quarter")]
        thickness: f32,
    },
    Cylinder {
        #[serde(default = "half")]
        radius: f32,
        #[serde(default = "one")]
        height: f32,
    },
    /// a capped cone along y: `radius` at the bottom, `top_radius` at the top
    Cone {
        #[serde(default = "half")]
        radius: f32,
        #[serde(default)]
        top_radius: f32,
        #[serde(default = "one")]
        height: f32,
    },
    Capsule {
        #[serde(default = "cap_a")]
        a: [f32; 3],
        #[serde(default = "cap_b")]
        b: [f32; 3],
        #[serde(default = "quarter")]
        radius: f32,
    },
    Ellipsoid {
        #[serde(default = "ell")]
        radii: [f32; 3],
    },
    Octahedron {
        #[serde(default = "one")]
        size: f32,
    },
    /// the gyroid minimal surface as a sheet; infinite, so intersect it with something
    Gyroid {
        /// cells per unit
        #[serde(default = "one")]
        frequency: f32,
        #[serde(default = "tenth")]
        thickness: f32,
    },
    // ---- fractals (unit size; use `scale`)
    Menger {
        #[serde(default = "four", deserialize_with = "whole")]
        iterations: u32,
    },
    Mandelbulb {
        #[serde(default = "eight")]
        power: f32,
        #[serde(default = "eight_u", deserialize_with = "whole")]
        iterations: u32,
    },
    Mandelbox {
        #[serde(default = "two")]
        fold_scale: f32,
        #[serde(default = "twelve", deserialize_with = "whole")]
        iterations: u32,
        #[serde(default = "half")]
        min_radius: f32,
    },
    Julia {
        #[serde(default = "julia_c")]
        c: [f32; 4],
        #[serde(default = "ten", deserialize_with = "whole")]
        iterations: u32,
    },
    Sierpinski {
        #[serde(default = "eight_u", deserialize_with = "whole")]
        iterations: u32,
    },
    /// packed spheres by inversion (iq's Apollonian): fills space, so
    /// intersect it with something
    Apollonian {
        /// the inversion's strength: bigger bubbles above 1
        #[serde(default = "apollonian_fold")]
        fold: f32,
        #[serde(default = "eight_u", deserialize_with = "whole")]
        iterations: u32,
    },
    /// a kaleidoscopic IFS: the Menger sponge's folds, scale and offset, with
    /// a rotation between steps (Knighty's)
    Kifs {
        #[serde(default = "three_f")]
        fold_scale: f32,
        #[serde(default = "ones")]
        offset: [f32; 3],
        /// Euler degrees turned at every step (0: the Menger sponge)
        #[serde(default)]
        angles: [f32; 3],
        #[serde(default = "six", deserialize_with = "whole")]
        iterations: u32,
    },
    /// the pseudo-Kleinian (Knighty's): box folds and inversions; fills
    /// space, so intersect it with something
    Kleinian {
        /// the box fold's half sizes
        #[serde(default = "kleinian_box")]
        fold_box: [f32; 3],
        /// the inversion's radius squared
        #[serde(default = "one")]
        inversion: f32,
        #[serde(default = "eight_u", deserialize_with = "whole")]
        iterations: u32,
    },
    // ---- blobs
    Metaballs {
        balls: Vec<BallDef>,
        #[serde(default = "half")]
        blend: f32,
    },
    // ---- combinators (children are combined; `smooth` > 0 blends them)
    Union {
        children: Vec<NodeDef>,
        #[serde(default)]
        smooth: f32,
    },
    Intersection {
        children: Vec<NodeDef>,
        #[serde(default)]
        smooth: f32,
    },
    /// the first child minus the others
    Difference {
        children: Vec<NodeDef>,
        #[serde(default)]
        smooth: f32,
    },
    /// whole objects kept together: each child is an object of its own (any
    /// shape, its own material), moved, turned and scaled with the group
    Group {
        children: Vec<NodeDef>,
    },
    // ---- modifiers of one child
    /// infinite (count absent) or limited (+-count cells per axis) repetition; period 0 = not repeated
    Repeat {
        period: [f32; 3],
        count: Option<[u32; 3]>,
        child: Box<NodeDef>,
    },
    /// mirror across the planes of the named axes, e.g. "x" or "xz"
    Mirror {
        axes: String,
        child: Box<NodeDef>,
    },
    /// twist about y: radians per unit of height
    Twist {
        amount: f32,
        child: Box<NodeDef>,
    },
    /// bend in the xy plane: radians per unit along x
    Bend {
        amount: f32,
        child: Box<NodeDef>,
    },
    Round {
        radius: f32,
        child: Box<NodeDef>,
    },
    /// hollow: a shell of this thickness around the surface
    Shell {
        thickness: f32,
        child: Box<NodeDef>,
    },
    /// Perlin fBm added to the surface
    Displace {
        #[serde(default = "tenth")]
        amplitude: f32,
        #[serde(default = "one")]
        frequency: f32,
        #[serde(default = "three", deserialize_with = "whole")]
        octaves: u32,
        child: Box<NodeDef>,
    },
    /// its child cut by a plane: what lies beyond `offset` along `normal` is
    /// gone (with `thickness`, only a slab that thick is kept)
    Slice {
        #[serde(default = "toward_z")]
        normal: [f32; 3],
        #[serde(default)]
        offset: f32,
        #[serde(default)]
        thickness: f32,
        child: Box<NodeDef>,
    },
    /// its child's surface pushed out by a pattern (in, with a negative depth):
    /// ripples, waves, bricks, hexagons... (render/pattern.rs)
    Relief {
        #[serde(default = "ripples")]
        pattern: String,
        /// pattern cells per metre
        #[serde(default = "four_per_metre")]
        frequency: f32,
        #[serde(default = "relief_depth")]
        depth: f32,
        #[serde(default)]
        warp: f32,
        #[serde(default = "one")]
        sharpness: f32,
        child: Box<NodeDef>,
    },
    // ---- special top-level objects
    Terrain(TerrainDef),
    /// an infinite water plane at `level` with waves, refraction and caustics
    Water {
        #[serde(default)]
        level: f32,
        /// the waves' size and speed, as multiples (0 = flat, still water)
        #[serde(default = "one")]
        waves: f32,
        #[serde(default = "one")]
        wave_speed: f32,
    },
    /// a NURBS surface (top level only), traced exactly: `points[i][j]` is
    /// [x, y, z] or [x, y, z, weight], i along u
    Nurbs {
        #[serde(default = "bicubic", deserialize_with = "whole2")]
        degree: [u32; 2],
        #[serde(default = "sheet")]
        points: Vec<Vec<Vec<f32>>>,
        /// clamped knot vectors (default: even)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        knots_u: Option<Vec<f32>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        knots_v: Option<Vec<f32>>,
        /// closed, with an inside
        #[serde(default)]
        solid: bool,
    },
    /// a NURBS curve `profile` ([radius, height] or [radius, height, weight])
    /// turned `angle` degrees about the y axis (top level only)
    Revolve {
        #[serde(default = "vase")]
        profile: Vec<Vec<f32>>,
        #[serde(default = "three", deserialize_with = "whole")]
        degree: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        knots: Option<Vec<f32>>,
        #[serde(default = "full_turn")]
        angle: f32,
        /// default: closed when the profile ends on the axis at both ends or loops
        #[serde(default, skip_serializing_if = "Option::is_none")]
        solid: Option<bool>,
    },
    /// sends out copies of `emit` (top level only): `rate` a second, growing
    /// in over `grow` seconds, living `lifespan` (0: for ever) and shrinking
    /// away over the last `fade`; launched at `speed` along its own +y,
    /// within `spread` degrees; with `physics` they collide and pile up. The
    /// copies are not part of the file. `visible`: show the emitter itself.
    Emitter {
        #[serde(default = "emit_rate")]
        rate: f32,
        #[serde(default = "emit_life")]
        lifespan: f32,
        #[serde(default = "emit_grow")]
        grow: f32,
        #[serde(default = "emit_fade")]
        fade: f32,
        #[serde(default = "emit_speed")]
        speed: f32,
        #[serde(default = "emit_spread")]
        spread: f32,
        #[serde(default = "yes")]
        physics: bool,
        #[serde(default = "emit_max", deserialize_with = "whole")]
        max: u32,
        #[serde(default = "yes")]
        visible: bool,
        #[serde(default, deserialize_with = "whole")]
        seed: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        emit: Option<Box<NodeDef>>,
    },
    /// a volume cloud (top level only): scattering droplets filling a smooth
    /// union of `puffs`, eroded at the edges by turbulence noise
    Cloud {
        /// the domain's balls, in the cloud's own metres (default: a cumulus)
        #[serde(default, skip_serializing_if = "Option::is_none")]
        puffs: Option<Vec<BallDef>>,
        #[serde(default = "cloud_blend")]
        blend: f32,
        #[serde(default = "cloud_density")]
        density: f32,
        #[serde(default = "cloud_turbulence")]
        turbulence: f32,
        /// 0..1: the inside carved into wisps by a finer noise
        #[serde(default)]
        wisps: f32,
        #[serde(default = "cloud_frequency")]
        frequency: f32,
        #[serde(default = "cloud_detail", deserialize_with = "whole")]
        detail: u32,
        #[serde(default = "cloud_softness")]
        softness: f32,
        #[serde(default, deserialize_with = "whole")]
        seed: u32,
        #[serde(default = "white")]
        color: [f32; 3],
        /// how much longer it is along its own x, y and z (on top of `scale`)
        #[serde(default = "ones")]
        stretch: [f32; 3],
    },
}

/// A whole number, also when written with a fraction (4.0 or 3.6 gives 4):
/// sliders and other tools write numbers as floats.
fn whole<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let x = f64::deserialize(d)?;
    if x.is_finite() && x >= 0.0 && x <= u32::MAX as f64 {
        Ok(x.round() as u32)
    } else {
        Err(serde::de::Error::custom(format!("expected a whole number of 0 or more, not {x}")))
    }
}

fn whole2<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u32; 2], D::Error> {
    let [a, b] = <[f64; 2]>::deserialize(d)?;
    let w = |x: f64| {
        if x.is_finite() && x >= 0.0 && x <= u32::MAX as f64 {
            Ok(x.round() as u32)
        } else {
            Err(serde::de::Error::custom(format!("expected a whole number of 0 or more, not {x}")))
        }
    };
    Ok([w(a)?, w(b)?])
}

fn yes() -> bool {
    true
}
fn emit_rate() -> f32 {
    2.0
}
fn emit_life() -> f32 {
    8.0
}
fn emit_grow() -> f32 {
    0.3
}
fn emit_fade() -> f32 {
    0.6
}
fn emit_speed() -> f32 {
    2.5
}
fn emit_spread() -> f32 {
    20.0
}
fn emit_max() -> u32 {
    40
}
fn toward_z() -> [f32; 3] {
    [0.0, 0.0, 1.0]
}
fn ripples() -> String {
    "ripples".into()
}
fn four_per_metre() -> f32 {
    4.0
}
fn relief_depth() -> f32 {
    0.03
}
fn bicubic() -> [u32; 2] {
    [3, 3]
}
fn full_turn() -> f32 {
    360.0
}
/// A 2 m square sheet with a bump and a dip (5 x 5 bicubic).
pub fn sheet() -> Vec<Vec<Vec<f32>>> {
    let h = [[0.0, 0.1, 0.0, -0.1, 0.0], [0.1, 0.6, 0.3, -0.2, 0.0], [0.0, 0.4, 0.2, -0.5, -0.1], [-0.1, 0.0, -0.3, -0.6, 0.0], [0.0, 0.0, 0.0, 0.0, 0.0]];
    (0..5).map(|i| (0..5).map(|j| vec![i as f32 * 0.5 - 1.0, h[i][j], j as f32 * 0.5 - 1.0]).collect()).collect()
}
/// A vase with walls 3 cm thick, 1.15 m tall (a closed profile: solid).
pub fn vase() -> Vec<Vec<f32>> {
    [
        [0.0, 0.0], [0.3, 0.0], [0.42, 0.06], [0.5, 0.4], [0.34, 0.78], [0.22, 0.98], [0.27, 1.15], [0.24, 1.15],
        [0.19, 0.98], [0.31, 0.78], [0.47, 0.4], [0.39, 0.09], [0.27, 0.03], [0.0, 0.03],
    ]
    .iter()
    .map(|p| p.to_vec())
    .collect()
}

fn cloud_blend() -> f32 {
    1.2
}
fn cloud_turbulence() -> f32 {
    0.75
}
fn cloud_frequency() -> f32 {
    0.35
}
fn cloud_detail() -> u32 {
    5
}
fn cloud_softness() -> f32 {
    0.25
}
fn cloud_density() -> f32 {
    1.5
}
fn white() -> [f32; 3] {
    [1.0; 3]
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct BallDef {
    pub center: [f32; 3],
    #[serde(default = "half")]
    pub radius: f32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(default)]
pub struct TerrainDef {
    /// metres per side; the map is centred on `position`
    pub size: f32,
    /// samples per side
    #[serde(deserialize_with = "whole")]
    pub resolution: u32,
    /// "eroded" (fraktal's derivative-damped value noise), "perlin", "ridged", "billow"
    pub noise: String,
    pub height: f32,
    /// added to every height
    pub base: f32,
    /// first octave, cycles per metre
    pub frequency: f32,
    #[serde(deserialize_with = "whole")]
    pub octaves: u32,
    #[serde(deserialize_with = "whole")]
    pub seed: u32,
    /// raise a bowl rim outside this radius (metres; 0 = none)
    pub rim: f32,
    /// with a rim: a ring of mountains this high (metres) instead of the
    /// bowl, rising from `rim` and falling away again over `rim_width`, its
    /// crest halfway (0: the bowl, steepening for ever)
    pub rim_height: f32,
    pub rim_width: f32,
    /// an island: the ground sinks under the sea toward this radius
    /// (metres; 0 = none), the coast following the noise's contours
    pub island: f32,
    /// no height above this
    pub cap: Option<f32>,
    /// march relaxation: lower for steep terrain
    pub relax: f32,
    pub rock: [f32; 3],
    pub grass: [f32; 3],
    pub snow: [f32; 3],
    pub sand: [f32; 3],
    /// snow above this height (default: 30% of the height range above the shore)
    pub snow_line: Option<f32>,
    /// sand below this height (defaults to the water level, if any)
    pub sand_line: Option<f32>,
}

impl Default for TerrainDef {
    fn default() -> Self {
        TerrainDef {
            size: 512.0,
            resolution: 1024,
            noise: "eroded".into(),
            height: 70.0,
            base: 18.0,
            frequency: 0.0075,
            octaves: 8,
            seed: 1,
            rim: 0.0,
            rim_height: 0.0,
            rim_width: 200.0,
            island: 0.0,
            cap: None,
            relax: 0.65,
            rock: [0.30, 0.26, 0.22],
            grass: [0.16, 0.24, 0.07],
            snow: [0.92, 0.94, 1.00],
            sand: [0.50, 0.43, 0.31],
            snow_line: None,
            sand_line: None,
        }
    }
}

fn quarter() -> f32 {
    0.25
}
fn tenth() -> f32 {
    0.1
}
fn two() -> f32 {
    2.0
}
fn eight() -> f32 {
    8.0
}
fn three() -> u32 {
    3
}
fn four() -> u32 {
    4
}
fn eight_u() -> u32 {
    8
}
fn ten() -> u32 {
    10
}
fn twelve() -> u32 {
    12
}
fn cap_a() -> [f32; 3] {
    [0.0, -0.5, 0.0]
}
fn cap_b() -> [f32; 3] {
    [0.0, 0.5, 0.0]
}
fn ell() -> [f32; 3] {
    [1.0, 0.5, 0.75]
}
fn apollonian_fold() -> f32 {
    1.1
}
fn three_f() -> f32 {
    3.0
}
fn six() -> u32 {
    6
}
fn kleinian_box() -> [f32; 3] {
    [0.924_36, 0.907_56, 0.924_36]
}
fn julia_c() -> [f32; 4] {
    [-0.2, 0.6, 0.2, 0.2]
}
