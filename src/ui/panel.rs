//! The render panel: statistics and every render setting, each with a
//! description of what it does on hover; and the heat map's legend.

use super::{App, style};
use crate::settings::{Device, Mode};
use eframe::egui;

fn mode_tip(m: Mode) -> &'static str {
    match m {
        Mode::Direct => "Whitted-style ray tracing: sun with soft shadows, sky light with ambient occlusion, point lights, mirror reflections and refraction. Noise free, so it looks right even while moving.",
        Mode::PathTrace => "Monte Carlo path tracing: light bounces between surfaces (colour bleeding, soft indirect light, glowing objects lighting the room). Noisy while moving; converges as samples accumulate while the view is still.",
        Mode::Normals => "Surface directions as colours (x red, y green, z blue): shows the shape without lighting.",
        Mode::Albedo => "Surface colours without lighting: checks materials and patterns.",
        Mode::Depth => "Distance from the camera: near is bright, far is dark.",
        Mode::Cost => "Each object coloured by what one evaluation of its shape costs (eight points at a time, at the detail this view gives fractals), from cheap blue to dear red on a log scale; the list ranks them by kind. A march pays it at every step, so a costly kind that fills much of the view (or is crossed by many rays) is what slows a scene. It measures the shapes alone: the time a pixel actually took is the cost heat map (H).",
        Mode::Steps => "How many steps each camera ray's search takes, as a heat map: sphere-tracing steps for distance shapes and terrain, Newton iterations for NURBS surfaces (exact spheres, boxes and planes take none). It counts steps, not time: a fractal's step costs far more than a sphere's, and shadows, AO, reflections and clouds are not in it. For what everything costs, use the cost heat map (H).",
    }
}

