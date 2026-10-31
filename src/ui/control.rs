//! The commands the control socket (remote.rs) carries out on the UI thread:
//! what the MCP server's tools do. Every edit is checked by compiling the
//! changed scene first; one that does not compile is refused with the error,
//! and the scene stays as it was. Edits are undoable like any other.

use super::{App, Selection, Tool, actions, doc, physics, remote::Request, schema, why_not};
use crate::math::Vec3;
use crate::render::{Pick, pick_ray};
use crate::scene::Scene;
use crate::settings::Settings;
use serde_json::{Map, Value, json};
use std::time::{Duration, Instant};

/// A screenshot waiting for the image to show the latest changes and to
/// gather enough samples.
pub struct PendingShot {
    reply: std::sync::mpsc::Sender<Result<Value, String>>,
    width: usize,
    samples: u32,
    deadline: Instant,
    /// the job whose image it waits for (set when the frame that took the
    /// request has posted its job)
    serial: Option<u64>,
}

impl App {
    /// Carries out the commands that came in since the last frame.
    pub(super) fn serve_remote(&mut self) {
        let Some(r) = &self.remote else { return };
        let requests: Vec<Request> = r.rx.try_iter().collect();
        for q in requests {
            if q.cmd == "screenshot" {
                let a = &q.args;
                let width = a.get("width").and_then(Value::as_u64).unwrap_or(800).clamp(64, 4096) as usize;
                let samples = a.get("samples").and_then(Value::as_u64).unwrap_or(16).clamp(1, 4096) as u32;
                let secs = a.get("timeout").and_then(Value::as_f64).unwrap_or(15.0).clamp(0.5, 110.0);
                self.shots.push(PendingShot { reply: q.reply, width, samples, deadline: Instant::now() + Duration::from_secs_f64(secs), serial: None });
                continue;
            }
            let rendering = self.still.is_some();
            let r = self.command(&q.cmd, &q.args);
            // (run_command screenshot: answered once the picture, rendered
            // whole first, is written)
            if r.is_ok() && !rendering && self.still.is_some() {
                self.still_waiters.push(q.reply);
                continue;
            }
            let _ = q.reply.send(r);
        }
    }

    /// Answers the run_command screenshots waiting for the one rendering,
    /// once it is written (or failed): the status, what it said, its file.
    pub(super) fn finish_stills(&mut self) {
        if self.still_waiters.is_empty() || self.still.is_some() {
            return;
        }
        let mut status = self.status();
        match &self.last_still {
            Some(Ok(p)) => {
                status["said"] = json!(format!("saved {}", p.display()));
                status["file"] = json!(p);
            }
            Some(Err(e)) => status["said"] = json!(format!("screenshot failed: {e}")),
            None => {}
        }
        for w in self.still_waiters.drain(..) {
            let _ = w.send(Ok(status.clone()));
        }
    }

    /// Answers the screenshots whose image is ready (or that waited long
    /// enough). Runs after the frame's job is posted.
    pub(super) fn finish_shots(&mut self) {
        if self.shots.is_empty() {
            return;
        }
        let posted = self.posted;
        let mut waiting = Vec::new();
        for mut p in std::mem::take(&mut self.shots) {
            let serial = *p.serial.get_or_insert(posted);
            let current = self.shown_serial >= serial;
            // moving water and falling objects restart the image every frame:
            // a current one is as settled as it gets
            let moving = self.waves_moving() || !self.world.bodies.is_empty();
            let settled = self.stats.samples >= p.samples || !self.stats.traced || moving;
            let late = Instant::now() > p.deadline;
            if !(current && settled || late) {
                waiting.push(p);
                continue;
            }
            let r = match &self.shown {
                Some(img) => png_of(img, p.width).map(|(png, w, h)| {
                    json!({
                        "png_base64": base64(&png),
                        "width": w,
                        "height": h,
                        "samples": self.stats.samples,
                        "settled": current && settled,
                    })
                }),
                None => Err("no image yet".into()),
            };
            let _ = p.reply.send(r);
        }
        self.shots = waiting;
    }

