//! ray's preferences: what holds for every scene (where recordings and
//! screenshots go, the microphone, the mouse, the interface's size...), kept
//! in ~/.config/ray/config.json and edited in the Preferences window (File >
//! Preferences, Ctrl+,). A scene's render settings stay in the scene.

use super::{App, style};
use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// where recordings (F10) go: the desktop's Videos folder by default
    pub videos: String,
    /// recordings take in the system's default microphone
    pub microphone: bool,
    pub video_fps: u32,
    /// the encoder's quality: lower is better and bigger (NVENC's cq, x264's crf)
    pub video_quality: u32,
    /// steady recordings: of the view, every frame drawn whole, the scene
    /// 1/fps on from the one before however long it takes (smooth video of a
    /// slow scene, recorded slower than it plays; no sound), with this many
    /// samples a pixel (0: until the frame converges)
    pub video_steady: bool,
    pub video_steady_samples: u32,
    /// steady recordings' frame size, pixels, whatever the window's ([0, 0]:
    /// the view's, at full resolution)
    pub video_size: [u32; 2],
    /// where screenshots (P, F12) go: a ray folder in the desktop's Pictures
    /// folder by default
    pub screenshots: String,
    /// screenshots are the whole window as drawn (panels, menus), not the
    /// 3D view alone
    pub screenshot_interface: bool,
    /// screenshots of the view are rendered whole first, as a steady
    /// recording's frames are: this many samples a pixel (0: until the image
    /// converges), at this size ([0, 0]: the view's, at full resolution)
    pub screenshot_samples: u32,
    pub screenshot_size: [u32; 2],
    /// the quick save slot (F5, F9)
    pub quick_save: String,
    /// mouse look, times the usual speed; and up and down the other way
    pub look_speed: f32,
    pub invert_look: bool,
    /// the interface's size (1: as the desktop has it)
    pub ui_scale: f32,
    /// a scene changed on disk without unsaved edits is loaded again
    pub auto_reload: bool,
    /// the share of the CPU's cores the renderer uses (from the next start)
    pub cpu_share: f32,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            videos: desktop_dir("VIDEOS", "Videos"),
            microphone: true,
            video_fps: 30,
            video_quality: 21,
            video_steady: false,
            video_steady_samples: 16,
            video_size: [1920, 1080],
            screenshots: format!("{}/ray", desktop_dir("PICTURES", "Pictures")),
            screenshot_interface: false,
            screenshot_samples: 256,
            screenshot_size: [0, 0],
            quick_save: "saves/quicksave.json".into(),
            look_speed: 1.0,
            invert_look: false,
            ui_scale: 1.0,
            auto_reload: true,
            cpu_share: 0.9,
        }
    }
}

impl Config {
    /// Where it is kept: $XDG_CONFIG_HOME/ray/config.json (~/.config/ray).
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"));
        base.join("ray").join("config.json")
    }

    /// The preferences (the defaults where there are none yet), and what was
    /// wrong with the file if it could not be read.
    pub fn load() -> (Config, Option<String>) {
        Self::load_from(&Self::path())
    }

    /// load, from file `p`.
    fn load_from(p: &std::path::Path) -> (Config, Option<String>) {
        match std::fs::read_to_string(p) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(c) => (c, None),
                // (kept aside first: the next change saves the defaults over it)
                Err(e) => {
                    let bad = p.with_extension("json.bad");
                    let kept = std::fs::write(&bad, &text).map_or(String::new(), |_| format!("; it is kept as {}", bad.display()));
                    (Config::default(), Some(format!("{}: {e} (using the defaults{kept})", p.display())))
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            // (there but unreadable: say so; saving over it will fail the same way)
            Err(e) => (Config::default(), Some(format!("{}: {e} (using the defaults)", p.display()))),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to(&Self::path())
    }

    /// save, to file `p`.
    fn save_to(&self, p: &std::path::Path) -> Result<(), String> {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        // (to a new file, then over the old one: a crash midway leaves one or
        // the other whole)
        let tmp = p.with_extension("json.new");
        std::fs::write(&tmp, text + "\n").and_then(|_| std::fs::rename(&tmp, p)).map_err(|e| format!("{}: {e}", p.display()))
    }
}

