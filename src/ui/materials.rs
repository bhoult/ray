//! Materials in the editor: the library (library/materials.json, or the
//! copy built into the binary), assigning a library material to a node (a
//! copy the node owns, so every assignment can be changed on its own), the
//! material editor, and random procedural materials.

use super::previews::Previews;
use super::{schema, style};
use crate::scene::json::{ColorDef, MaterialDef};
use crate::scene::tex;
use eframe::egui;
use serde_json::{Map, Value};
use std::path::PathBuf;

const BUILT_IN: &str = include_str!("../../library/materials.json");

pub struct Library {
    pub path: PathBuf,
    pub items: Vec<(String, Value)>,
    pub error: Option<String>,
}

impl Library {
    /// `library/materials.json` next to the working directory, else the
    /// built-in copy.
    pub fn load() -> Library {
        let path = PathBuf::from("library/materials.json");
        let (text, error) = match std::fs::read_to_string(&path) {
            Ok(t) => (t, None),
            Err(_) => (BUILT_IN.to_string(), None),
        };
        let mut lib = Library { path, items: Vec::new(), error };
        match serde_json::from_str::<Map<String, Value>>(&text) {
            Ok(m) => lib.items = m.into_iter().collect(),
            Err(e) => {
                lib.error = Some(format!("{}: {e}", lib.path.display()));
                if let Ok(m) = serde_json::from_str::<Map<String, Value>>(BUILT_IN) {
                    lib.items = m.into_iter().collect();
                }
            }
        }
        lib
    }

    /// Adds or replaces `name` and writes the library file.
    pub fn store(&mut self, name: &str, v: Value) -> Result<(), String> {
        match self.items.iter_mut().find(|i| i.0 == name) {
            Some(i) => i.1 = v,
            None => self.items.push((name.to_string(), v)),
        }
        self.write()
    }

    /// `base`, or `base 2`, `base 3`... whichever the library does not have yet.
    pub fn free_name(&self, base: &str) -> String {
        free_name(base, |n| self.get(n).is_some())
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.items.iter().find(|i| i.0 == name).map(|i| &i.1)
    }

    /// Takes `name` out of the library file.
    pub fn remove(&mut self, name: &str) -> Result<(), String> {
        self.items.retain(|i| i.0 != name);
        self.write()
    }

    /// Renames `old` to `new` (in place in the list) and sets its
    /// description (none when empty).
    pub fn edit(&mut self, old: &str, new: &str, description: &str) -> Result<(), String> {
        let new = new.trim();
        if new.is_empty() {
            return Err("a material needs a name".into());
        }
        if new != old && self.get(new).is_some() {
            return Err(format!("the library already has a material named '{new}'"));
        }
        let Some(item) = self.items.iter_mut().find(|i| i.0 == old) else { return Err(format!("'{old}' is not in the library")) };
        item.0 = new.to_string();
        if let Some(o) = item.1.as_object_mut() {
            match description.trim() {
                "" => o.remove("description"),
                d => o.insert("description".into(), Value::from(d)),
            };
        }
        self.write()
    }

    /// Writes the library file (the order of the list is kept); not over a
    /// file that did not read (it would lose what is in it).
    fn write(&self) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!("not saved: the library file does not read ({e}); fix it by hand first"));
        }
        let map: Map<String, Value> = self.items.iter().cloned().collect();
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let text = serde_json::to_string_pretty(&Value::Object(map)).map_err(|e| e.to_string())?;
        std::fs::write(&self.path, text + "\n").map_err(|e| format!("{}: {e}", self.path.display()))
    }
}

/// `base`, or `base 2`, `base 3`... the first that `taken` says is free.
pub fn free_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|k| format!("{base} {k}")).find(|n| !taken(n)).expect("a free name")
}

fn rgb(c: &ColorDef) -> [f32; 3] {
    c.rgb()
}

/// An HDR colour: a picker for the hue, a slider for the strength.
fn hdr(ui: &mut egui::Ui, name: &str, c: &mut [f32; 3], tip: &str) -> bool {
    let strength = c.iter().cloned().fold(0.0f32, f32::max);
    let mut hue = if strength > 0.0 { c.map(|v| v / strength) } else { [1.0, 1.0, 1.0] };
    let mut s = strength;
    let changed = style::row(ui, name, tip, |ui| {
        ui.horizontal(|ui| {
            let a = ui.color_edit_button_rgb(&mut hue).on_hover_text(tip).changed();
            ui.spacing_mut().slider_width -= 46.0;
            let b = ui.add(egui::Slider::new(&mut s, 0.0..=1000.0).logarithmic(true)).on_hover_text(tip).changed();
            a || b
        })
        .inner
    });
    if changed {
        *c = hue.map(|v| v * s);
    }
    changed
}

fn slider(ui: &mut egui::Ui, v: &mut f32, range: std::ops::RangeInclusive<f32>, name: &str, key: &str, log: bool) -> bool {
    style::slider(ui, name, schema::material_desc(key), v, range, log)
}