    fn command(&mut self, cmd: &str, a: &Value) -> Result<Value, String> {
        match cmd {
            "status" => Ok(self.status()),
            "scene" => Ok(json!({
                "status": self.status(),
                "document": self.doc.v,
                "camera": self.camera_json(),
                "settings": serde_json::to_value(self.settings).map_err(|e| e.to_string())?,
            })),
            "objects" => Ok(self.objects_json()),
            "describe" => Ok(describe()),
            "materials" => Ok(self.materials_json()),
            "add" => self.cmd_add(a),
            "add_many" => {
                let nodes = a.get("nodes").and_then(Value::as_array).cloned().ok_or("add_objects needs \"nodes\": a list of scene objects")?;
                if nodes.iter().any(|n| !n.is_object()) {
                    return Err("every node must be a JSON object".into());
                }
                let first = self.doc.objects().len();
                let count = nodes.len();
                self.edit(|v| {
                    let objs = v.as_object_mut().ok_or("the scene is not an object")?.entry("objects").or_insert_with(|| json!([]));
                    objs.as_array_mut().ok_or("objects is not a list")?.extend(nodes);
                    Ok(())
                })?;
                Ok(json!({"added": count, "first_index": first, "objects": self.doc.objects().len()}))
            }
            "update" => self.cmd_update(a),
            "delete" => {
                let top = self.find_object(a.get("object"))?;
                let name = doc::label(&self.doc.objects()[top]);
                self.sel = self.doc.delete(&Selection::Node { top, sub: vec![] });
                self.lost_bodies();
                Ok(json!({"deleted": name}))
            }
            "select" => {
                let top = self.find_object(a.get("object"))?;
                self.sel = Selection::Node { top, sub: vec![] };
                if let Some(t) = a.get("tool").and_then(Value::as_str) {
                    self.tool = match t {
                        "rotate" => Tool::Rotate,
                        "scale" => Tool::Scale,
                        _ => Tool::Move,
                    };
                }
                Ok(json!({"selected": doc::label(&self.doc.objects()[top]), "index": top}))
            }
            "settings" => {
                let mut v = serde_json::to_value(self.settings).map_err(|e| e.to_string())?;
                merge(&mut v, a.get("set").unwrap_or(&Value::Null))?;
                self.settings = Settings::from_json(&v)?;
                serde_json::to_value(self.settings).map_err(|e| e.to_string())
            }
            // the preferences (config.rs): saved as they change, as in their
            // window; an unknown field is refused
            "preferences" => {
                let mut v = serde_json::to_value(&self.config).map_err(|e| e.to_string())?;
                let set = a.get("set").unwrap_or(&Value::Null);
                if let Some(k) = set.as_object().and_then(|o| o.keys().find(|k| v.get(k.as_str()).is_none())) {
                    return Err(format!("no preference '{k}'"));
                }
                merge(&mut v, set)?;
                self.config = serde_json::from_value(v).map_err(|e| e.to_string())?;
                serde_json::to_value(&self.config).map_err(|e| e.to_string())
            }
            "environment" => {
                let set = a.get("set").cloned().unwrap_or(Value::Null);
                self.edit(|v| {
                    let env = v.as_object_mut().ok_or("the scene is not an object")?.entry("environment").or_insert_with(|| json!({}));
                    merge(env, &set)
                })?;
                Ok(self.doc.v.get("environment").cloned().unwrap_or(Value::Null))
            }
            "material" => {
                let name = a.get("name").and_then(Value::as_str).filter(|n| !n.is_empty()).ok_or("set_material needs a name")?.to_string();
                let def = a.get("definition").cloned().ok_or("set_material needs a definition (a material object)")?;
                self.edit(|v| {
                    let mats = v.as_object_mut().ok_or("the scene is not an object")?.entry("materials").or_insert_with(|| json!({}));
                    mats.as_object_mut().ok_or("materials is not an object")?.insert(name.clone(), def);
                    Ok(())
                })?;
                Ok(json!({"material": name}))
            }
            "camera" => self.cmd_camera(a),
            "glide" => self.cmd_glide(a),
            "render_video" => self.cmd_render_video(a),
            "raycast" => self.cmd_raycast(a),
            "hold" => {
                let keys: Vec<String> = a.get("keys").and_then(Value::as_array).map(|k| k.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
                if keys.is_empty() {
                    return Err("hold needs keys, e.g. [\"w\", \"shift\"]".into());
                }
                let seconds = a.get("seconds").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                self.hold(&keys, seconds);
                Ok(json!({"held": keys, "seconds": seconds}))
            }
            "drop_all" => {
                self.drop_all();
                Ok(json!({"falling": self.world.bodies.len()}))
            }
            "undo" => Ok(json!({"undone": self.doc.can_undo() && {
                self.undo();
                true
            }})),
            "redo" => Ok(json!({"redone": self.doc.can_redo() && {
                self.redo();
                true
            }})),
            "save" => {
                match a.get("path").and_then(Value::as_str) {
                    Some(p) => self.save_as(p.into())?,
                    None => self.save()?,
                }
                Ok(json!({"saved": self.path.display().to_string(), "dirty": self.doc.dirty}))
            }
            "command" => {
                let name = a.get("name").and_then(Value::as_str).ok_or("run_command needs a name")?;
                let act = actions::by_command(name).ok_or_else(|| format!("no command '{name}' (see run_command's description for the list)"))?;
                if !self.can(act.act) {
                    let why = why_not(act.act);
                    return Err(format!("{} cannot run now{}", actions::command_name(act), if why.is_empty() { String::new() } else { format!(": {why}") }));
                }
                let before = Instant::now();
                self.run_act(act.act);
                let mut status = self.status();
                // (and what it said on screen, if anything)
                if let Some((m, at)) = &self.message
                    && *at >= before
                {
                    status["said"] = json!(m);
                }
                Ok(status)
            }
            "quick_save" => {
                self.quick_save()?;
                Ok(json!({"saved": self.quick_save_path().display().to_string()}))
            }
            "quick_load" => {
                // (forced: a remote has no second click; the document keeps
                // its unsaved edits unless the quick save loads)
                self.quick_load_as(true)?;
                Ok(self.status())
            }
            "open" => {
                let p = a.get("path").and_then(Value::as_str).ok_or("open needs a path")?;
                self.open(p.into())?;
                Ok(self.status())
            }
            "" => Err("no command".into()),
            other => Err(format!("unknown command '{other}'")),
        }
    }

    fn status(&self) -> Value {
        json!({
            "file": self.path.display().to_string(),
            "unsaved_edits": self.doc.dirty,
            "objects": self.doc.objects().len(),
            "error": self.error,
            "samples": self.stats.samples,
            "render_size": self.render_size,
            "device": self.settings.device.label(),
            "falling_objects": self.world.bodies.len(),
            "moving": self.world.bodies.iter().map(|b| json!({
                "object": b.top,
                "speed": (b.v.length() * 1000.0).round() / 1000.0,
                "spin": (b.w.length() * 1000.0).round() / 1000.0,
                "still_for": (b.rest * 100.0).round() / 100.0,
            })).collect::<Vec<_>>(),
            "ui_frame_ms": self.frame_ms,
            "ui_frames": self.frames,
            // the last traced frame: its time (trace and post), and how the
            // GPU and the CPU shared it
            "render_ms": self.stats.trace_ms + self.stats.post_ms,
            "gpu_ms": self.stats.gpu_ms,
            "cpu_ms": self.stats.cpu_ms,
            "gpu_share": self.stats.gpu_share,
            "frame_traced": self.stats.traced,
            // the scene's clock (seconds: a steady recording moves it a frame
            // at a time), and what moves on it
            "clock": self.now(),
            "gliding": !self.glides.is_empty(),
            "holding": self.held.iter().map(|h| h.0.clone()).collect::<Vec<_>>(),
            "recording": self.recorder.as_ref().map(|r| json!({
                "file": r.path.display().to_string(),
                "steady": self.steady.as_ref().map(|s| json!({
                    "frames": s.frames,
                    "seconds": s.frames as f32 / s.fps as f32,
                    "of_frames": s.until,
                    "size": s.size,
                    "fps": s.fps,
                    "samples": s.wants(self.settings.max_samples),
                    "max_load": s.load,
                    // the wall clock's: how long a frame takes, and the rest
                    "seconds_a_frame": s.secs_a_frame(),
                    "eta_seconds": s.until.map(|n| n.saturating_sub(s.frames) as f32 * s.secs_a_frame()),
                })),
            })),
            // the last recording finished (its file is whole), or why it failed
            "last_video": self.last_video.as_ref().map(|v| v.0.display().to_string()),
            "recording_error": self.record_error,
            // game mode's walker (null: editing)
            "game": self.game.as_ref().map(|g| json!({ "height_m": g.height, "grounded": g.grounded, "feet": g.feet, "in_water": g.wet, "diving": g.diving })),
        })
    }

    fn camera_json(&self) -> Value {
        let c = self.cam;
        let (sy, cy) = c.yaw.sin_cos();
        let (sp, cp) = c.pitch.sin_cos();
        json!({
            "position": c.exact(),
            "yaw_degrees": c.yaw.to_degrees(),
            "pitch_degrees": c.pitch.to_degrees(),
            "fov_degrees": c.fov,
            "forward": [-sy * cp, sp, -cy * cp],
        })
    }

    fn objects_json(&self) -> Value {
        let list: Vec<Value> = self
            .doc
            .objects()
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let bounds = self.scene.objects.iter().filter(|o| o.doc_index == i).map(|o| o.aabb).chain(self.scene.volumes.iter().filter(|v| v.doc_index == i).map(|v| v.aabb)).reduce(|a, b| a.union(b));
                let mut o = Map::new();
                o.insert("index".into(), i.into());
                o.insert("name".into(), doc::label(n).into());
                for k in ["type", "position", "rotation", "scale", "material", "locked"] {
                    if let Some(v) = n.get(k) {
                        // a material written out in full is long: name it
                        let v = if k == "material" && v.is_object() { json!("(own material)") } else { v.clone() };
                        o.insert(k.into(), v);
                    }
                }
                if let Some(b) = bounds.filter(|b| b.is_finite()) {
                    o.insert("bounds".into(), json!({"min": b.min.to_array(), "max": b.max.to_array()}));
                }
                Value::Object(o)
            })
            .collect();
        Value::Array(list)
    }

    fn materials_json(&self) -> Value {
        let scene: Vec<&String> = self.doc.v.get("materials").and_then(Value::as_object).map(|m| m.keys().collect()).unwrap_or_default();
        let library: Value = std::fs::read_to_string("library/materials.json").ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
        let library: Vec<Value> = library
            .as_object()
            .map(|m| m.iter().map(|(k, v)| json!({"name": k, "definition": v})).collect())
            .unwrap_or_default();
        json!({
            "scene_materials": scene,
            "library": library,
            "how": "Give a node \"material\": \"name\" for one of the scene's materials, or a full material object (a library definition copied, or your own: see the material fields in describe_scene_format). set_material adds or changes a named scene material that many objects can share.",
        })
    }

    /// Applies `f` to a copy of the document; if the result compiles, it
    /// becomes the document (undoably), else the error comes back.
    fn edit(&mut self, f: impl FnOnce(&mut Value) -> Result<(), String>) -> Result<(), String> {
        let mut v = self.doc.v.clone();
        f(&mut v)?;
        // refused when it breaks an object (problems already there are not its fault)
        let s = Scene::from_value(&v, &mut self.cache)?;
        if let Some(p) = s.problems.iter().find(|p| !self.scene.problems.contains(p)) {
            return Err(p.clone());
        }
        self.doc.checkpoint(true);
        self.doc.v = v;
        self.doc.touch();
        // compiled now, so a body for a new object finds it this frame (the
        // check's compile is it, unless emitters' copies join the document's
        // objects)
        if self.emitters.particles.is_empty() {
            self.adopt(s);
            self.compiled = self.doc.version;
            self.compiled_emit = self.emitters.version;
        } else {
            self.compile();
        }
        Ok(())
    }

    /// An object by index, or by name (exact, then ignoring case).
    fn find_object(&self, r: Option<&Value>) -> Result<usize, String> {
        let objs = self.doc.objects();
        match r {
            Some(Value::Number(n)) => n.as_u64().map(|i| i as usize).filter(|&i| i < objs.len()).ok_or(format!("no object {n} (there are {})", objs.len())),
            Some(Value::String(s)) => {
                let by = |f: &dyn Fn(&str) -> bool| objs.iter().position(|n| f(&doc::label(n)) || n.get("name").and_then(Value::as_str).is_some_and(f));
                by(&|l: &str| l == s).or_else(|| by(&|l: &str| l.eq_ignore_ascii_case(s))).ok_or(format!("no object named '{s}'"))
            }
            _ => Err("give \"object\" as an index or a name".into()),
        }
    }

    fn cmd_add(&mut self, a: &Value) -> Result<Value, String> {
        let node = a.get("node").cloned().ok_or("add needs a \"node\" (a scene object as JSON)")?;
        if !node.is_object() {
            return Err("the node must be a JSON object".into());
        }
        self.edit(|v| {
            let objs = v.as_object_mut().ok_or("the scene is not an object")?.entry("objects").or_insert_with(|| json!([]));
            objs.as_array_mut().ok_or("objects is not a list")?.push(node);
            Ok(())
        })?;
        let top = self.doc.objects().len() - 1;
        self.sel = Selection::Node { top, sub: vec![] };
        // "drop": let it fall and settle with the physics
        if a.get("drop").and_then(Value::as_bool) == Some(true)
            && let Some(b) = physics::body_of(&self.doc.v, top)
        {
            self.world.bodies.push(b);
        }
        Ok(json!({"index": top, "name": doc::label(&self.doc.objects()[top])}))
    }

    fn cmd_update(&mut self, a: &Value) -> Result<Value, String> {
        let top = self.find_object(a.get("object"))?;
        let set = a.get("set").cloned().unwrap_or(Value::Null);
        let remove: Vec<String> = a.get("remove").and_then(Value::as_array).map(|r| r.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
        self.edit(|v| {
            let n = v.get_mut("objects").and_then(|o| o.get_mut(top)).ok_or("the object went away")?;
            merge(n, &set)?;
            let n = n.as_object_mut().ok_or("the object is not a JSON object")?;
            for k in &remove {
                n.remove(k);
            }
            Ok(())
        })?;
        // it is where it was put: no longer falling, and the waves leave it be
        if let Some(i) = self.world.find(top) {
            self.world.bodies.swap_remove(i);
        }
        self.placed.0.insert(top);
        if a.get("drop").and_then(Value::as_bool) == Some(true) {
            self.placed.0.remove(&top);
            if let Some(b) = physics::body_of(&self.doc.v, top) {
                self.world.bodies.push(b);
            }
        }
        Ok(self.doc.objects()[top].clone())
    }

    fn cmd_camera(&mut self, a: &Value) -> Result<Value, String> {
        let v3 = |k: &str| a.get(k).and_then(doc::as_vec3).map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
        if let Some(p) = a.get("position").and_then(doc::as_vec3) {
            self.cam.set_exact(p);
        }
        if let Some(t) = v3("look_at") {
            let d = (t - self.cam.pos).normalize();
            if d.is_finite() {
                self.cam.pitch = d.y.clamp(-1.0, 1.0).asin();
                self.cam.yaw = (-d.x).atan2(-d.z);
            }
        }
        if let Some(y) = a.get("yaw_degrees").and_then(Value::as_f64) {
            self.cam.yaw = (y as f32).to_radians();
        }
        if let Some(p) = a.get("pitch_degrees").and_then(Value::as_f64) {
            self.cam.pitch = (p as f32).to_radians().clamp(-1.55, 1.55);
        }
        if let Some(f) = a.get("fov_degrees").and_then(Value::as_f64) {
            self.cam.fov = (f as f32).clamp(5.0, 150.0);
        }
        Ok(self.camera_json())
    }

    /// The camera eases to a pose over `seconds` of the scene's clock (as
    /// set_camera takes it; yaw the short way round, or `turn_degrees` by
    /// that much either way; smoothly, or at an even rate with `even`). In
    /// game mode only the look: the walker turns where it stands. `queue`:
    /// after the glides already going, from where they end and exactly when
    /// (a path of legs, sent at once, with no pause between them).
    fn cmd_glide(&mut self, a: &Value) -> Result<Value, String> {
        let num = |k: &str| a.get(k).and_then(Value::as_f64).map(|v| v as f32);
        let secs = num("seconds").unwrap_or(2.0).clamp(0.0, 600.0);
        let queue = a.get("queue").and_then(Value::as_bool).unwrap_or(false);
        if !queue {
            self.glides.clear();
        }
        // where it starts from: the last queued glide's end, or the camera now
        let (base_pos, [base_yaw, base_pitch, base_fov], t0) = match self.glides.back() {
            Some(g) => (g.to_pos.unwrap_or(g.from_pos), g.to, g.t0 + g.secs as f64),
            None => (self.cam.exact(), [self.cam.yaw, self.cam.pitch, self.cam.fov], self.now()),
        };
        let to_pos = a.get("position").and_then(doc::as_vec3).filter(|_| self.game.is_none());
        let (mut yaw, mut pitch) = (base_yaw, base_pitch);
        if let Some(t) = a.get("look_at").and_then(doc::as_vec3) {
            let from = to_pos.unwrap_or(base_pos);
            let d = Vec3::new((t[0] - from[0]) as f32, (t[1] - from[1]) as f32, (t[2] - from[2]) as f32).normalize();
            if d.is_finite() {
                pitch = d.y.clamp(-1.0, 1.0).asin();
                yaw = (-d.x).atan2(-d.z);
            }
        }
        if let Some(y) = num("yaw_degrees") {
            yaw = y.to_radians();
        }
        // (the short way round)
        let tau = std::f32::consts::TAU;
        yaw = base_yaw + (yaw - base_yaw + std::f32::consts::PI).rem_euclid(tau) - std::f32::consts::PI;
        if let Some(t) = num("turn_degrees") {
            yaw = base_yaw + t.to_radians();
        }
        if let Some(p) = num("pitch_degrees") {
            pitch = p.to_radians();
        }
        let fov = num("fov_degrees").unwrap_or(base_fov).clamp(5.0, 150.0);
        self.glides.push_back(super::Glide {
            from_pos: base_pos,
            to_pos,
            from: [base_yaw, base_pitch, base_fov],
            to: [yaw, pitch.clamp(-1.55, 1.55), fov],
            t0,
            secs,
            linear: a.get("even").and_then(Value::as_bool).unwrap_or(false),
        });
        Ok(json!({"seconds": secs, "starts_in": t0 - self.now(), "position": to_pos.unwrap_or(base_pos), "yaw_degrees": yaw.to_degrees(), "pitch_degrees": pitch.to_degrees(), "fov_degrees": fov}))
    }

    /// A steady recording at a size, rate and samples of its own (not the
    /// preferences'), ending by itself after `seconds`; or (stop) the
    /// recording ended.
    fn cmd_render_video(&mut self, a: &Value) -> Result<Value, String> {
        if a.get("stop").and_then(Value::as_bool).unwrap_or(false) {
            let file = self.recorder.as_ref().map(|r| r.path.display().to_string()).ok_or("not recording")?;
            self.stop_recording();
            return Ok(json!({"finishing": file}));
        }
        if self.recorder.is_some() {
            return Err("already recording (stop: true ends it)".into());
        }
        let int = |k: &str, d: u64| a.get(k).and_then(Value::as_u64).unwrap_or(d);
        let size = super::video_size(int("width", 1920) as f32, int("height", 1080) as f32);
        let fps = int("fps", 30).clamp(5, 120) as u32;
        let until = a.get("seconds").and_then(Value::as_f64).map(|s| (s * fps as f64).round().max(1.0) as u64);
        let quality = int("quality", 16).min(51) as u32;
        let mut steady = super::record::Steady::new(fps, int("samples", 16).min(1 << 16) as u32, size, until, self.now());
        steady.load = a.get("max_load").and_then(Value::as_f64).unwrap_or(0.9).clamp(0.1, 1.0) as f32;
        let (samples, load) = (steady.wants(self.settings.max_samples), steady.load);
        let file = self.start_recording(Some(steady), quality)?;
        Ok(json!({"file": file, "size": size, "fps": fps, "samples": samples, "quality": quality, "frames": until, "max_load": load}))
    }

    fn cmd_raycast(&mut self, a: &Value) -> Result<Value, String> {
        let v3 = |k: &str| a.get(k).and_then(doc::as_vec3).map(|p| Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
        let num = |k: &str| a.get(k).and_then(Value::as_f64).map(|v| v as f32);
        // straight down onto (x, z), or along a ray
        let (o, d) = match (num("x"), num("z"), v3("origin"), v3("direction")) {
            (Some(x), Some(z), _, _) => (Vec3::new(x, 900.0, z), Vec3::new(0.0, -1.0, 0.0)),
            (_, _, Some(o), Some(d)) => (o, d.normalize()),
            _ => return Err("give x and z (the ground below), or origin and direction".into()),
        };
        // (as the view sees it: its fractal detail is the frame's)
        let c = self.view_cam();
        let Some(mut p) = pick_ray(&self.scene, &c, &self.settings, o, d) else { return Ok(json!({"hit": false})) };
        // (a march stops within a pixel's footprint of a surface, as seen from
        // where the ray began: from 900 m up, 8 cm off on a fractal. Begun
        // again a little short of the hit, twice, it lands within a micrometre;
        // a ray that then misses only grazed it, and the first hit stands)
        let (mut from, mut leg) = (0.0, p.t);
        for _ in 0..2 {
            let s = p.t - (2.0 * leg * c.pixel_angle + 1e-4);
            if s <= from {
                break;
            }
            let Some(q) = pick_ray(&self.scene, &c, &self.settings, o + d * s, d) else { break };
            (from, leg) = (s, q.t);
            p = Pick { t: s + q.t, ..q };
        }
        let ob = self.scene.objects.get(p.object as usize);
        Ok(json!({
            "hit": true,
            "point": p.point.to_array(),
            "normal": p.normal.to_array(),
            "distance": p.t,
            "object": ob.map(|o| o.doc_index),
            "object_name": ob.and_then(|o| self.doc.objects().get(o.doc_index)).map(doc::label),
        }))
    }
}

/// Merges `set` into `v`: its keys replace v's (an object merges into an
/// object), and a null removes the key.
fn merge(v: &mut Value, set: &Value) -> Result<(), String> {
    let Some(set) = set.as_object() else {
        return if set.is_null() { Ok(()) } else { Err("\"set\" must be a JSON object".into()) };
    };
    let v = v.as_object_mut().ok_or("cannot set fields on something that is not an object")?;
    for (k, x) in set {
        match (v.get_mut(k), x) {
            (_, Value::Null) => {
                v.remove(k);
            }
            (Some(old @ Value::Object(_)), Value::Object(_)) if k != "material" && k != "child" => merge(old, x)?,
            _ => {
                v.insert(k.clone(), x.clone());
            }
        }
    }
    Ok(())
}

/// The scene format, for building objects: every shape type with its fields
/// and an example, the fields every object has, and the material fields.
fn describe() -> Value {
    let shapes: Vec<Value> = schema::shapes()
        .iter()
        .map(|s| {
            json!({
                "type": s.ty,
                "category": s.category,
                "description": s.desc,
                "example": serde_json::from_str::<Value>(s.new).unwrap_or(Value::Null),
                "fields": s.fields.iter().map(|(k, d)| (k.to_string(), Value::from(*d))).collect::<Map<_, _>>(),
            })
        })
        .collect();
    json!({
        "conventions": "Units are metres; y is up. A scene object is a JSON node with a \"type\". Its \"position\" [x,y,z] places its origin (shapes are centred on it; a cylinder or box spans height/2 above and below). \"rotation\" is [x,y,z] in degrees, applied x first, then y, then z. \"scale\" is uniform. Build compound objects (a vehicle, a house) as one \"union\" node with \"children\": each child has its own position/rotation relative to the union, and the whole moves as one body. \"smooth\" on a union blends children; leave it 0 for hard joins. \"difference\" carves later children out of the first. NURBS surfaces (\"nurbs\", \"revolve\") are exact and smooth but top-level only: they cannot be children of a union. Materials: \"material\" is a scene material's name or a full material object.",
        "shapes": shapes,
        "object_fields": schema::COMMON.iter().map(|(k, d)| (k.to_string(), Value::from(*d))).collect::<Map<_, _>>(),
        "material_fields": schema::MATERIAL.iter().map(|(k, d)| (k.to_string(), Value::from(*d))).collect::<Map<_, _>>(),
        "environment_fields": schema::ENVIRONMENT.iter().map(|(k, d)| (k.to_string(), Value::from(*d))).collect::<Map<_, _>>(),
    })
}

/// The image `width` wide (no wider than it is), as PNG bytes.
fn png_of(img: &eframe::egui::ColorImage, width: usize) -> Result<(Vec<u8>, usize, usize), String> {
    let [sw, sh] = img.size;
    let (w, h) = if sw > width { (width, (sh * width / sw).max(1)) } else { (sw, sh) };
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let (y0, y1) = (y * sh / h, ((y + 1) * sh / h).max(y * sh / h + 1));
        for x in 0..w {
            let (x0, x1) = (x * sw / w, ((x + 1) * sw / w).max(x * sw / w + 1));
            // the average of the source pixels this one covers
            let mut sum = [0u32; 3];
            for yy in y0..y1 {
                for p in &img.pixels[yy * sw + x0..yy * sw + x1] {
                    sum[0] += p.r() as u32;
                    sum[1] += p.g() as u32;
                    sum[2] += p.b() as u32;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as u32;
            rgb.extend(sum.map(|s| ((s + n / 2) / n) as u8));
        }
    }
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&rgb).map_err(|e| e.to_string())?;
    }
    Ok((out, w, h))
}

fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}
