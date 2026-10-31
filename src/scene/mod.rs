//! The renderable scene, compiled from JSON (json.rs): the environment (sun,
//! sky, fog), materials, point lights and top-level objects with a BVH over
//! their bounds. Objects never blend with each other; shapes that should
//! blend belong in one object's tree (union with `smooth`).
//!
//! A top-level object is one of:
//! - an analytic sphere, box or plane (exact intersection), when the node is
//!   just that primitive with a transform;
//! - a signed-distance program (sdf.rs), sphere-traced inside its bounds;
//! - a heightfield (terrain.rs), marched like fraktal256's terrain;
//! - a NURBS surface (nurbs.rs), hit exactly by Newton's method on its patches.

pub mod json;
pub mod nurbs;
pub mod sdf;
pub mod terrain;
pub mod volume;

use crate::math::{Aabb, Mat3, Vec3};
use json::{MatRef, MaterialDef, NodeDef, SceneDef, ShapeDef};
use sdf::{Compiler, MaterialSource, Program};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use terrain::Terrain;

/// Procedural patterns (`Material::tex`).
pub mod tex {
    pub const NONE: i32 = 0;
    pub const CHECKER: i32 = 1;
    pub const GRID: i32 = 2;
    pub const NOISE: i32 = 3;
    pub const TERRAIN: i32 = 4;
    pub const WATER: i32 = 5;
    pub const MARBLE: i32 = 6;
    pub const WOOD: i32 = 7;
    pub const CELLS: i32 = 8;
    pub const CRACKS: i32 = 9;
    pub const SPOTS: i32 = 10;
    pub const STRIPES: i32 = 11;
    pub const BRICKS: i32 = 12;
    pub const HEXAGONS: i32 = 13;
    pub const WAVES: i32 = 14;
    pub const RIPPLES: i32 = 15;
    pub const SCALES: i32 = 16;
    pub const WEAVE: i32 = 17;
    pub const ORBIT: i32 = 18;
    pub const WINDOWS: i32 = 19;

    /// Every pattern: name in the file, kind, what it looks like.
    pub const PATTERNS: [(&str, i32, &str); 20] = [
        ("none", NONE, "Plain: the first colour, roughness, metal and glow everywhere."),
        ("checker", CHECKER, "3D checkerboard of the two colours; one cell per unit at scale 1."),
        ("grid", GRID, "Thin lines of the second colour on a unit grid (tiles, graph paper)."),
        ("noise", NOISE, "Smooth fBm blotches blending the two sets of values: rust, moss, camouflage. Raise sharpness for hard-edged patches."),
        ("marble", MARBLE, "Veins of the second colour through the first, bent by warp; sharpness thins the veins."),
        ("wood", WOOD, "Growth rings around the object's y axis, distorted by warp; the second colour is the late wood."),
        ("cells", CELLS, "Voronoi stones or tiles: the second colour fills the gaps between cells (cobbles, scales, cracked mud)."),
        ("cracks", CRACKS, "Thin bright cracks along Voronoi cell edges; give the second set an emission for lava."),
        ("spots", SPOTS, "Random round spots of the second colour (polka dots, lichen, leopard)."),
        ("stripes", STRIPES, "Bands along the y axis; warp makes them wavy, sharpness makes them crisp."),
        ("bricks", BRICKS, "Running-bond bricks projected along the surface's main axis; the second colour is the mortar."),
        ("hexagons", HEXAGONS, "Hexagonal tiles (honeycomb, floor tiles) laid along the surface's main axis; the second colour is the grout. Each tile's shade varies a little."),
        ("waves", WAVES, "Soft wavy bands, like wind on sand or water: a smooth sine across the y axis, bent by gentle waves (and by warp). Good as a relief."),
        ("ripples", RIPPLES, "Rings spreading from the object's y axis, like a stone dropped in water: a smooth sine; warp makes them wobble. Good as a relief."),
        ("scales", SCALES, "Overlapping rounded scales in staggered rows (fish, dragon, roof shingles); the second colour darkens toward each scale's rim."),
        ("weave", WEAVE, "A basket weave of over-and-under strands (wicker, cloth, carbon fibre); the second colour shows in the gaps between strands."),
        ("windows", WINDOWS, "Building facades: a window in each cell (one unit wide, 1.4 tall: a storey), some lit and some dark, more lit on some floors than others; roofs stay plain. The first set is the wall; the second is a lit window (give it a glow), between half and full strength. Use the world space so neighbouring buildings differ."),
        ("orbit", ORBIT, "Fractals: colours each point by its orbit trap (how near its iteration came to the centre, or which fold or hole level shaped it), cycling from the first colour to the second and back as the scale grows. Other shapes stay the first colour."),
        ("terrain", TERRAIN, "fraktal256's landscape material: rock strata, grass on flats, snow up high, sand at the shore (terrain objects)."),
        ("water", WATER, "Animated-looking wave normals for the water plane."),
    ];

    pub fn by_name(n: &str) -> Option<i32> {
        PATTERNS.iter().find(|p| p.0 == n).map(|p| p.1)
    }
}

#[derive(Clone, Debug)]
pub struct Material {
    pub name: String,
    pub color: Vec3,
    pub emission: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    /// what the pattern blends to
    pub color2: Vec3,
    pub emission2: Vec3,
    pub roughness2: f32,
    pub metallic2: f32,
    pub transmission: f32,
    pub ior: f32,
    /// absorption inside transmissive material, log2 units per metre per channel
    pub absorb: Vec3,
    pub tex: i32,
    pub tex_scale: f32,
    pub warp: f32,
    pub sharpness: f32,
    pub bump: f32,
    /// the pattern and scale the bump follows (by default the colour's)
    pub bump_tex: i32,
    pub bump_scale: f32,
    /// pattern in world coordinates instead of the object's
    pub world: bool,
    /// metres the light of its emission reaches (0: automatic)
    pub light_range: f32,
    /// sunlight scattered through it, 0..1 (clouds)
    pub translucency: f32,
}

impl Material {
    fn from_def(name: &str, d: &MaterialDef) -> Result<Material, String> {
        let color = Vec3::from_array(d.color.rgb());
        let tex = match d.texture.as_deref() {
            None => tex::NONE,
            Some(t) => tex::by_name(t).ok_or_else(|| {
                let names: Vec<&str> = tex::PATTERNS.iter().map(|p| p.0).collect();
                format!("material '{name}': unknown pattern '{t}' ({})", names.join(", "))
            })?,
        };
        let absorb = (Vec3::ONE - color.min(Vec3::ONE).max(Vec3::ZERO)) * d.absorption.max(0.0);
        let emission = Vec3::from_array(d.emission.rgb());
        let roughness = d.roughness.clamp(0.0, 1.0);
        let metallic = d.metallic.clamp(0.0, 1.0);
        Ok(Material {
            name: name.to_string(),
            color,
            emission,
            roughness,
            metallic,
            color2: d.color2.map(|c| Vec3::from_array(c.rgb())).unwrap_or(color * 0.25),
            emission2: d.emission2.map(|c| Vec3::from_array(c.rgb())).unwrap_or(emission),
            roughness2: d.roughness2.unwrap_or(roughness).clamp(0.0, 1.0),
            metallic2: d.metallic2.unwrap_or(metallic).clamp(0.0, 1.0),
            transmission: d.transmission.clamp(0.0, 1.0),
            ior: d.ior.clamp(1.0, 3.0),
            absorb,
            tex,
            tex_scale: if d.texture_scale.abs() > 1e-6 { d.texture_scale } else { 1.0 },
            bump_tex: match d.bump_pattern.as_deref() {
                None => tex,
                Some(t) => tex::by_name(t).ok_or_else(|| format!("material '{name}': unknown bump pattern '{t}'"))?,
            },
            bump_scale: match d.bump_scale.filter(|v| v.abs() > 1e-6) {
                Some(v) => v,
                None if d.texture_scale.abs() > 1e-6 => d.texture_scale,
                None => 1.0,
            },
            warp: d.warp,
            sharpness: d.sharpness.max(0.01),
            bump: d.bump,
            world: d.space.as_deref() == Some("world"),
            light_range: d.light_range.unwrap_or(0.0).max(0.0),
            translucency: d.translucency.unwrap_or(0.0).clamp(0.0, 1.0),
        })
    }

