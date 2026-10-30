//! Emitters: top-level dummy objects that send out copies of an object at a
//! rate. A copy grows in, lives for its lifespan (or for ever) and shrinks
//! away at the end. Copies fly from the emitter; with physics they are bodies
//! that collide, roll and pile up, and rest as fixed objects once still (a
//! knock wakes them). They are never part of the document, so they are not
//! saved and not undone: the App compiles them in after the document's
//! objects, each known by its id (from BASE up) where objects have indices.

use super::physics::{self, Body};
use crate::math::{Mat3, Vec3};
use crate::scene::json::{NodeDef, ShapeDef};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Emitted objects' ids start here (documents never have this many objects).
pub const BASE: usize = 1 << 24;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    /// flying on its own (no physics: it passes through things)
    Flying,
    /// a physics body
    Body,
    /// still, where it stopped (or shrinking away)
    Resting,
}

pub struct Particle {
    pub id: usize,
    /// the document index of the emitter that sent it
    pub emitter: usize,
    /// the emitted node, placed where it is now (at full scale)
    pub node: Value,
    full_scale: f64,
    born: f32,
    vel: Vec3,
    gravity: bool,
    pub state: State,
    /// collides (off: it flies through everything, and nothing knocks it)
    pub physics: bool,
    lifespan: f32,
    grow: f32,
    fade: f32,
}

impl Particle {
    /// Its size now, as a fraction of its full size: growing in, shrinking away.
    fn size(&self, now: f32) -> f32 {
        let age = now - self.born;
        let ease = |t: f32| {
            let t = t.clamp(0.0, 1.0);
            t * t * (3.0 - 2.0 * t)
        };
        let grown = if self.grow > 0.0 { ease(age / self.grow) } else { 1.0 };
        let left = if self.lifespan > 0.0 && self.fade > 0.0 { ease((self.lifespan - age) / self.fade) } else { 1.0 };
        grown * left
    }

    fn fading(&self, now: f32) -> bool {
        self.lifespan > 0.0 && now - self.born > self.lifespan - self.fade
    }

    fn changing(&self, now: f32) -> bool {
        self.state != State::Resting || now - self.born < self.grow || self.fading(now)
    }
}

/// An emitter's settings, read from its node.
struct Settings {
    rate: f32,
    lifespan: f32,
    grow: f32,
    fade: f32,
    speed: f32,
    spread: f32,
    physics: bool,
    max: usize,
    seed: u64,
    emit: Value,
    pos: Vec3,
    rot: Mat3,
    scale: f32,
}

fn settings(node: &Value) -> Option<Settings> {
    let def: NodeDef = serde_json::from_value(node.clone()).ok()?;
    if def.disabled {
        return None;
    }
    let ShapeDef::Emitter { rate, lifespan, grow, fade, speed, spread, physics, max, seed, emit, .. } = def.shape else { return None };
    let emit = match node.get("emit") {
        Some(e) if emit.is_some() => e.clone(),
        _ => json!({ "type": "sphere", "radius": 0.12, "material": { "color": [0.85, 0.3, 0.2], "roughness": 0.4 } }),
    };
    // a switched-off object is not sent out
    if emit.get("disabled").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    Some(Settings {
        rate: rate.max(0.0),
        lifespan: lifespan.max(0.0),
        grow: grow.max(0.0),
        fade: fade.max(0.0),
        speed,
        spread: spread.clamp(0.0, 180.0),
        physics,
        max: max.max(1) as usize,
        seed: seed as u64,
        emit,
        pos: def.position.map(Vec3::from_array).unwrap_or(Vec3::ZERO),
        rot: def.rotation.map(Mat3::euler_deg).unwrap_or(Mat3::IDENTITY),
        scale: def.scale.unwrap_or(1.0),
    })
}