impl App {
    pub(super) fn render_panel(&mut self, ui: &mut egui::Ui) {
        let st = self.stats;
        style::title(ui, "Render", |_| {});
        style::note(ui, "How the viewer draws the scene. Saving the scene (Ctrl+S) writes these settings into it, with the camera where it is, so it opens as you left it; F5 keeps them all in a quick save.");
        style::section(ui, "stats", "Stats", true, |ui| {
            style::props(ui, "stats", |ui| {
                let stat = |ui: &mut egui::Ui, k: &str, v: String, tip: &str| {
                    style::row(ui, k, tip, |ui| ui.label(v).on_hover_text(tip));
                };
                stat(ui, "render", format!("{:.1} ms", st.trace_ms + st.post_ms), "Time the last frame took on the render thread: trace plus post-processing.");
                stat(ui, "trace / post", format!("{:.1} / {:.1} ms", st.trace_ms, st.post_ms), "Tracing the rays, then bloom, anti-aliasing and tone mapping.");
                stat(ui, "window", format!("{:.1} ms ({:.0} fps)", self.frame_ms, 1000.0 / self.frame_ms.max(0.01)), "Time between redraws of the window. The image updates when the render thread finishes a frame.");
                stat(ui, "resolution", format!("{} x {}", self.render_size.0, self.render_size.1), "Pixels traced per frame (the view size times the resolution scale).");
                stat(ui, "samples", format!("{} / {}", st.samples, self.settings.max_samples), "Samples per pixel averaged so far. Accumulation restarts whenever the view or the scene changes.");
                stat(ui, "camera rays", format!("{:.1} M/s", st.mrays), "Primary rays traced per second (every bounce, shadow and AO ray comes on top).");
                stat(ui, "search steps / ray", format!("{:.1}", st.steps), "Average search steps per camera ray (sphere tracing, terrain marching, NURBS Newton iterations): how hard the geometry is to find, not what it costs (the cost heat map, H, shows that).");
                stat(ui, "threads", format!("{}", rayon::current_num_threads()), "CPU threads the renderer uses (Preferences > Performance: 90% of the cores unless changed there).");
                if st.gpu_share > 0.0 {
                    stat(
                        ui,
                        "GPU / CPU rows",
                        format!("{:.0}% / {:.0}%", st.gpu_share * 100.0, 100.0 - st.gpu_share * 100.0),
                        "How the last frame's rows were shared: the split follows each device's measured speed so both finish together.",
                    );
                    stat(ui, "GPU / CPU time", format!("{:.1} / {:.1} ms", st.gpu_ms, st.cpu_ms), "How long each device took for its share of the last frame (they run at the same time).");
                }
                style::check(
                    ui,
                    "cost per object",
                    "Time every object's work with the CPU's cycle counter, for the cost bars in the scene panel. Measured on the rows the CPU traces (spread over the whole picture); with the device set to GPU alone the bars keep their last values. Slows the CPU's part by roughly 10-15%; turn it off for clean speed measurements.",
                    &mut self.profile,
                );
                style::check(ui, "cost heat map", "Show how many CPU cycles each pixel took instead of the image, with the current lighting mode: bright = expensive (H).", &mut self.heat);
            });
            match &self.gpu_state {
                Some((name, None, _, true)) => {
                    style::note(ui, format!("GPU: {name} (building its shader; the CPU renders meanwhile)"))
                        .on_hover_text("The first time a shader is built takes the graphics driver several seconds; it keeps it, so the next time is quick. The shader changes when the scene gains a new kind of shape tree, or with a new build of ray.");
                }
                Some((name, None, ms, false)) => {
                    style::note(ui, format!("GPU: {name} (shader built in {ms:.0} ms)"))
                        .on_hover_text("The GPU renderer's shader is rebuilt only when the scene gains a new kind of shape tree; changing numbers never rebuilds it.");
                }
                Some((name, Some(e), _, _)) => {
                    ui.colored_label(style::ERROR, format!("GPU ({name}) cannot render this scene: {e}"));
                }
                None => {}
            }
            let sc = &self.scene.stats;
            let mut kinds = Vec::new();
            if sc.analytic > 0 {
                kinds.push(format!("{} exact", sc.analytic));
            }
            if sc.nurbs > 0 {
                kinds.push(format!("{} NURBS ({} patches)", sc.nurbs, sc.patches));
            }
            if sc.sdf > 0 {
                kinds.push(format!("{} distance programs ({} ops)", sc.sdf, sc.sdf_ops));
            }
            if sc.terrains > 0 {
                kinds.push(format!("{} terrain ({:.1} M samples)", sc.terrains, sc.terrain_samples as f32 / 1e6));
            }
            style::note(ui, format!("{} objects: {}", sc.objects, kinds.join(", "))).on_hover_text(
                "Exact objects (plain spheres, boxes, planes) are intersected with formulas; NURBS surfaces are solved exactly on their bicubic patches; everything else is a compiled distance program that is sphere-traced.",
            );
        });

        // known once the render thread has looked for one
        let no_gpu = self.gpu_state.is_none() && self.stats.samples > 0;
        if no_gpu {
            self.settings.device = Device::Cpu;
        }
        let s = &mut self.settings;
        style::section(ui, "render", "Render", true, |ui| {
            style::props(ui, "render", |ui| {
                style::row(ui, "mode", "How the image is computed (keys 1-7).", |ui| {
                    egui::ComboBox::from_id_salt("mode")
                        .selected_text(s.mode.label())
                        .show_ui(ui, |ui| {
                            for m in Mode::ALL {
                                ui.selectable_value(&mut s.mode, m, m.label()).on_hover_text(mode_tip(m));
                            }
                        })
                        .response
                        .on_hover_text(format!("Keys 1-7. Now: {}", mode_tip(s.mode)));
                });
                let devices = [
                    (Device::Both, "GPU + CPU", "Every frame is shared: the GPU and the CPU trace interleaved bands of rows at the same time, the split steered so both finish together."),
                    (Device::Gpu, "GPU", "The GPU traces everything (the CPU only post-processes)."),
                    (Device::Cpu, "CPU", "The CPU traces everything (AVX2, on the share of the cores set in Preferences)."),
                ];
                let tip = "Which processors trace the image (G cycles through them). The cost heat map always uses the CPU (it measures CPU cycles).";
                if let Some(d) = style::row(ui, "device", tip, |ui| style::segmented(ui, s.device, &devices, |d| d == Device::Cpu || !no_gpu)) {
                    s.device = d;
                }
                style::slider(ui, "bounces", "The most times a ray may reflect or refract (direct mode), or the longest light path (path traced). More = deeper mirrors and glass, brighter indirect light, more cost. 0 = no reflections at all.", &mut s.bounces, 0..=16, false);
                style::slider(ui, "resolution", "Render resolution relative to the window. Halving it makes frames about 4x faster; above 1 supersamples (up to about 12 megapixels; beyond that both sides shrink together, so the picture never distorts).", &mut s.scale, 0.1..=2.0, false);
                style::slider(ui, "samples / frame", "Samples per pixel traced each frame. More gives smoother results per frame (anti-aliasing, less noise) at proportional cost.", &mut s.spp, 1..=16, false);
                style::check(ui, "accumulate", "Keep averaging new jittered samples while nothing changes: edges get anti-aliased and path-traced noise fades.", &mut s.accumulate);
                style::slider_if(ui, s.accumulate, "max samples", "Stop accumulating after this many samples per pixel; the CPU then rests until something changes.", &mut s.max_samples, 1..=8192, true);
                style::check(ui, "fast while moving", "While the camera or an object moves: one sample per frame (and the resolution below), so motion stays smooth; full quality returns as soon as everything stops.", &mut s.fast_motion);
                style::slider_if(ui, s.fast_motion, "moving resolution", "Fraction of the resolution used while moving: 0.5 renders a quarter of the pixels.", &mut s.motion_scale, 0.25..=1.0, false);
            });
        });

        let mut to_env = false;
        style::section(ui, "lighting", "Lighting", true, |ui| {
            to_env = ui
                .button("☀ Environment…")
                .on_hover_text("They belong to the scene: Environment, at the top of the scene panel, edits them.")
                .clicked();
            style::props(ui, "lighting", |ui| {
                style::check(ui, "sun shadows", "Trace a ray toward the sun from every lit point.", &mut s.shadows);
                style::check_if(ui, s.shadows, "soft shadows", "Direct mode: estimate soft shadow edges from how closely the shadow ray passes other surfaces (free with distance fields). Path tracing always samples the sun's disk.", &mut s.soft_shadows);
                style::slider_if(ui, s.shadows && s.soft_shadows, "shadow hardness", "Sharpness of soft shadow edges: low = wide penumbras, high = crisp.", &mut s.shadow_hardness, 2.0..=128.0, true);
                style::check(ui, "ambient occlusion", "Direct mode: darken creases and contact points by sampling the scene's distance along the normal (path tracing gets it for free).", &mut s.ao);
                style::slider_if(ui, s.ao, "AO radius", "How far ambient occlusion looks (metres): small = tight creases, large = broad darkening.", &mut s.ao_radius, 0.05..=5.0, true);
                style::check(ui, "glowing objects", "Light from glowing objects: every object whose material has emission lights its surroundings (each glowing part acts as a sphere light of its size).", &mut s.lights);
                style::check_if(ui, s.lights, "their shadows", "Trace a shadow ray toward every glowing object in range (one more ray per light per point).", &mut s.light_shadows);
                style::check(ui, "fog", "Distance haze with a sun-coloured glow toward the sun.", &mut s.fog);
                style::check(ui, "sky clouds", "Noise clouds on a plane high above (cloud objects are separate).", &mut s.clouds);
                style::slider(ui, "headlight", "A light at the camera, like a miner's lamp: it lights whatever you look at, so the inside of an object or a fractal's crevices show (L turns it on and off). 0 = off.", &mut s.headlight, 0.0..=4.0, false);
                style::slider_if(ui, s.headlight > 0.0, "headlight reach", "Metres the headlight reaches: it fades out smoothly to nothing there (a few centimetres inside a fractal, tens of metres in a landscape).", &mut s.headlight_reach, 0.001..=1000.0, true);
                style::check_if(ui, s.headlight > 0.0, "headlight shadows", "The headlight casts hard shadows. A lamp at the eye hides every shadow it casts, so the lamp then hangs a little above the camera (a thirtieth of its reach) and the shadows show below things.", &mut s.headlight_shadows);
                style::check(ui, "twinkling stars", "The stars swell and fade a little (only in the shown image, which goes on refining).", &mut s.twinkle);
                style::check(
                    ui,
                    "animate water",
                    "Let the water's waves move, carrying floating objects with them. The picture then changes every frame, so it never settles into a smooth accumulated image. \
                     Ripples from things moving in the water show either way, and fade within a few seconds.",
                    &mut s.animate_water,
                );
            });
        });

        style::section(ui, "quality", "Quality", false, |ui| {
            style::props(ui, "quality", |ui| {
                style::check(ui, "fractal detail", "Close up, fractals iterate further, as finely as the pixels can show, so zooming in keeps finding detail. Off: always their own iteration counts (faster up close, smoother and less accurate).", &mut s.fractal_detail);
                style::slider(ui, "max march steps", "The most sphere-tracing iterations per object per ray. Too few leaves holes in fractals; more only costs where rays graze surfaces.", &mut s.max_steps, 8..=1024, true);
                style::slider(ui, "hit threshold (px)", "How close (in pixels of screen size) a ray must get to count as a hit. Larger is faster and blurs fractal detail; smaller is sharper and slower.", &mut s.detail, 0.1..=16.0, true);
                style::slider(ui, "max distance (m)", "Rays stop after this many metres (sky beyond).", &mut s.max_dist, 10.0..=20000.0, true);
            });
        });

        style::section(ui, "post", "Post", false, |ui| {
            style::props(ui, "post", |ui| {
                style::slider(ui, "exposure", "Overall brightness, like a camera's exposure.", &mut s.exposure, 0.05..=16.0, true);
                style::check(ui, "tone map (ACES)", "Film-like compression of bright light into the screen's range; off clips it.", &mut s.tonemap);
                style::check(ui, "bloom", "Glow around very bright light (the sun, glints, emissive surfaces).", &mut s.bloom);
                style::check(ui, "edge smoothing", "Smooth jagged edges in a single frame; accumulation takes over once 4 samples are in.", &mut s.edge_aa);
                style::check(ui, "vignette", "Darken the corners slightly.", &mut s.vignette);
            });
        });

        style::section(ui, "camera", "Camera", false, |ui| {
            let c = self.cam;
            style::props(ui, "camera", |ui| {
                style::row(ui, "position", "Where the camera is (metres).", |ui| ui.label(format!("{:.2}, {:.2}, {:.2}", c.pos.x, c.pos.y, c.pos.z)));
                style::row(ui, "direction", "Yaw (0 looks along -z) and pitch, in degrees.", |ui| ui.label(format!("yaw {:.1}°, pitch {:.1}°", c.yaw.to_degrees(), c.pitch.to_degrees())));
                style::slider(ui, "fly speed (m/s)", "WASD speed. [ and ] change it; Shift is 4x faster, Alt 4x slower.", &mut self.speed, 0.01..=5000.0, true);
                style::slider(ui, "field of view", "Vertical field of view in degrees: small = telephoto, large = wide angle.", &mut self.cam.fov, 10.0..=120.0, false);
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Copy as --camera").on_hover_text("Copy the pose as the --camera argument of a headless render.").clicked() {
                    let arg = format!("{:.3},{:.3},{:.3},{:.2},{:.2}", c.pos.x, c.pos.y, c.pos.z, c.yaw.to_degrees(), c.pitch.to_degrees());
                    ui.ctx().copy_text(arg);
                }
            });
        });

