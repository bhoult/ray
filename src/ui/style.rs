//! The editor's look and its common rows: one theme, section headers, and
//! the property rows every panel builds from (the name on the left, its
//! editor on the right), so that the same kind of value is edited the same
//! way everywhere: a bounded number with a slider, an open-ended one (a
//! position, a size) by dragging its box, a switch with a checkbox, one of a
//! few choices with buttons side by side.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Pos2, Stroke, TextStyle, Vec2};
use std::ops::RangeInclusive;
use std::sync::Arc;

/// The accent: selections, the cost bars, the active tool.
pub const ACCENT: Color32 = Color32::from_rgb(0x39, 0x87, 0xe5);
pub const WARN: Color32 = Color32::from_rgb(0xe6, 0xa2, 0x3c);
pub const ERROR: Color32 = Color32::from_rgb(0xe0, 0x4f, 0x4f);
/// The width of the name column of property rows.
pub const LABEL_W: f32 = 118.0;

/// Regular-weight UI fonts from the system, ahead of egui's light default
/// (which stays behind them, with the emoji and symbol fallbacks): Noto Sans
/// for the text, DejaVu Sans for the symbols it lacks (arrows, ✕, ☀...).
const FONTS: &[(&str, &str)] = &[("ui", "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf"), ("ui symbols", "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")];

pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let mut at = 0;
    for (name, path) in FONTS {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(name.to_string(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts.families.entry(FontFamily::Proportional).or_default().insert(at, name.to_string());
            at += 1;
        }
    }
    if at > 0 {
        ctx.set_fonts(fonts);
    }
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(11.0)),
            (TextStyle::Body, FontId::proportional(13.0)),
            (TextStyle::Button, FontId::proportional(13.0)),
            (TextStyle::Heading, FontId::proportional(17.0)),
            (TextStyle::Monospace, FontId::monospace(12.5)),
        ]
        .into();
        let sp = &mut s.spacing;
        sp.item_spacing = Vec2::new(7.0, 5.0);
        sp.button_padding = Vec2::new(7.0, 3.0);
        sp.interact_size.y = 21.0;
        sp.indent = 16.0;
        sp.text_edit_width = 180.0;
        sp.menu_margin = egui::Margin::same(6);
        let v = &mut s.visuals;
        *v = egui::Visuals::dark();
        v.panel_fill = Color32::from_rgb(0x1d, 0x1f, 0x23);
        v.window_fill = Color32::from_rgb(0x24, 0x26, 0x2b);
        v.extreme_bg_color = Color32::from_rgb(0x14, 0x15, 0x18);
        v.faint_bg_color = Color32::from_rgb(0x22, 0x24, 0x29);
        v.window_stroke = Stroke::new(1.0, Color32::from_rgb(0x3a, 0x3d, 0x44));
        v.window_corner_radius = CornerRadius::same(8);
        v.menu_corner_radius = CornerRadius::same(6);
        v.selection.bg_fill = ACCENT.linear_multiply(0.55);
        v.selection.stroke = Stroke::new(1.0, Color32::from_rgb(0xd8, 0xe8, 0xff));
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARN;
        v.error_fg_color = ERROR;
        for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            w.corner_radius = CornerRadius::same(4);
        }
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x30, 0x33, 0x39));
        v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(0xb4, 0xb8, 0xc0);
        v.widgets.inactive.fg_stroke.color = Color32::from_rgb(0xd2, 0xd5, 0xdb);
        v.widgets.inactive.weak_bg_fill = Color32::from_rgb(0x2c, 0x2f, 0x35);
        v.widgets.inactive.bg_fill = Color32::from_rgb(0x34, 0x37, 0x3e);
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x38, 0x3c, 0x44);
        v.widgets.hovered.bg_fill = Color32::from_rgb(0x40, 0x44, 0x4d);
        v.widgets.active.weak_bg_fill = ACCENT.linear_multiply(0.7);
        v.widgets.active.bg_fill = ACCENT.linear_multiply(0.8);
        v.slider_trailing_fill = true;
        v.striped = false;
    });
}

/// A panel's title, with optional buttons to its right.
pub fn title(ui: &mut egui::Ui, text: &str, right: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text).heading().strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right);
    });
    ui.add_space(2.0);
}

/// A section of a panel: a header that folds it away.
pub fn section<R>(ui: &mut egui::Ui, id: impl std::hash::Hash, text: &str, open: bool, body: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    ui.add_space(4.0);
    egui::CollapsingHeader::new(egui::RichText::new(text).strong().color(ui.visuals().strong_text_color()))
        .id_salt(id)
        .default_open(open)
        .show(ui, body)
        .body_returned
}

/// Text that explains, dimmer than the values around it.
pub fn note(ui: &mut egui::Ui, text: impl Into<String>) -> egui::Response {
    ui.add(egui::Label::new(egui::RichText::new(text).small().color(ui.visuals().weak_text_color())).wrap())
}