/// A group of rows under a small heading.
fn group(ui: &mut egui::Ui, id: &str, heading: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(heading).small().strong().color(ui.visuals().weak_text_color()));
    style::props(ui, id, body);
}

/// A pattern picker: the current one's name, opening every pattern on a
/// sphere to choose from. Returns the one picked ("none" for plain).
fn pattern_picker(ui: &mut egui::Ui, id: &str, current: &str, previews: &mut Previews) -> Option<String> {
    let mut picked = None;
    ui.menu_button(format!("{current}  ▾"), |ui| {
        ui.set_width(5.0 * 98.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
            for (name, _, desc) in tex::PATTERNS.iter().filter(|p| p.0 != "water") {
                let tex = previews.get(&format!("pattern {name}"), || super::previews::pattern_scene(name)).cloned();
                if super::library::tile(ui, tex.as_ref(), name, *name == current).on_hover_text(*desc).clicked() {
                    picked = Some(name.to_string());
                    ui.close();
                }
            }
        });
    })
    .response
    .on_hover_text(format!("{} ({id})", schema::material_desc("texture")));
    picked
}

/// Edits a material; returns whether it changed.
pub fn editor(ui: &mut egui::Ui, m: &mut MaterialDef, previews: &mut Previews) -> bool {
    let mut changed = false;
    let pattern = m.texture.clone().unwrap_or_else(|| "none".into());
    style::props(ui, "mat about", |ui| {
        let mut d = m.description.clone().unwrap_or_default();
        let tip = schema::material_desc("description");
        let r = style::row(ui, "description", tip, |ui| {
            ui.add(egui::TextEdit::multiline(&mut d).desired_rows(2).desired_width(f32::INFINITY).hint_text("what it is, or what it is for")).on_hover_text(tip)
        });
        if r.changed() {
            m.description = (!d.trim().is_empty()).then_some(d);
            changed = true;
        }
    });
    group(ui, "mat surface", "Surface", |ui| {
        style::row(ui, "pattern", schema::material_desc("texture"), |ui| {
            if let Some(name) = pattern_picker(ui, "the colour pattern", &pattern, previews) {
                m.texture = (name != "none").then_some(name);
                changed = true;
            }
        });
        let mut c = rgb(&m.color);
        if style::colour(ui, "colour", schema::material_desc("color"), &mut c) {
            m.color = ColorDef::Rgb(c);
            changed = true;
        }
        changed |= slider(ui, &mut m.roughness, 0.0..=1.0, "roughness", "roughness", false);
        changed |= slider(ui, &mut m.metallic, 0.0..=1.0, "metallic", "metallic", false);
        let mut e = rgb(&m.emission);
        if hdr(ui, "emission", &mut e, schema::material_desc("emission")) {
            m.emission = ColorDef::Rgb(e);
            changed = true;
        }
    });
    if m.texture.is_some() {
        group(ui, "mat pattern", "Where the pattern shows (blended from the values above)", |ui| {
            let base = rgb(&m.color);
            let mut c2 = m.color2.map(|c| c.rgb()).unwrap_or(base.map(|v| v * 0.25));
            if style::colour(ui, "colour 2", schema::material_desc("color2"), &mut c2) {
                m.color2 = Some(ColorDef::Rgb(c2));
                changed = true;
            }
            let mut r2 = m.roughness2.unwrap_or(m.roughness);
            if slider(ui, &mut r2, 0.0..=1.0, "roughness 2", "roughness2", false) {
                m.roughness2 = Some(r2);
                changed = true;
            }
            let mut m2 = m.metallic2.unwrap_or(m.metallic);
            if slider(ui, &mut m2, 0.0..=1.0, "metallic 2", "metallic2", false) {
                m.metallic2 = Some(m2);
                changed = true;
            }
            let mut e2 = m.emission2.map(|c| c.rgb()).unwrap_or(rgb(&m.emission));
            if hdr(ui, "emission 2", &mut e2, schema::material_desc("emission2")) {
                m.emission2 = Some(ColorDef::Rgb(e2));
                changed = true;
            }
            changed |= slider(ui, &mut m.texture_scale, 0.05..=100.0, "pattern scale", "texture_scale", true);
            changed |= slider(ui, &mut m.warp, 0.0..=3.0, "warp", "warp", false);
            changed |= slider(ui, &mut m.sharpness, 0.1..=10.0, "sharpness", "sharpness", true);
            let mut w = m.space.as_deref() == Some("world");
            if style::check(ui, "fixed in world", schema::material_desc("space"), &mut w) {
                m.space = w.then(|| "world".to_string());
                changed = true;
            }
        });
    }
    group(ui, "mat relief", "Relief", |ui| {
        changed |= slider(ui, &mut m.bump, -2.0..=2.0, "bump", "bump", false);
        if m.bump != 0.0 {
            let own = m.bump_pattern.clone();
            let shown = own.clone().unwrap_or_else(|| format!("{} (the colour's)", m.texture.as_deref().unwrap_or("none")));
            style::row(ui, "relief pattern", schema::material_desc("bump_pattern"), |ui| {
                ui.horizontal(|ui| {
                    if let Some(name) = pattern_picker(ui, "the relief's pattern", &shown, previews) {
                        m.bump_pattern = Some(name);
                        changed = true;
                    }
                    if own.is_some() && ui.small_button("colour's").on_hover_text("Follow the colour pattern again.").clicked() {
                        m.bump_pattern = None;
                        changed = true;
                    }
                });
            });
            let first = m.texture_scale;
            changed |= optional(ui, &mut m.bump_scale, first, 0.05..=100.0, "relief scale", "bump_scale");
        }
    });
    group(ui, "mat light", "Light through it", |ui| {
        changed |= slider(ui, &mut m.transmission, 0.0..=1.0, "transmission", "transmission", false);
        if m.transmission > 0.0 {
            changed |= slider(ui, &mut m.ior, 1.0..=2.5, "refraction index", "ior", false);
            changed |= slider(ui, &mut m.absorption, 0.0..=10.0, "absorption", "absorption", false);
        }
        changed |= optional(ui, &mut m.translucency, 0.6, 0.0..=1.0, "translucency", "translucency");
    });
    group(ui, "mat physical", "Physical", |ui| {
        changed |= optional(ui, &mut m.density, 1000.0, 10.0..=20000.0, "density (kg/m³)", "density");
        let glows = rgb(&m.emission).iter().chain(m.emission2.map(|c| c.rgb()).iter().flatten()).any(|&v| v > 0.0);
        if glows {
            changed |= optional(ui, &mut m.light_range, 10.0, 0.5..=200.0, "light range (m)", "light_range");
        }
    });
    changed
}

