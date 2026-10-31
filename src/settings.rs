//! Render settings: everything the settings panel controls. A scene file can
//! preset any subset in its "render" block. Changing a trace setting
//! restarts accumulation; post settings (exposure, bloom, ...) do not.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Whitted-style: sun, sky ambient with AO, point lights, mirror
    /// reflection and refraction chains. Deterministic, noise free.
    Direct,
    /// Monte Carlo path tracing: global illumination, glossy and diffuse
    /// bounces, emissive objects light the scene. Noisy until accumulated.
    PathTrace,
    Normals,
    Albedo,
    Depth,
    /// march iterations of the primary ray, as a heat map: the cost of a scene
    Steps,
    /// each object coloured by what one evaluation of its shape costs
    /// (render/mod.rs prices): which kinds of object are dear or cheap
    Cost,
}

impl Mode {
    pub const ALL: [Mode; 7] = [Mode::Direct, Mode::PathTrace, Mode::Normals, Mode::Albedo, Mode::Depth, Mode::Steps, Mode::Cost];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Direct => "Direct (Whitted)",
            Mode::PathTrace => "Path traced (GI)",
            Mode::Normals => "Normals",
            Mode::Albedo => "Albedo",
            Mode::Depth => "Depth",
            Mode::Steps => "Search steps",
            Mode::Cost => "Object cost",
        }
    }
}

/// Which processors trace the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    Cpu,
    Gpu,
    /// both, the rows split between them by their measured speed
    Both,
}

impl Device {
    pub const ALL: [Device; 3] = [Device::Both, Device::Gpu, Device::Cpu];

    pub fn label(self) -> &'static str {
        match self {
            Device::Cpu => "CPU only",
            Device::Gpu => "GPU only",
            Device::Both => "GPU + CPU",
        }
    }

    /// The next one in `ALL`, round (the G key).
    pub fn next(self) -> Device {
        let i = Device::ALL.iter().position(|&d| d == self).unwrap_or(0);
        Device::ALL[(i + 1) % Device::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub mode: Mode,
    /// who traces: the GPU, the CPU, or both sharing every frame
    pub device: Device,
    /// render resolution as a fraction of the window (0.25 .. 2)
    pub scale: f32,
    /// samples per pixel per frame
    pub spp: u32,
    /// average frames while nothing changes (progressive anti-aliasing and GI)
    pub accumulate: bool,
    pub max_samples: u32,
    /// reflection / refraction / GI bounces
    pub bounces: u32,
    pub shadows: bool,
    /// SDF penumbra estimates (Direct) instead of hard shadows
    pub soft_shadows: bool,
    /// penumbra sharpness: higher = harder
    pub shadow_hardness: f32,
    pub ao: bool,
    pub ao_radius: f32,
    pub lights: bool,
    pub light_shadows: bool,
    pub fog: bool,
    /// the environment's haze, as a multiple of its density (0: none), and
    /// the points along each ray its light shafts are found at a sample
    /// (more: smoother sooner, slower)
    pub haze_strength: f32,
    pub haze_detail: u32,
    /// the haze's light bent along a curve: 1 as the light makes it; more,
    /// the haze where the sun reaches holds and brighter grows stronger,
    /// dimmer (in shadow) weaker: more defined beams
    pub haze_contrast: f32,
    pub clouds: bool,
    /// sphere-tracing iterations per object
    pub max_steps: u32,
    /// hit threshold in pixels: larger is faster and blurrier
    pub detail: f32,
    pub max_dist: f32,
    /// while the view or the scene changes: one sample per frame, no extra samples
    pub fast_motion: bool,
    /// while the view or the scene changes: render at this fraction of the resolution
    pub motion_scale: f32,
    /// water waves move (the picture never settles while there is water)
    pub animate_water: bool,
    // ---- post: no accumulation restart
    pub exposure: f32,
    pub bloom: bool,
    pub edge_aa: bool,
    pub vignette: bool,
    /// ACES filmic tone map (off: clamp)
    pub tonemap: bool,
    /// the stars twinkle (in the post pass: the image keeps accumulating)
    pub twinkle: bool,
    /// a light at the camera (0 = off): it lights what the camera sees, so
    /// the inside of an object or a fractal's crevices show
    pub headlight: f32,
    /// metres over which the headlight fades out
    pub headlight_reach: f32,
    /// the headlight casts shadows (it then hangs a little above the camera,
    /// a thirtieth of its reach, so that they show)
    pub headlight_shadows: bool,
    /// fractals iterate further close up, as fine as the pixels show
    /// (off: always their own iteration counts)
    pub fractal_detail: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mode: Mode::Direct,
            device: Device::Both,
            scale: 1.0,
            spp: 1,
            accumulate: true,
            max_samples: 1024,
            bounces: 4,
            shadows: true,
            soft_shadows: true,
            shadow_hardness: 24.0,
            ao: true,
            ao_radius: 0.6,
            lights: true,
            light_shadows: true,
            fog: true,
            haze_strength: 1.0,
            haze_detail: 2,
            haze_contrast: 1.0,
            clouds: true,
            max_steps: 160,
            detail: 1.0,
            max_dist: 1000.0,
            fast_motion: true,
            motion_scale: 1.0,
            animate_water: false,
            exposure: 1.0,
            bloom: true,
            edge_aa: true,
            vignette: true,
            tonemap: true,
            twinkle: true,
            headlight: 0.0,
            headlight_reach: 25.0,
            headlight_shadows: false,
            fractal_detail: true,
        }
    }
}

impl Settings {
    /// The settings that change the traced image (a change restarts
    /// accumulation; samples per frame do not: the samples go on where they
    /// were, however many a frame).
    pub fn trace_key(&self) -> Settings {
        Settings {
            spp: 1,
            exposure: 1.0,
            bloom: true,
            edge_aa: true,
            vignette: true,
            tonemap: true,
            twinkle: true,
            scale: 1.0,
            fast_motion: true,
            motion_scale: 1.0,
            device: Device::Both,
            ..*self
        }
    }

    /// Applies a scene's "render" block over the defaults.
    pub fn from_json(v: &serde_json::Value) -> Result<Settings, String> {
        serde_json::from_value(v.clone()).map_err(|e| format!("render settings: {e}"))
    }
}