        if to_env {
            self.show_env = true;
        }
    }

    /// The heat map's scale: the ramp with its cycle counts.
    pub(super) fn heat_legend(&self, ui: &egui::Ui, rect: egui::Rect) {
        let p = ui.painter();
        let w = 220.0;
        let bar = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(12.0, -34.0), egui::vec2(w, 10.0));
        p.rect_filled(bar.expand(18.0), egui::CornerRadius::same(4), egui::Color32::from_black_alpha(170));
        let n = 44;
        for i in 0..n {
            let x = i as f32 / n as f32;
            let c = crate::render::ramp_rgb(x + 0.5 / n as f32);
            let r = egui::Rect::from_min_size(bar.min + egui::vec2(x * w, 0.0), egui::vec2(w / n as f32 + 0.5, bar.height()));
            p.rect_filled(r, 0.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
        }
        let us = self.stats.heat_max as f64 / crate::render::ticks_per_us();
        let text = ui.visuals().text_color();
        p.text(bar.left_top() - egui::vec2(0.0, 3.0), egui::Align2::LEFT_BOTTOM, "CPU time per pixel", egui::FontId::proportional(12.0), text);
        p.text(bar.left_bottom() + egui::vec2(0.0, 2.0), egui::Align2::LEFT_TOP, "0", egui::FontId::proportional(11.0), text);
        p.text(
            bar.right_bottom() + egui::vec2(0.0, 2.0),
            egui::Align2::RIGHT_TOP,
            format!("{:.1} µs or more", us),
            egui::FontId::proportional(11.0),
            text,
        );
    }

    /// The cost view's key: the ramp from the cheapest object to the dearest,
    /// and the kinds of object ranked by what one evaluation of their shape
    /// costs (the median of each kind).
    pub(super) fn cost_legend(&self, ui: &egui::Ui, rect: egui::Rect) {
        if self.prices.is_empty() || self.prices.len() != self.scene.objects.len() {
            return;
        }
        let mut kinds: Vec<(String, Vec<f32>)> = Vec::new();
        for (o, &p) in self.scene.objects.iter().zip(&self.prices) {
            let label = self.doc.node(o.doc_index, &o.path).map_or_else(|| o.name.clone(), kind_label);
            match kinds.iter_mut().find(|k| k.0 == label) {
                Some(k) => k.1.push(p),
                None => kinds.push((label, vec![p])),
            }
        }
        // (label, how many, median, cheapest, dearest)
        let mut rows: Vec<(String, usize, f32, f32, f32)> = kinds
            .into_iter()
            .map(|(l, mut v)| {
                v.sort_by(f32::total_cmp);
                let n = v.len();
                let median = if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) * 0.5 };
                (l, n, median, v[0], v[n - 1])
            })
            .collect();
        rows.sort_by(|a, b| b.2.total_cmp(&a.2));
        let lo = self.prices.iter().copied().filter(|p| *p > 0.0).fold(f32::INFINITY, f32::min).max(0.1);
        let hi = self.prices.iter().copied().fold(0.0f32, f32::max).max(lo * 1.0001);
        let k0 = crate::render::PRICE_RAMP_START;
        let at = |p: f32| ((p.max(lo) / lo).ln() / (hi / lo).ln()).clamp(0.0, 1.0).mul_add(1.0 - k0, k0);
        const SHOWN: usize = 14;
        let p = ui.painter();
        let (w, row) = (300.0, 16.0);
        let n = rows.len().min(SHOWN);
        let h = 58.0 + row * n as f32 + if rows.len() > SHOWN { row } else { 0.0 };
        let panel = egui::Rect::from_min_size(rect.left_bottom() + egui::vec2(12.0, -12.0 - h), egui::vec2(w, h));
        p.rect_filled(panel, egui::CornerRadius::same(4), egui::Color32::from_black_alpha(185));
        let text = egui::Color32::from_gray(225);
        let small = egui::FontId::proportional(11.0);
        let mut y = panel.top() + 8.0;
        p.text(egui::pos2(panel.left() + 10.0, y), egui::Align2::LEFT_TOP, "One evaluation of each shape (8 points)", egui::FontId::proportional(12.0), text);
        y += 20.0;
        let bar = egui::Rect::from_min_size(egui::pos2(panel.left() + 10.0, y), egui::vec2(w - 20.0, 9.0));
        let steps = 44;
        for i in 0..steps {
            let x = i as f32 / steps as f32;
            let c = crate::render::ramp_rgb((x + 0.5 / steps as f32).mul_add(1.0 - k0, k0));
            let r = egui::Rect::from_min_size(bar.min + egui::vec2(x * bar.width(), 0.0), egui::vec2(bar.width() / steps as f32 + 0.5, bar.height()));
            p.rect_filled(r, 0.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
        }
        p.text(bar.left_bottom() + egui::vec2(0.0, 2.0), egui::Align2::LEFT_TOP, time_text(lo), small.clone(), text);
        p.text(bar.right_bottom() + egui::vec2(0.0, 2.0), egui::Align2::RIGHT_TOP, time_text(hi), small.clone(), text);
        y += 30.0;
        for (label, count, price, cheapest, dearest) in rows.iter().take(SHOWN) {
            let c = crate::render::ramp_rgb(at(*price));
            p.rect_filled(egui::Rect::from_min_size(egui::pos2(panel.left() + 10.0, y + 3.0), egui::vec2(10.0, 10.0)), 2.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
            let name = if *count > 1 { format!("{label}  ×{count}") } else { label.clone() };
            p.text(egui::pos2(panel.left() + 28.0, y), egui::Align2::LEFT_TOP, name, small.clone(), text);
            // a kind whose objects cost quite differently (more iterations,
            // another power): their range
            let price = if dearest > &(cheapest * 1.3) { time_range(*cheapest, *dearest) } else { time_text(*price) };
            p.text(egui::pos2(panel.right() - 10.0, y), egui::Align2::RIGHT_TOP, price, small.clone(), text);
            y += row;
        }
        if rows.len() > SHOWN {
            p.text(egui::pos2(panel.left() + 28.0, y), egui::Align2::LEFT_TOP, format!("… {} more kinds", rows.len() - SHOWN), small, text);
        }
    }
}