/// A grid of property rows (`row`, `slider`, `check`, `choice`), its
/// sliders as wide as the panel allows.
pub fn props<R>(ui: &mut egui::Ui, id: impl std::hash::Hash, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let w = ui.available_width();
    ui.scope(|ui| {
        // set before the grid: inside it the cells' widths come from the last frame
        // (room for the value box too: a row wider than the panel would widen
        // it, and the next frame's sliders with it)
        ui.spacing_mut().slider_width = (w - LABEL_W - 104.0).clamp(60.0, 260.0);
        egui::Grid::new(id).num_columns(2).min_col_width(LABEL_W).spacing([10.0, 5.0]).show(ui, body).inner
    })
    .inner
}

/// A property's name: words, not the file's snake_case.
pub fn pretty(key: &str) -> String {
    key.replace('_', " ")
}

/// One property row: its name (with `tip` on hover), then what `add` puts
/// beside it.
pub fn row<R>(ui: &mut egui::Ui, name: &str, tip: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let l = ui.label(name);
    if !tip.is_empty() {
        l.on_hover_text(tip);
    }
    let r = add(ui);
    ui.end_row();
    r
}

/// A bounded number: a slider with its value box.
pub fn slider<N: egui::emath::Numeric>(ui: &mut egui::Ui, name: &str, tip: &str, v: &mut N, range: RangeInclusive<N>, log: bool) -> bool {
    row(ui, name, tip, |ui| ui.add(egui::Slider::new(v, range).logarithmic(log)).on_hover_text(tip).changed())
}

/// `slider`, greyed out unless `on`.
pub fn slider_if<N: egui::emath::Numeric>(ui: &mut egui::Ui, on: bool, name: &str, tip: &str, v: &mut N, range: RangeInclusive<N>, log: bool) -> bool {
    row(ui, name, tip, |ui| ui.add_enabled(on, egui::Slider::new(v, range).logarithmic(log)).on_hover_text(tip).changed())
}

/// A switch.
pub fn check(ui: &mut egui::Ui, name: &str, tip: &str, v: &mut bool) -> bool {
    row(ui, name, tip, |ui| ui.checkbox(v, "").on_hover_text(tip).changed())
}

/// `check`, greyed out unless `on`.
pub fn check_if(ui: &mut egui::Ui, on: bool, name: &str, tip: &str, v: &mut bool) -> bool {
    row(ui, name, tip, |ui| ui.add_enabled(on, egui::Checkbox::without_text(v)).on_hover_text(tip).changed())
}

/// One of a few choices, as buttons side by side: (value, name, tip) each.
/// Returns the one clicked, if it differs from `current`.
pub fn choice<T: PartialEq + Copy>(ui: &mut egui::Ui, name: &str, tip: &str, current: T, options: &[(T, &str, &str)]) -> Option<T> {
    row(ui, name, tip, |ui| segmented(ui, current, options, |_| true))
}

/// The buttons of `choice`, on their own; `enabled` can grey some out.
pub fn segmented<T: PartialEq + Copy>(ui: &mut egui::Ui, current: T, options: &[(T, &str, &str)], enabled: impl Fn(T) -> bool) -> Option<T> {
    let mut picked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for &(v, name, tip) in options {
            let r = ui.add_enabled(enabled(v), egui::Button::selectable(current == v, name)).on_hover_text(tip);
            if r.clicked() && current != v {
                picked = Some(v);
            }
        }
    });
    picked
}

/// A colour (linear RGB 0..1).
pub fn colour(ui: &mut egui::Ui, name: &str, tip: &str, c: &mut [f32; 3]) -> bool {
    row(ui, name, tip, |ui| ui.color_edit_button_rgb(c).on_hover_text(tip).changed())
}

/// A row of a list (the scene's objects): `width` wide, highlighted when
/// selected or hovered, its text clipped to it, `kind` after it in a smaller,
/// dimmer face; `dim` greys the text (a hidden object).
pub fn list_row(ui: &mut egui::Ui, selected: bool, text: &str, kind: &str, width: f32, dim: bool) -> egui::Response {
    let h = ui.spacing().interact_size.y;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width.max(24.0), h), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        let v = ui.visuals();
        let fill = if selected {
            v.selection.bg_fill
        } else if resp.hovered() {
            v.widgets.hovered.weak_bg_fill
        } else {
            Color32::TRANSPARENT
        };
        let p = ui.painter_at(rect);
        p.rect_filled(rect, CornerRadius::same(4), fill);
        let col = if dim {
            v.weak_text_color()
        } else if selected {
            v.strong_text_color()
        } else {
            v.text_color()
        };
        let g = p.layout_no_wrap(text.to_string(), TextStyle::Body.resolve(ui.style()), col);
        let x = rect.left() + 6.0;
        let tw = g.size().x;
        p.galley(Pos2::new(x, rect.center().y - g.size().y * 0.5), g, col);
        if !kind.is_empty() {
            let weak = v.weak_text_color();
            let g = p.layout_no_wrap(kind.to_string(), TextStyle::Small.resolve(ui.style()), weak);
            p.galley(Pos2::new(x + tw + 6.0, rect.center().y - g.size().y * 0.5), g, weak);
        }
    }
    resp
}