/// The user's home folder.
fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

/// A folder as written in the preferences: `~` is the home folder; a
/// relative one is from where ray was started (its own folder).
pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix('~') {
        Some(rest) => home().join(rest.trim_start_matches('/')),
        None => PathBuf::from(p),
    }
}

/// The desktop's folder of a kind (VIDEOS, PICTURES...: user-dirs.dirs, as
/// xdg-user-dir reads it, without starting it at every launch), as `~/...`
/// when it is in the home folder; else ~/`fallback`.
fn desktop_dir(kind: &str, fallback: &str) -> String {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home().join(".config"));
    desktop_dir_in(std::fs::read_to_string(base.join("user-dirs.dirs")).ok().as_deref(), kind, fallback)
}

/// desktop_dir from the text of user-dirs.dirs (None: there is none).
fn desktop_dir_in(dirs: Option<&str>, kind: &str, fallback: &str) -> String {
    let key = format!("XDG_{kind}_DIR=");
    let found = dirs.and_then(|text| text.lines().find_map(|l| l.trim().strip_prefix(&key).map(|v| v.trim().trim_matches('"').replace("$HOME", "~"))));
    match found {
        // (a folder set to the home folder itself means "none")
        Some(d) if !d.is_empty() && d != "~" && d != "~/" => d,
        _ => format!("~/{fallback}"),
    }
}

