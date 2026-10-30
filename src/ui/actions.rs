//! The editor's commands: name, keys and description in one table, which the
//! menus, the keyboard and the help screen all read, so a command is called
//! and bound the same everywhere.

use eframe::egui::{self, Key, KeyboardShortcut as Ks, Modifiers};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Act {
    Open,
    Save,
    SaveAs,
    QuickSave,
    QuickLoad,
    Revert,
    Screenshot,
    Undo,
    Redo,
    Duplicate,
    Delete,
    DropAll,
    Reset,
    Frame,
    StartView,
    Panels,
    Heat,
    Device,
    Tool,
    Help,
    Library,
    Modifiers,
    Headlight,
    Game,
    Record,
    Preferences,
}

pub struct Action {
    pub act: Act,
    pub name: &'static str,
    /// the first is the one menus show
    pub keys: &'static [Ks],
    pub tip: &'static str,
}

const fn key(k: Key) -> Ks {
    Ks::new(Modifiers::NONE, k)
}
const fn cmd(k: Key) -> Ks {
    Ks::new(Modifiers::COMMAND, k)
}
const fn shift(k: Key) -> Ks {
    Ks::new(Modifiers::SHIFT, k)
}
const fn cmd_shift(k: Key) -> Ks {
    Ks::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), k)
}

pub const ACTIONS: &[Action] = &[
    Action { act: Act::Open, name: "Open…", keys: &[cmd(Key::O)], tip: "Open another scene file (the folder of the scene open now is listed: saves/ holds every scene and the quick save)." },
    Action { act: Act::Save, name: "Save", keys: &[cmd(Key::S)], tip: "Write the scene, with your edits, back to its file, along with the camera where it is now and every render setting: the file opens as you left it." },
    Action { act: Act::SaveAs, name: "Save as…", keys: &[cmd_shift(Key::S)], tip: "Write the scene, the camera and every render setting to a new file, and go on editing that one." },
    Action {
        act: Act::QuickSave,
        name: "Quick save",
        keys: &[key(Key::F5)],
        tip: "Keep everything as it is now, in one slot (the quick save file in Preferences; saves/quicksave.json by default): the scene with your unsaved edits, the camera exactly where it is and every render setting. F9 brings it back, after a restart too.",
    },
    Action {
        act: Act::QuickLoad,
        name: "Quick load",
        keys: &[key(Key::F9)],
        tip: "Back to the quick save (F5): its scene, camera and render settings. Saving then writes the quick save, not the file it came from. With unsaved edits it asks a second time.",
    },
    Action { act: Act::Revert, name: "Revert", keys: &[cmd(Key::R)], tip: "Load the file again from disk. With unsaved edits it asks a second time, since they would be lost." },
    Action { act: Act::Screenshot, name: "Screenshot", keys: &[key(Key::P), key(Key::F12)], tip: "Save a PNG in the screenshots folder (Preferences; your Pictures folder by default): the 3D view alone, or the whole window with File > Screenshots include the interface." },
    Action {
        act: Act::Record,
        name: "Record video",
        keys: &[key(Key::F10)],
        tip: "Start or stop recording the window as a video in the videos folder (~/Videos; Preferences change it): the interface, the pointer, game mode, whatever shows; with the microphone too when that is on (Preferences > Recording, or the File menu). The title bar shows the time while it records.",
    },
    Action { act: Act::Undo, name: "Undo", keys: &[cmd(Key::Z)], tip: "Take back the last edit (also what the physics moved)." },
    Action { act: Act::Redo, name: "Redo", keys: &[cmd_shift(Key::Z), cmd(Key::Y)], tip: "Do again what Undo took back." },
    Action { act: Act::Duplicate, name: "Duplicate", keys: &[cmd(Key::D)], tip: "Copy the selected object (or part) next to it." },
    Action { act: Act::Delete, name: "Delete", keys: &[key(Key::Delete)], tip: "Remove the selected object or part (a combinator keeps at least one child)." },
    Action {
        act: Act::DropAll,
        name: "Drop all",
        keys: &[],
        tip: "Let every object that can move fall and settle under the physics (and float or sink in water). Locked objects, terrain and water stay put.",
    },
    Action {
        act: Act::Reset,
        name: "Reset scene",
        keys: &[],
        tip: "Put the scene back as it was opened: undoes every edit and everything the physics moved, and returns to the start view. Undo brings it all back.",
    },
    Action { act: Act::Frame, name: "Frame selection", keys: &[key(Key::Period)], tip: "Point the camera at the selected object, from a distance that fits it." },
    Action { act: Act::StartView, name: "Start view", keys: &[key(Key::Home)], tip: "Back to the view the scene was opened or last saved with." },
    Action {
        act: Act::Game,
        name: "Game mode",
        keys: &[key(Key::Tab)],
        tip: "Walk around inside the scene, first person, with the interface gone: W A S D walk, Shift runs, Space jumps (Shift runs; tapped while moving, it keeps running), in water you walk the bottom or float and swim (Space kicks up, looking down dives; under water you swim where you look; swim into a ledge to climb out), the left mouse button is a gravity gun (hold to charge, let go to send what is under the crosshair flying), the right mouse button fires a rocket pack (it builds up; look down to lean forward), R and F grow and shrink you, the mouse looks. You start where the camera is, sized to fit; Tab or Esc comes back here with the camera where you were.",
    },
    Action { act: Act::Panels, name: "Hide panels", keys: &[shift(Key::Tab)], tip: "Hide every panel for a clear view, or show them again." },
    Action { act: Act::Heat, name: "Cost heat map", keys: &[key(Key::H)], tip: "Show how many CPU cycles each pixel took instead of the image: bright = expensive." },
    Action { act: Act::Device, name: "Next device", keys: &[key(Key::G)], tip: "Trace on GPU + CPU, the GPU only, or the CPU only (in turn)." },
    Action { act: Act::Tool, name: "Next handle tool", keys: &[key(Key::T)], tip: "The selection's handles: move, rotate, scale (in turn)." },
    Action { act: Act::Library, name: "Material library", keys: &[key(Key::M)], tip: "Every material on a sphere: click one to give it to the selection; right-click to rename, describe or delete it." },
    Action { act: Act::Modifiers, name: "Modifiers", keys: &[key(Key::K)], tip: "Twist, bend, ripples, bumps, bricks...: each on a sample block; click one to apply it to the selection." },
    Action { act: Act::Headlight, name: "Headlight", keys: &[key(Key::L)], tip: "A light at the camera, on or off: it lights whatever you look at (inside objects, a fractal's crevices). Its strength is in Render > Lighting." },
    Action { act: Act::Help, name: "Help", keys: &[key(Key::F1)], tip: "Every shape, parameter, material setting and control." },
    Action {
        act: Act::Preferences,
        name: "Preferences…",
        keys: &[cmd(Key::Comma)],
        tip: "Settings for every scene: where recordings, screenshots and the quick save go, the microphone and video quality, reloading a changed file, mouse look, the interface's size, the CPU share for rendering. Kept in ~/.config/ray/config.json.",
    },
];

