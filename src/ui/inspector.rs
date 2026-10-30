//! The scene panel: the outliner (objects with what they cost, the selected
//! object's tree, lights, environment), the inspector of the selection (every
//! parameter, with its description on hover) and the material section.

use super::doc::{self, Selection};
use super::{App, materials, schema, style};
use crate::scene::json::MaterialDef;
use crate::scene::{sun_angles, sun_from_angles};
use eframe::egui;
use serde_json::{Value, json};
use std::ops::RangeInclusive;

/// Node fields the inspector shows in their own sections.
const OWN_SECTION: &[&str] = &[
    "type", "name", "position", "rotation", "scale", "material", "disabled", "shadow", "step", "bound", "friction", "bounce", "locked", "mass", "gravity",
    "child", "children",
];

/// Optional fields that can go back to automatic.
const OPTIONAL: &[&str] = &["cap", "snow_line", "sand_line", "count"];

/// Integer fields (everything else numeric is a float, however the file
/// happens to write it: `"radius": 1` must still edit as 1.2).
const INTEGER: &[&str] = &["iterations", "octaves", "seed", "resolution", "count", "detail", "degree", "max"];

/// Ranged fields whose slider is logarithmic (spanning orders of magnitude).
const LOG: &[&str] = &["resolution", "softness"];


/// The inspector's node with its defaults filled in, for which object and
/// part, from which node.
pub(super) struct DefaultsCache {
    top: usize,
    sub: Vec<usize>,
    node: Value,
    full: Value,
}

/// An object's mass from its size or material, from which node and materials.
pub(super) struct MassCache {
    top: usize,
    node: Value,
    materials: Option<Value>,
    mass: f32,
    dense: bool,
}

/// The same node but perhaps for its position and rotation (compared in
/// place: writing both out to compare them cost more than the work kept).
fn same_but_pose(a: &Value, b: &Value) -> bool {
    let pose = |k: &str| k == "position" || k == "rotation";
    match (a.as_object(), b.as_object()) {
        (Some(a), Some(b)) => {
            a.iter().filter(|(k, _)| !pose(k)).count() == b.iter().filter(|(k, _)| !pose(k)).count()
                && a.iter().all(|(k, v)| pose(k) || b.get(k) == Some(v))
        }
        _ => a == b,
    }
}

fn round(x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    let p = 10f64.powi(4 - x.abs().log10().floor() as i32);
    (x * p).round() / p
}

fn float_drag(ui: &mut egui::Ui, x: &mut f64, key: &str, tip: &str) -> bool {
    let (lo, hi) = schema::field_range(key);
    let speed = if key == "rotation" { 0.5 } else { (x.abs() * 0.01).max(0.001) };
    let r = ui.add(egui::DragValue::new(x).speed(speed).range(lo..=hi).max_decimals(4)).on_hover_text(tip);
    if r.changed() {
        *x = round(*x);
    }
    r.changed()
}

/// A NURBS node's own knots after its points or degree changed: when they no
/// longer fit (points + degree + 1 of them), they go, for even spacing, and
/// the message says so. (Left in, they kept the object from compiling.)
fn fit_knots(n: &mut Value) -> Option<String> {
    let len = |v: Option<&Value>| v.and_then(Value::as_array).map(Vec::len);
    let deg = |v: Option<&Value>, i: Option<usize>, d: u64| {
        let v = match i {
            Some(i) => v.and_then(|v| v.get(i)),
            None => v,
        };
        v.and_then(Value::as_f64).map_or(d, |x| x.round().max(0.0) as u64) as usize
    };
    let o = n.as_object_mut()?;
    let mut out = Vec::new();
    match o.get("type").and_then(Value::as_str) {
        Some("revolve") => {
            let pts = len(o.get("profile")).unwrap_or(crate::scene::json::vase().len());
            let q = deg(o.get("degree"), None, 3);
            if len(o.get("knots")).is_some_and(|k| k != pts + q + 1) {
                o.remove("knots");
                out.push(format!("knots (now {pts} profile points of degree {q})"));
            }
        }
        Some("nurbs") => {
            let rows = o.get("points").and_then(Value::as_array);
            let (nu, nv) = match rows {
                Some(r) => (r.len(), r.first().and_then(Value::as_array).map_or(0, Vec::len)),
                None => (5, 5),
            };
            let (p, q) = (deg(o.get("degree"), Some(0), 3), deg(o.get("degree"), Some(1), 3));
            for (key, n, d) in [("knots_u", nu, p), ("knots_v", nv, q)] {
                if len(o.get(key)).is_some_and(|k| k != n + d + 1) {
                    o.remove(key);
                    out.push(format!("{key} (now {n} points of degree {d})"));
                }
            }
        }
        _ => {}
    }
    (!out.is_empty()).then(|| format!("{} no longer fitted: back to even spacing", out.join(" and ")))
}

/// Whether a field has a range of its own (a slider edits it), rather than
/// an open-ended size or position (dragging its box does).
fn bounded(key: &str) -> bool {
    let (lo, hi) = schema::field_range(key);
    lo > -1e8 && hi < 1e5
}