impl App {
    /// The Preferences window: every global setting, saved as it changes.
    pub(super) fn preferences_window(&mut self, ctx: &egui::Context) {
        if !self.show_prefs {
            return;
        }
        let mut open = true;
        egui::Window::new("⚙ Preferences")
            .open(&mut open)
            .default_width(460.0)
            .default_height(560.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| self.preferences(ui));
            });
        // (any change is saved by update: App::config_saved)
        self.show_prefs = open;
    }

    fn preferences(&mut self, ui: &mut egui::Ui) {
        style::note(ui, format!("For every scene; kept in {} (a scene's render settings are in the Render panel and saved with the scene).", Config::path().display()));
        if let Some(e) = &self.config_error {
            ui.colored_label(style::ERROR, e);
        }
        let c = &mut self.config;
        style::section(ui, "prefs recording", "Recording (F10)", true, |ui| {
            style::props(ui, "prefs recording", |ui| {
                folder(ui, "videos folder", "Where recordings go (~ is your home folder). Created when needed.", &mut c.videos);
                style::check(ui, "microphone", "Recordings take in the system's default microphone (Settings > Sound > Input), for narrating.", &mut c.microphone);
                style::slider(ui, "frame rate", "Frames a second in the video. The window's own frame rate is what it is: slower frames are shown for longer.", &mut c.video_fps, 15..=60, false);
                style::slider(ui, "quality", "The encoder's quality: lower is sharper and the file bigger (21 is very good; 28 is smaller and softer).", &mut c.video_quality, 15..=35, false);
                style::check(ui, "steady", "A recording of the view (not the window: no panels, crosshair or pointer) in which every frame is drawn whole and shows the scene one frame later than the one before, however long it takes to draw: a slow scene plays smoothly, recorded slower than it plays (the window runs slow meanwhile). No sound.", &mut c.video_steady);
                if c.video_steady {
                    let sizes: [(&str, [u32; 2]); 5] = [("the view's", [0, 0]), ("1280 x 720", [1280, 720]), ("1920 x 1080", [1920, 1080]), ("2560 x 1440", [2560, 1440]), ("3840 x 2160", [3840, 2160])];
                    let shown = sizes.iter().find(|(_, v)| *v == c.video_size).map_or_else(|| format!("{} x {}", c.video_size[0], c.video_size[1]), |(n, _)| n.to_string());
                    style::row(ui, "size", "Steady recordings: the frames' size, whatever the window's (the view's: as big as the view is, at full resolution).", |ui| {
                        egui::ComboBox::from_id_salt("video size").selected_text(shown).show_ui(ui, |ui| {
                            for (name, v) in sizes {
                                ui.selectable_value(&mut c.video_size, v, name);
                            }
                        });
                    });
                    style::slider(ui, "samples a frame", "Steady recordings: the samples a pixel each frame waits for (more is smoother and slower to record; 16 smooths the edges; path tracing wants hundreds; 0: until the frame converges at the render settings' max samples).", &mut c.video_steady_samples, 0..=1024, true);
                }
            });
        });
        style::section(ui, "prefs files", "Files", true, |ui| {
            style::props(ui, "prefs files", |ui| {
                folder(ui, "screenshots folder", "Where screenshots (P, F12) go: a ray folder in your Pictures folder unless set otherwise (~ is your home folder; a relative folder is from where ray was started: ray's own folder from the desktop launcher).", &mut c.screenshots);
                style::check(ui, "screenshots include the interface", "Screenshots are the whole window as you see it (panels, menus, the game-mode gauge), not the 3D view alone. Also in the File menu.", &mut c.screenshot_interface);
                if !c.screenshot_interface {
                    let sizes: [(&str, [u32; 2]); 5] = [("the view's", [0, 0]), ("1280 x 720", [1280, 720]), ("1920 x 1080", [1920, 1080]), ("2560 x 1440", [2560, 1440]), ("3840 x 2160", [3840, 2160])];
                    let shown = sizes.iter().find(|(_, v)| *v == c.screenshot_size).map_or_else(|| format!("{} x {}", c.screenshot_size[0], c.screenshot_size[1]), |(n, _)| n.to_string());
                    style::row(ui, "screenshot size", "Screenshots are rendered whole at this size, whatever the window's (the view's: as big as the view is, at full resolution).", |ui| {
                        egui::ComboBox::from_id_salt("screenshot size").selected_text(shown).show_ui(ui, |ui| {
                            for (name, v) in sizes {
                                ui.selectable_value(&mut c.screenshot_size, v, name);
                            }
                        });
                    });
                    style::slider(ui, "screenshot samples", "A screenshot waits for this many samples a pixel before it is saved, the scene held still meanwhile (more is smoother and slower; path tracing wants hundreds; 0: until the image converges at the render settings' max samples).", &mut c.screenshot_samples, 0..=4096, true);
                }
                style::row(ui, "quick save file", "The slot F5 saves to and F9 loads: the scene, the camera and every setting (~ is your home folder; a relative path is from where ray was started).", |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut c.quick_save).desired_width(200.0));
                        if ui.small_button("📁").on_hover_text("Open the folder it is in").clicked() {
                            let p = expand(&c.quick_save);
                            let dir = p.parent().map(PathBuf::from).unwrap_or_default();
                            let _ = std::fs::create_dir_all(&dir);
                            super::open_externally(&std::fs::canonicalize(&dir).unwrap_or(dir));
                        }
                    });
                });
                style::check(ui, "reload on change", "Load a scene again when its file changes on disk (while it has no unsaved edits).", &mut c.auto_reload);
            });
        });
        style::section(ui, "prefs mouse", "Mouse", true, |ui| {
            style::props(ui, "prefs mouse", |ui| {
                style::slider(ui, "look speed", "How fast the view turns with the mouse (looking around and in game mode), times the usual.", &mut c.look_speed, 0.2..=4.0, true);
                style::check(ui, "invert up / down", "Moving the mouse up looks down, as in a flight simulator.", &mut c.invert_look);
            });
        });
        style::section(ui, "prefs interface", "Interface", true, |ui| {
            style::props(ui, "prefs interface", |ui| {
                style::slider(ui, "size", "How big the menus, panels and text are (1: as the desktop has it).", &mut c.ui_scale, 0.6..=2.5, false);
            });
        });
        style::section(ui, "prefs performance", "Performance", true, |ui| {
            style::props(ui, "prefs performance", |ui| {
                let tip = "The share of the CPU's cores the renderer may use; the rest keeps the interface and the desktop smooth. Takes effect when ray next starts.";
                let mut pct = c.cpu_share * 100.0;
                if style::slider(ui, "CPU for rendering (%)", tip, &mut pct, 10.0..=100.0, false) {
                    c.cpu_share = (pct / 100.0).clamp(0.1, 1.0);
                }
            });
        });
        ui.add_space(8.0);
        // (every preference at once, with no undo: it asks twice)
        let armed = self.prefs_reset_armed.is_some_and(|t| t.elapsed().as_secs_f32() < 4.0);
        let label = if armed { "Click again to reset every preference" } else { "Back to the defaults" };
        if ui.button(label).on_hover_text("Every preference as ray first had it.").clicked() {
            if armed {
                self.config = Config::default();
                self.prefs_reset_armed = None;
            } else {
                self.prefs_reset_armed = Some(std::time::Instant::now());
            }
        }
    }
}

