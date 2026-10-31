//! The help screen (F1): every shape with its parameters, defaults and
//! meaning; materials and patterns; the controls.

use super::{App, actions, schema};
use crate::scene::tex;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Shapes,
    Materials,
    Controls,
}

/// The mouse and the keys that are not commands (those come from
/// actions::ACTIONS, with the menus' names).
const CONTROLS: &[(&str, &str)] = &[
    ("left click", "Select the object (and the part of its shape tree) under the cursor."),
    ("left drag on an object", "Pick it up and carry it; let go to drop it. It falls, collides and settles with physics (mass, friction and bounce per object); whatever it hits is knocked about too."),
    ("while carrying: ↑ / ↓", "Move it farther away / nearer (also the wheel)."),
    ("while carrying: ← / →", "Tip it forward (away from you) / turn it clockwise as you see it. Hold Shift to turn the other way."),
    ("while carrying: right button", "Hold to charge a throw (full strength after 1.2 s; the ring at the pointer shows it), let go to throw along the view. Heavier objects fly slower."),
    ("left drag elsewhere", "Look around (also middle drag)."),
    ("right drag", "Pan: slide the camera sideways and up or down; the point you grabbed follows the cursor."),
    ("mouse wheel", "Move toward or away from the point under the cursor: each notch takes 15% off the distance (or puts it back), so it gets finer the nearer you are and never goes through."),
    ("Shift + mouse wheel", "A plain step forward or back (0.3 x the fly speed a notch, [ and ] change it), through walls and all."),
    ("drag a handle", "The selected object's handles. Move (arrows): slide it along x, y or z (Ctrl: on a 10 cm grid). Rotate (rings): turn it freely; Shift: only about that ring's axis. Scale (cube ends, its own axes): resize it evenly; Shift: stretch it along that axis only. It then stays where it is put until dropped."),
    ("W A S D", "Fly forward, left, back, right. Flying slows down toward whatever is ahead and stops short of it (a fractal's crevices too)."),
    ("R / F (Space / C)", "Fly up / down."),
    ("Q / E, arrows", "Turn left / right; ↑ / ↓ look up / down. While carrying, the arrows move and tip what you carry instead."),
    ("Shift, Alt", "Fly 4x faster, 4x slower."),
    ("[ and ]", "Fly speed down / up."),
    ("1 - 7", "Render modes: direct, path traced, normals, albedo, depth, search steps, object cost."),
    ("game mode: mouse, W A S D, Shift", "Look, walk, run (hold Shift, or tap it while moving: you run on until you stop). You hop onto ledges up to a third of your height."),
    ("game mode: Space", "Jump: a metre on Earth, higher where gravity is weaker (Environment > World). With W, A, S or D held, a leap that way."),
    ("game mode: left button", "Gravity gun: hold to charge (the ring round the crosshair fills over a second and a half), let go to fire. Whatever is under the crosshair within about 30 m is sent off the way you look, harder the longer you held it: a tap nudges, a full charge sends 20 kg at 40 m/s (heavier things slower), spinning it when hit off centre. The crosshair turns orange over something it can move; locked objects stay put. Ctrl+Z (back in editing) puts a shot object back."),
    ("game mode: right button", "Rocket pack, held: it builds up over two seconds (the gauge bottom right; past its tick it lifts you). It is on your back: look down to lean forward and fly forward, up to lean back; never more than 40 degrees."),
    ("game mode: R / F", "Grow / shrink with your feet where they are (Shift: faster). You grow only where you fit; shrink to walk into a fractal's tunnels."),
    ("game mode: in water", "On the bottom you walk on, however deep (slower, the deeper). With nothing under you, you float chest deep, eyes out, riding the waves: W A S D swim, Space kicks up, and Space looking down dives. Under water you swim where you look (Space swims up), neither sinking nor rising, until your head is out again. Swim into rock or a ledge to scramble out onto it (up to a little over your height)."),
    ("game mode: L, P or F12, F5 / F9, F10, F1", "Headlight, screenshot, quick save / load, record and these keys again work with the interface gone; other commands do nothing until you are back (Tab or Esc)."),
    ("Esc", "Close the help, the file window, Preferences, the Environment window or the material / modifier library; otherwise clear the selection. In game mode: back to editing."),
];

/// Opens the help on `tab` (the window shows when `App::help` is set).
pub(super) fn show_tab(ctx: &egui::Context, tab: Tab) {
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("help_tab"), tab));
}

impl App {
    pub(super) fn help_window(&mut self, ctx: &egui::Context) {
        if !self.help {
            return;
        }
        let tab_id = egui::Id::new("help_tab");
        let mut tab: Tab = ctx.data(|d| d.get_temp(tab_id)).unwrap_or_default();
        let mut close = false;
        // a dialog over the view (Esc or a click beside it closes it); a fixed
        // width and a most height are what keep a modal its own size
        let r = egui::Modal::new(egui::Id::new("help")).backdrop_color(egui::Color32::from_black_alpha(90)).show(ctx, |ui| {
            ui.set_width(820.0);
            let h = (ui.ctx().content_rect().height() * 0.72).min(740.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Help").heading().strong());
                ui.add_space(12.0);
                ui.selectable_value(&mut tab, Tab::Shapes, "Shapes").on_hover_text("Every shape type and its parameters.");
                ui.selectable_value(&mut tab, Tab::Materials, "Materials").on_hover_text("Material settings, the patterns and lights.");
                ui.selectable_value(&mut tab, Tab::Controls, "Controls").on_hover_text("Mouse and keyboard.");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    close = ui.button("×").on_hover_text("Close (Esc, or F1).").clicked();
                });
            });
            ui.separator();
            egui::ScrollArea::vertical().max_height(h).auto_shrink([false, true]).show(ui, |ui| {
                // as tall as the dialog whatever the tab holds
                ui.set_min_height(h - 4.0);
                match tab {
                    Tab::Shapes => shapes(ui),
                    Tab::Materials => materials(ui),
                    Tab::Controls => controls(ui),
                }
            });
        });
        ctx.data_mut(|d| d.insert_temp(tab_id, tab));
        // (Esc is the keyboard's: keys() closes the help first)
        if close || r.backdrop_response.clicked() {
            self.help = false;
        }
    }
}