/// Edits one field value of a node of type `ty`; returns whether it changed.
fn edit_value(ui: &mut egui::Ui, ty: &str, key: &str, v: &mut Value) -> bool {
    let tip = schema::field_desc(ty, key);
    let (lo, hi) = schema::field_range(key);
    let optional = OPTIONAL.contains(&key);
    let mut clear = false;
    let integer = INTEGER.contains(&key);
    let changed = match v {
        Value::Number(n) if integer => {
            let mut x = n.as_f64().unwrap_or(0.0).round() as i64;
            let c = if bounded(key) {
                ui.add(egui::Slider::new(&mut x, lo as i64..=hi as i64).logarithmic(LOG.contains(&key))).on_hover_text(tip).changed()
            } else {
                // (every integer field is a count or a seed: never negative)
                ui.add(egui::DragValue::new(&mut x).speed(0.1).range(lo.max(0.0) as i64..=hi as i64)).on_hover_text(tip).changed()
            };
            if c {
                *v = json!(x);
            }
            c
        }
        Value::Number(n) if bounded(key) => {
            let mut x = n.as_f64().unwrap_or(0.0);
            let c = ui.add(egui::Slider::new(&mut x, lo..=hi).logarithmic(LOG.contains(&key))).on_hover_text(tip).changed();
            if c {
                *v = json!(round(x));
            }
            c
        }
        Value::Number(n) => {
            let mut x = n.as_f64().unwrap_or(0.0);
            let c = ui.horizontal(|ui| {
                let c = float_drag(ui, &mut x, key, tip);
                if optional && ui.small_button("auto").on_hover_text("Go back to the automatic value.").clicked() {
                    clear = true;
                }
                c
            });
            if c.inner {
                *v = json!(x);
            }
            c.inner
        }
        Value::Bool(b) => ui.checkbox(b, "").on_hover_text(tip).changed(),
        Value::String(s) if key == "noise" => {
            let mut c = false;
            egui::ComboBox::from_id_salt(key)
                .selected_text(s.as_str())
                .show_ui(ui, |ui| {
                    for opt in ["eroded", "perlin", "ridged", "billow"] {
                        c |= ui.selectable_value(s, opt.to_string(), opt).changed();
                    }
                })
                .response
                .on_hover_text(tip);
            c
        }
        Value::String(s) if key == "pattern" => {
            let mut c = false;
            egui::ComboBox::from_id_salt(("pattern", ty))
                .selected_text(s.as_str())
                .show_ui(ui, |ui| {
                    for (name, _, desc) in crate::scene::tex::PATTERNS.iter().filter(|p| !["none", "terrain", "water"].contains(&p.0)) {
                        c |= ui.selectable_value(s, name.to_string(), *name).on_hover_text(*desc).changed();
                    }
                })
                .response
                .on_hover_text(tip);
            c
        }
        Value::String(s) => ui.text_edit_singleline(s).on_hover_text(tip).changed(),
        Value::Null => {
            let mut c = false;
            ui.horizontal(|ui| {
                // a repeat without a count goes on forever
                let (state, set, set_tip) = if key == "count" {
                    ("infinite", "limit", "Repeat only so many copies on each side.")
                } else {
                    ("auto", "set", "Override the automatic value.")
                };
                ui.label(state).on_hover_text(tip);
                if ui.small_button(set).on_hover_text(set_tip).clicked() {
                    *v = if key == "count" { json!([2, 0, 2]) } else { json!(0.0) };
                    c = true;
                }
            });
            c
        }
        Value::Array(a) if a.iter().all(Value::is_number) && (2..=4).contains(&a.len()) => {
            let colour = matches!(key, "rock" | "grass" | "snow" | "sand" | "color") && a.len() == 3;
            ui.horizontal(|ui| {
                let mut c = false;
                if colour {
                    let mut rgb = [0.0f32; 3];
                    for (i, x) in a.iter().enumerate() {
                        rgb[i] = x.as_f64().unwrap_or(0.0) as f32;
                    }
                    if ui.color_edit_button_rgb(&mut rgb).on_hover_text(tip).changed() {
                        for (i, x) in a.iter_mut().enumerate() {
                            *x = json!(round(rgb[i] as f64));
                        }
                        c = true;
                    }
                }
                for x in a.iter_mut() {
                    if integer {
                        let mut n = x.as_f64().unwrap_or(0.0).round() as i64;
                        if ui.add(egui::DragValue::new(&mut n).speed(0.1).range(lo.max(0.0) as i64..=hi.min(1000.0) as i64)).on_hover_text(tip).changed() {
                            *x = json!(n);
                            c = true;
                        }
                    } else {
                        let mut f = x.as_f64().unwrap_or(0.0);
                        if float_drag(ui, &mut f, key, tip) {
                            *x = json!(f);
                            c = true;
                        }
                    }
                }
                let (button, button_tip) = if key == "count" {
                    ("infinite", "Repeat forever along the repeated axes (no count).")
                } else {
                    ("auto", "Go back to the automatic value.")
                };
                if optional && ui.small_button(button).on_hover_text(button_tip).clicked() {
                    clear = true;
                }
                c
            })
            .inner
        }
        Value::Array(a) if key == "balls" => balls(ui, a, tip),
        Value::Array(a) if key == "profile" => profile(ui, a, tip),
        Value::Array(a) if key == "points" => {
            let cols = a.first().and_then(Value::as_array).map_or(0, Vec::len);
            ui.label(format!("{} x {cols} control points", a.len())).on_hover_text(tip);
            false
        }
        Value::Array(a) => {
            ui.label(format!("{} items", a.len())).on_hover_text(tip);
            false
        }
        Value::Object(_) => {
            ui.label("(object)").on_hover_text(tip);
            false
        }
    };
    if clear {
        *v = Value::Null;
        return true;
    }
    changed
}

/// The metaballs' list: centre and radius per ball, add and remove.
fn balls(ui: &mut egui::Ui, a: &mut Vec<Value>, tip: &str) -> bool {
    let mut changed = false;
    let mut remove = None;
    ui.vertical(|ui| {
        for (i, b) in a.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{i}")).on_hover_text(tip);
                if let Some(Value::Array(c)) = b.get_mut("center") {
                    for x in c.iter_mut() {
                        let mut f = x.as_f64().unwrap_or(0.0);
                        if float_drag(ui, &mut f, "", "Centre of the ball (x, y, z), in the node's coordinates.") {
                            *x = json!(f);
                            changed = true;
                        }
                    }
                }
                let mut r = b.get("radius").and_then(Value::as_f64).unwrap_or(0.5);
                if float_drag(ui, &mut r, "radius", "Radius of the ball (m).") {
                    b["radius"] = json!(r);
                    changed = true;
                }
                if ui.small_button("−").on_hover_text("Remove this ball.").clicked() {
                    remove = Some(i);
                }
            });
        }
        if ui.small_button("+ ball").on_hover_text("Add a ball next to the last one.").clicked() {
            let mut nb = a.last().cloned().unwrap_or(json!({"center": [0, 0.5, 0], "radius": 0.3}));
            if let Some(c) = nb.get("center").and_then(doc::as_vec3) {
                nb["center"] = doc::vec3([c[0] + 0.3, c[1], c[2]]);
            }
            a.push(nb);
            changed = true;
        }
    });
    if let Some(i) = remove
        && a.len() > 1
    {
        a.remove(i);
        changed = true;
    }
    changed
}

/// A revolve's profile: radius, height (and weight) per point, add and remove.
fn profile(ui: &mut egui::Ui, a: &mut Vec<Value>, tip: &str) -> bool {
    let mut changed = false;
    let mut remove = None;
    ui.vertical(|ui| {
        for (i, pt) in a.iter_mut().enumerate() {
            let Some(c) = pt.as_array_mut() else { continue };
            ui.horizontal(|ui| {
                ui.label(format!("{i}")).on_hover_text(tip);
                for (k, x) in c.iter_mut().enumerate() {
                    let what = ["Radius: the distance from the axis (m).", "Height (m).", "Weight: how hard this point pulls the curve (1 = normal)."];
                    let mut f = x.as_f64().unwrap_or(0.0);
                    if float_drag(ui, &mut f, "", what[k.min(2)]) {
                        *x = json!(f);
                        changed = true;
                    }
                }
                if ui.small_button("−").on_hover_text("Remove this point.").clicked() {
                    remove = Some(i);
                }
            });
        }
        if ui.small_button("+ point").on_hover_text("Add a point above the last one (a profile with its own knots needs them updated too).").clicked() {
            let mut np = a.last().cloned().unwrap_or(json!([0.3, 0.0]));
            if let Some(y) = np.get(1).and_then(Value::as_f64) {
                np[1] = json!(round(y + 0.1));
            }
            a.push(np);
            changed = true;
        }
    });
    if let Some(i) = remove
        && a.len() > 2
    {
        a.remove(i);
        changed = true;
    }
    changed
}

/// A row of the object list: the node at `path` in top-level object `top`
/// (an empty path: the object), how deep, its name and type (the type alone
/// as the name when it has none), whether it is hidden, has parts and shows
/// them, and can be hidden by itself.
struct OutlineRow {
    top: usize,
    path: Vec<usize>,
    depth: usize,
    name: String,
    ty: String,
    hidden: bool,
    kids: bool,
    open: bool,
    can_hide: bool,
}

/// The rows of the object list: each object, and under the open ones their
/// parts and members, depth first.
fn outline_rows(objects: &[Value], is_open: &dyn Fn(usize, &[usize]) -> bool) -> Vec<OutlineRow> {
    fn walk(n: &Value, top: usize, path: Vec<usize>, can_hide: bool, is_open: &dyn Fn(usize, &[usize]) -> bool, out: &mut Vec<OutlineRow>) {
        let ty = n.get("type").and_then(Value::as_str).unwrap_or("?").to_string();
        let (name, ty) = match n.get("name").and_then(Value::as_str) {
            Some(name) => (name.to_string(), ty),
            None => (ty, String::new()),
        };
        let kids = doc::children(n);
        let open = !kids.is_empty() && is_open(top, &path);
        let depth = path.len();
        let hidden = n.get("disabled").and_then(Value::as_bool).unwrap_or(false);
        // (a list of them: a group's or a boolean's; a modifier has one child)
        let many = n.get("children").is_some();
        out.push(OutlineRow { top, path: path.clone(), depth, name, ty, hidden, kids: !kids.is_empty(), open, can_hide });
        if open {
            for (k, c) in kids.into_iter().enumerate() {
                let mut p = path.clone();
                p.push(k);
                walk(c, top, p, many, is_open, out);
            }
        }
    }
    let mut out = Vec::new();
    for (i, n) in objects.iter().enumerate() {
        walk(n, i, Vec::new(), true, is_open, &mut out);
    }
    out
}