#[derive(Default)]
pub struct Emitters {
    pub particles: Vec<Particle>,
    next: usize,
    /// emissions owed per emitter (fractions carry over between frames),
    /// and how many each has sent (its random directions follow from that
    /// and its seed)
    owed: HashMap<usize, f32>,
    sent: HashMap<usize, u64>,
    /// each emitter's node as last seen, by index, and the object count then:
    /// how copies find their emitter again when objects before it come and go
    seen: HashMap<usize, u64>,
    objects: usize,
    /// seconds the emitters have run: the physics' time (the frames' time,
    /// a twentieth of a second at most each), so that copies come as fast as
    /// the physics can take them
    time: f32,
    /// counts changes: the scene is compiled again when it moves
    pub version: u64,
}

/// What a step asks of the physics: bodies to add, and ids whose bodies go.
pub struct Changes {
    pub bodies: Vec<Body>,
    pub gone: Vec<usize>,
    pub spawned: bool,
}

impl Emitters {
    pub fn clear(&mut self) {
        if !self.particles.is_empty() {
            self.particles.clear();
            self.version += 1;
        }
        self.owed.clear();
        self.sent.clear();
        self.seen.clear();
    }

    /// Follows the emitters to where they are in `objs` now (objects before
    /// them added or removed, by hand, undo or remotely): their copies, debts
    /// and counts go with them; an emitter that is gone takes its copies along.
    fn follow(&mut self, objs: &[Value]) {
        let print = |o: &Value| {
            let mut h = DefaultHasher::new();
            o.to_string().hash(&mut h);
            h.finish()
        };
        let prints: HashMap<usize, u64> =
            objs.iter().enumerate().filter(|(_, o)| o.get("type").and_then(Value::as_str) == Some("emitter")).map(|(i, o)| (i, print(o))).collect();
        if prints == self.seen && objs.len() == self.objects {
            return;
        }
        let same_count = objs.len() == self.objects;
        let mut to: HashMap<usize, usize> = HashMap::new();
        for (&old, &fp) in &self.seen {
            if prints.get(&old) == Some(&fp) {
                to.insert(old, old);
                continue;
            }
            let mut found = prints.iter().filter(|(_, f)| **f == fp).map(|(i, _)| *i);
            match (found.next(), found.next()) {
                (Some(i), None) => {
                    to.insert(old, i);
                }
                // edited where it is
                _ if same_count && prints.contains_key(&old) => {
                    to.insert(old, old);
                }
                _ => {}
            }
        }
        for p in &mut self.particles {
            p.emitter = to.get(&p.emitter).copied().unwrap_or(usize::MAX);
        }
        self.owed = self.owed.drain().filter_map(|(k, v)| to.get(&k).map(|&k| (k, v))).collect();
        self.sent = self.sent.drain().filter_map(|(k, v)| to.get(&k).map(|&k| (k, v))).collect();
        self.seen = prints;
        self.objects = objs.len();
    }