/// A value that can be left automatic: a checkbox, and a slider while set.
fn optional(ui: &mut egui::Ui, v: &mut Option<f32>, first: f32, range: std::ops::RangeInclusive<f32>, name: &str, key: &str) -> bool {
    let tip = schema::material_desc(key);
    style::row(ui, name, tip, |ui| {
        ui.horizontal(|ui| {
            let mut set = v.is_some();
            let mut changed = ui.checkbox(&mut set, "").on_hover_text(tip).changed();
            if changed {
                *v = set.then_some(first);
            }
            ui.spacing_mut().slider_width -= 26.0;
            match v {
                Some(x) => changed |= ui.add(egui::Slider::new(x, range).logarithmic(true)).on_hover_text(tip).changed(),
                None => {
                    ui.weak("automatic").on_hover_text(tip);
                }
            }
            changed
        })
        .inner
    })
}

/// A small xorshift generator for the randomiser.
pub struct Rand(u64);

impl Rand {
    pub fn new(seed: u64) -> Rand {
        Rand(seed | 1)
    }
    pub fn f(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    pub fn pick<'a, T>(&mut self, v: &'a [T]) -> &'a T {
        &v[((self.f() * v.len() as f32) as usize).min(v.len() - 1)]
    }
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    // to linear, roughly (gamma 2)
    [(r + m).powi(2), (g + m).powi(2), (b + m).powi(2)]
}

/// A random procedural material: a pattern, a palette, finish and relief.
pub fn random(seed: u64) -> MaterialDef {
    let mut r = Rand::new(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    let patterns = ["noise", "marble", "wood", "cells", "cracks", "spots", "stripes", "bricks", "checker", "grid", "noise"];
    let pattern = *r.pick(&patterns);
    let h = r.f();
    let h2 = h + r.pick(&[0.0, 0.05, 0.5, 0.33, -0.08]);
    let c1 = hsv(h, r.range(0.2, 0.85), r.range(0.45, 0.95));
    let c2 = hsv(h2, r.range(0.2, 0.9), r.range(0.1, 0.7));
    let metal = r.f() < 0.25;
    let lava = pattern == "cracks" && r.f() < 0.6;
    let glow2 = if lava { Some(ColorDef::Rgb(hsv(h, 0.9, 1.0).map(|v| v * r.range(4.0, 14.0)))) } else { None };
    MaterialDef {
        color: ColorDef::Rgb(c1),
        color2: Some(ColorDef::Rgb(c2)),
        roughness: r.range(0.05, 0.9),
        roughness2: Some(r.range(0.2, 1.0)),
        metallic: if metal { 1.0 } else { 0.0 },
        metallic2: Some(if metal && r.f() < 0.5 { 1.0 } else { 0.0 }),
        emission2: glow2,
        texture: Some(pattern.to_string()),
        texture_scale: r.range(0.6, 5.0),
        warp: r.range(0.0, 1.6),
        sharpness: r.range(0.6, 3.0),
        bump: if r.f() < 0.6 { r.range(-0.6, 0.8) } else { 0.0 },
        ..MaterialDef::default()
    }
}