/// A paragraph. Wrapping is set explicitly: a new dialog is first laid out
/// to measure it, and a plain label measured at its full length there made
/// the dialog as wide as the screen.
fn para(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(text).wrap());
}

/// A section that opens on a click of its name.
fn fold(ui: &mut egui::Ui, id: &str, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    let id = ui.make_persistent_id(("help fold", id));
    let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
    let text = egui::RichText::new(format!("{} {title}", if open { "▾" } else { "▸" })).strong().monospace();
    if ui.add(egui::Button::new(text).frame(false)).clicked() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(id, open));
    }
    if open {
        ui.indent(id, body);
    }
}

/// The mouse and movement, then every command with its keys.
fn controls(ui: &mut egui::Ui) {
    ui.heading("Mouse and movement");
    egui::Grid::new("controls").num_columns(2).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
        for (k, v) in CONTROLS {
            ui.label(egui::RichText::new(*k).strong());
            ui.add(egui::Label::new(*v).wrap());
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    ui.heading("Commands");
    egui::Grid::new("commands").num_columns(3).striped(true).spacing([14.0, 6.0]).show(ui, |ui| {
        for a in actions::ACTIONS {
            ui.label(egui::RichText::new(actions::keys_text(ui.ctx(), a)).strong());
            ui.label(a.name);
            ui.add(egui::Label::new(egui::RichText::new(a.tip).color(ui.visuals().weak_text_color())).wrap());
            ui.end_row();
        }
    });
}

fn shapes(ui: &mut egui::Ui) {
    para(
        ui,
        "A scene is a list of top-level objects. Each is a shape tree: primitives and fractals at the leaves, combinators \
         (union, intersection, difference) and modifiers (repeat, twist, ...) above them. Every node also takes the common fields at the end. \
         Add shapes with Add or + Add; the JSON for each is the type name plus the fields below.",
    );
    let cats = schema::CATEGORIES.iter().flat_map(|&c| std::iter::once(c).chain(if c == "Fractals" { &crate::formulas::CATEGORIES[..] } else { &[] }.iter().copied()));
    for cat in cats {
        ui.add_space(6.0);
        ui.heading(cat);
        for s in schema::shapes().iter().filter(|s| s.category == cat) {
            fold(ui, s.ty, s.ty, |ui| {
                para(ui, s.desc);
                egui::Grid::new(("fields", s.ty)).num_columns(3).striped(true).show(ui, |ui| {
                    ui.label(egui::RichText::new("parameter").strong());
                    ui.label(egui::RichText::new("default").strong());
                    ui.label(egui::RichText::new("meaning").strong());
                    ui.end_row();
                    for (f, d) in s.fields {
                        ui.label(egui::RichText::new(*f).monospace());
                        ui.label(egui::RichText::new(schema::default_of(s.ty, f)).monospace());
                        ui.add(egui::Label::new(*d).wrap());
                        ui.end_row();
                    }
                });
                ui.label(egui::RichText::new(format!("example: {}", s.new)).monospace().small().color(ui.visuals().weak_text_color()));
            });
        }
    }
    ui.add_space(6.0);
    ui.heading("Common fields");
    egui::Grid::new("common").num_columns(2).striped(true).show(ui, |ui| {
        for (f, d) in schema::COMMON {
            ui.label(egui::RichText::new(*f).monospace());
            ui.add(egui::Label::new(*d).wrap());
            ui.end_row();
        }
    });
}

fn materials(ui: &mut egui::Ui) {
    para(
        ui,
        "A material is a set of values (colour, roughness, metal, emission) that a procedural pattern can blend into a second set. \
         Library materials (library/materials.json) are copied into an object when assigned, so each assignment can be edited on its own; \
         scene materials (the file's \"materials\") are shared by name.",
    );
    ui.add_space(6.0);
    ui.heading("Settings");
    egui::Grid::new("mat").num_columns(2).striped(true).show(ui, |ui| {
        for (f, d) in schema::MATERIAL {
            ui.label(egui::RichText::new(*f).monospace());
            ui.add(egui::Label::new(*d).wrap());
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.heading("Patterns");
    egui::Grid::new("patterns").num_columns(2).striped(true).show(ui, |ui| {
        for (name, _, d) in tex::PATTERNS {
            ui.label(egui::RichText::new(name).monospace());
            ui.add(egui::Label::new(d).wrap());
            ui.end_row();
        }
    });
    ui.add_space(6.0);
    ui.heading("Lights");
    para(
        ui,
        "A light is any object whose material has emission: it lights what is around it and casts shadows, softer the bigger it is. \
         + Light (or Add > light) makes a small glowing sphere that floats where you put it (its gravity is off). \
         Older scenes' \"lights\" (point lights) load as such spheres. With the sun level, the ambient level and the sky off, \
         lights are the only light in the scene.",
    );
}
