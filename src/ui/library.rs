//! The library windows. Materials: every library material and every scene
//! material on a sphere; a click gives one to the selected object (a library
//! material as a copy of its own, a scene material shared by name); a right
//! click edits its name and description, or deletes it. Modifiers: each one
//! applied to a sample block; a click wraps the selection in it.

use super::doc::{self, Selection};
use super::{App, previews, style};
use eframe::egui;
use serde_json::{Value, json};

/// Name and description being edited: of a library material, or of a scene
/// material (`scene`, whose name stays: objects refer to it).
pub struct LibEdit {
    pub scene: bool,
    pub name: String,
    pub new_name: String,
    pub description: String,
}

const TILE: f32 = 92.0;

/// A material without its description: what its preview depends on.
pub fn look(def: &Value) -> Value {
    let mut v = def.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("description");
    }
    v
}

pub fn preview_key(def: &Value) -> String {
    format!("material {}", look(def))
}

/// One material: its preview and name; highlighted on hover.
pub(super) fn tile(ui: &mut egui::Ui, tex: Option<&egui::TextureHandle>, name: &str, current: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(TILE, TILE + 20.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let p = ui.painter();
        let fill = if current {
            v.selection.bg_fill
        } else if resp.hovered() {
            v.widgets.hovered.weak_bg_fill
        } else {
            v.faint_bg_color
        };
        p.rect_filled(rect, egui::CornerRadius::same(6), fill);
        let img = egui::Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(TILE - 8.0, TILE - 8.0));
        match tex {
            Some(t) => {
                egui::Image::new(t).corner_radius(egui::CornerRadius::same(4)).paint_at(ui, img);
            }
            None => {
                p.rect_filled(img, egui::CornerRadius::same(4), v.extreme_bg_color);
                p.text(img.center(), egui::Align2::CENTER_CENTER, "…", egui::FontId::proportional(18.0), v.weak_text_color());
            }
        }
        let font = egui::TextStyle::Small.resolve(ui.style());
        let col = if current { v.strong_text_color() } else { v.text_color() };
        let g = p.layout(name.to_string(), font, col, TILE - 6.0);
        let text_at = egui::pos2(rect.center().x - g.size().x * 0.5, img.bottom() + 3.0);
        p.with_clip_rect(rect).galley(text_at, g, col);
    }
    resp
}

impl App {
    /// Gives the selected node `material` (a definition, or a scene name).
    fn give_material(&mut self, material: Value) {
        let Selection::Node { top, sub } = self.sel.clone() else {
            self.say("select an object first, then click a material");
            return;
        };
        self.doc.checkpoint(true);
        if let Some(n) = self.doc.node_mut(top, &sub) {
            // an emitter's nozzle keeps its colour: what it sends out takes the material
            if n.get("type").and_then(Value::as_str) == Some("emitter") {
                let emit = n.as_object_mut().map(|o| o.entry("emit").or_insert_with(|| serde_json::json!({ "type": "sphere", "radius": 0.12 })));
                if let Some(e) = emit {
                    e["material"] = material;
                }
            } else {
                n["material"] = material;
            }
        }
        self.doc.touch();
    }