    fn water(color: Vec3) -> Material {
        Material {
            name: "water".into(),
            color: Vec3::ONE,
            emission: Vec3::ZERO,
            roughness: 0.02,
            metallic: 0.0,
            color2: color,
            emission2: Vec3::ZERO,
            roughness2: 0.02,
            metallic2: 0.0,
            transmission: 1.0,
            ior: 1.33,
            // fraktal's per-channel absorption (exp2 exponents per metre)
            absorb: Vec3::new(1.0, 0.40, 0.28),
            tex: tex::WATER,
            tex_scale: 1.0,
            bump_tex: tex::NONE,
            bump_scale: 1.0,
            warp: 0.0,
            sharpness: 1.0,
            bump: 0.0,
            world: true,
            light_range: 0.0,
            translucency: 0.0,
        }
    }
}

/// Material parameters as separate arrays, for 8-lane gathers.
#[derive(Default)]
pub struct MaterialSoa {
    pub color: [Vec<f32>; 3],
    pub color2: [Vec<f32>; 3],
    pub emission: [Vec<f32>; 3],
    pub emission2: [Vec<f32>; 3],
    pub absorb: [Vec<f32>; 3],
    pub roughness: Vec<f32>,
    pub roughness2: Vec<f32>,
    pub metallic: Vec<f32>,
    pub metallic2: Vec<f32>,
    pub transmission: Vec<f32>,
    pub ior: Vec<f32>,
    pub tex: Vec<i32>,
    pub tex_scale: Vec<f32>,
    pub warp: Vec<f32>,
    pub sharpness: Vec<f32>,
    pub bump: Vec<f32>,
    pub bump_tex: Vec<i32>,
    pub bump_scale: Vec<f32>,
    pub world: Vec<i32>,
    pub translucency: Vec<f32>,
}

impl MaterialSoa {
    fn build(ms: &[Material]) -> MaterialSoa {
        let mut s = MaterialSoa::default();
        for m in ms {
            for c in 0..3 {
                s.color[c].push(m.color.get(c));
                s.color2[c].push(m.color2.get(c));
                s.emission[c].push(m.emission.get(c));
                s.emission2[c].push(m.emission2.get(c));
                s.absorb[c].push(m.absorb.get(c));
            }
            s.roughness.push(m.roughness);
            s.roughness2.push(m.roughness2);
            s.metallic.push(m.metallic);
            s.metallic2.push(m.metallic2);
            s.transmission.push(m.transmission);
            s.ior.push(m.ior);
            s.tex.push(m.tex);
            s.tex_scale.push(m.tex_scale);
            s.warp.push(m.warp);
            s.sharpness.push(m.sharpness);
            s.bump.push(m.bump);
            s.bump_tex.push(m.bump_tex);
            s.bump_scale.push(m.bump_scale);
            s.world.push(m.world as i32);
            s.translucency.push(m.translucency);
        }
        s
    }
}

/// What the sky shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sky {
    Day,
    Stars,
    Off,
}

/// Sun, sky and fog (fraktal256's palette).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Env {
    /// unit vector toward the sun
    pub sun_dir: Vec3,
    pub sun_col: Vec3,
    pub amb_col: Vec3,
    pub sky_zen: Vec3,
    pub sky_hor: Vec3,
    pub fog_col: Vec3,
    /// fog per metre (exp2(-fog * distance) is the transmittance)
    pub fog: f32,
    /// cloud cover 0..1
    pub clouds: f32,
    /// multiplier of the sky's fill light
    pub ambient: f32,
    pub sky: Sky,
    /// how much light the water scatters back (its deep colour): the brighter
    /// of the sun and the sky's fill light
    pub daylight: f32,
    /// where the sky draws the sun's disk and glow, and their colour: the
    /// light's own, except at dusk, when the light is the moon and the glow
    /// lingers where the sun went down
    pub glow_dir: Vec3,
    pub glow_col: Vec3,
    /// the moon (toward it), and how bright its disk is (0: none)
    pub moon_dir: Vec3,
    pub moon: f32,
    /// how much of the starry sky shows over the day sky (time of day)
    pub night: f32,
}

struct Preset {
    name: &'static str,
    sun_dir: [f32; 3],
    sun_col: [f32; 3],
    amb_col: [f32; 3],
    sky_zen: [f32; 3],
    sky_hor: [f32; 3],
    fog_col: [f32; 3],
    water_col: [f32; 3],
    fog: f32,
    clouds: f32,
}

/// fraktal256's four palettes (its cloud thresholds converted to cover).
const PRESETS: [Preset; 4] = [
    Preset {
        name: "golden",
        sun_dir: [-0.55, 0.42, -0.72],
        sun_col: [6.0, 4.9, 3.6],
        amb_col: [0.30, 0.42, 0.62],
        sky_zen: [0.20, 0.38, 0.85],
        sky_hor: [0.72, 0.78, 0.86],
        fog_col: [0.62, 0.68, 0.76],
        water_col: [0.02, 0.07, 0.09],
        fog: 0.0036,
        clouds: 0.42,
    },
    Preset {
        name: "dusk",
        sun_dir: [0.70, 0.13, -0.60],
        sun_col: [7.0, 3.2, 1.6],
        amb_col: [0.30, 0.20, 0.36],
        sky_zen: [0.10, 0.08, 0.30],
        sky_hor: [0.85, 0.42, 0.40],
        fog_col: [0.70, 0.36, 0.40],
        water_col: [0.03, 0.03, 0.08],
        fog: 0.0045,
        clouds: 0.45,
    },
    Preset {
        name: "frozen",
        sun_dir: [0.35, 0.30, 0.88],
        sun_col: [4.6, 4.8, 5.2],
        amb_col: [0.30, 0.38, 0.52],
        sky_zen: [0.30, 0.45, 0.80],
        sky_hor: [0.76, 0.82, 0.92],
        fog_col: [0.70, 0.76, 0.86],
        water_col: [0.02, 0.06, 0.10],
        fog: 0.0040,
        clouds: 0.38,
    },
    Preset {
        name: "storm",
        sun_dir: [-0.20, 0.30, 0.93],
        sun_col: [3.6, 4.4, 3.4],
        amb_col: [0.18, 0.28, 0.26],
        sky_zen: [0.10, 0.22, 0.24],
        sky_hor: [0.46, 0.58, 0.56],
        fog_col: [0.40, 0.52, 0.52],
        water_col: [0.01, 0.06, 0.05],
        fog: 0.0058,
        clouds: 0.32,
    },
];

