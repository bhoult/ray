//! The scene being edited: the JSON document itself (so saving writes back
//! what was loaded, plus the edits, in the same key order), node addressing,
//! and undo / redo.

use crate::math::{Mat3, Vec3};
use serde_json::{Map, Value};
use std::time::Instant;

/// What the inspector shows.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum Selection {
    #[default]
    None,
    /// `objects[top]`, then child indices down its tree
    Node { top: usize, sub: Vec<usize> },
}

pub struct Doc {
    pub v: Value,
    undo: Vec<Value>,
    redo: Vec<Value>,
    last_edit: Option<Instant>,
    pub dirty: bool,
    /// new on every change (recompile), and never the same as any other
    /// document's: what is kept for one version (compiled, the inspector's
    /// work) is not taken for a scene opened since
    pub version: u64,
}

/// The next document version (process-wide: see Doc::version).
fn next_version() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

const UNDO_LIMIT: usize = 200;

impl Doc {
    pub fn new(v: Value) -> Doc {
        let mut v = v;
        if !v.is_object() {
            v = Value::Object(Map::new());
        }
        Doc { v, undo: Vec::new(), redo: Vec::new(), last_edit: None, dirty: false, version: next_version() }
    }

    pub fn objects(&self) -> &[Value] {
        self.v.get("objects").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn array_mut(&mut self, key: &str) -> &mut Vec<Value> {
        let o = self.v.as_object_mut().unwrap();
        let e = o.entry(key.to_string()).or_insert_with(|| Value::Array(Vec::new()));
        if !e.is_array() {
            *e = Value::Array(Vec::new());
        }
        e.as_array_mut().unwrap()
    }

    pub fn object_mut(&mut self, key: &str) -> &mut Map<String, Value> {
        let o = self.v.as_object_mut().unwrap();
        let e = o.entry(key.to_string()).or_insert_with(|| Value::Object(Map::new()));
        if !e.is_object() {
            *e = Value::Object(Map::new());
        }
        e.as_object_mut().unwrap()
    }

    pub fn node(&self, top: usize, sub: &[usize]) -> Option<&Value> {
        let mut n = self.objects().get(top)?;
        for &k in sub {
            n = child(n, k)?;
        }
        Some(n)
    }

    pub fn node_mut(&mut self, top: usize, sub: &[usize]) -> Option<&mut Value> {
        let mut n = self.v.get_mut("objects")?.as_array_mut()?.get_mut(top)?;
        for &k in sub {
            n = child_mut(n, k)?;
        }
        Some(n)
    }

    /// Records the state before an edit. Edits within half a second of each
    /// other (a slider drag) share one undo step unless `force`.
    pub fn checkpoint(&mut self, force: bool) {
        let now = Instant::now();
        let fresh = self.last_edit.is_none_or(|t| (now - t).as_secs_f32() > 0.5);
        if force || fresh {
            self.undo.push(self.v.clone());
            if self.undo.len() > UNDO_LIMIT {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.last_edit = Some(now);
    }

    /// Marks the document changed (after an edit).
    pub fn touch(&mut self) {
        self.dirty = true;
        self.version = next_version();
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else { return false };
        self.redo.push(std::mem::replace(&mut self.v, prev));
        self.last_edit = None;
        self.touch();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else { return false };
        self.undo.push(std::mem::replace(&mut self.v, next));
        self.last_edit = None;
        self.touch();
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Removes the selected node or light; returns the selection to use next.
    pub fn delete(&mut self, sel: &Selection) -> Selection {
        match sel {
            Selection::Node { top, sub } if sub.is_empty() => {
                self.checkpoint(true);
                let objs = self.array_mut("objects");
                if *top < objs.len() {
                    objs.remove(*top);
                }
                self.touch();
                Selection::None
            }
            Selection::Node { top, sub } => {
                let (parent, last) = sub.split_at(sub.len() - 1);
                let Some(p) = self.node(*top, parent) else { return sel.clone() };
                // a combinator keeps at least one child; a modifier's only child cannot go
                let Some(kids) = p.get("children").and_then(Value::as_array) else { return sel.clone() };
                if kids.len() <= 1 || last[0] >= kids.len() {
                    return sel.clone();
                }
                self.checkpoint(true);
                if let Some(kids) = self.node_mut(*top, parent).and_then(|p| p.get_mut("children")).and_then(Value::as_array_mut) {
                    kids.remove(last[0]);
                }
                self.touch();
                Selection::Node { top: *top, sub: parent.to_vec() }
            }
            _ => sel.clone(),
        }
    }

    /// Duplicates the selected top-level object or child; selects the copy.
    pub fn duplicate(&mut self, sel: &Selection, offset: [f64; 3]) -> Selection {
        match sel {
            Selection::Node { top, sub } => {
                let Some(mut copy) = self.node(*top, sub).cloned() else { return sel.clone() };
                if let Some(o) = copy.as_object_mut() {
                    let p = o.get("position").and_then(as_vec3).unwrap_or([0.0; 3]);
                    o.insert("position".into(), vec3([p[0] + offset[0], p[1] + offset[1], p[2] + offset[2]]));
                    if let Some(Value::String(n)) = o.get("name").cloned() {
                        o.insert("name".into(), Value::String(format!("{n} copy")));
                    }
                }
                if sub.is_empty() {
                    self.checkpoint(true);
                    let objs = self.array_mut("objects");
                    objs.insert(top + 1, copy);
                    self.touch();
                    return Selection::Node { top: top + 1, sub: vec![] };
                }
                let (parent, last) = sub.split_at(sub.len() - 1);
                // (an only child, a modifier's or what an emitter sends out, has no sibling list)
                if self.node(*top, parent).and_then(|p| p.get("children")).is_none_or(|k| !k.is_array()) {
                    return sel.clone();
                }
                self.checkpoint(true);
                if let Some(kids) = self.node_mut(*top, parent).and_then(|p| p.get_mut("children")).and_then(Value::as_array_mut) {
                    kids.insert(last[0] + 1, copy);
                    self.touch();
                    let mut s = parent.to_vec();
                    s.push(last[0] + 1);
                    return Selection::Node { top: *top, sub: s };
                }
                sel.clone()
            }
            _ => sel.clone(),
        }
    }

    /// Adds a top-level object; selects it.
    pub fn add_object(&mut self, node: Value) -> Selection {
        self.checkpoint(true);
        let objs = self.array_mut("objects");
        objs.push(node);
        let top = objs.len() - 1;
        self.touch();
        Selection::Node { top, sub: vec![] }
    }

    /// Adds `node` as a new child of the selected combinator; selects it.
    pub fn add_child(&mut self, top: usize, sub: &[usize], node: Value) -> Option<Selection> {
        let has_children = self.node(top, sub)?.get("children").is_some_and(Value::is_array);
        if !has_children {
            return None;
        }
        self.checkpoint(true);
        let kids = self.node_mut(top, sub)?.get_mut("children")?.as_array_mut()?;
        kids.push(node);
        let k = kids.len() - 1;
        self.touch();
        let mut s = sub.to_vec();
        s.push(k);
        Some(Selection::Node { top, sub: s })
    }

    /// Wraps the selected node in a combinator or modifier of type `ty`: the
    /// node becomes its (first) child and keeps its place, transform and material.
    pub fn wrap(&mut self, top: usize, sub: &[usize], ty: &str) -> Option<Selection> {
        let info = super::schema::shape(ty)?;
        self.wrap_with(top, sub, serde_json::from_str(info.new).ok()?)
    }

    /// `wrap`, in the node `wrapper` (a modifier with its settings; its
    /// child is replaced by the node wrapped).
    pub fn wrap_with(&mut self, top: usize, sub: &[usize], mut wrapper: Value) -> Option<Selection> {
        let node = self.node(top, sub)?.clone();
        let w = wrapper.as_object_mut()?;
        let mut inner = node.clone();
        // the wrapper takes over the placement and the name
        let io = inner.as_object_mut()?;
        for key in ["position", "rotation", "scale", "name", "material", "shadow", "step", "bound", "friction", "bounce", "locked"] {
            if let Some(v) = io.remove(key) {
                w.insert(key.into(), v);
            }
        }
        if w.contains_key("children") {
            let mut kids = w.get("children").and_then(Value::as_array).cloned().unwrap_or_default();
            if kids.is_empty() {
                kids.push(inner);
            } else {
                kids[0] = inner;
            }
            w.insert("children".into(), Value::Array(kids));
        } else {
            w.insert("child".into(), inner);
        }
        self.checkpoint(true);
        *self.node_mut(top, sub)? = wrapper;
        self.touch();
        Some(Selection::Node { top, sub: sub.to_vec() })
    }

    /// Moves top-level object `top` into group `group` (also top-level),
    /// where it stays where it is: its placement becomes the group's frame.
    pub fn into_group(&mut self, top: usize, group: usize) -> Option<Selection> {
        let g = self.node(group, &[])?;
        if top == group || g.get("type").and_then(Value::as_str) != Some("group") {
            return None;
        }
        let gx = placement(g);
        let mut n = self.node(top, &[])?.clone();
        let (t, r, s) = placement(&n);
        let back = gx.1.transpose();
        set_placement(&mut n, (back.apply(t - gx.0) * (1.0 / gx.2), back.mul(r), s / gx.2));
        self.checkpoint(true);
        self.array_mut("objects").remove(top);
        let g = if top < group { group - 1 } else { group };
        let kids = self.node_mut(g, &[])?.get_mut("children")?.as_array_mut()?;
        kids.push(n);
        let k = kids.len() - 1;
        self.touch();
        Some(Selection::Node { top: g, sub: vec![k] })
    }

    /// Takes group member `sub` of `objects[top]` out of its group, to a
    /// top-level object of its own right after the group, where it is (with
    /// the material and settings it had from its groups).
    pub fn out_of_group(&mut self, top: usize, sub: &[usize]) -> Option<Selection> {
        let (&k, parent) = sub.split_last()?;
        // every node on the way down is a group: their placements stack up
        let (mut t, mut r, mut s) = (Vec3::ZERO, Mat3::IDENTITY, 1.0f32);
        let mut inherited = Map::new();
        for depth in 0..sub.len() {
            let g = self.node(top, &sub[..depth])?;
            if g.get("type").and_then(Value::as_str) != Some("group") {
                return None;
            }
            let (gt, gr, gs) = placement(g);
            (t, r, s) = (t + r.apply(gt * s), r.mul(gr), s * gs);
            for key in ["material", "shadow", "friction", "bounce"] {
                if let Some(v) = g.get(key) {
                    inherited.insert(key.into(), v.clone());
                }
            }
        }
        let mut n = self.node(top, sub)?.clone();
        let (mt, mr, ms) = placement(&n);
        set_placement(&mut n, (t + r.apply(mt * s), r.mul(mr), s * ms));
        if let Some(o) = n.as_object_mut() {
            for (key, v) in inherited {
                o.entry(key).or_insert(v);
            }
        }
        self.checkpoint(true);
        self.node_mut(top, parent)?.get_mut("children")?.as_array_mut()?.remove(k);
        self.array_mut("objects").insert(top + 1, n);
        self.touch();
        Some(Selection::Node { top: top + 1, sub: vec![] })
    }

    /// Saves to `path` (pretty JSON).
    pub fn save(&mut self, path: &std::path::Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(&self.v).map_err(|e| e.to_string())?;
        std::fs::write(path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))?;
        self.dirty = false;
        Ok(())
    }
}

/// A node's position, rotation and scale.
fn placement(n: &Value) -> (Vec3, Mat3, f32) {
    let v = |key: &str| n.get(key).and_then(as_vec3).map(|a| Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32));
    let s = n.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
    (v("position").unwrap_or(Vec3::ZERO), v("rotation").map_or(Mat3::IDENTITY, |a| Mat3::euler_deg(a.to_array())), s)
}

/// Sets a node's position, rotation and scale (leaving out what is none).
fn set_placement(n: &mut Value, (t, r, s): (Vec3, Mat3, f32)) {
    let Some(o) = n.as_object_mut() else { return };
    o.insert("position".into(), vec3(t.to_array().map(f64::from)));
    let a = r.to_euler_deg().map(|v| (v as f64 * 100.0).round() / 100.0);
    if a.iter().all(|v| *v == 0.0) {
        o.remove("rotation");
    } else {
        o.insert("rotation".into(), vec3(a));
    }
    if (s - 1.0).abs() < 1e-5 {
        o.remove("scale");
    } else {
        o.insert("scale".into(), serde_json::json!((s as f64 * 1e4).round() / 1e4));
    }
}

/// Child `k` of a node: `children[k]`, or `child` for k = 0.
pub fn child(n: &Value, k: usize) -> Option<&Value> {
    if let Some(kids) = n.get("children").and_then(Value::as_array) {
        return kids.get(k);
    }
    if k == 0 { n.get("child").or_else(|| n.get("emit")) } else { None }
}

fn child_mut(n: &mut Value, k: usize) -> Option<&mut Value> {
    if n.get("children").is_some_and(Value::is_array) {
        return n.get_mut("children")?.as_array_mut()?.get_mut(k);
    }
    if k == 0 {
        if n.get("child").is_some() { n.get_mut("child") } else { n.get_mut("emit") }
    } else {
        None
    }
}

/// The children of a node, in order.
pub fn children(n: &Value) -> Vec<&Value> {
    if let Some(kids) = n.get("children").and_then(Value::as_array) {
        return kids.iter().collect();
    }
    // (an emitter's emitted object shows as its child)
    n.get("child").or_else(|| n.get("emit")).into_iter().collect()
}

pub fn as_vec3(v: &Value) -> Option<[f64; 3]> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
}

pub fn vec3(a: [f64; 3]) -> Value {
    let r = |x: f64| (x * 1e4).round() / 1e4;
    serde_json::json!([r(a[0]), r(a[1]), r(a[2])])
}

/// A node's label: its name, or its type.
pub fn label(n: &Value) -> String {
    let ty = n.get("type").and_then(Value::as_str).unwrap_or("?");
    match n.get("name").and_then(Value::as_str) {
        Some(name) => format!("{name}  ({ty})"),
        None => ty.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Every change gives the document a new version, and no two documents
    /// ever share one (a scene opened after another at version 1 was taken
    /// for it: not compiled, the inspector's cache kept).
    #[test]
    fn versions_are_never_reused() {
        let (mut a, b) = (Doc::new(json!({})), Doc::new(json!({})));
        assert_ne!(a.version, b.version);
        let before = a.version;
        a.touch();
        assert!(a.version > before && a.version != b.version);
        a.checkpoint(true);
        a.v = json!({ "objects": [] });
        a.touch();
        let edited = a.version;
        assert!(a.undo() && a.version > edited, "undo is a change too");
    }

    /// Into a turned, scaled group and out again: it stays where it was.
    #[test]
    fn into_and_out_of_a_group() {
        let mut d = Doc::new(json!({ "objects": [
            { "type": "sphere", "radius": 1, "position": [3, 1, -2], "rotation": [0, 30, 0], "scale": 1.5 },
            { "type": "group", "position": [10, 0, 0], "rotation": [0, 90, 0], "scale": 2, "material": "red", "children": [] }
        ]}));
        let sel = d.into_group(0, 1).expect("moved in");
        assert_eq!(sel, Selection::Node { top: 0, sub: vec![0] });
        assert_eq!(d.objects().len(), 1);
        let local = d.node(0, &[0]).unwrap().clone();
        assert!((local["scale"].as_f64().unwrap() - 0.75).abs() < 1e-4, "{local}");
        let sel = d.out_of_group(0, &[0]).expect("moved out");
        assert_eq!(sel, Selection::Node { top: 1, sub: vec![] });
        let n = d.node(1, &[]).unwrap();
        let p = as_vec3(&n["position"]).unwrap();
        assert!((p[0] - 3.0).abs() < 1e-3 && (p[1] - 1.0).abs() < 1e-3 && (p[2] + 2.0).abs() < 1e-3, "{n}");
        let r = as_vec3(&n["rotation"]).unwrap();
        assert!(r[0].abs() < 0.02 && (r[1] - 30.0).abs() < 0.02 && r[2].abs() < 0.02, "{n}");
        assert!((n["scale"].as_f64().unwrap() - 1.5).abs() < 1e-4);
        // it keeps the group's material, which it had while in it
        assert_eq!(n["material"], json!("red"));
        // only a group's members come out
        assert!(d.out_of_group(0, &[]).is_none());
    }
}