/// A folder's row: its path to edit, and a button to open it.
fn folder(ui: &mut egui::Ui, name: &str, tip: &str, v: &mut String) {
    style::row(ui, name, tip, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(v).desired_width(200.0)).on_hover_text(tip);
            if ui.small_button("📁").on_hover_text("Open it").clicked() {
                let p = expand(v);
                let _ = std::fs::create_dir_all(&p);
                super::open_externally(&std::fs::canonicalize(&p).unwrap_or(p));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own (removed when done): tests run at once,
    /// so none of them reads or writes ~/.config/ray.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(name: &str) -> Scratch {
            let d = std::env::temp_dir().join(format!("ray-test-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            Scratch(d)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Saved and read back, every preference is the same; a missing file is
    /// the defaults, quietly; a file with only some is the defaults for the rest.
    #[test]
    fn preferences_keep() {
        let d = Scratch::new("keep");
        let p = d.0.join("ray").join("config.json");
        assert_eq!(Config::load_from(&p), (Config::default(), None));
        let c = Config { videos: "~/clips".into(), microphone: false, screenshot_interface: true, look_speed: 2.5, invert_look: true, ..Config::default() };
        c.save_to(&p).unwrap();
        assert_eq!(Config::load_from(&p), (c, None));
        assert!(!p.with_extension("json.new").exists(), "the new file replaced the old");
        std::fs::write(&p, r#"{ "screenshots": "~/shots" }"#).unwrap();
        let (some, err) = Config::load_from(&p);
        assert!(err.is_none() && some.screenshots == "~/shots" && some.video_fps == Config::default().video_fps);
    }

    /// A file that does not read as preferences is kept aside as
    /// config.json.bad (the next change saves the defaults over it) and said;
    /// one that cannot be read at all is said too, not quietly the defaults.
    #[test]
    fn a_bad_file_is_kept_and_said() {
        let d = Scratch::new("bad");
        let p = d.0.join("config.json");
        std::fs::write(&p, "{ not json").unwrap();
        let (c, err) = Config::load_from(&p);
        assert_eq!(c, Config::default());
        assert!(err.as_deref().is_some_and(|e| e.contains("config.json.bad")), "{err:?}");
        assert_eq!(std::fs::read_to_string(p.with_extension("json.bad")).unwrap(), "{ not json");
        let dir = d.0.join("dir.json");
        std::fs::create_dir_all(&dir).unwrap();
        assert!(Config::load_from(&dir).1.is_some(), "a folder where the file should be is said");
    }

    /// The desktop's folders, as user-dirs.dirs names them; ~ for the home
    /// folder; one set to the home folder itself (none) or not named at all
    /// is the usual one. Screenshots go in a ray folder inside Pictures.
    #[test]
    fn the_desktops_folders() {
        assert!(Config::default().screenshots.ends_with("/ray"));
        let dirs = "# comment\nXDG_PICTURES_DIR=\"$HOME/Bilder\"\nXDG_VIDEOS_DIR=\"$HOME/\"\n";
        assert_eq!(desktop_dir_in(Some(dirs), "PICTURES", "Pictures"), "~/Bilder");
        assert_eq!(desktop_dir_in(Some(dirs), "VIDEOS", "Videos"), "~/Videos");
        assert_eq!(desktop_dir_in(Some(dirs), "MUSIC", "Music"), "~/Music");
        assert_eq!(desktop_dir_in(None, "PICTURES", "Pictures"), "~/Pictures");
        assert_eq!(expand("~/x/y"), home().join("x/y"));
        assert_eq!(expand("shots"), PathBuf::from("shots"));
    }
}