    pub(super) fn library_window(&mut self, ctx: &egui::Context) {
        if !self.show_library {
            return;
        }
        let mut open = true;
        let selected = match &self.sel {
            Selection::Node { top, sub } => self.doc.node(*top, sub).map(doc::label),
            _ => None,
        };
        let current = match &self.sel {
            Selection::Node { top, sub } => self.doc.node(*top, sub).and_then(|n| n.get("material").cloned()),
            _ => None,
        };
        egui::Window::new("Material library")
            .open(&mut open)
            .collapsible(false)
            .resizable([false, true])
            .default_height(560.0)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
            // a set width: the window is laid out once to measure it, and the
            // tiles' rows would otherwise take the whole screen
            ui.set_width(5.0 * (TILE + 6.0));
            style::note(
                ui,
                match &selected {
                    Some(name) => format!("Click a material to give it to {name}. Right-click one to rename it, describe it or delete it."),
                    None => "Select an object, then click a material to give it. Right-click one to rename it, describe it or delete it.".to_string(),
                },
            );
            if let Some(e) = &self.library.error {
                ui.colored_label(egui::Color32::from_rgb(240, 120, 90), format!("The library file does not read, so the built-in materials show and nothing is saved until it is fixed: {e}"));
            }
            ui.horizontal(|ui| {
                ui.label("Find");
                ui.add(egui::TextEdit::singleline(&mut self.lib_filter).hint_text("name or description").desired_width(220.0));
            });
            let filter = self.lib_filter.to_lowercase();
            let shows = |name: &str, def: &Value| {
                filter.is_empty()
                    || name.to_lowercase().contains(&filter)
                    || def.get("description").and_then(Value::as_str).is_some_and(|d| d.to_lowercase().contains(&filter))
            };
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.6).show(ui, |ui| {
                // the library: copies
                ui.label(egui::RichText::new("Library").strong()).on_hover_text("library/materials.json: copied into an object when given, so each copy can change on its own.");
                let items = self.library.items.clone();
                let mut give = None;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                    for (name, def) in items.iter().filter(|(n, d)| shows(n, d)) {
                        let key = preview_key(def);
                        let tex = self.previews.get(&key, || previews::material_scene(&look(def))).cloned();
                        let is_current = current.as_ref().is_some_and(|c| look(c) == look(def));
                        let r = tile(ui, tex.as_ref(), name, is_current);
                        let pattern = def.get("texture").or_else(|| def.get("pattern")).and_then(Value::as_str).unwrap_or("none");
                        let desc = def.get("description").and_then(Value::as_str).unwrap_or("");
                        let r = r.on_hover_text(format!("{name}\n{desc}{}pattern: {pattern}", if desc.is_empty() { "" } else { "\n" }));
                        if r.clicked() {
                            give = Some(def.clone());
                        }
                        r.context_menu(|ui| {
                            if ui.add_enabled(selected.is_some(), egui::Button::new("Give to the selection")).clicked() {
                                give = Some(def.clone());
                                ui.close();
                            }
                            if ui.button("Rename and describe…").clicked() {
                                self.lib_edit = Some(LibEdit { scene: false, name: name.clone(), new_name: name.clone(), description: desc.to_string() });
                                ui.close();
                            }
                            ui.menu_button("Delete", |ui| {
                                if ui.button(format!("Delete '{name}' from the library")).on_hover_text("Objects that were given a copy keep theirs.").clicked() {
                                    let r = self.library.remove(name);
                                    self.say(match r {
                                        Ok(()) => format!("deleted '{name}' from the library"),
                                        Err(e) => e,
                                    });
                                    ui.close();
                                }
                            });
                        });
                    }
                });
                if let Some(def) = give.take() {
                    self.give_material(def);
                }

