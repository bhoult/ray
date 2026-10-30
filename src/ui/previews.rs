//! Small renders for the libraries: a material on a sphere, a modifier on a
//! sample shape. A thread of their own renders them on a few cores (the view
//! keeps the rest), the one being edited first; finished ones are kept by
//! key as textures.

use crate::render::{Renderer, STILL, View};
use crate::scene::{Scene, TerrainCache};
use crate::settings::Settings;
use eframe::egui;
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};

/// Pixels on a side.
pub const SIZE: usize = 112;
/// Samples per pixel.
const FRAMES: usize = 12;

#[derive(Default)]
struct Queue {
    /// what is being edited: only its newest version matters
    live: Option<(String, Value)>,
    list: VecDeque<(String, Value)>,
}

pub struct Previews {
    queue: Arc<(Mutex<Queue>, Condvar)>,
    /// finished: key, image, and whether it was the live one
    done: Receiver<(String, egui::ColorImage, bool)>,
    textures: HashMap<String, egui::TextureHandle>,
    pending: HashSet<String>,
    /// the live one's key, and its newest finished texture (maybe of an older key)
    live: Option<(String, egui::TextureHandle)>,
    live_key: String,
}

impl Previews {
    pub fn new(ctx: egui::Context) -> Previews {
        let queue = Arc::new((Mutex::new(Queue::default()), Condvar::new()));
        let (tx, done) = channel();
        let q = queue.clone();
        let _ = std::thread::Builder::new().name("previews".into()).spawn(move || run(q, tx, ctx));
        Previews { queue, done, textures: HashMap::new(), pending: HashSet::new(), live: None, live_key: String::new() }
    }

    /// Takes in what the thread finished.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((key, img, live)) = self.done.try_recv() {
            // kept by key when asked for by key (a live one may have been too)
            let keep = self.pending.remove(&key) || !live;
            if !live {
                let t = ctx.load_texture(format!("preview {key}"), img, egui::TextureOptions::LINEAR);
                self.textures.insert(key, t);
                continue;
            }
            if keep {
                let t = ctx.load_texture(format!("preview {key}"), img.clone(), egui::TextureOptions::LINEAR);
                self.textures.insert(key.clone(), t);
            }
            // the newest finished version of what is being edited, even when
            // the edit has moved on since (it shows while the next renders)
            match &mut self.live {
                Some((k, t)) => {
                    *k = key;
                    t.set(img, egui::TextureOptions::LINEAR);
                }
                None => self.live = Some((key, ctx.load_texture("live preview", img, egui::TextureOptions::LINEAR))),
            }
        }
    }

    /// The preview of the small scene `scene()` (a scene document), known by
    /// `key`; asked for when it is not there yet.
    pub fn get(&mut self, key: &str, scene: impl FnOnce() -> Value) -> Option<&egui::TextureHandle> {
        if let Some((k, t)) = &self.live
            && k == key
            && !self.textures.contains_key(key)
        {
            return Some(t);
        }
        if !self.textures.contains_key(key) && !self.pending.contains(key) {
            self.pending.insert(key.to_string());
            let (m, cv) = &*self.queue;
            m.lock().unwrap().list.push_back((key.to_string(), scene()));
            cv.notify_one();
        }
        self.textures.get(key)
    }

    /// The preview of what is being edited: the newest one finished (perhaps
    /// of an earlier version) while the current one renders.
    pub fn live(&mut self, key: &str, scene: impl FnOnce() -> Value) -> Option<&egui::TextureHandle> {
        if let Some(t) = self.textures.get(key) {
            return Some(t);
        }
        if self.live_key != key {
            self.live_key = key.to_string();
            let (m, cv) = &*self.queue;
            m.lock().unwrap().live = Some((key.to_string(), scene()));
            cv.notify_one();
        }
        self.live.as_ref().map(|(_, t)| t)
    }
}