/// A glowing part of an object, lighting the scene as a sphere light.
#[derive(Clone, Copy, Debug)]
pub struct Emitter {
    pub pos: Vec3,
    pub radius: f32,
    /// average emitted radiance
    pub color: Vec3,
    /// beyond this distance its light is negligible
    pub range: f32,
    /// the object it belongs to (its shadow rays ignore it)
    pub obj: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Light {
    pub pos: Vec3,
    /// colour times intensity
    pub color: Vec3,
    pub range: f32,
}

pub enum Shape {
    Sphere { c: Vec3, r: f32 },
    /// `rot` takes world directions into the box's frame
    Box { c: Vec3, rot: Mat3, half: Vec3 },
    /// n.p = d
    Plane { n: Vec3, d: f32 },
    Sdf(Program),
    Terrain(Arc<Terrain>),
    /// a surface in its own frame: q = rot (p - c) / s
    Nurbs { surf: Arc<nurbs::Nurbs>, c: Vec3, rot: Mat3, s: f32 },
}

pub struct Object {
    pub name: String,
    pub shape: Shape,
    pub aabb: Aabb,
    /// the material of analytic shapes and terrain (SDF leaves carry their own)
    pub material: u16,
    pub shadow: bool,
    /// march step factor (1 for true distances)
    pub step: f32,
    /// the entry of the file's `objects` this came from
    pub doc_index: usize,
    /// world to object space (patterns): q = local_m (p - local_t)
    pub local_m: Mat3,
    pub local_t: Vec3,
    /// estimated cycles of one 8-lane evaluation of each node's subtree (by
    /// node id; node 0 is the whole object)
    pub node_cycles: Vec<f32>,
    pub friction: f32,
    pub bounce: f32,
    /// cannot be dragged: planes, terrain, water, unbounded shapes, or `locked`
    pub locked: bool,
    /// nothing collides with it (an emitter's marker)
    pub ghost: bool,
    /// the member of a group it is: child indices down from `doc_index`'s
    /// entry (empty for an object of its own)
    pub path: Vec<usize>,
    /// distance programs: where the object stands. The program works in
    /// metres from here (points are given less this), so that close up its
    /// precision is a step of a position this near, not of one far from the
    /// world's origin; zero for the other shapes
    pub org: Vec3,
}

impl Object {
    /// The child-index path of node `id` in this object's tree.
    pub fn node_path(&self, id: usize) -> Vec<usize> {
        match &self.shape {
            Shape::Sdf(p) => p.paths.get(id).cloned().unwrap_or_default(),
            _ => Vec::new(),
        }
    }

