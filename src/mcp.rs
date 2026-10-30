//! `ray mcp`: an MCP server (Model Context Protocol, over stdin / stdout)
//! that lets an AI assistant work in the running viewer: read the scene,
//! add, change and remove objects, set the lights, sky and render settings,
//! move the camera, cast rays, drop things with the physics, and look at the
//! result. Each tool call becomes one command on the viewer's control
//! socket (ui/remote.rs, carried out by ui/control.rs).
//!
//! Stdout carries only the protocol (one JSON-RPC message per line); notes
//! go to stderr.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// The viewer's control socket: RAY_SOCKET, else in the runtime directory.
pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RAY_SOCKET").filter(|p| !p.is_empty()) {
        return p.into();
    }
    match std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        Some(d) => PathBuf::from(d).join("ray-viewer.sock"),
        None => std::env::temp_dir().join(format!("ray-viewer-{}.sock", std::env::var("USER").unwrap_or_default())),
    }
}

const INSTRUCTIONS: &str = "Tools for the ray viewer: a real-time ray tracer of mesh-free scenes (signed distance shapes, NURBS surfaces, fractals, terrain, water) with rigid-body physics. \
Start with viewer_status (launch_viewer if none is running) and describe_scene_format, which lists every shape, its fields and the conventions. \
Build things out of primitives with add_object: one \"union\" node whose \"children\" are the parts keeps a compound object (a vehicle, a house) together. \
Use raycast with x and z to find the ground's height before placing something on it, then screenshot to look at the result and adjust. \
Edits are checked by compiling the scene: a bad node is refused with the reason, and every edit can be undone.";

/// run_command's description: every command of the editor's menus and keys,
/// by name (from the same table, so the two never disagree).
fn commands_text() -> &'static str {
    static TEXT: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TEXT.get_or_init(|| {
        let names: Vec<String> = crate::ui::actions::ACTIONS.iter().map(crate::ui::actions::command_name).collect();
        format!(
            "Run one of the editor's commands, as its menu entry or key does (toggles toggle): {}. Returns the viewer's status; a command that cannot run now (undo with nothing to undo) says why.",
            names.join(", ")
        )
    })
}