/// How many nodes in `n`'s tree use the scene material `name`.
fn uses(n: &Value, name: &str) -> usize {
    let me = (n.get("material").and_then(Value::as_str) == Some(name)) as usize;
    me + doc::children(n).into_iter().map(|c| uses(c, name)).sum::<usize>()
}

impl App {
    pub(super) fn scene_panel(&mut self, ui: &mut egui::Ui) {
        self.outliner(ui);
        ui.separator();
        match self.sel.clone() {
            Selection::Node { top, sub } => self.node_inspector(ui, top, sub),
            Selection::None => {
                ui.label(
                    egui::RichText::new("Click an object in the view or in the list to see and edit its parameters.")
                        .color(ui.visuals().weak_text_color()),
                );
            }
        }
    }

    pub(super) fn add_menu(&mut self, ui: &mut egui::Ui) {
        for cat in schema::CATEGORIES {
            ui.menu_button(cat, |ui| {
                for s in schema::SHAPES.iter().filter(|s| s.category == cat) {
                    if ui.button(s.ty).on_hover_text(s.desc).clicked() {
                        self.add_object(s.ty);
                        ui.close();
                    }
                }
                if cat == "Landscape"
                    && ui
                        .button("checkerboard floor")
                        .on_hover_text("Flat ground with a checkerboard: a terrain of the flat kind, 2 km across, at height 0 under the camera.")
                        .clicked()
                {
                    self.add_floor();
                    ui.close();
                }
            });
        }
        if ui.button("light").on_hover_text("A light in front of the camera: a small glowing sphere that floats (drag it anywhere; set its emission, or its radius for softer shadows).").clicked() {
            self.add_light();
            ui.close();
        }
    }