                // the scene's own: shared by name
                let scene: Vec<(String, Value)> = self.doc.v.get("materials").and_then(Value::as_object).map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()).unwrap_or_default();
                if !scene.is_empty() {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("In this scene").strong()).on_hover_text("The scene file's materials: shared by name, so editing one changes every object that uses it.");
                    let mut give_name = None;
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for (name, def) in scene.iter().filter(|(n, d)| shows(n, d)) {
                            let key = preview_key(def);
                            let tex = self.previews.get(&key, || previews::material_scene(&look(def))).cloned();
                            let is_current = current.as_ref().and_then(Value::as_str) == Some(name.as_str());
                            let desc = def.get("description").and_then(Value::as_str).unwrap_or("");
                            let r = tile(ui, tex.as_ref(), name, is_current).on_hover_text(format!("{name} (shared)\n{desc}"));
                            if r.clicked() {
                                give_name = Some(name.clone());
                            }
                            r.context_menu(|ui| {
                                if ui.add_enabled(selected.is_some(), egui::Button::new("Give to the selection")).clicked() {
                                    give_name = Some(name.clone());
                                    ui.close();
                                }
                                if ui.button("Describe…").clicked() {
                                    self.lib_edit = Some(LibEdit { scene: true, name: name.clone(), new_name: name.clone(), description: desc.to_string() });
                                    ui.close();
                                }
                                if ui.button("Copy to the library").on_hover_text("Keep it for every scene (under the same name).").clicked() {
                                    let to = self.library.free_name(name);
                                    let r = self.library.store(&to, def.clone());
                                    self.say(match r {
                                        Ok(()) if &to != name => format!("'{name}' copied to the library as '{to}' (it already has a '{name}')"),
                                        Ok(()) => format!("'{name}' copied to the library"),
                                        Err(e) => e,
                                    });
                                    ui.close();
                                }
                            });
                        }
                    });
                    if let Some(n) = give_name {
                        self.give_material(json!(n));
                    }
                }
            });

            // renaming and describing
            if let Some(e) = &mut self.lib_edit {
                ui.separator();
                let mut done = None;
                style::props(ui, "lib edit", |ui| {
                    style::row(ui, "name", "The material's name in the library.", |ui| {
                        ui.add_enabled(!e.scene, egui::TextEdit::singleline(&mut e.new_name).desired_width(f32::INFINITY))
                            .on_disabled_hover_text("A scene material keeps its name: objects refer to it by name.");
                    });
                    style::row(ui, "description", "What it is or is for; shown under its preview.", |ui| {
                        ui.add(egui::TextEdit::multiline(&mut e.description).desired_rows(3).desired_width(f32::INFINITY));
                    });
                });
                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new("Save").strong()).clicked() {
                        done = Some(true);
                    }
                    if ui.button("Cancel").clicked() {
                        done = Some(false);
                    }
                });
                match done {
                    Some(true) => {
                        let e = self.lib_edit.take().unwrap();
                        if e.scene {
                            self.doc.checkpoint(true);
                            if let Some(m) = self.doc.object_mut("materials").get_mut(&e.name).and_then(Value::as_object_mut) {
                                match e.description.trim() {
                                    "" => m.remove("description"),
                                    d => m.insert("description".into(), json!(d)),
                                };
                            }
                            self.doc.touch();
                        } else if let Err(err) = self.library.edit(&e.name, &e.new_name, &e.description) {
                            self.say(err);
                            self.lib_edit = Some(e);
                        }
                    }
                    Some(false) => self.lib_edit = None,
                    None => {}
                }
            }
        });
        if !open {
            self.show_library = false;
            self.lib_edit = None;
        }
    }
}