/// The tools: name, description, input schema, and the viewer command.
fn tools() -> Vec<(&'static str, &'static str, Value, &'static str)> {
    let object = json!({"description": "The object: its index in the scene's object list, or its name.", "oneOf": [{"type": "integer"}, {"type": "string"}]});
    let vec3 = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    let set = |what: &str| json!({"type": "object", "description": format!("Fields to set on {what}; an object merges into an object, null removes a field.")});
    vec![
        ("viewer_status", "Whether a viewer is running, its scene file, unsaved edits, object count, samples gathered, device, the last frame's render time (render_ms, and the GPU's and CPU's parts), the window's frame time, and what is still moving.", json!({"type": "object", "properties": {}}), "status"),
        (
            "launch_viewer",
            "Start the viewer (a window on the user's desktop) when none is running, optionally with a scene file (default saves/showcase.json). Waits until it takes commands.",
            json!({"type": "object", "properties": {"scene": {"type": "string", "description": "Path of a scene .json, e.g. saves/water.json"}}}),
            "launch",
        ),
        ("describe_scene_format", "The scene format: conventions (units, axes, rotation order, compound objects), every shape type with its fields and an example, the fields every object has, and the material fields. Read this before building.", json!({"type": "object", "properties": {}}), "describe"),
        ("list_materials", "The scene's named materials and the material library's definitions (to copy into a node, or to name with set_material).", json!({"type": "object", "properties": {}}), "materials"),
        ("get_scene", "The whole scene document (JSON), the camera and the render settings.", json!({"type": "object", "properties": {}}), "scene"),
        ("list_objects", "The scene's objects: index, name, type, position, rotation, scale, material and world bounds.", json!({"type": "object", "properties": {}}), "objects"),
        (
            "add_object",
            "Add an object (a scene node as JSON, e.g. {\"type\":\"box\",\"size\":[1,0.5,2],\"position\":[0,1,0],\"material\":{\"color\":[0.8,0.1,0.1]}}). A \"union\" node with \"children\" makes a compound object. drop: let it fall and settle with the physics. Returns its index.",
            json!({"type": "object", "properties": {"node": {"type": "object"}, "drop": {"type": "boolean"}}, "required": ["node"]}),
            "add",
        ),
        (
            "update_object",
            "Change an object's fields (position, rotation, scale, size, material, children...). The object stays where it is put (no physics) unless drop is true.",
            json!({"type": "object", "properties": {"object": object, "set": set("the object"), "remove": {"type": "array", "items": {"type": "string"}}, "drop": {"type": "boolean"}}, "required": ["object"]}),
            "update",
        ),
        (
            "add_objects",
            "Add many objects at once (a list of scene nodes): one compile for all of them, where add_object would recompile per object. Returns the first new index.",
            json!({"type": "object", "properties": {"nodes": {"type": "array", "items": {"type": "object"}}}, "required": ["nodes"]}),
            "add_many",
        ),
        ("delete_object", "Remove an object.", json!({"type": "object", "properties": {"object": object}, "required": ["object"]}), "delete"),
        (
            "select_object",
            "Select an object in the viewer (it is outlined and shows its handles), optionally with the handle tool: move, rotate or scale.",
            json!({"type": "object", "properties": {"object": object, "tool": {"type": "string", "enum": ["move", "rotate", "scale"]}}, "required": ["object"]}),
            "select",
        ),
        (
            "set_material",
            "Add or replace a named scene material (objects use it with \"material\": \"name\").",
            json!({"type": "object", "properties": {"name": {"type": "string"}, "definition": {"type": "object", "description": "Material fields: color, roughness, metallic, transmission, ior, emission, texture... (see describe_scene_format)."}}, "required": ["name", "definition"]}),
            "material",
        ),
        (
            "set_environment",
            "Change the scene's environment: preset, time_of_day, sun_azimuth / sun_elevation, sun_level, ambient_level, sky (day, stars, off), clouds, fog_density, gravity (m/s²: 9.81 Earth, 3.71 Mars, 1.62 Moon, 0 none)...",
            json!({"type": "object", "properties": {"set": set("the environment")}, "required": ["set"]}),
            "environment",
        ),
        (
            "set_render_settings",
            "Change the viewer's render settings (the right panel): mode (direct, path_trace, normals, albedo, depth, steps, cost), bounces, spp, device (both, gpu, cpu), scale (resolution), shadows, ao, bloom, exposure, headlight, animate_water... get_scene shows them all.",
            json!({"type": "object", "properties": {"set": set("the settings")}, "required": ["set"]}),
            "settings",
        ),
        (
            "set_camera",
            "Move the viewer's camera: position and look_at (points), or yaw / pitch in degrees (yaw 0 looks along -z), and the vertical field of view.",
            json!({"type": "object", "properties": {"position": vec3, "look_at": vec3, "yaw_degrees": {"type": "number"}, "pitch_degrees": {"type": "number"}, "fov_degrees": {"type": "number"}}}),
            "camera",
        ),
        (
            "raycast",
            "Cast a ray into the scene: with x and z, straight down from high above (the ground's height there); or from origin along direction. Returns the hit point, normal, distance and object.",
            json!({"type": "object", "properties": {"x": {"type": "number"}, "z": {"type": "number"}, "origin": vec3, "direction": vec3}}),
            "raycast",
        ),
        (
            "screenshot",
            "The viewer's current image (the 3D view, without the panels) as a PNG. Waits until the image shows the latest changes and has gathered `samples` samples (or `timeout` seconds pass).",
            json!({"type": "object", "properties": {"width": {"type": "integer", "description": "Scaled down to this width (default 800)."}, "samples": {"type": "integer", "description": "Wait for this many samples per pixel (default 16)."}, "timeout": {"type": "number", "description": "Seconds to wait at most (default 15)."}}}),
            "screenshot",
        ),
        ("drop_all", "Let every movable object fall and settle with the physics (locked objects, terrain and water stay).", json!({"type": "object", "properties": {}}), "drop_all"),
        (
            "hold_keys",
            "Hold keys or mouse buttons for some seconds, as if pressed: in game mode w a s d walk (or swim), shift runs, space jumps (looking down in water: dives), r / f grow and shrink, lmb is the gravity gun (held to charge, fired on release), rmb the rocket; elsewhere the editor's keys (w a s d fly, q / e turn). Look with set_camera's yaw_degrees and pitch_degrees (in game mode that turns the walker without moving it). Holding a key already held holds it longer.",
            json!({"type": "object", "properties": {"keys": {"type": "array", "items": {"type": "string"}, "description": "e.g. [\"w\", \"shift\"]"}, "seconds": {"type": "number", "description": "How long (default 1)."}}, "required": ["keys"]}),
            "hold",
        ),
        ("run_command", commands_text(), json!({"type": "object", "properties": {"name": {"type": "string", "description": "The command, e.g. game_mode, record_video, reset_scene, headlight."}}, "required": ["name"]}), "command"),
        ("undo", "Undo the last edit (redo: true redoes).", json!({"type": "object", "properties": {"redo": {"type": "boolean"}}}), "undo"),
        (
            "save_scene",
            "Save the scene: to its own file, or to path (and go on editing that file).",
            json!({"type": "object", "properties": {"path": {"type": "string"}}}),
            "save",
        ),
        ("open_scene", "Open another scene file in the viewer (unsaved edits to the current one are lost).", json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}), "open"),
        (
            "quick_save",
            "Keep everything as it is now in the viewer's quick save slot (F5; the quick save file in the viewer's Preferences, saves/quicksave.json by default; the reply's \"saved\" gives the path): the scene with its unsaved edits, the camera exactly where it is, every render setting. `ray <that file> -o shot.png` renders that view headless.",
            json!({"type": "object", "properties": {}}),
            "quick_save",
        ),
        ("quick_load", "Back to the quick save (F9): its scene, camera and render settings (unsaved edits are lost).", json!({"type": "object", "properties": {}}), "quick_load"),
    ]
}