fn run(queue: Arc<(Mutex<Queue>, Condvar)>, done: Sender<(String, egui::ColorImage, bool)>, ctx: egui::Context) {
    let Ok(pool) = rayon::ThreadPoolBuilder::new().num_threads(3).thread_name(|i| format!("preview {i}")).build() else { return };
    let mut renderer = Renderer::new();
    let mut cache = TerrainCache::default();
    let mut id = 1u64 << 40;
    loop {
        let (key, doc, live) = {
            let (m, cv) = &*queue;
            let mut q = m.lock().unwrap();
            loop {
                if let Some((k, d)) = q.live.take() {
                    break (k, d, true);
                }
                if let Some((k, d)) = q.list.pop_front() {
                    break (k, d, false);
                }
                q = cv.wait(q).unwrap();
            }
        };
        let Ok(scene) = Scene::from_value(&doc, &mut cache) else { continue };
        let cam = crate::start_camera(&scene);
        let s = crate::initial_settings(&scene, None).unwrap_or_default();
        let s = Settings { max_samples: FRAMES as u32, accumulate: true, fast_motion: false, vignette: false, ..s };
        id += 1;
        let pixels = pool.install(|| {
            for _ in 0..FRAMES - 1 {
                renderer.render(&scene, &cam, &s, SIZE, SIZE, id, &View::default(), STILL);
            }
            let f = renderer.render(&scene, &cam, &s, SIZE, SIZE, id, &View::default(), STILL);
            (0..f.h).flat_map(|row| f.out[row * f.pw..][..f.w].iter().map(|&p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, p as u8)).collect::<Vec<_>>()).collect::<Vec<_>>()
        });
        if done.send((key, egui::ColorImage::new([SIZE, SIZE], pixels), live)).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

/// A material on a sphere over a checkerboard, in the afternoon sun.
pub fn material_scene(def: &Value) -> Value {
    serde_json::json!({
        "camera": { "position": [0.0, 0.55, 3.3], "look_at": [0.0, -0.08, 0.0], "fov": 36 },
        "environment": { "preset": "golden", "sun_azimuth": -35, "sun_elevation": 40, "clouds": 0.2 },
        "render": { "bounces": 4 },
        "materials": { "m": def },
        "objects": [
            { "type": "sphere", "radius": 1.0, "material": "m" },
            { "type": "plane", "offset": -1.0, "material": { "color": [0.55, 0.55, 0.55], "color2": [0.22, 0.22, 0.22], "texture": "checker", "texture_scale": 1.5 } }
        ]
    })
}

/// A pattern on a sphere: a light and a dark colour, with a little relief
/// (the orbit pattern, which colours fractals, on a Mandelbulb).
pub fn pattern_scene(name: &str) -> Value {
    let orbit = name == "orbit";
    let mut v = material_scene(&serde_json::json!({
        "color": [0.86, 0.82, 0.74], "color2": [0.16, 0.22, 0.36], "texture": name, "texture_scale": if orbit { 2.0 } else { 2.5 }, "roughness": 0.45, "bump": if orbit { 0.0 } else { 0.35 }
    }));
    if orbit {
        v["objects"][0] = serde_json::json!({ "type": "mandelbulb", "scale": 0.82, "material": "m" });
    }
    // (facades on a tower block, its lit windows glowing)
    if name == "windows" {
        v["materials"]["m"] = serde_json::json!({
            "color": [0.3, 0.3, 0.32], "color2": [1.0, 0.82, 0.55], "emission2": [2.0, 1.5, 0.8], "texture": name, "texture_scale": 3.2, "roughness": 0.7
        });
        v["objects"][0] = serde_json::json!({ "type": "box", "size": [1.0, 1.7, 1.0], "position": [0.0, -0.15, 0.0], "rotation": [0, 35, 0], "material": "m" });
        v["environment"]["time_of_day"] = serde_json::json!(18.6);
    }
    v
}