/// The modifier presets: name, what it does, the modifier (its child is the
/// selection, or the sample block in the icon).
pub const MODIFIERS: &[(&str, &str, &str)] = &[
    ("twist", "Twists it about its y axis, more the higher it goes (screws, drills, twisted columns).", r#"{"type":"twist","amount":1.6}"#),
    ("bend", "Bends it along x into an arc (tubes, banana shapes, arches).", r#"{"type":"bend","amount":0.9}"#),
    ("round", "Rounds every edge and swells it a little (soft, pillowy versions of hard shapes).", r#"{"type":"round","radius":0.08}"#),
    ("bumps", "Roughens the surface with noise: stone, bark, asteroids.", r#"{"type":"displace","amplitude":0.05,"frequency":4,"octaves":4}"#),
    ("ripples", "Rings spreading from its axis, like water after a stone.", r#"{"type":"relief","pattern":"ripples","frequency":5,"depth":0.025}"#),
    ("waves", "Soft wavy ridges, like wind on sand.", r#"{"type":"relief","pattern":"waves","frequency":4,"depth":0.03}"#),
    ("grooves", "Straight bands carved round it (stripes, carved in).", r#"{"type":"relief","pattern":"stripes","frequency":5,"depth":-0.015}"#),
    ("bricks", "Bricks raised out of the surface, mortar sunk between them.", r#"{"type":"relief","pattern":"bricks","frequency":4,"depth":-0.015}"#),
    ("hexagons", "A honeycomb of raised hexagonal tiles.", r#"{"type":"relief","pattern":"hexagons","frequency":4,"depth":-0.015}"#),
    ("scales", "Overlapping scales (fish, dragons, shingles).", r#"{"type":"relief","pattern":"scales","frequency":5,"depth":-0.02}"#),
    ("weave", "Over-and-under strands, like wicker.", r#"{"type":"relief","pattern":"weave","frequency":4,"depth":-0.015}"#),
    ("cells", "Stones or cobbles: cells raised between sunk gaps.", r#"{"type":"relief","pattern":"cells","frequency":4,"depth":-0.02}"#),
    ("cracks", "Cracks cut into the surface (dry mud, old paint).", r#"{"type":"relief","pattern":"cracks","frequency":3,"depth":-0.025}"#),
    ("grid", "A raised grid of ridges (tiles, waffle).", r#"{"type":"relief","pattern":"grid","frequency":4,"depth":0.012}"#),
    ("shell", "Hollows it into a thin shell (the icon cuts it open to show the inside).", r#"{"type":"shell","thickness":0.04}"#),
    ("slice", "Cuts it open with a plane, to see inside (a fractal's interior). Drag the offset to sweep the cut.", r#"{"type":"slice","normal":[0.6,0,0.8],"offset":0}"#),
    ("cross-section", "Keeps only a thin slab of it: a cross-section to sweep through.", r#"{"type":"slice","normal":[0,0,1],"offset":0,"thickness":0.25}"#),
];

/// A modifier's icon: the sample block (a rounded box) in it, in clay.
pub fn modifier_scene(modifier: &Value) -> Value {
    let mut m = modifier.clone();
    m["child"] = json!({"type": "box", "size": [0.8, 1.3, 0.8], "rounding": 0.06});
    let mut node = m;
    // a shell looks like its child from outside: cut a corner away
    if node.get("type").and_then(Value::as_str) == Some("shell") {
        node = json!({"type": "difference", "children": [node, {"type": "box", "size": [1, 1, 1], "position": [0.45, 0.45, 0.45]}]});
    }
    node["material"] = json!({"color": [0.78, 0.74, 0.68], "roughness": 0.55});
    node["rotation"] = json!([0, 30, 0]);
    json!({
        "camera": { "position": [0.0, 1.15, 3.8], "look_at": [0.0, 0.02, 0.0], "fov": 38 },
        "environment": { "preset": "golden", "sun_azimuth": -50, "sun_elevation": 40, "clouds": 0.2 },
        "render": { "bounces": 2 },
        "objects": [
            node,
            { "type": "plane", "offset": -0.66, "material": { "color": [0.55, 0.55, 0.55], "color2": [0.25, 0.25, 0.25], "texture": "checker", "texture_scale": 1.5 } }
        ]
    })
}

impl App {
    pub(super) fn modifier_window(&mut self, ctx: &egui::Context) {
        if !self.show_modifiers {
            return;
        }
        let mut open = true;
        let target = match &self.sel {
            Selection::Node { top, sub } => self.doc.node(*top, sub).map(|n| (doc::label(n), n.get("type").and_then(Value::as_str).unwrap_or("").to_string())),
            _ => None,
        };
        egui::Window::new("Modifiers")
            .open(&mut open)
            .collapsible(false)
            .resizable([false, true])
            .default_height(520.0)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .show(ctx, |ui| {
                ui.set_width(5.0 * (TILE + 6.0));
                style::note(
                    ui,
                    match &target {
                        Some((name, _)) => format!("Click a modifier to apply it to {name} (it wraps it: the modifier's settings then show in the inspector, and Ctrl+Z takes it off)."),
                        None => "Select an object (or a part of one), then click a modifier to apply it.".to_string(),
                    },
                );
                let mut apply = None;
                egui::ScrollArea::vertical().max_height(ctx.content_rect().height() * 0.6).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        for (name, desc, template) in MODIFIERS {
                            let Ok(m) = serde_json::from_str::<Value>(template) else { continue };
                            let tex = self.previews.get(&format!("modifier {template}"), || modifier_scene(&m)).cloned();
                            let ty = m.get("type").and_then(Value::as_str).unwrap_or("");
                            if tile(ui, tex.as_ref(), name, false).on_hover_text(format!("{name} ({ty})\n{desc}")).clicked() {
                                apply = Some(m);
                            }
                        }
                    });
                });
                if let Some(m) = apply {
                    match (&self.sel, &target) {
                        (Selection::Node { top, sub }, Some((_, ty))) => {
                            if ["nurbs", "revolve", "terrain", "water", "cloud", "emitter", "group"].contains(&ty.as_str()) {
                                self.say(format!("a {ty} cannot go inside a modifier (it must stay a top-level object)"));
                            } else {
                                let (top, sub) = (*top, sub.clone());
                                if let Some(s) = self.doc.wrap_with(top, &sub, m) {
                                    self.sel = s;
                                }
                            }
                        }
                        _ => self.say("select an object first, then click a modifier"),
                    }
                }
            });
        if !open {
            self.show_modifiers = false;
        }
    }
}