/// Runs the server until stdin closes.
pub fn run() -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                reply(&mut out, json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}}))?;
                continue;
            }
        };
        // notifications (no id) need no answer
        let Some(id) = msg.get("id").cloned() else { continue };
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match msg.get("method").and_then(Value::as_str).unwrap_or("") {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").cloned().unwrap_or_else(|| json!("2025-06-18")),
                "capabilities": {"tools": {"listChanged": false}},
                "serverInfo": {"name": "ray", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools().into_iter().map(|(name, desc, schema, _)| json!({"name": name, "description": desc, "inputSchema": schema})).collect::<Vec<_>>()})),
            "tools/call" => Ok(call(params.get("name").and_then(Value::as_str).unwrap_or(""), params.get("arguments").cloned().unwrap_or_else(|| json!({})))),
            m => Err(json!({"code": -32601, "message": format!("unknown method {m}")})),
        };
        let msg = match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
        };
        reply(&mut out, msg)?;
    }
    Ok(())
}

fn reply(out: &mut impl Write, msg: Value) -> Result<(), String> {
    writeln!(out, "{msg}").and_then(|_| out.flush()).map_err(|e| e.to_string())
}

/// One tool call, as MCP content.
fn call(name: &str, args: Value) -> Value {
    let Some((_, _, _, cmd)) = tools().into_iter().find(|t| t.0 == name) else {
        return error(format!("unknown tool {name}"));
    };
    let r = match cmd {
        "launch" => launch(args.get("scene").and_then(Value::as_str)),
        "status" => match send("status", &args) {
            Err(_) if UnixStream::connect(socket_path()).is_err() => Ok(json!({"running": false, "hint": "launch_viewer starts one"})),
            r => r.map(|mut v| {
                v["running"] = json!(true);
                v
            }),
        },
        "undo" if args.get("redo").and_then(Value::as_bool) == Some(true) => send("redo", &json!({})),
        _ => send(cmd, &args),
    };
    match r {
        Ok(v) if cmd == "screenshot" => {
            let data = v.get("png_base64").and_then(Value::as_str).unwrap_or("").to_string();
            let note = format!(
                "{}x{}, {} samples per pixel{}",
                v["width"],
                v["height"],
                v["samples"],
                if v["settled"] == json!(true) { "" } else { " (taken at the timeout: the image may not show the latest changes yet)" }
            );
            json!({"content": [{"type": "image", "data": data, "mimeType": "image/png"}, {"type": "text", "text": note}]})
        }
        Ok(v) => json!({"content": [{"type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default()}]}),
        Err(e) => error(e),
    }
}

fn error(e: String) -> Value {
    json!({"content": [{"type": "text", "text": e}], "isError": true})
}

/// Sends one command to the viewer and waits for its answer.
fn send(cmd: &str, args: &Value) -> Result<Value, String> {
    let path = socket_path();
    let conn = UnixStream::connect(&path).map_err(|e| format!("no viewer is running ({}: {e}); launch_viewer starts one", path.display()))?;
    conn.set_read_timeout(Some(Duration::from_secs(130))).map_err(|e| e.to_string())?;
    let mut w = conn.try_clone().map_err(|e| e.to_string())?;
    writeln!(w, "{}", json!({"cmd": cmd, "args": args})).map_err(|e| e.to_string())?;
    let mut line = String::new();
    BufReader::new(conn).read_line(&mut line).map_err(|e| format!("the viewer did not answer: {e}"))?;
    let v: Value = serde_json::from_str(&line).map_err(|e| format!("bad answer from the viewer: {e}"))?;
    if v.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(v.get("error").and_then(Value::as_str).unwrap_or("the viewer refused").to_string())
    }
}

/// Starts a viewer, unless one answers already, and waits for its socket.
fn launch(scene: Option<&str>) -> Result<Value, String> {
    if UnixStream::connect(socket_path()).is_ok() {
        return send("status", &json!({})).map(|mut v| {
            v["running"] = json!(true);
            v["note"] = json!("a viewer was already running; open_scene loads another scene in it");
            v
        });
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // the project folder (saves/, library/): here, or the one the binary was built in
    let root = std::env::current_dir()
        .ok()
        .filter(|d| d.join("saves").is_dir())
        .or_else(|| exe.ancestors().nth(3).map(PathBuf::from).filter(|d| d.join("saves").is_dir()))
        .unwrap_or_else(|| PathBuf::from("."));
    let mut c = std::process::Command::new(&exe);
    if let Some(s) = scene {
        c.arg(s);
    }
    use std::os::unix::process::CommandExt;
    // its own process group, so it outlives this server; nothing on our stdout
    c.current_dir(&root).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).process_group(0);
    let child = c.spawn().map_err(|e| format!("could not start {}: {e}", exe.display()))?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(30) {
        std::thread::sleep(Duration::from_millis(250));
        if UnixStream::connect(socket_path()).is_ok() {
            return send("status", &json!({})).map(|mut v| {
                v["running"] = json!(true);
                v["pid"] = json!(child.id());
                v
            });
        }
    }
    Err(format!("the viewer (pid {}) did not open its control socket within 30 s", child.id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tools: each named once, each with an object schema and a
    /// description; each one the viewer is sent is a command it answers
    /// (control.rs), or one answered here (launch); run_command's
    /// description names every command of the menus and keys.
    #[test]
    fn the_tools_hold_together() {
        let all = tools();
        let control = include_str!("ui/control.rs");
        for (i, (name, desc, schema, cmd)) in all.iter().enumerate() {
            assert!(!all[..i].iter().any(|t| t.0 == *name), "{name} twice");
            assert!(desc.len() > 10, "{name}: a description");
            assert_eq!(schema.get("type").and_then(Value::as_str), Some("object"), "{name}: its schema");
            let answered = *cmd == "launch" || *cmd == "screenshot" || control.contains(&format!("\"{cmd}\" =>"));
            assert!(answered, "{name}: the viewer has no command '{cmd}'");
        }
        let run = all.iter().find(|t| t.0 == "run_command").expect("run_command");
        for a in crate::ui::actions::ACTIONS {
            let n = crate::ui::actions::command_name(a);
            assert!(run.1.contains(&n), "run_command does not name {n}");
        }
        assert!(call("no_such_tool", json!({}))["isError"] == json!(true));
    }
}