    /// The node id at a child-index path.
    pub fn node_id(&self, path: &[usize]) -> Option<usize> {
        match &self.shape {
            Shape::Sdf(p) => p.paths.iter().position(|q| q == path),
            _ => path.is_empty().then_some(0),
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self.shape {
            Shape::Sphere { .. } => "sphere",
            Shape::Box { .. } => "box",
            Shape::Plane { .. } => "plane",
            Shape::Sdf(_) => "sdf",
            Shape::Terrain(_) => "terrain",
            Shape::Nurbs { .. } => "nurbs",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BvhNode {
    pub min: Vec3,
    pub max: Vec3,
    /// leaf: first item; inner: left child (right = left + 1)
    pub index: u32,
    /// items in a leaf, 0 for an inner node
    pub count: u32,
}

#[derive(Default)]
pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    pub items: Vec<u32>,
}

impl Bvh {
    /// Median split on the longest axis of the centroids, two items per leaf.
    fn build(boxes: &[Aabb], ids: Vec<u32>) -> Bvh {
        let mut b = Bvh { nodes: Vec::new(), items: ids };
        if b.items.is_empty() {
            return b;
        }
        b.nodes.push(BvhNode { min: Vec3::ZERO, max: Vec3::ZERO, index: 0, count: 0 });
        let n = b.items.len();
        b.split(boxes, 0, 0, n);
        b
    }

    fn split(&mut self, boxes: &[Aabb], node: usize, lo: usize, hi: usize) {
        let mut bound = Aabb::EMPTY;
        let mut cent = Aabb::EMPTY;
        for &i in &self.items[lo..hi] {
            let a = boxes[i as usize];
            bound = bound.union(a);
            cent = cent.union(Aabb::new(a.center(), a.center()));
        }
        self.nodes[node].min = bound.min;
        self.nodes[node].max = bound.max;
        if hi - lo <= 2 {
            self.nodes[node].index = lo as u32;
            self.nodes[node].count = (hi - lo) as u32;
            return;
        }
        let s = cent.size();
        let axis = if s.x >= s.y && s.x >= s.z { 0 } else if s.y >= s.z { 1 } else { 2 };
        let mid = (lo + hi) / 2;
        self.items[lo..hi].select_nth_unstable_by(mid - lo, |&a, &b| {
            let ca = boxes[a as usize].center().get(axis);
            let cb = boxes[b as usize].center().get(axis);
            ca.total_cmp(&cb)
        });
        let left = self.nodes.len();
        self.nodes[node].index = left as u32;
        self.nodes.push(BvhNode { min: Vec3::ZERO, max: Vec3::ZERO, index: 0, count: 0 });
        self.nodes.push(BvhNode { min: Vec3::ZERO, max: Vec3::ZERO, index: 0, count: 0 });
        self.split(boxes, left, lo, mid);
        self.split(boxes, left + 1, mid, hi);
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CameraStart {
    pub pos: Vec3,
    /// what f32 cannot hold of the position (Camera::fine)
    pub fine: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub speed: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub objects: usize,
    pub analytic: usize,
    pub sdf: usize,
    pub sdf_ops: usize,
    pub leaves: usize,
    pub terrains: usize,
    pub terrain_samples: usize,
    pub bvh_nodes: usize,
    pub unbounded: usize,
    /// NURBS surfaces and their bicubic patches
    pub nurbs: usize,
    pub patches: usize,
}

/// Earth's gravity, m/s²: the environment's `gravity` when it gives none.
pub const EARTH_GRAVITY: f32 = 9.81;

pub struct Scene {
    pub camera: CameraStart,
    pub env: Env,
    pub materials: Vec<Material>,
    pub mats: MaterialSoa,
    pub lights: Vec<Light>,
    /// glowing objects as lights (the brightest 32)
    pub emitters: Vec<Emitter>,
    /// per object: 1 when it has an emitter (the path tracer samples those directly)
    pub emitter_flags: Vec<i32>,
    pub objects: Vec<Object>,
    /// volume clouds (not objects: rays pass through them)
    pub volumes: Vec<volume::Volume>,
    pub bvh: Bvh,
    /// objects with infinite bounds, tested by every ray
    pub unbounded: Vec<u32>,
    /// the water plane's height and deep colour, when there is water
    pub water: Option<(f32, Vec3)>,
    /// m/s², down (the environment's `gravity`; Earth's by default)
    pub gravity: f32,
    /// the size and speed of its waves (multiples)
    pub wave_height: f32,
    pub wave_speed: f32,
    /// render settings from the file
    pub render: Option<serde_json::Value>,
    /// the radius of the finite part of the scene around the origin
    pub extent: f32,
    pub stats: Stats,
    /// objects left out because they would not compile, each with why: the
    /// rest of the scene still works (an edit that breaks one object does not
    /// freeze them all)
    pub problems: Vec<String>,
}

struct MatTable {
    defs: BTreeMap<String, MaterialDef>,
    list: Vec<Material>,
    by_name: BTreeMap<String, u16>,
}

impl MaterialSource for MatTable {
    fn resolve(&mut self, m: &MatRef) -> Result<u16, String> {
        match m {
            MatRef::Name(n) => {
                if let Some(&i) = self.by_name.get(n) {
                    return Ok(i);
                }
                let def = self.defs.get(n).ok_or_else(|| format!("unknown material '{n}'"))?;
                let mat = Material::from_def(n, def)?;
                Ok(self.push(n.clone(), mat))
            }
            MatRef::Inline(d) => {
                let name = format!("#{}", self.list.len());
                let mat = Material::from_def(&name, d)?;
                Ok(self.push(name, mat))
            }
        }
    }
}

impl MatTable {
    fn push(&mut self, name: String, m: Material) -> u16 {
        let i = self.list.len() as u16;
        self.list.push(m);
        self.by_name.insert(name, i);
        i
    }
}

/// Heightmaps by their parameters, so recompiling a scene only rebuilds the
/// terrain that changed.
#[derive(Default)]
pub struct TerrainCache {
    map: HashMap<String, Arc<Terrain>>,
    used: Vec<String>,
    /// NURBS surfaces by their definition (built in their own frame, so a
    /// moved surface is the same one): building the glass chess set's took
    /// 57 ms, every compile, so every frame something moved
    surfaces: HashMap<String, Arc<nurbs::Nurbs>>,
    used_surfaces: Vec<String>,
}

impl TerrainCache {
    fn get(&mut self, key: String, build: impl FnOnce() -> Result<Terrain, String>) -> Result<Arc<Terrain>, String> {
        self.used.push(key.clone());
        if let Some(t) = self.map.get(&key) {
            return Ok(t.clone());
        }
        let t = Arc::new(build()?);
        self.map.insert(key, t.clone());
        Ok(t)
    }

    /// The NURBS surface of definition `key`, built once.
    fn surface(&mut self, key: String, build: impl FnOnce() -> Result<nurbs::Nurbs, String>) -> Result<Arc<nurbs::Nurbs>, String> {
        self.used_surfaces.push(key.clone());
        if let Some(s) = self.surfaces.get(&key) {
            return Ok(s.clone());
        }
        let s = Arc::new(build()?);
        self.surfaces.insert(key, s.clone());
        Ok(s)
    }

    /// Forgets the maps and surfaces the last compile did not use.
    fn sweep(&mut self) {
        let used = std::mem::take(&mut self.used);
        self.map.retain(|k, _| used.contains(k));
        let used: std::collections::HashSet<String> = std::mem::take(&mut self.used_surfaces).into_iter().collect();
        self.surfaces.retain(|k, _| used.contains(k));
    }
}

/// The text chunks a screenshot carries: the scene as it was, a document
/// that opens as it is (the view and every setting in it, as a quick save
/// has them), and the rest of the moment (the file it came from, the
/// walker...), apart since a scene has no place for them.
pub const STILL_SCENE: &str = "ray-scene";
pub const STILL_MOMENT: &str = "ray-still";

/// A scene document from a scene file or from a screenshot (a PNG with the
/// scene in it), with the screenshot's moment. Not migrated.
pub fn read_doc(path: &std::path::Path) -> Result<(serde_json::Value, Option<serde_json::Value>), String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok((serde_json::from_slice(&bytes).map_err(|e| e.to_string())?, None));
    }
    let t = crate::render::png_text(std::io::Cursor::new(&bytes), &[STILL_SCENE, STILL_MOMENT])?;
    let [scene, moment] = [&t[0], &t[1]];
    let scene = scene.as_deref().ok_or("a picture with no scene in it (ray's screenshots carry theirs)")?;
    let moment = moment.as_deref().map(serde_json::from_str).transpose().map_err(|e| e.to_string())?;
    Ok((serde_json::from_str(scene).map_err(|e| e.to_string())?, moment))
}

/// Two scene documents alike but for the view and the render settings (a
/// screenshot of a file not since changed shows nothing unsaved).
pub fn same_scene(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    let bare = |v: &serde_json::Value| {
        let mut v = v.clone();
        if let Some(o) = v.as_object_mut() {
            o.remove("camera");
            o.remove("render");
        }
        v
    };
    bare(a) == bare(b)
}

impl Scene {
    /// A scene file, or the scene in a screenshot.
    pub fn load(path: &std::path::Path) -> Result<Scene, String> {
        let (v, _) = read_doc(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let s = Scene::from_value(&v, &mut TerrainCache::default()).map_err(|e| format!("{}: {e}", path.display()))?;
        for p in &s.problems {
            eprintln!("ray: {}: {p} (left out)", path.display());
        }
        Ok(s)
    }

    /// Compiles a scene document (the JSON as a value).
    pub fn from_value(v: &serde_json::Value, cache: &mut TerrainCache) -> Result<Scene, String> {
        let mut v = v.clone();
        json::migrate(&mut v);
        // an object that does not parse is left out (hidden in its place, so
        // the others keep their indices), and said so
        let mut problems = Vec::new();
        if let Some(objs) = v.get_mut("objects").and_then(serde_json::Value::as_array_mut) {
            for (i, o) in objs.iter_mut().enumerate() {
                if let Err(e) = serde_json::from_value::<NodeDef>(o.clone()) {
                    let name = o.get("name").and_then(serde_json::Value::as_str).map(str::to_string).unwrap_or_else(|| format!("#{i}"));
                    problems.push(format!("object '{name}': {e}"));
                    *o = serde_json::json!({ "type": "sphere", "disabled": true });
                }
            }
        }
        let def: SceneDef = serde_json::from_value(v).map_err(|e| e.to_string())?;
        let r = Scene::compile(&def, cache, problems);
        cache.sweep();
        r
    }

    fn compile(def: &SceneDef, cache: &mut TerrainCache, mut problems: Vec<String>) -> Result<Scene, String> {
        let env = compile_env(def)?;
        let mut mats = MatTable { defs: def.materials.clone(), list: Vec::new(), by_name: BTreeMap::new() };
        let default = MaterialDef::default();
        mats.push("default".into(), Material::from_def("default", &default)?);

        // the water level first: terrain uses it for its sand line
        let water_col = preset_of(&def.environment)?.water_col;
        let mut water = None;
        let (mut wave_height, mut wave_speed) = (1.0, 1.0);
        let members = flatten(&def.objects, &mut problems);
        for (_, _, o) in &members {
            if let ShapeDef::Water { level, waves, wave_speed: speed } = o.shape {
                water = Some((level + o.position.map(|p| p[1]).unwrap_or(0.0), Vec3::from_array(water_col) * env.daylight));
                (wave_height, wave_speed) = (waves.max(0.0), speed.max(0.0));
            }
        }

        let mut objects = Vec::new();
        let mut volumes = Vec::new();
        let mut stats = Stats::default();
        for (i, path, node) in &members {
            let i = *i;
            let name = node.name.clone().unwrap_or_else(|| format!("{}#{i}", shape_name(&node.shape)));
            // clouds are volumes: marched through, not hit
            if let ShapeDef::Cloud { puffs, blend, density, turbulence, wisps, frequency, detail, softness, seed, color, stretch } = &node.shape {
                let default = volume::default_puffs();
                let v = volume::build(
                    &name,
                    i,
                    puffs.as_deref().unwrap_or(&default),
                    *blend,
                    *density,
                    *turbulence,
                    *wisps,
                    *frequency,
                    *detail,
                    *softness,
                    *seed,
                    *color,
                    node.position,
                    node.rotation,
                    node.scale,
                    *stretch,
                );
                match v {
                    Ok(v) => volumes.push(v),
                    Err(e) => problems.push(format!("object '{name}': {e}")),
                }
                continue;
            }
            if matches!(node.shape, ShapeDef::Emitter { .. }) && !path.is_empty() {
                problems.push(format!("object '{name}': an emitter must be a top-level object, not in a group"));
                continue;
            }
            // a hidden emitter is nothing to see
            if matches!(node.shape, ShapeDef::Emitter { visible: false, .. }) {
                continue;
            }
            let mut obj = match compile_object(node, i, &name, &mut mats, water.map(|w| w.0), cache) {
                Ok(o) => o,
                Err(e) => {
                    problems.push(format!("object '{name}': {e}"));
                    continue;
                }
            };
            if !path.is_empty() {
                obj.path = path.clone();
            }
            // glass lets light through (approximately: no caustics)
            if node.shadow.is_none() && mats.list[obj.material as usize].transmission > 0.5 {
                obj.shadow = false;
            }
            match &obj.shape {
                Shape::Sdf(p) => {
                    stats.sdf += 1;
                    stats.sdf_ops += p.ops.len();
                    stats.leaves += p.leaves;
                }
                Shape::Terrain(t) => {
                    stats.terrains += 1;
                    stats.terrain_samples += t.n * t.n;
                }
                Shape::Nurbs { surf, .. } => {
                    stats.nurbs += 1;
                    stats.patches += surf.patches.len();
                }
                _ => stats.analytic += 1,
            }
            objects.push(obj);
        }
        stats.objects = objects.len();

        let mut bounded = Vec::new();
        let mut unbounded = Vec::new();
        let mut extent: f32 = 10.0;
        for (i, o) in objects.iter().enumerate() {
            if o.aabb.is_empty() {
                continue;
            }
            if o.aabb.is_finite() {
                bounded.push(i as u32);
                extent = extent.max(o.aabb.min.length()).max(o.aabb.max.length());
            } else {
                unbounded.push(i as u32);
            }
        }
        let boxes: Vec<Aabb> = objects.iter().map(|o| o.aabb).collect();
        let bvh = Bvh::build(&boxes, bounded);
        stats.bvh_nodes = bvh.nodes.len();
        stats.unbounded = unbounded.len();

        let lights = def
            .lights
            .iter()
            .map(|l| Light {
                pos: Vec3::from_array(l.position),
                color: Vec3::from_array(l.color.rgb()) * l.intensity,
                range: l.range.max(0.01),
            })
            .collect();

        let (emitters, emitter_flags) = find_emitters(&objects, &mats.list);
        let c = &def.camera;
        let mut start = crate::camera::Camera::default();
        start.set_exact(c.position);
        let (pos, fine) = (start.pos, start.fine);
        let (yaw, pitch) = match c.look_at {
            Some(t) => {
                let d = (Vec3::from_array(t) - pos).normalize();
                ((-d.x).atan2(-d.z), d.y.clamp(-1.0, 1.0).asin())
            }
            None => (c.yaw.to_radians(), c.pitch.to_radians()),
        };
        let materials = mats.list;
        Ok(Scene {
            camera: CameraStart { pos, fine, yaw, pitch, fov: c.fov.clamp(5.0, 170.0), speed: c.speed.max(0.01) },
            env,
            mats: MaterialSoa::build(&materials),
            materials,
            lights,
            emitters,
            emitter_flags,
            objects,
            volumes,
            bvh,
            unbounded,
            water,
            wave_height,
            gravity: def.environment.gravity.unwrap_or(EARTH_GRAVITY).clamp(0.0, 1000.0),
            wave_speed,
            render: def.render.clone(),
            extent,
            stats,
            problems,
        })
    }

    /// The ground height under (x, z) for the camera, if any terrain covers it.
    pub fn ground(&self, x: f32, z: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for o in &self.objects {
            if let Shape::Terrain(t) = &o.shape {
                let b = o.aabb;
                if x >= b.min.x && x <= b.max.x && z >= b.min.z && z <= b.max.z {
                    let h = t.height(x, z);
                    best = Some(best.map_or(h, |v: f32| v.max(h)));
                }
            }
        }
        best
    }
}

/// The average glow of a material (the pattern blends the two emissions).
fn glow(m: &Material) -> Vec3 {
    match m.tex {
        tex::NONE => m.emission,
        // lit windows are many small lights: one sphere light in the middle
        // of the building would light the street wrongly (and crowd out the lamps)
        tex::WINDOWS => m.emission * 0.6,
        _ => m.emission * 0.6 + m.emission2 * 0.4,
    }
}

/// Sphere lights for every glowing leaf (distance programs), object (exact
/// shapes, terrain) or ball; the 64 brightest are kept.
fn find_emitters(objects: &[Object], mats: &[Material]) -> (Vec<Emitter>, Vec<i32>) {
    let mut out = Vec::new();
    let mut add = |obj: usize, b: Aabb, mat: u16| {
        let Some(m) = mats.get(mat as usize) else { return };
        let c = glow(m);
        let peak = c.max_comp();
        if peak <= 0.01 || !b.is_finite() || b.is_empty() {
            return;
        }
        let s = b.size().max(Vec3::splat(1e-3));
        let radius = (s.x * s.y * s.z).cbrt() * 0.5;
        // the material's, else where radiance * (r / d)^2 drops below 0.002
        // (the light fades out toward there, without an edge even in a dark scene)
        let range = if m.light_range > 0.0 { m.light_range.max(radius * 1.5) } else { (radius * (peak / 0.002).sqrt()).min(200.0) };
        out.push(Emitter { pos: b.center(), radius, color: c, range, obj: obj as u32 });
    };
    for (i, o) in objects.iter().enumerate() {
        match &o.shape {
            Shape::Sdf(p) => {
                for &(tag, b) in &p.leaf_bounds {
                    add(i, b, (tag & 0xffff) as u16);
                }
            }
            Shape::Plane { .. } | Shape::Terrain(_) => {}
            _ => add(i, o.aabb, o.material),
        }
    }
    out.sort_by(|a, b| (b.color.max_comp() * b.radius * b.radius).total_cmp(&(a.color.max_comp() * a.radius * a.radius)));
    out.truncate(64);
    let mut flags = vec![0; objects.len().max(1)];
    for e in &out {
        flags[e.obj as usize] = 1;
    }
    (out, flags)
}

fn shape_name(s: &ShapeDef) -> &'static str {
    use ShapeDef as S;
    match s {
        S::Sphere { .. } => "sphere",
        S::Box { .. } => "box",
        S::Plane { .. } => "plane",
        S::Torus { .. } => "torus",
        S::Cylinder { .. } => "cylinder",
        S::Cone { .. } => "cone",
        S::Capsule { .. } => "capsule",
        S::Ellipsoid { .. } => "ellipsoid",
        S::Octahedron { .. } => "octahedron",
        S::Gyroid { .. } => "gyroid",
        S::Menger { .. } => "menger",
        S::Mandelbulb { .. } => "mandelbulb",
        S::Mandelbox { .. } => "mandelbox",
        S::Julia { .. } => "julia",
        S::Sierpinski { .. } => "sierpinski",
        S::Metaballs { .. } => "metaballs",
        S::Union { .. } => "union",
        S::Intersection { .. } => "intersection",
        S::Difference { .. } => "difference",
        S::Repeat { .. } => "repeat",
        S::Mirror { .. } => "mirror",
        S::Twist { .. } => "twist",
        S::Bend { .. } => "bend",
        S::Round { .. } => "round",
        S::Shell { .. } => "shell",
        S::Displace { .. } => "displace",
        S::Relief { .. } => "relief",
        S::Apollonian { .. } => "apollonian",
        S::Kifs { .. } => "kifs",
        S::Kleinian { .. } => "kleinian",
        S::Slice { .. } => "slice",
        S::Emitter { .. } => "emitter",
        S::Group { .. } => "group",
        S::Terrain(_) => "terrain",
        S::Water { .. } => "water",
        S::Cloud { .. } => "cloud",
        S::Nurbs { .. } => "nurbs",
        S::Revolve { .. } => "revolve",
    }
}

fn preset_of(e: &json::EnvDef) -> Result<&'static Preset, String> {
    let name = e.preset.as_deref().unwrap_or("golden");
    PRESETS.iter().find(|p| p.name == name).ok_or_else(|| format!("unknown environment preset '{name}' (golden, dusk, frozen, storm)"))
}

/// The environment's settings as the file has them, not as the hour shows
/// them: the sun's azimuth and elevation (with a time of day, at noon) and
/// the sky overhead, the sky at the horizon, the fog and the ambient colours.
pub struct EnvBase {
    pub sun: (f32, f32),
    pub colours: [Vec3; 4],
}

/// The base settings of an `environment` (the JSON object), with the
/// preset's values where it has none.
pub fn env_base(v: &serde_json::Value) -> Option<EnvBase> {
    let e: json::EnvDef = serde_json::from_value(v.clone()).ok()?;
    let p = preset_of(&e).ok()?;
    let sun = if e.time_of_day.is_some() {
        (e.sun_azimuth.unwrap_or(180.0), e.sun_elevation.unwrap_or(60.0))
    } else if e.sun_azimuth.is_some() || e.sun_elevation.is_some() {
        (e.sun_azimuth.unwrap_or(0.0), e.sun_elevation.unwrap_or(30.0))
    } else {
        sun_angles(Vec3::from_array(e.sun_direction.unwrap_or(p.sun_dir)).normalize())
    };
    let v = |o: Option<[f32; 3]>, d: [f32; 3]| Vec3::from_array(o.unwrap_or(d));
    Some(EnvBase { sun, colours: [v(e.sky_zenith, p.sky_zen), v(e.sky_horizon, p.sky_hor), v(e.fog_color, p.fog_col), v(e.ambient, p.amb_col)] })
}

fn compile_env(def: &SceneDef) -> Result<Env, String> {
    let e = &def.environment;
    let p = preset_of(e)?;
    let v = |o: Option<[f32; 3]>, d: [f32; 3]| Vec3::from_array(o.unwrap_or(d));
    let sun_level = e.sun_level.unwrap_or(1.0).max(0.0);
    let mut sun_dir = v(e.sun_direction, p.sun_dir).normalize();
    if e.sun_azimuth.is_some() || e.sun_elevation.is_some() {
        sun_dir = sun_from_angles(e.sun_azimuth.unwrap_or(0.0), e.sun_elevation.unwrap_or(30.0));
    }
    let sky = match e.sky.as_deref().unwrap_or("day") {
        "day" => Sky::Day,
        "stars" => Sky::Stars,
        "off" => Sky::Off,
        s => return Err(format!("unknown sky '{s}' (day, stars, off)")),
    };
    // the light the sky lends the air (the fog's own colour) and the water: less
    // at night, none without a sky, so that with the sun and ambient light off
    // too only glowing objects light anything
    let sky_light = match sky {
        Sky::Day => 1.0,
        Sky::Stars => 0.04,
        Sky::Off => 0.0,
    };
    let ambient = e.ambient_level.unwrap_or(1.0).max(0.0);
    let sun_col = v(e.sun_color, p.sun_col) * sun_level;
    let mut env = Env {
        sun_dir,
        sun_col,
        amb_col: v(e.ambient, p.amb_col),
        sky_zen: v(e.sky_zenith, p.sky_zen),
        sky_hor: v(e.sky_horizon, p.sky_hor),
        fog_col: v(e.fog_color, p.fog_col) * sky_light,
        fog: e.fog_density.unwrap_or(p.fog).max(0.0),
        clouds: e.clouds.unwrap_or(p.clouds).clamp(0.0, 1.0),
        ambient,
        sky,
        daylight: sun_level.max(ambient * sky_light),
        glow_dir: sun_dir,
        glow_col: sun_col,
        moon_dir: -sun_dir,
        moon: 0.0,
        night: 0.0,
    };
    if let Some(h) = e.time_of_day {
        time_of_day(&mut env, h, e, sun_level, sky_light);
    }
    Ok(env)
}

/// Smoothstep from `a` to `b`.
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The sun and the moon at hour `h`, and the light, sky and air that go with
/// them. The palette's colours are its noon; low down, the sunlight crosses
/// more air, which takes out blue first (the light reddens, and the sky
/// near the sun with it); after sunset the glow lingers, the sky darkens to
/// night and the stars come out, and the full moon (opposite the sun)
/// becomes the light.
fn time_of_day(env: &mut Env, h: f32, e: &json::EnvDef, sun_level: f32, sky_light: f32) {
    let h = h.rem_euclid(24.0);
    let noon_az = e.sun_azimuth.unwrap_or(180.0);
    let noon_el = e.sun_elevation.unwrap_or(60.0).clamp(5.0, 90.0);
    let el = noon_el * (std::f32::consts::PI * (h - 6.0) / 12.0).sin();
    let sun = sun_from_angles(noon_az + 15.0 * (h - 12.0), el);
    let moon = -sun;
    // air the sunlight crosses (1 straight up), and what is left of each
    // channel, relative to noon
    let air = |el: f32| 1.0 / (el.max(0.0).to_radians().sin() + 0.05);
    let through = |el: f32| {
        let k = Vec3::new(0.06, 0.13, 0.28);
        let a = air(el) - air(noon_el);
        Vec3::new((-k.x * a).exp(), (-k.y * a).exp(), (-k.z * a).exp())
    };
    let day = smooth(-6.0, 8.0, el);
    let up = smooth(-1.0, 1.5, el);
    // sunset colours: strongest with the sun just below the horizon
    let warm = smooth(-9.0, -1.0, el) * (1.0 - smooth(3.0, 22.0, el));
    let dim = 0.02 + 0.98 * day;
    let base = env.sun_col;
    env.glow_dir = sun;
    env.glow_col = base.hadamard(through(el)) * smooth(-9.0, 0.0, el);
    env.night = 1.0 - smooth(-14.0, -4.0, el);
    let sunset_hor = Vec3::new(1.0, 0.42, 0.18) * (env.sky_hor.length() * 0.8);
    env.sky_hor = (env.sky_hor + (sunset_hor - env.sky_hor) * warm) * dim;
    env.sky_zen = (env.sky_zen + (Vec3::new(0.06, 0.09, 0.22) - env.sky_zen) * (1.0 - day)) * dim;
    env.fog_col = (env.fog_col + (sunset_hor * 0.6 - env.fog_col) * warm) * dim;
    // the sky's light: warm while the sun is low (the glow around it lights
    // the shadows too), then the blue of night
    let amb = env.amb_col.length();
    let warm_amb = Vec3::new(0.62, 0.42, 0.30) * (amb / Vec3::new(0.62, 0.42, 0.30).length());
    env.amb_col = env.amb_col + (warm_amb - env.amb_col) * (warm * 0.7);
    env.amb_col = (env.amb_col + (Vec3::new(0.22, 0.27, 0.42) - env.amb_col) * (1.0 - day)) * (0.05 + 0.95 * day);
    if el > -0.5 {
        env.sun_dir = sun;
        env.sun_col = base.hadamard(through(el)) * up;
    } else {
        // the moon lights the night, coming up as the sun's glow fades
        env.sun_dir = moon;
        env.sun_col = Vec3::new(0.30, 0.34, 0.45) * (0.45 * sun_level * (1.0 - smooth(-6.0, -0.5, el)) * smooth(-1.0, 2.0, -el));
    }
    env.moon_dir = moon;
    env.moon = smooth(-1.0, 2.0, -el) * (0.35 + 0.65 * env.night);
    let sky_light = sky_light * (0.04 + 0.96 * day);
    env.daylight = (sun_level * up).max(env.ambient * sky_light);
}

/// Unit vector toward the sun from azimuth (degrees from -z toward +x) and
/// elevation (degrees above the horizon).
pub fn sun_from_angles(azimuth: f32, elevation: f32) -> Vec3 {
    let (sa, ca) = azimuth.to_radians().sin_cos();
    let (se, ce) = elevation.to_radians().sin_cos();
    Vec3::new(sa * ce, se, -ca * ce)
}

/// Azimuth and elevation (degrees) of a direction.
pub fn sun_angles(d: Vec3) -> (f32, f32) {
    (d.x.atan2(-d.z).to_degrees(), d.y.clamp(-1.0, 1.0).asin().to_degrees())
}

/// The document's objects with their groups opened: every member (at any
/// depth) an object of its own in the world frame, with its top-level index
/// and its path of child indices in there.
fn flatten(objects: &[NodeDef], problems: &mut Vec<String>) -> Vec<(usize, Vec<usize>, NodeDef)> {
    fn open(node: &NodeDef, top: usize, path: &mut Vec<usize>, out: &mut Vec<(usize, Vec<usize>, NodeDef)>, problems: &mut Vec<String>) {
        let ShapeDef::Group { children } = &node.shape else {
            out.push((top, path.clone(), node.clone()));
            return;
        };
        let group = node.name.clone().unwrap_or_else(|| format!("group#{top}"));
        if path.len() > 16 {
            problems.push(format!("object '{group}': groups nested too deep"));
            return;
        }
        for (k, c) in children.iter().enumerate().filter(|(_, c)| !c.disabled) {
            let mut m = c.clone();
            compose(node, &mut m);
            m.name = Some(c.name.clone().unwrap_or_else(|| format!("{group}/{}", shape_name(&c.shape))));
            path.push(k);
            open(&m, top, path, out, problems);
            path.pop();
        }
    }
    let mut out = Vec::new();
    for (i, node) in objects.iter().enumerate().filter(|(_, o)| !o.disabled) {
        open(node, i, &mut Vec::new(), &mut out, problems);
    }
    out
}

/// Member `m` of group `g` in the group's parent frame: the group's
/// transform after its own, and the group's material, shadow, friction and
/// bounce where it has none of its own. A group moves as one body, so its
/// members are locked when it is (and only then).
fn compose(g: &NodeDef, m: &mut NodeDef) {
    let gr = g.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
    let gs = g.scale.unwrap_or(1.0);
    let gt = g.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
    let mt = m.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
    m.position = Some((gt + gr.apply(mt * gs)).to_array());
    if g.rotation.is_some() {
        let mr = m.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
        m.rotation = Some(gr.mul(mr).to_euler_deg());
    }
    m.scale = Some(gs * m.scale.unwrap_or(1.0));
    m.material = m.material.take().or_else(|| g.material.clone());
    m.shadow = m.shadow.or(g.shadow);
    m.friction = m.friction.or(g.friction);
    m.bounce = m.bounce.or(g.bounce);
    let inner = matches!(m.shape, ShapeDef::Group { .. }) && m.locked == Some(true);
    m.locked = Some(g.locked == Some(true) || inner);
}

fn compile_object(
    node: &NodeDef,
    doc_index: usize,
    name: &str,
    mats: &mut MatTable,
    water_level: Option<f32>,
    cache: &mut TerrainCache,
) -> Result<Object, String> {
    let rot = node.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY);
    let s = node.scale.unwrap_or(1.0);
    if !(s > 0.0) {
        return Err("scale must be positive".into());
    }
    let t = node.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO);
    let mat = match &node.material {
        Some(m) => mats.resolve(m)?,
        None => 0,
    };
    let step = node.step.unwrap_or(1.0).clamp(0.05, 2.0);
    let shadow = node.shadow.unwrap_or(true);
    let obj = |shape: Shape, aabb: Aabb, material: u16, shadow: bool| Object {
        name: name.to_string(),
        shape,
        aabb,
        material,
        shadow,
        step,
        doc_index,
        local_m: rot.transpose().scale(1.0 / s),
        local_t: t,
        node_cycles: vec![0.0],
        friction: node.friction.unwrap_or(0.5).max(0.0),
        bounce: node.bounce.unwrap_or(0.15).clamp(0.0, 1.0),
        locked: node.locked.unwrap_or(false) || !aabb.is_finite(),
        ghost: false,
        path: Vec::new(),
        org: Vec3::ZERO,
    };
    match node.shape {
        ShapeDef::Sphere { radius } => {
            let r = radius * s;
            return Ok(obj(Shape::Sphere { c: t, r }, Aabb::cube(r).transform(Mat3::IDENTITY, 1.0, t), mat, shadow));
        }
        ShapeDef::Box { size, rounding } if rounding <= 0.0 && size.iter().all(|&v| v >= 0.0) => {
            let half = Vec3::from_array(size) * (0.5 * s);
            let aabb = Aabb::half(half).transform(rot, 1.0, t);
            return Ok(obj(Shape::Box { c: t, rot: rot.transpose(), half }, aabb, mat, shadow));
        }
        ShapeDef::Plane { normal, offset } => {
            let n = rot.apply(Vec3::from_array(normal).normalize());
            let d = offset * s + n.dot(t);
            return Ok(obj(Shape::Plane { n, d }, Aabb::INFINITE, mat, shadow));
        }
        ShapeDef::Water { .. } => {
            let level = water_level.unwrap_or(0.0);
            let col = Vec3::new(0.02, 0.07, 0.09);
            let m = match &node.material {
                Some(_) => mat,
                None => {
                    let i = mats.list.len() as u16;
                    mats.list.push(Material::water(col));
                    i
                }
            };
            return Ok(obj(Shape::Plane { n: Vec3::Y, d: level }, Aabb::INFINITE, m, node.shadow.unwrap_or(false)));
        }
        ShapeDef::Nurbs { .. } | ShapeDef::Revolve { .. } => {
            let key = serde_json::to_string(&node.shape).unwrap_or_default();
            let surf = cache.surface(key, || Ok(nurbs::Nurbs::from_shape(&node.shape)?.expect("a surface")))?;
            let aabb = surf.aabb.transform(rot, s, t);
            return Ok(obj(Shape::Nurbs { surf, c: t, rot: rot.transpose(), s }, aabb, mat, shadow));
        }
        ShapeDef::Emitter { .. } => {
            // the emitter itself: a small glowing nozzle pointing along its +y
            let marker: NodeDef = serde_json::from_value(serde_json::json!({
                "type": "cone", "radius": 0.12, "top_radius": 0.05, "height": 0.22,
                "position": node.position, "rotation": node.rotation, "scale": node.scale,
                "material": { "color": [0.95, 0.6, 0.2], "emission": [0.5, 0.2, 0.04], "roughness": 0.4 }
            }))
            .map_err(|e| e.to_string())?;
            let mut o = compile_object(&marker, doc_index, name, mats, water_level, cache)?;
            (o.locked, o.ghost, o.shadow) = (true, true, false);
            return Ok(o);
        }
        ShapeDef::Terrain(ref td) => {
            let key = format!("{}|{t:?}|{water_level:?}", serde_json::to_string(td).unwrap_or_default());
            let terrain = cache.get(key, || Terrain::build(td, t, water_level))?;
            let m = match &node.material {
                Some(_) => mat,
                None => {
                    let def = MaterialDef { roughness: 0.9, texture: Some("terrain".into()), ..Default::default() };
                    let i = mats.list.len() as u16;
                    mats.list.push(Material::from_def("terrain", &def)?);
                    i
                }
            };
            let aabb = terrain.aabb();
            let mut o = obj(Shape::Terrain(terrain), aabb, m, shadow);
            o.locked = true;
            return Ok(o);
        }
        _ => {}
    }
    // (in metres from the object's position: see Object.org)
    let mut here = node.clone();
    here.position = None;
    let mut c = Compiler::new(mats).with_cache(cache);
    let (d, bound) = c.node(&here, 0, mat)?;
    debug_assert_eq!(d, 0);
    let factor = c.step;
    let mut prog = c.finish();
    for (_, b) in &mut prog.leaf_bounds {
        *b = b.transform(Mat3::IDENTITY, 1.0, t);
    }
    let mut aabb = bound.transform(Mat3::IDENTITY, 1.0, t);
    if let Some(r) = node.bound {
        aabb = aabb.intersect(Aabb::cube(r).transform(Mat3::IDENTITY, 1.0, t));
    }
    let node_cycles = prog
        .ranges
        .iter()
        .map(|&(a, b)| prog.ops[a as usize..b as usize].iter().map(crate::render::vm::op_cycles).sum())
        .collect();
    let mut o = obj(Shape::Sdf(prog), aabb, mat, shadow);
    o.org = t;
    o.node_cycles = node_cycles;
    if node.step.is_none() {
        o.step = factor;
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compiling a scene again (every edit does, a slider drag every frame)
    /// takes its NURBS surfaces, and their distance grids, from the cache
    /// instead of cutting them into pieces again.
    #[test]
    fn surfaces_are_built_once() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let v = serde_json::json!({ "objects": [
            { "type": "difference", "children": [{ "type": "box", "size": [1, 1, 1] }, { "type": "revolve", "profile": [[0, -0.6], [0.6, -0.6, h], [0.6, 0], [0.6, 0.6, h], [0, 0.6]], "degree": 2, "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1] }] },
            { "type": "revolve", "position": [3, 0, 0] }
        ]});
        let mut cache = TerrainCache::default();
        let (a, b) = (Scene::from_value(&v, &mut cache).unwrap(), Scene::from_value(&v, &mut cache).unwrap());
        let leaf = |s: &Scene| match &s.objects[0].shape {
            Shape::Sdf(p) => p.surfaces[0].clone(),
            _ => panic!("not a program"),
        };
        let alone = |s: &Scene| match &s.objects[1].shape {
            Shape::Nurbs { surf, .. } => surf.clone(),
            _ => panic!("not a surface"),
        };
        assert!(Arc::ptr_eq(&leaf(&a), &leaf(&b)), "the leaf's surface was built again");
        assert!(leaf(&b).grid.get().is_some(), "its grid is built with it");
        assert!(Arc::ptr_eq(&alone(&a), &alone(&b)), "the surface alone was built again");
    }

    /// Every scene in saves/ loads with nothing left out (a renamed field, a
    /// shape that stopped compiling, a scene the generators wrote wrongly).
    #[test]
    fn every_saved_scene_loads() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("saves");
        let mut seen = 0;
        for e in std::fs::read_dir(&dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().is_none_or(|x| x != "json") || p.file_stem().is_some_and(|s| s == "quicksave") {
                continue;
            }
            let text = std::fs::read_to_string(&p).unwrap();
            let mut v: serde_json::Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            json::migrate(&mut v);
            let sc = Scene::from_value(&v, &mut TerrainCache::default()).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            assert!(sc.problems.is_empty(), "{}: {:?}", p.display(), sc.problems);
            assert!(!sc.objects.is_empty(), "{}: no objects", p.display());
            seen += 1;
        }
        assert!(seen >= 15, "only {seen} scenes in {}", dir.display());
    }

    /// A NURBS surface in a boolean is a leaf measured by its signed
    /// distance: an exact sphere (a revolve) cut from a box leaves a hole a
    /// radius from its centre (deep inside the sphere it is negative, so the
    /// cut is there), and the box's wall is solid; under a twist it compiles
    /// too.
    #[test]
    fn nurbs_in_booleans() {
        use crate::render::vm::{Regs, eval};
        use crate::simd::V8;
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let ball = serde_json::json!({ "type": "revolve", "profile": [[0, -1], [1, -1, h], [1, 0], [1, 1, h], [0, 1]], "degree": 2, "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1] });
        let v = serde_json::json!({ "objects": [
            { "type": "difference", "children": [{ "type": "box", "size": [4, 4, 4] }, ball.clone()] },
            { "type": "twist", "amount": 0.5, "position": [10, 0, 0], "child": ball }
        ]});
        let sc = Scene::from_value(&v, &mut TerrainCache::default()).unwrap();
        assert!(sc.problems.is_empty(), "{:?}", sc.problems);
        let o = &sc.objects[0];
        let Shape::Sdf(prog) = &o.shape else { panic!("not a program") };
        assert_eq!(prog.surfaces.len(), 1);
        let mut regs = Regs::new();
        let mut d = |p: Vec3| eval(prog, V8::splat(p - o.org), &mut regs, false).0.lane(0);
        let at_centre = d(Vec3::ZERO);
        // (there the distance comes from the surface's grid: within its slack,
        // half a cell's diagonal, about 0.05 here)
        assert!((at_centre - 1.0).abs() < 0.06, "the hole's centre: {at_centre}");
        assert!(d(Vec3::new(1.5, 0.2, 0.1)) < 0.0, "the wall should be solid");
        assert!((d(Vec3::new(3.0, 0.0, 0.0)) - 1.0).abs() < 0.02, "outside the box");
        assert!(matches!(sc.objects[1].shape, Shape::Sdf(_)), "a twisted revolve is a program");
    }

    /// A leaf two groups down lands where the groups' transforms put it, and
    /// knows its path.
    #[test]
    fn groups_compose() {
        let v = serde_json::json!({
            "objects": [
                { "type": "sphere", "radius": 1 },
                { "type": "group", "position": [10, 0, 0], "rotation": [0, 90, 0], "scale": 2, "material": { "color": [1, 0, 0] }, "children": [
                    { "type": "box", "size": [1, 1, 1], "disabled": true },
                    { "type": "group", "position": [1, 0, 0], "children": [
                        { "type": "box", "size": [1, 2, 3], "position": [0, 1, 0], "rotation": [0, 0, 30] }
                    ]}
                ]}
            ]
        });
        let s = Scene::from_value(&v, &mut TerrainCache::default()).unwrap();
        assert!(s.problems.is_empty(), "{:?}", s.problems);
        assert_eq!(s.objects.len(), 2);
        let o = &s.objects[1];
        assert_eq!((o.doc_index, o.path.clone()), (1, vec![1, 0]));
        assert!(!o.locked, "an unlocked group moves");
        // (1, 1, 0) in the outer group's frame: turned 90 degrees about y and doubled
        let c = o.aabb.center();
        assert!((c - Vec3::new(10.0, 2.0, -2.0)).length() < 1e-3, "{c:?}");
        let Shape::Box { rot, half, .. } = o.shape else { panic!("an analytic box") };
        assert!((half - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-4);
        let want = Mat3::euler_deg([0.0, 90.0, 0.0]).mul(Mat3::euler_deg([0.0, 0.0, 30.0])).transpose();
        assert!(rot.m.iter().zip(want.m).all(|(a, b)| (*a - b).length() < 1e-4));
        // the group's material reaches the member
        assert_eq!(s.materials[o.material as usize].color, Vec3::new(1.0, 0.0, 0.0));
    }

    #[test]
    fn euler_round_trip() {
        for a in [[10.0, 20.0, 30.0], [-45.0, 60.0, 170.0], [0.0, -80.0, 5.0]] {
            let m = Mat3::euler_deg(a);
            let b = Mat3::euler_deg(m.to_euler_deg());
            assert!(m.m.iter().zip(b.m).all(|(x, y)| (*x - y).length() < 1e-4), "{a:?}");
        }
    }
}