pub fn get(act: Act) -> &'static Action {
    ACTIONS.iter().find(|a| a.act == act).expect("every Act is in ACTIONS")
}

/// The keys of an action as text ("Ctrl+Shift+S / Ctrl+Y").
pub fn keys_text(ctx: &egui::Context, a: &Action) -> String {
    a.keys.iter().map(|k| ctx.format_shortcut(k)).collect::<Vec<_>>().join(" / ")
}

/// The actions whose key went down this frame, with longer chords first (so
/// Ctrl+Shift+S is not taken for Ctrl+S).
pub fn pressed(ctx: &egui::Context) -> Vec<Act> {
    let mut all: Vec<(&Action, &Ks)> = ACTIONS.iter().flat_map(|a| a.keys.iter().map(move |k| (a, k))).collect();
    let weight = |m: Modifiers| m.command as u8 + m.ctrl as u8 + m.shift as u8 + m.alt as u8;
    all.sort_by_key(|(_, k)| std::cmp::Reverse(weight(k.modifiers)));
    let mut out = Vec::new();
    for (a, k) in all {
        // once per press: a held key repeats (held, F9 asked to discard
        // the edits and then said yes itself; P wrote a screenshot a repeat)
        let repeat = ctx.input(|i| {
            i.events.iter().any(|e| matches!(e, egui::Event::Key { key, pressed: true, repeat: true, modifiers, .. } if *key == k.logical_key && modifiers.matches_logically(k.modifiers)))
        });
        if ctx.input_mut(|i| i.consume_shortcut(k)) && !repeat && !out.contains(&a.act) {
            out.push(a.act);
        }
    }
    out
}

/// A command's name for the MCP's run_command: its menu name in snake case
/// ("Record video" -> record_video, "Open…" -> open).
pub fn command_name(a: &Action) -> String {
    a.name.trim_end_matches('…').to_lowercase().replace([' ', '-'], "_")
}

/// The command `name` (as command_name gives it; case and … do not matter).
pub fn by_command(name: &str) -> Option<&'static Action> {
    let want = name.trim().trim_end_matches('…').to_lowercase().replace([' ', '-'], "_");
    ACTIONS.iter().find(|a| command_name(a) == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command has a name of its own for the MCP's run_command, which
    /// finds it again, however it is written.
    #[test]
    fn commands_have_names() {
        let names: Vec<String> = ACTIONS.iter().map(command_name).collect();
        for (i, n) in names.iter().enumerate() {
            assert!(!n.is_empty() && n.chars().all(|c| c.is_ascii_lowercase() || c == '_'), "'{n}'");
            assert!(!names[..i].contains(n), "'{n}' twice");
            assert!(by_command(n).is_some_and(|a| a.act == ACTIONS[i].act), "'{n}' not found again");
        }
        assert!(by_command("Record video").is_some_and(|a| a.act == Act::Record));
        assert!(by_command("open…").is_some_and(|a| a.act == Act::Open));
        assert!(by_command("fly").is_none());
    }
}