/// What an object is, for the cost view's list: its shape's type, with the
/// modifiers it is wrapped in ("menger (slice)"); a combination by its
/// parts ("kleinian ∩ box").
fn kind_label(node: &serde_json::Value) -> String {
    let mut mods = Vec::new();
    let mut n = node;
    loop {
        let t = n.get("type").and_then(serde_json::Value::as_str).unwrap_or("?");
        if let Some(c) = n.get("child") {
            mods.push(t.to_string());
            n = c;
            continue;
        }
        let base = match n.get("children").and_then(serde_json::Value::as_array) {
            Some(kids) if !kids.is_empty() => {
                let join = match t {
                    "intersection" => " ∩ ",
                    "difference" => " − ",
                    _ => " + ",
                };
                let parts: Vec<String> = kids.iter().take(3).map(kind_label).collect();
                let more = if kids.len() > 3 { format!("{join}…") } else { String::new() };
                format!("{}{more}", parts.join(join))
            }
            _ => t.to_string(),
        };
        return if mods.is_empty() { base } else { format!("{base} ({})", mods.join(", ")) };
    }
}

/// Two times as a range, the unit once when they share it ("99–300 ns").
fn time_range(a: f32, b: f32) -> String {
    let (ta, tb) = (time_text(a), time_text(b));
    match (ta.rsplit_once(' '), tb.rsplit_once(' ')) {
        (Some((na, ua)), Some(_)) if tb.ends_with(ua) => format!("{na}–{tb}"),
        _ => format!("{ta}–{tb}"),
    }
}

/// A time as people say it.
fn time_text(ns: f32) -> String {
    match ns {
        n if n >= 1e6 => format!("{:.1} ms", n / 1e6),
        n if n >= 1e3 => format!("{:.1} µs", n / 1e3),
        n => format!("{n:.0} ns"),
    }
}