    fn outliner(&mut self, ui: &mut egui::Ui) {
        let mut click = None;
        style::title(ui, "Scene", |ui| {
            // (right to left)
            if ui
                .button("+ Light")
                .on_hover_text("Add a light in front of the camera: a small glowing sphere that floats. Any object whose material has emission is a light; this is just a ready-made one.")
                .clicked()
            {
                self.add_light();
            }
            ui.menu_button("+ Add", |ui| self.add_menu(ui))
                .response
                .on_hover_text("Add a shape in front of the camera (it drops onto whatever is there).");
        });
        let w = ui.available_width();
        if style::list_row(ui, self.show_env, "☀ Environment", "sun, sky, fog, gravity", w, false)
            .on_hover_text("The scene's sun, sky, time of day, light levels, fog, colours and gravity (in a window of their own).")
            .clicked()
        {
            self.show_env = !self.show_env;
        }
        // what each object cost in the last traced frame
        let total = self.stats.cycles.max(1) as f64;
        let mut cost = vec![None; self.doc.objects().len()];
        for (i, o) in self.scene.objects.iter().enumerate() {
            // (a group's members add up)
            if let (Some(c), Some(slot)) = (self.costs.get(i), cost.get_mut(o.doc_index)) {
                let (share, evals, per) = slot.unwrap_or((0.0, 0, o.node_cycles.first().copied().unwrap_or(0.0)));
                *slot = Some((share + c.cycles as f64 / total, evals + c.evals, per));
            }
        }
        let max_share = cost.iter().flatten().map(|c| c.0).fold(0.0f64, f64::max).max(1e-9);
        // what each group member cost (it is an object of its own)
        let mut member_cost: std::collections::HashMap<(usize, Vec<usize>), f64> = std::collections::HashMap::new();
        for (i, o) in self.scene.objects.iter().enumerate() {
            if !o.path.is_empty()
                && let Some(c) = self.costs.get(i)
            {
                *member_cost.entry((o.doc_index, o.path.clone())).or_insert(0.0) += c.cycles as f64 / total;
            }
        }
        // the rows that show: every top-level object, and the nodes of the
        // open ones (groups' members, booleans' and modifiers' children),
        // indented as a tree. A selection (a click in the view too) opens its
        // ancestors once, so that it shows; they can be closed again
        let ctx = ui.ctx().clone();
        let open_id = |top: usize, path: &[usize]| egui::Id::new(("outline", top, path.to_vec()));
        let revealed = egui::Id::new("outline revealed");
        let mut reveal = false;
        if let Selection::Node { top, sub } = &self.sel
            && ctx.data(|d| d.get_temp::<Selection>(revealed)).as_ref() != Some(&self.sel)
        {
            for k in 0..sub.len() {
                ctx.data_mut(|d| d.insert_temp(open_id(*top, &sub[..k]), true));
            }
            ctx.data_mut(|d| d.insert_temp(revealed, self.sel.clone()));
            reveal = true;
        }
        let is_open = |top: usize, path: &[usize]| ctx.data(|d| d.get_temp::<bool>(open_id(top, path))).unwrap_or(false);
        let rows = outline_rows(self.doc.objects(), &is_open);
        let mut toggle: Option<(usize, Vec<usize>)> = None;
        let mut flip: Option<(usize, Vec<usize>, bool)> = None;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("Objects ({})", self.doc.objects().len())).strong())
                .on_hover_text("Top-level objects, and inside them (▶ opens one) the members of groups and the parts of booleans and modifiers: click one to edit it, tick to show or hide it.");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                style::note(ui, "CPU cost").on_hover_text("Each object's share of the last frame's CPU time (intersections, shadows, AO, normals); a group's members each have their own.");
            });
        });
        let (bar_w, pct_w, arrow_w, indent) = (48.0, 42.0, 14.0, 14.0);
        let row_h = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
        let max_h = (ui.ctx().content_rect().height() * 0.38).clamp(150.0, 560.0);
        // (and scrolled to it, a couple of rows down from the top)
        let mut list = egui::ScrollArea::vertical().id_salt("outliner").max_height(max_h).auto_shrink([false, true]);
        if reveal
            && let Some(k) = rows.iter().position(|r| self.sel == Selection::Node { top: r.top, sub: r.path.clone() })
        {
            // (show_rows spaces its rows by row_h and the item spacing again)
            list = list.vertical_scroll_offset((k as f32 - 2.0).max(0.0) * (row_h + ui.spacing().item_spacing.y));
        }
        list.show_rows(ui, row_h, rows.len(), |ui, range| {
            for r in &rows[range] {
                ui.horizontal(|ui| {
                    ui.add_space(indent * r.depth as f32);
                    // open / close (a node with parts or members)
                    let h = ui.spacing().interact_size.y;
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(arrow_w, h), if r.kids { egui::Sense::click() } else { egui::Sense::hover() });
                    if r.kids {
                        let col = if resp.hovered() { ui.visuals().strong_text_color() } else { ui.visuals().weak_text_color() };
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, if r.open { "▼" } else { "▶" }, egui::FontId::proportional(10.0), col);
                        if resp.on_hover_text(if r.open { "Close: hide its parts" } else { "Open: show its parts" }).clicked() {
                            flip = Some((r.top, r.path.clone(), !r.open));
                        }
                    }
                    // (a modifier's one child cannot be hidden on its own: the
                    // modifier would have nothing to change)
                    let mut on = !r.hidden;
                    let tip = if r.path.is_empty() { "Show or hide this object (hidden objects are not compiled)." } else { "Show or hide this part (left out when hidden)." };
                    if ui.add_enabled(r.can_hide, egui::Checkbox::without_text(&mut on)).on_hover_text(tip).changed() {
                        toggle = Some((r.top, r.path.clone()));
                    }
                    let gap = ui.spacing().item_spacing.x;
                    let name_w = ui.available_width() - bar_w - pct_w - gap * 2.0;
                    let desc = schema::shape(if r.ty.is_empty() { &r.name } else { &r.ty }).map(|s| s.desc).unwrap_or("");
                    let here = Selection::Node { top: r.top, sub: r.path.clone() };
                    let resp = style::list_row(ui, self.sel == here, &r.name, &r.ty, name_w, r.hidden);
                    // the cost: an object's (its members' summed), a member's own,
                    // or, for a part of a shape, its share of the shape's
                    let share = if r.path.is_empty() { cost.get(r.top).copied().flatten().map(|c| c.0) } else { member_cost.get(&(r.top, r.path.clone())).copied() };
                    let (bar, _) = ui.allocate_exact_size(egui::vec2(bar_w, 8.0), egui::Sense::hover());
                    let mut hover = desc.to_string();
                    if let Some(share) = share {
                        let p = ui.painter();
                        p.rect_filled(bar, egui::CornerRadius::same(2), ui.visuals().faint_bg_color);
                        p.rect_filled(
                            egui::Rect::from_min_size(bar.min, egui::vec2(bar.width() * (share / max_share) as f32, bar.height())),
                            egui::CornerRadius::same(2),
                            style::ACCENT,
                        );
                        ui.allocate_ui_with_layout(egui::vec2(pct_w, row_h), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(format!("{:.1}%", share * 100.0)).small());
                        });
                        hover.push_str(&format!("\n\n{:.1}% of the last frame's CPU time", share * 100.0));
                        if r.path.is_empty()
                            && let Some((_, evals, est)) = cost.get(r.top).copied().flatten()
                        {
                            hover.push_str(&format!("\n{evals} evaluations of 8 points\n≈{est:.0} cycles per evaluation (estimate)"));
                        }
                    } else if let Some(o) = self.scene.objects.iter().find(|o| o.doc_index == r.top && r.path.starts_with(&o.path))
                        && let Some(id) = o.node_id(&r.path[o.path.len()..])
                    {
                        let all = o.node_cycles.first().copied().unwrap_or(0.0).max(1e-6);
                        let mine = o.node_cycles.get(id).copied().unwrap_or(0.0);
                        hover.push_str(&format!("\n\n≈{:.0}% of its object's distance-evaluation cost ({mine:.0} of {all:.0} cycles per 8 points, measured per operation)", mine / all * 100.0));
                    }
                    if resp.on_hover_text(hover.trim_start()).clicked() {
                        click = Some(here);
                    }
                });
            }
        });
        if let Some((top, path, open)) = flip {
            ctx.data_mut(|d| d.insert_temp(open_id(top, &path), open));
        }
        if let Some((top, path)) = toggle {
            self.doc.checkpoint(true);
            if let Some(n) = self.doc.node_mut(top, &path).and_then(Value::as_object_mut) {
                let off = n.get("disabled").and_then(Value::as_bool).unwrap_or(false);
                if off {
                    n.remove("disabled");
                } else {
                    n.insert("disabled".into(), json!(true));
                }
            }
            self.doc.touch();
        }
        if let Some(s) = click {
            self.sel = s;
        }
    }

    fn node_inspector(&mut self, ui: &mut egui::Ui, top: usize, sub: Vec<usize>) {
        let Some(node) = self.doc.node(top, &sub).cloned() else {
            self.sel = Selection::None;
            return;
        };
        let ty = node.get("type").and_then(Value::as_str).unwrap_or("?").to_string();
        let info = schema::shape(&ty);
        let name = node.get("name").and_then(Value::as_str).unwrap_or(&ty).to_string();
        style::title(ui, &name, |ui| {
            ui.label(egui::RichText::new(&ty).color(ui.visuals().weak_text_color()));
        });
        if let Some(i) = info {
            style::note(ui, i.desc);
        }
        // actions
        let mut action = None;
        let mut into_group = None;
        let parent_ty = sub.split_last().and_then(|(_, p)| self.doc.node(top, p)).and_then(|p| p.get("type")).and_then(Value::as_str).map(str::to_string);
        let in_group = parent_ty.as_deref() == Some("group");
        let groups: Vec<(usize, String)> = self
            .doc
            .objects()
            .iter()
            .enumerate()
            .filter(|(i, o)| *i != top && o.get("type").and_then(Value::as_str) == Some("group"))
            .map(|(i, o)| (i, doc::label(o)))
            .collect();
        ui.horizontal_wrapped(|ui| {
            // (buttons that do not fit go to the next line whole: the last one
            // was squeezed into a column a letter wide)
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if !sub.is_empty() && ui.button("↑ Parent").on_hover_text("Select the node this one belongs to.").clicked() {
                action = Some("parent");
            }
            if ui.button("Duplicate").on_hover_text(format!("Copy it next to the original ({}).", super::actions::keys_text(ui.ctx(), super::actions::get(super::actions::Act::Duplicate)))).clicked() {
                action = Some("dup");
            }
            if ui.button("Delete").on_hover_text(format!("Remove it ({}). A combinator keeps at least one child.", super::actions::keys_text(ui.ctx(), super::actions::get(super::actions::Act::Delete)))).clicked() {
                action = Some("del");
            }
            if ui.button("Frame").on_hover_text(format!("Point the camera at it ({}).", super::actions::keys_text(ui.ctx(), super::actions::get(super::actions::Act::Frame)))).clicked() {
                action = Some("frame");
            }
            if sub.is_empty() && ty != "emitter" {
                ui.add_enabled_ui(!groups.is_empty(), |ui| {
                    ui.menu_button("Into group", |ui| {
                        for (g, label) in &groups {
                            if ui.button(label).clicked() {
                                into_group = Some(*g);
                                ui.close();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Move it into a group (it stays where it is; the group then moves it with the rest).")
                    .on_disabled_hover_text("There is no group to put it in yet: add one (Combine > group), or Wrap in > group.");
                });
            }
            if in_group && ui.button("Out of group").on_hover_text("Make it an object of its own again, where it is (with the material it had from the group).").clicked() {
                action = Some("ungroup");
            }
            if ui.button("Modifiers…").on_hover_text(format!("Twist, bend, ripples, bumps, bricks...: pick one from the library to apply it ({}).", super::actions::keys_text(ui.ctx(), super::actions::get(super::actions::Act::Modifiers)))).clicked() {
                self.show_modifiers = true;
            }
            let emitter = ty == "emitter" && sub.is_empty();
            ui.add_enabled_ui(!emitter, |ui| {
            ui.menu_button("Wrap in", |ui| {
                // (a group holds whole objects: only those can go into one)
                let whole = (sub.is_empty() || in_group).then_some("group");
                for w in ["union", "intersection", "difference", "repeat", "mirror", "twist", "bend", "round", "shell", "displace", "relief", "slice"].into_iter().chain(whole) {
                    let desc = schema::shape(w).map(|s| s.desc).unwrap_or("");
                    if ui.button(w).on_hover_text(desc).clicked() {
                        if let Some(s) = self.doc.wrap(top, &sub, w) {
                            self.sel = s;
                        }
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text("Put this node inside a combinator or modifier (it becomes the first child).")
            .on_disabled_hover_text("An emitter stays a top-level object: it cannot go inside another.");
            });
            if node.get("children").is_some_and(Value::is_array) {
                ui.menu_button("Add child", |ui| {
                    // a group takes whole objects of any kind
                    let group = ty == "group";
                    let cats: &[&str] = if group { &["Primitives", "Surfaces", "Fractals", "Blobs", "Combine", "Modify", "Landscape"] } else { &["Primitives", "Fractals", "Blobs", "Combine", "Modify"] };
                    for &cat in cats {
                        ui.menu_button(cat, |ui| {
                            for s in schema::SHAPES.iter().filter(|s| s.category == cat && (group || s.ty != "group")) {
                                if ui.button(s.ty).on_hover_text(s.desc).clicked() {
                                    if let Ok(v) = serde_json::from_str(s.new)
                                        && let Some(sel) = self.doc.add_child(top, &sub, v)
                                    {
                                        self.sel = sel;
                                    }
                                    ui.close();
                                }
                            }
                        });
                    }
                })
                .response
                .on_hover_text("Add a shape to this combinator.");
            }
        });
        match action {
            Some("parent") => {
                self.sel = Selection::Node { top, sub: sub[..sub.len() - 1].to_vec() };
                return;
            }
            Some("dup") => {
                self.duplicate();
                return;
            }
            Some("del") => {
                self.delete();
                return;
            }
            Some("frame") => self.frame_selected(),
            Some("ungroup") => {
                if let Some(s) = self.doc.out_of_group(top, &sub) {
                    self.sel = s;
                    self.placed.0.clear();
                    self.lost_bodies();
                }
                return;
            }
            _ => {}
        }
        if let Some(g) = into_group {
            if let Some(s) = self.doc.into_group(top, g) {
                self.sel = s;
                self.placed.0.clear();
                self.lost_bodies();
            }
            return;
        }
        if self.doc.node(top, &sub).is_none() {
            return;
        }

        // what it costs (a group: all its members)
        if let Some((first, end)) = self.selected_objects() {
            let o = &self.scene.objects[first as usize];
            let total = self.stats.cycles.max(1) as f64;
            let (cycles, evals) = (first..end).filter_map(|i| self.costs.get(i as usize)).fold((0, 0), |(c, e), k| (c + k.cycles, e + k.evals));
            let tpu = crate::render::ticks_per_us();
            let mut text = format!(
                "Cost: {:.1}% of the last frame's CPU time ({:.2} ms over all cores), {} evaluations of 8 points",
                cycles as f64 / total * 100.0,
                cycles as f64 / tpu / 1000.0,
                evals
            );
            // (within one object: a group member's path goes on inside it)
            if end == first + 1
                && let Some(id) = sub.get(o.path.len()..).and_then(|rest| o.node_id(rest))
            {
                let all = o.node_cycles.first().copied().unwrap_or(0.0);
                let mine = o.node_cycles.get(id).copied().unwrap_or(0.0);
                if all > 0.0 {
                    text += &format!("\nOne evaluation ≈ {all:.0} cycles; this node ≈ {mine:.0} ({:.0}%)", mine / all * 100.0);
                }
            }
            style::note(ui, text).on_hover_text(
                "Measured while rendering: CPU cycles spent intersecting, shadowing, occluding and shading this object, summed over all threads. \
                 The per-node figure is an estimate from timing each operation of the object's distance program.",
            );
        }

        let mut n = node.clone();
        let mut changed = false;
        style::section(ui, ("shape", top, sub.clone()), "Shape", true, |ui| {
        style::props(ui, ("node", top, sub.clone()), |ui| {
            // name and placement
            let mut name = n.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            ui.label("name").on_hover_text(schema::field_desc(&ty, "name"));
            if ui.add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY)).on_hover_text(schema::field_desc(&ty, "name")).changed() {
                if name.is_empty() {
                    n.as_object_mut().map(|o| o.remove("name"));
                } else {
                    n["name"] = json!(name);
                }
                changed = true;
            }
            ui.end_row();
            for (key, default) in [("position", json!([0.0, 0.0, 0.0])), ("rotation", json!([0.0, 0.0, 0.0])), ("scale", json!(1.0))] {
                let mut v = n.get(key).cloned().filter(|v| !v.is_null()).unwrap_or(default);
                ui.label(style::pretty(key)).on_hover_text(schema::field_desc(&ty, key));
                if edit_value(ui, &ty, key, &mut v) {
                    n[key] = v;
                    changed = true;
                }
                ui.end_row();
            }
            // the shape's own parameters, every one (defaults filled in: kept
            // while the node is the same, as the filling parses it)
            let cache = match self.defaults_cache.take() {
                Some(c) if c.top == top && c.sub == sub && same_but_pose(&c.node, &n) => c,
                _ => DefaultsCache { top, sub: sub.clone(), node: n.clone(), full: schema::with_defaults(&n).unwrap_or(n.clone()) },
            };
            if let Some(obj) = cache.full.as_object() {
                for (k, dv) in obj {
                    if OWN_SECTION.contains(&k.as_str()) {
                        continue;
                    }
                    let mut v = n.get(k).cloned().unwrap_or(dv.clone());
                    ui.label(style::pretty(k)).on_hover_text(schema::field_desc(&ty, k));
                    if edit_value(ui, &ty, k, &mut v) {
                        if v.is_null() {
                            n.as_object_mut().map(|o| o.remove(k));
                        } else {
                            n[k.as_str()] = v;
                        }
                        changed = true;
                    }
                    ui.end_row();
                }
            }
            self.defaults_cache = Some(cache);
        });
        });

        // top level: physics and render options
        if sub.is_empty() {
            style::section(ui, ("phys", top), "Physics and rendering", false, |ui| {
            style::props(ui, ("top", top), |ui| {
                let mut friction = n.get("friction").and_then(Value::as_f64).unwrap_or(0.5) as f32;
                if style::slider(ui, "friction", schema::field_desc(&ty, "friction"), &mut friction, 0.0..=2.0, false) {
                    n["friction"] = json!(round(friction as f64));
                    changed = true;
                }
                let mut bounce = n.get("bounce").and_then(Value::as_f64).unwrap_or(0.15) as f32;
                if style::slider(ui, "bounce", schema::field_desc(&ty, "bounce"), &mut bounce, 0.0..=1.0, false) {
                    n["bounce"] = json!(round(bounce as f64));
                    changed = true;
                }
                // mass: automatic (from the size) or set (moving does not
                // change it: worked out again only for a new shape or
                // material; from the document's node, which it is worked out
                // from, not this frame's edit of it)
                let doc_node = self.doc.node(top, &[]).cloned().unwrap_or_default();
                let materials = self.doc.v.get("materials").cloned();
                let fresh = self.natural_mass.as_ref().is_some_and(|c| c.top == top && c.materials == materials && same_but_pose(&c.node, &doc_node));
                if !fresh {
                    let (mass, dense) = super::physics::natural_mass(&self.doc.v, top).unwrap_or((1.0, false));
                    self.natural_mass = Some(MassCache { top, node: doc_node, materials, mass, dense });
                }
                let (auto, dense) = self.natural_mass.as_ref().map_or((1.0, false), |c| (c.mass, c.dense));
                let whence = if dense { "its material's density times its volume" } else { "its size" };
                ui.label("mass").on_hover_text(schema::field_desc(&ty, "mass"));
                ui.horizontal(|ui| match n.get("mass").and_then(Value::as_f64) {
                    Some(m) => {
                        let speed = m * 0.01;
                        let mut m = m;
                        let r = ui
                            .add(egui::DragValue::new(&mut m).speed(speed).range(0.01..=1e6).suffix(" kg").max_decimals(2))
                            .on_hover_text(schema::field_desc(&ty, "mass"));
                        if r.changed() {
                            n["mass"] = json!(round(m));
                            changed = true;
                        }
                        if ui.small_button("auto").on_hover_text("Go back to the mass from its size.").clicked() {
                            n.as_object_mut().map(|o| o.remove("mass"));
                            changed = true;
                        }
                    }
                    None => {
                        ui.label(format!("auto (≈{auto:.1} kg)")).on_hover_text(format!("{} Now: from {whence}.", schema::field_desc(&ty, "mass")));
                        if ui.small_button("set").on_hover_text("Give it a mass of its own.").clicked() {
                            n["mass"] = json!(round(auto as f64));
                            changed = true;
                        }
                    }
                });
                ui.end_row();
                for (key, default) in [("locked", false), ("gravity", true), ("shadow", true)] {
                    let mut b = n.get(key).and_then(Value::as_bool).unwrap_or(default);
                    // ("gravity" is whether it falls; how hard is the World's)
                    let label = if key == "gravity" { "falls" } else { key };
                    if style::check(ui, label, schema::field_desc(&ty, key), &mut b) {
                        n[key] = json!(b);
                        changed = true;
                    }
                }
                for key in ["step", "bound"] {
                    let mut v = n.get(key).cloned().unwrap_or(Value::Null);
                    ui.label(key).on_hover_text(schema::field_desc(&ty, key));
                    let c = match &mut v {
                        Value::Null => {
                            let mut c = false;
                            ui.horizontal(|ui| {
                                ui.label("auto").on_hover_text(schema::field_desc(&ty, key));
                                if ui.small_button("set").on_hover_text("Override the automatic value.").clicked() {
                                    v = json!(if key == "step" { 0.8 } else { 2.0 });
                                    c = true;
                                }
                            });
                            c
                        }
                        _ => {
                            let mut x = v.as_f64().unwrap_or(1.0);
                            let mut clear = false;
                            let c = ui
                                .horizontal(|ui| {
                                    let c = float_drag(ui, &mut x, key, schema::field_desc(&ty, key));
                                    clear = ui.small_button("auto").on_hover_text("Go back to the automatic value.").clicked();
                                    c
                                })
                                .inner;
                            v = if clear { Value::Null } else { json!(x) };
                            c || clear
                        }
                    };
                    if c {
                        if v.is_null() {
                            n.as_object_mut().map(|o| o.remove(key));
                        } else {
                            n[key] = v;
                        }
                        changed = true;
                    }
                    ui.end_row();
                }
            });
            });
        }
        // what an emitter sends out: how its copies move (they are bodies of their own)
        let emitted = sub.len() == 1 && self.doc.node(top, &[]).and_then(|t| t.get("type")).and_then(Value::as_str) == Some("emitter");
        if emitted {
            style::section(ui, ("emit phys", top), "Physics of the copies", false, |ui| {
                style::props(ui, ("emit phys props", top), |ui| {
                    for (key, default, range) in [("friction", 0.5, 0.0..=2.0), ("bounce", 0.15, 0.0..=1.0)] {
                        let mut v = n.get(key).and_then(Value::as_f64).unwrap_or(default) as f32;
                        if style::slider(ui, key, schema::field_desc(&ty, key), &mut v, range, false) {
                            n[key] = json!(round(v as f64));
                            changed = true;
                        }
                    }
                    ui.label("mass").on_hover_text(schema::field_desc(&ty, "mass"));
                    ui.horizontal(|ui| match n.get("mass").and_then(Value::as_f64) {
                        Some(mut m) => {
                            let speed = m * 0.01;
                            if ui.add(egui::DragValue::new(&mut m).speed(speed).range(0.01..=1e6).suffix(" kg").max_decimals(2)).changed() {
                                n["mass"] = json!(round(m));
                                changed = true;
                            }
                            if ui.small_button("auto").on_hover_text("Go back to the mass from its material and size.").clicked() {
                                n.as_object_mut().map(|o| o.remove("mass"));
                                changed = true;
                            }
                        }
                        None => {
                            ui.label("auto").on_hover_text("From its material's density and its volume, else its size.");
                            if ui.small_button("set").on_hover_text("Give the copies a mass of their own.").clicked() {
                                n["mass"] = json!(1.0);
                                changed = true;
                            }
                        }
                    });
                    ui.end_row();
                    for (key, default) in [("gravity", true), ("shadow", true)] {
                        let mut b = n.get(key).and_then(Value::as_bool).unwrap_or(default);
                        let label = if key == "gravity" { "falls" } else { key };
                        if style::check(ui, label, schema::field_desc(&ty, key), &mut b) {
                            n[key] = json!(b);
                            changed = true;
                        }
                    }
                });
            });
        }
        if changed {
            if let Some(m) = fit_knots(&mut n) {
                self.say(m);
            }
            // a physics setting changed: the object moves again at once, to show it
            let old = self.doc.node(top, &sub).cloned().unwrap_or_default();
            if sub.is_empty() && ["friction", "bounce", "mass", "locked", "gravity"].iter().any(|k| old.get(*k) != n.get(*k)) {
                self.wake = Some(top);
            }
            self.doc.checkpoint(false);
            if let Some(slot) = self.doc.node_mut(top, &sub) {
                *slot = n;
            }
            self.doc.touch();
        }
        let density = |d: &doc::Doc| d.node(top, &[]).and_then(|t| super::physics::density_of(&d.v, t));
        let before = density(&self.doc);
        style::section(ui, ("material", top, sub.clone()), "Material", true, |ui| self.material_section(ui, top, &sub));
        if density(&self.doc) != before {
            self.wake = Some(top);
        }
    }

    fn material_section(&mut self, ui: &mut egui::Ui, top: usize, sub: &[usize]) {
        let Some(current) = self.doc.node(top, sub).map(|n| n.get("material").cloned()) else { return };
        let mut assign: Option<Value> = None;
        let mut remove = false;
        // the material as it renders, on a sphere, and what it is
        let def = match &current {
            Some(Value::Object(_)) => current.clone(),
            Some(Value::String(name)) => self.doc.v.get("materials").and_then(|m| m.get(name.as_str())).cloned(),
            _ => None,
        }
        .unwrap_or_else(|| serde_json::to_value(MaterialDef::default()).unwrap_or_default());
        let kind = match &current {
            Some(Value::Object(_)) => "Its own material (only this object)".to_string(),
            Some(Value::String(name)) => {
                let users: usize = self.doc.objects().iter().map(|o| uses(o, name)).sum();
                format!("Scene material '{name}', shared by {users} object(s): editing it changes all of them")
            }
            _ => "No material of its own: it uses its parent's, or plain grey".to_string(),
        };
        ui.horizontal(|ui| {
            let key = super::library::preview_key(&def);
            let look = super::library::look(&def);
            let size = egui::vec2(72.0, 72.0);
            match self.previews.live(&key, || super::previews::material_scene(&look)) {
                Some(t) => {
                    ui.add(egui::Image::new(t).fit_to_exact_size(size).corner_radius(egui::CornerRadius::same(6)));
                }
                None => {
                    ui.allocate_exact_size(size, egui::Sense::hover());
                }
            }
            ui.vertical(|ui| {
                style::note(ui, kind.clone());
                ui.horizontal_wrapped(|ui| {
                    let a = super::actions::get(super::actions::Act::Library);
                    if ui.button("Library…").on_hover_text(format!("{} ({})", a.tip, super::actions::keys_text(ui.ctx(), a))).clicked() {
                        self.show_library = true;
                    }
                    if ui.button("🎲 Random").on_hover_text("Generate a new random procedural material for it.").clicked() {
                        self.rng = self.rng.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                        assign = serde_json::to_value(materials::random(self.rng)).ok();
                    }
                    if current.is_some() && ui.button("Use parent's").on_hover_text("No material of its own: it uses its parent's, or plain grey.").clicked() {
                        remove = true;
                    }
                });
            });
        });
        if remove || assign.is_some() {
            self.doc.checkpoint(true);
            if let Some(n) = self.doc.node_mut(top, sub).and_then(Value::as_object_mut) {
                match assign {
                    Some(v) => {
                        n.insert("material".into(), v);
                    }
                    None => {
                        n.remove("material");
                    }
                }
            }
            self.doc.touch();
            return;
        }
        match current {
            Some(Value::Object(m)) => {
                let mut def: MaterialDef = match serde_json::from_value(Value::Object(m.clone())) {
                    Ok(d) => d,
                    Err(e) => {
                        ui.colored_label(style::ERROR, e.to_string());
                        return;
                    }
                };
                if materials::editor(ui, &mut def, &mut self.previews)
                    && let Ok(v) = serde_json::to_value(&def)
                {
                    self.doc.checkpoint(false);
                    if let Some(n) = self.doc.node_mut(top, sub) {
                        n["material"] = v;
                    }
                    self.doc.touch();
                }
                ui.horizontal(|ui| {
                    ui.add(egui::TextEdit::singleline(&mut self.lib_name).hint_text("name").desired_width(120.0))
                        .on_hover_text("A name for saving this material.");
                    let named = !self.lib_name.trim().is_empty();
                    if ui.add_enabled(named, egui::Button::new("Save to library")).on_hover_text("Add it to library/materials.json for every scene.").clicked() {
                        // (a material of that name already there stays)
                        let want = self.lib_name.trim().to_string();
                        let name = self.library.free_name(&want);
                        let r = self.library.store(&name, Value::Object(m.clone()));
                        self.say(match r {
                            Ok(()) if name != want => format!("saved as '{name}' (the library already has a '{want}')"),
                            Ok(()) => format!("saved '{name}' to the library"),
                            Err(e) => e,
                        });
                    }
                    if ui
                        .add_enabled(named, egui::Button::new("Share in scene"))
                        .on_hover_text("Make it a named scene material that other objects can share.")
                        .clicked()
                    {
                        // (not over a scene material of that name: other objects use it)
                        let want = self.lib_name.trim().to_string();
                        let mats = self.doc.v.get("materials").cloned().unwrap_or_default();
                        let name = super::materials::free_name(&want, |n| mats.get(n).is_some());
                        if name != want {
                            self.say(format!("shared as '{name}' (the scene already has a '{want}')"));
                        }
                        self.doc.checkpoint(true);
                        self.doc.object_mut("materials").insert(name.clone(), Value::Object(m.clone()));
                        if let Some(n) = self.doc.node_mut(top, sub) {
                            n["material"] = json!(name);
                        }
                        self.doc.touch();
                    }
                });
            }
            Some(Value::String(name)) => {
                if ui.button("Make unique").on_hover_text("Copy it into this node so it can change on its own.").clicked() {
                    if let Some(v) = self.doc.v.get("materials").and_then(|m| m.get(&name)).cloned() {
                        self.doc.checkpoint(true);
                        if let Some(n) = self.doc.node_mut(top, sub) {
                            n["material"] = v;
                        }
                        self.doc.touch();
                    }
                    return;
                }
                let Some(v) = self.doc.v.get("materials").and_then(|m| m.get(&name)).cloned() else {
                    ui.colored_label(style::ERROR, "not defined in the scene's materials");
                    return;
                };
                if let Ok(mut def) = serde_json::from_value::<MaterialDef>(v)
                    && materials::editor(ui, &mut def, &mut self.previews)
                    && let Ok(v) = serde_json::to_value(&def)
                {
                    self.doc.checkpoint(false);
                    self.doc.object_mut("materials").insert(name.clone(), v);
                    self.doc.touch();
                }
            }
            _ => {
                style::note(ui, "Give it a material from the library (or a random one) to edit it.");
            }
        }
    }

    pub(super) fn env_inspector(&mut self, ui: &mut egui::Ui) {
        style::note(ui, "The scene's sun, sky, light, fog and gravity (saved with the scene; Ctrl+Z undoes).");
        let env = self.scene.env;
        let e = self.doc.v.get("environment").cloned().unwrap_or(json!({}));
        // one edit of the environment: `force` makes it an undo step of its own
        // (a choice or a switch), else it joins a drag's step
        let set = |doc: &mut doc::Doc, key: &str, v: Option<Value>, force: bool| {
            doc.checkpoint(force);
            let o = doc.object_mut("environment");
            match v {
                Some(v) => {
                    o.insert(key.into(), v);
                }
                None => {
                    o.remove(key);
                }
            }
            doc.touch();
        };
        style::section(ui, "env sun", "Sun and sky", true, |ui| {
            style::props(ui, "env sun", |ui| {
                let preset = e.get("preset").and_then(Value::as_str).unwrap_or("golden").to_string();
                let tip = "fraktal256's palettes: sun, sky, fog and water colours. The settings below override parts of it.";
                style::row(ui, "preset", tip, |ui| {
                    egui::ComboBox::from_id_salt("env preset")
                        .selected_text(&preset)
                        .show_ui(ui, |ui| {
                            for p in ["golden", "dusk", "frozen", "storm"] {
                                if ui.selectable_label(preset == p, p).clicked() && preset != p {
                                    set(&mut self.doc, "preset", Some(json!(p)), true);
                                }
                            }
                        })
                        .response
                        .on_hover_text(tip);
                });
                let sky = e.get("sky").and_then(Value::as_str).unwrap_or("day").to_string();
                let skies = [
                    ("day", "day", "The daytime sky: gradient, clouds, the sun's disk. It lights the scene through the ambient level, reflections and the fog."),
                    ("stars", "stars", "A night sky of stars, very dim: with the sun and ambient levels at 0, glowing objects are nearly the only light."),
                    ("off", "off", "No sky: black. With the sun and ambient levels at 0 too, only glowing objects light the scene, and without any the image is black."),
                ];
                if let Some(v) = style::choice(ui, "sky", "What is seen beyond everything, and the light it gives (reflections, fog, the water's colour).", sky.as_str(), &skies) {
                    set(&mut self.doc, "sky", Some(json!(v)), true);
                }
                // the time of day: the sun and the moon move, with dawn, dusk and night
                let tod = e.get("time_of_day").and_then(Value::as_f64);
                let tip = "Place the sun and the moon by the hour: the sun rises at 6 in the east, is highest at 12 (where the sun azimuth and elevation point) and sets at 18 in the west; the full moon is opposite and lights the night. Sunrise and sunset redden the light and the sky; the stars come out after dusk. Unticked: the sun stays where azimuth and elevation put it.";
                style::row(ui, "time of day", tip, |ui| {
                    ui.horizontal(|ui| {
                        let mut on = tod.is_some();
                        if ui.checkbox(&mut on, "").on_hover_text(tip).changed() {
                            set(&mut self.doc, "time_of_day", on.then(|| json!(17.5)), true);
                        }
                        let mut h = tod.unwrap_or(12.0) as f32;
                        let clock = |v: f64, _: RangeInclusive<usize>| {
                            let m = (v * 60.0).round() as i64;
                            format!("{:02}:{:02}", m / 60 % 24, m % 60)
                        };
                        ui.spacing_mut().slider_width -= 26.0;
                        if ui.add_enabled(on, egui::Slider::new(&mut h, 0.0..=24.0).custom_formatter(clock)).on_hover_text(tip).changed() {
                            set(&mut self.doc, "time_of_day", Some(json!((h as f64 * 100.0).round() / 100.0)), false);
                        }
                    });
                });
                // what the file says, not where the hour has put the sun (or the moon)
                let base = crate::scene::env_base(&e);
                let (az, mut el) = base.as_ref().map_or_else(|| sun_angles(env.sun_dir), |b| b.sun);
                // (the file may say 215: the slider shows it as -145)
                let mut az = (az + 180.0).rem_euclid(360.0) - 180.0;
                let a = style::slider(ui, "sun azimuth", "Compass direction of the sun, degrees from -z toward +x (with a time of day: where it stands at noon).", &mut az, -180.0..=180.0, false);
                let b = style::slider(ui, "sun elevation", "Height of the sun above the horizon, degrees. Low suns give long shadows (with a time of day: its height at noon).", &mut el, -10.0..=90.0, false);
                if a || b {
                    self.doc.checkpoint(false);
                    let o = self.doc.object_mut("environment");
                    o.remove("sun_direction");
                    o.insert("sun_azimuth".into(), json!(round(az as f64)));
                    o.insert("sun_elevation".into(), json!(round(el as f64)));
                    self.doc.touch();
                }
                for (key, name, tip) in [
                    ("sun_level", "sun level", "Brightness of direct sunlight, as a multiple of the scene's sun (0 = no sun: night, lit by glowing objects)."),
                    ("ambient_level", "ambient level", "Brightness of the sky's fill light in shadows and on surfaces facing away from the sun, as a multiple (0 = black shadows)."),
                ] {
                    let mut v = e.get(key).and_then(Value::as_f64).unwrap_or(1.0) as f32;
                    if style::slider(ui, name, tip, &mut v, 0.0..=4.0, false) {
                        set(&mut self.doc, key, Some(json!((v * 1000.0).round() / 1000.0)), false);
                    }
                }
                let mut fog = env.fog;
                if style::slider(ui, "fog density", "How quickly distant things fade into the haze (per metre). 0 = clear air.", &mut fog, 0.0..=0.05, true) {
                    set(&mut self.doc, "fog_density", Some(json!(round(fog as f64))), false);
                }
                let mut clouds = env.clouds;
                if style::slider(ui, "cloud cover", "The sky's noise clouds: 0 = clear sky, 1 = overcast (cloud objects are separate).", &mut clouds, 0.0..=1.0, false) {
                    set(&mut self.doc, "clouds", Some(json!(round(clouds as f64))), false);
                }
            });
        });
        style::section(ui, "env world", "World", true, |ui| {
            style::props(ui, "env world", |ui| {
                let tip = "How hard things fall, m/s²: dropped objects, emitted copies, floating in water, and game mode's walker (its jump and rocket are a person's on Earth, so they carry further where gravity is weaker). Earth 9.81, Mars 3.71, the Moon 1.62; 0: nothing falls.";
                let mut g = self.scene.gravity;
                if style::slider(ui, "gravity (m/s²)", tip, &mut g, 0.0..=30.0, false) {
                    set(&mut self.doc, "gravity", Some(json!(round(g as f64))), false);
                }
                style::row(ui, "", tip, |ui| {
                    ui.horizontal(|ui| {
                        for (name, v) in [("Earth", 9.81), ("Mars", 3.71), ("Moon", 1.62), ("None", 0.0)] {
                            if ui.selectable_label((self.scene.gravity - v as f32).abs() < 1e-3, name).clicked() {
                                set(&mut self.doc, "gravity", Some(json!(v)), true);
                            }
                        }
                    });
                });
            });
        });
        style::section(ui, "env colours", "Colours", true, |ui| {
            style::props(ui, "env colours", |ui| {
                // the file's colours (the time of day and the sky dim and tint what is seen)
                let base = crate::scene::env_base(&e).map_or([env.sky_zen, env.sky_hor, env.fog_col, env.amb_col], |b| b.colours);
                for (key, name, tip, col) in [
                    ("sky_zenith", "sky overhead", "Sky colour straight up (at noon, with a time of day).", base[0]),
                    ("sky_horizon", "sky at horizon", "Sky colour at the horizon (at noon, with a time of day).", base[1]),
                    ("fog_color", "fog", "Colour of the haze (by day: the night and a starry or no sky darken it).", base[2]),
                    ("ambient", "ambient light", "Colour of the sky light filling shadows (direct mode; at noon, with a time of day).", base[3]),
                ] {
                    let mut c = col.to_array();
                    if style::colour(ui, name, tip, &mut c) {
                        set(&mut self.doc, key, Some(json!(c.map(|v| round(v as f64)))), false);
                    }
                }
            });
        });
        // the water object's waves (also in its own inspector)
        if let Some(i) = self.doc.objects().iter().position(|o| o.get("type").and_then(Value::as_str) == Some("water")) {
            let node = self.doc.objects()[i].clone();
            style::section(ui, "env water", "Water", true, |ui| {
                style::props(ui, "env water", |ui| {
                    for (key, name, tip) in [
                        ("waves", "wave height", "Size of the water's waves, as a multiple (0 = flat as a mirror). Floating objects ride them; bigger waves also draw stronger caustics on the bed."),
                        ("wave_speed", "wave speed", "How fast the waves move while the water is animated (Render > Lighting > animate water), as a multiple."),
                    ] {
                        let mut v = node.get(key).and_then(Value::as_f64).unwrap_or(1.0) as f32;
                        if style::slider(ui, name, tip, &mut v, 0.0..=4.0, false) {
                            self.doc.checkpoint(false);
                            if let Some(n) = self.doc.node_mut(i, &[]) {
                                n[key] = json!((v * 100.0).round() / 100.0);
                            }
                            self.doc.touch();
                        }
                    }
                });
            });
        }
        let _ = sun_from_angles;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The object list's tree: every object; under an open one its parts
    /// (a boolean's children, a modifier's child) and members, a level
    /// deeper; a closed one's stay hidden. A boolean's or group's parts can
    /// be hidden one by one, a modifier's one child cannot.
    #[test]
    fn the_object_list_is_a_tree() {
        let objects = vec![
            json!({ "type": "sphere", "name": "ball" }),
            json!({ "type": "union", "children": [
                { "type": "box", "disabled": true },
                { "type": "twist", "child": { "type": "sphere" } }
            ]}),
            json!({ "type": "group", "name": "pair", "children": [{ "type": "box" }, { "type": "box" }] }),
        ];
        let open = |top: usize, path: &[usize]| top == 1 && (path.is_empty() || path == [1]);
        let rows = outline_rows(&objects, &open);
        let seen: Vec<(usize, Vec<usize>, usize)> = rows.iter().map(|r| (r.top, r.path.clone(), r.depth)).collect();
        assert_eq!(seen, vec![(0, vec![], 0), (1, vec![], 0), (1, vec![0], 1), (1, vec![1], 1), (1, vec![1, 0], 2), (2, vec![], 0)]);
        assert_eq!((rows[0].name.as_str(), rows[0].ty.as_str(), rows[0].kids), ("ball", "sphere", false));
        assert_eq!((rows[1].name.as_str(), rows[1].kids, rows[1].open), ("union", true, true));
        assert!(rows[2].hidden && rows[2].can_hide, "a boolean's part: hidden, and can be shown again");
        assert!(rows[3].kids && rows[3].open && rows[3].can_hide);
        assert!(!rows[4].can_hide, "a modifier's only child cannot be hidden by itself");
        assert!(rows[5].kids && !rows[5].open, "a closed group keeps its members out of the list");
    }

    /// The inspector's caches hold while a node only moves.
    #[test]
    fn moving_is_not_a_change_of_shape() {
        let a = json!({ "type": "box", "size": [1, 2, 3], "position": [0, 0, 0], "rotation": [0, 10, 0] });
        let moved = json!({ "type": "box", "size": [1, 2, 3], "position": [5, 1, 0] });
        let resized = json!({ "type": "box", "size": [1, 2, 4], "position": [0, 0, 0] });
        assert!(same_but_pose(&a, &moved));
        assert!(!same_but_pose(&a, &resized));
        assert!(!same_but_pose(&a, &json!({ "type": "box", "size": [1, 2, 3], "material": "gold" })));
    }

    #[test]
    fn knots_that_no_longer_fit_go() {
        // the nurbs scene's torus: 9 points of degree 2, 12 knots; a point removed
        let mut n = json!({"type": "revolve", "degree": 2, "profile": [[0.6, 0], [0.6, 0.2, 0.7], [0.4, 0.2], [0.2, 0.2, 0.7], [0.2, 0], [0.2, -0.2, 0.7], [0.4, -0.2], [0.6, -0.2, 0.7]],
            "knots": [0, 0, 0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1, 1, 1]});
        assert!(fit_knots(&mut n).is_some());
        assert!(n.get("knots").is_none());
        // fitting knots stay
        let mut n = json!({"type": "revolve", "degree": 2, "profile": [[0, -1], [1, -1, 0.7], [1, 0], [1, 1, 0.7], [0, 1]], "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1]});
        assert!(fit_knots(&mut n).is_none());
        assert!(n.get("knots").is_some());
        // a net that lost a row
        let mut n = json!({"type": "nurbs", "degree": [3, 3], "points": vec![vec![[0, 0, 0]; 5]; 4], "knots_u": [0, 0, 0, 0, 0.5, 1, 1, 1, 1], "knots_v": [0, 0, 0, 0, 0.5, 1, 1, 1, 1]});
        assert!(fit_knots(&mut n).is_some());
        assert!(n.get("knots_u").is_none() && n.get("knots_v").is_some());
    }
}