    /// A random number in [0, 1) for emitter `i`'s `n`th copy, draw `k`.
    fn random(seed: u64, i: usize, n: u64, k: u64) -> f32 {
        // splitmix64
        let mut z = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9) ^ n.wrapping_mul(0x94D0_49BB_1331_11EB) ^ k;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        (z >> 40) as f32 / (1u64 << 24) as f32
    }

    /// The copies that were bodies, when the physics lost every body (an
    /// undo, a deletion): bodies again, from where they are, or resting.
    pub fn bodies_again(&mut self, doc: &Value) -> Vec<Body> {
        let mut out = Vec::new();
        for i in 0..self.particles.len() {
            if self.particles[i].state != State::Body {
                continue;
            }
            match self.body(doc, &self.particles[i]) {
                Some(b) => out.push(b),
                None => self.particles[i].state = State::Resting,
            }
        }
        out
    }

    /// Copies that should be bodies but have none (their object did not
    /// compile) rest where they are.
    pub fn orphans(&mut self, has_body: impl Fn(usize) -> bool) {
        for p in &mut self.particles {
            if p.state == State::Body && !has_body(p.id) {
                p.state = State::Resting;
                self.version += 1;
            }
        }
    }

    pub fn find(&self, id: usize) -> Option<&Particle> {
        self.particles.iter().find(|p| p.id == id)
    }

    /// Sends out what the emitters owe, ages and flies the copies (bodies
    /// move in the physics), and lets go of the dead.
    /// (`gravity`: the scene's, m/s²)
    pub fn step(&mut self, doc: &Value, dt: f32, gravity: f32) -> Changes {
        let mut ch = Changes { bodies: Vec::new(), gone: Vec::new(), spawned: false };
        // (borrowed: a clone of every object, every frame, was the cost here)
        let objs: &[Value] = doc.get("objects").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
        let mut gone = Vec::new();
        self.follow(objs);
        self.time += dt;
        let now = self.time;
        // copies of emitters that are gone (or no longer emitters) go too
        let live: Vec<bool> = objs.iter().map(|o| o.get("type").and_then(Value::as_str) == Some("emitter")).collect();
        let before = self.particles.len();
        self.particles.retain(|p| {
            let keep = live.get(p.emitter).copied().unwrap_or(false);
            if !keep {
                gone.push(p.id);
            }
            keep
        });
        // the dead
        self.particles.retain(|p| {
            let dead = p.lifespan > 0.0 && now - p.born >= p.lifespan;
            if dead {
                gone.push(p.id);
            }
            !dead
        });
        let mut changed = before != self.particles.len() || !gone.is_empty();
        // the fading stop moving (they shrink where they are)
        for p in &mut self.particles {
            if p.state == State::Body && p.fading(now) {
                p.state = State::Resting;
                gone.push(p.id);
            }
        }
        ch.gone = gone;
        // what each emitter owes
        for (i, o) in objs.iter().enumerate() {
            if !live[i] {
                continue;
            }
            let Some(s) = settings(o) else { continue };
            let owed = self.owed.entry(i).or_insert(0.0);
            *owed = (*owed + s.rate * dt).min(s.max as f32);
            let n = owed.floor() as usize;
            *owed -= n as f32;
            // the oldest go first when there are too many (also when the
            // maximum was lowered)
            let mine = self.particles.iter().filter(|p| p.emitter == i).count();
            let over = (mine + n).saturating_sub(s.max);
            if over > 0 {
                let mut k = 0;
                let gone = &mut ch.gone;
                self.particles.retain(|p| {
                    let old = p.emitter == i && k < over;
                    if p.emitter == i {
                        k += 1;
                    }
                    if old {
                        gone.push(p.id);
                    }
                    !old
                });
                changed = true;
            }
            for _ in 0..n.min(s.max) {
                if let Some(b) = self.spawn(doc, i, &s, now) {
                    ch.bodies.push(b);
                }
                ch.spawned = true;
            }
        }
        // the flying fly
        for p in &mut self.particles {
            if p.state != State::Flying {
                continue;
            }
            if p.gravity {
                p.vel.y -= gravity * dt;
            }
            if let Some(a) = p.node.get("position").and_then(super::doc::as_vec3) {
                let at = Vec3::new(a[0] as f32, a[1] as f32, a[2] as f32) + p.vel * dt;
                p.node["position"] = super::doc::vec3([at.x as f64, at.y as f64, at.z as f64]);
            }
            // far below everything: gone
            if p.node.get("position").and_then(super::doc::as_vec3).is_some_and(|a| a[1] < -200.0) {
                p.lifespan = 0.001;
            }
        }
        changed |= ch.spawned || !ch.gone.is_empty() || self.particles.iter().any(|p| p.changing(now));
        if changed {
            self.version += 1;
        }
        ch
    }

    fn spawn(&mut self, doc: &Value, emitter: usize, s: &Settings, now: f32) -> Option<Body> {
        // a direction in the cone about the emitter's +y
        let sent = self.sent.entry(emitter).or_insert(0);
        let n = *sent;
        *sent += 1;
        let (u1, u2) = (Self::random(s.seed, emitter, n, 1), Self::random(s.seed, emitter, n, 2));
        let theta = s.spread.to_radians() * u1.sqrt();
        let phi = u2 * std::f32::consts::TAU;
        let local = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
        let dir = s.rot.apply(local);
        // from the nozzle's tip
        let at = s.pos + s.rot.apply(Vec3::new(0.0, 0.15 * s.scale, 0.0));
        let mut node = s.emit.clone();
        let full_scale = node.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
        if let Some(o) = node.as_object_mut() {
            o.insert("position".into(), super::doc::vec3([at.x as f64, at.y as f64, at.z as f64]));
            o.remove("locked");
        }
        let gravity = node.get("gravity").and_then(Value::as_bool).unwrap_or(true);
        let id = BASE + self.next;
        self.next += 1;
        let mut p = Particle {
            id,
            emitter,
            node,
            full_scale,
            born: now,
            vel: dir * s.speed,
            gravity,
            state: State::Flying,
            physics: s.physics,
            lifespan: s.lifespan,
            grow: s.grow,
            fade: s.fade,
        };
        let body = if s.physics { self.body(doc, &p) } else { None };
        let body = body.map(|mut b| {
            b.v = p.vel;
            b
        });
        if body.is_some() {
            p.state = State::Body;
        }
        self.particles.push(p);
        body
    }

    fn body(&self, doc: &Value, p: &Particle) -> Option<Body> {
        let def: NodeDef = serde_json::from_value(p.node.clone()).ok()?;
        Body::new(p.id, &def, physics::density_of(doc, &p.node))
    }

    /// A body for a resting copy that something knocked.
    pub fn wake(&mut self, doc: &Value, id: usize) -> Option<Body> {
        let now = self.time;
        let p = self.particles.iter().find(|p| p.id == id)?;
        if !p.physics || p.fading(now) {
            return None;
        }
        let b = self.body(doc, p)?;
        if let Some(p) = self.particles.iter_mut().find(|p| p.id == id) {
            p.state = State::Body;
        }
        Some(b)
    }

    /// Where a copy's body moved to.
    pub fn place(&mut self, id: usize, pos: [f32; 3], rot: [f32; 3]) {
        if let Some(p) = self.particles.iter_mut().find(|p| p.id == id) {
            p.node["position"] = super::doc::vec3(pos.map(|v| v as f64));
            p.node["rotation"] = super::doc::vec3(rot.map(|v| v as f64));
        }
    }

    /// A copy's body came to rest: it stays where it is.
    pub fn rest(&mut self, id: usize) {
        if let Some(p) = self.particles.iter_mut().find(|p| p.id == id) {
            p.state = State::Resting;
        }
        self.version += 1;
    }

    /// The copies as nodes to compile after the document's, at their size now.
    pub fn nodes(&self) -> Vec<Value> {
        let now = self.time;
        self.particles
            .iter()
            .map(|p| {
                let mut n = p.node.clone();
                n["scale"] = json!((p.full_scale * p.size(now) as f64).max(1e-3));
                n
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_ages_and_lets_go() {
        let doc = json!({ "objects": [
            { "type": "plane" },
            { "type": "emitter", "rate": 10, "lifespan": 1, "grow": 0.2, "fade": 0.2, "physics": false, "max": 5, "position": [0, 1, 0] }
        ]});
        let mut e = Emitters::default();
        let mut spawned = 0;
        for _ in 0..30 {
            let ch = e.step(&doc, 0.05, crate::scene::EARTH_GRAVITY);
            spawned += ch.spawned as usize;
            assert!(ch.bodies.is_empty(), "no physics: no bodies");
            assert!(e.particles.len() <= 5, "max");
        }
        assert!(spawned > 0);
        // they fly from the nozzle and fall
        let p = &e.particles[0];
        assert_eq!(p.emitter, 1);
        let y = p.node["position"][1].as_f64().unwrap();
        assert!(y != 1.15, "it moved");
        // grow and fade
        let q = Particle { id: BASE, emitter: 1, node: json!({}), full_scale: 1.0, born: 0.0, vel: Vec3::ZERO, gravity: false, state: State::Resting, physics: false, lifespan: 1.0, grow: 0.2, fade: 0.2 };
        assert!(q.size(0.0) < 0.01 && (q.size(0.5) - 1.0).abs() < 1e-6 && q.size(0.99) < 0.05);
        // the emitter gone: its copies go
        let none = json!({ "objects": [{ "type": "plane" }] });
        e.step(&none, 0.05, crate::scene::EARTH_GRAVITY);
        assert!(e.particles.is_empty());
    }
}
