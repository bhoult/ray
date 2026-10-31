//! Game mode: a walker in the scene, seen through its eyes. WASD walks (Shift
//! runs), Space jumps, the right mouse button fires a rocket pack on its back
//! (it builds up while held; it leans with the look: looking down tilts the
//! push forward), R and F
//! grow and shrink it with its feet where they are, the mouse looks. In
//! water it walks the bottom as deep as it goes; with nothing under it, it
//! floats (eyes above the surface, riding the waves) and swims; Space kicks
//! up, Space looking down dives, and under water it swims where it looks
//! until it comes up; swimming into a ledge climbs out. Its body is three spheres stacked into a
//! capsule, pushed out of whatever it overlaps by the scene's distance: the
//! scene as the frame draws it (render::view_query: fractals at the frame's
//! detail, anchored at the camera), so it can walk into a sponge's tunnels a
//! micrometre wide. Speed, gravity and thrust scale with its height, so a
//! fall or a hop takes as long at any size; positions are f64, since a body
//! deep in a fractal is micrometres tall.

use crate::render::trace::dist_query_at;
use crate::render::{Ctx, vm::Regs};
use crate::simd::{F8, M8, V8};
use std::time::Instant;

type D3 = [f64; 3];

/// The eye's height and the body spheres' radius, as fractions of its height.
pub const EYE: f64 = 0.92;
const RADIUS: f64 = 0.2;
/// A person's height (m): speeds, gravity and thrust are a person's per this.
pub const HUMAN: f64 = 1.8;
/// Heights it can take (m): an anchored sponge's finest holes are 0.4 nm
/// (20 levels), so 10 nm still sees them.
pub const MIN_HEIGHT: f64 = 1e-8;
pub const MAX_HEIGHT: f64 = 1e4;
/// Walking and running (m/s for a person).
const WALK: f64 = 2.5;
const RUN: f64 = 6.5;
/// Earth's gravity (m/s²): the rocket's push and a jump are a person's on
/// Earth, so where the scene's gravity is weaker they go further.
const G: f64 = crate::scene::EARTH_GRAVITY as f64;
/// The rocket pack's push at full power (in Earth g) and the fastest it
/// climbs (m/s for a person); how far it leans with the look at most
/// (radians: past that it would tip over); how much the air holds it back
/// across (per second), and what WASD adds there (m/s² for a person).
const THRUST: f64 = 2.0;
/// Seconds the rocket takes to build from nothing to full power while held,
/// and to die away once let go.
const SPOOL_UP: f64 = 2.0;
const SPOOL_DOWN: f64 = 0.6;
const CLIMB: f64 = 8.0;
pub const LEAN: f64 = 40.0 * std::f64::consts::PI / 180.0;
const DRAG: f64 = 0.6;
const AIR: f64 = 4.0;
/// A jump's push (m/s for a person: a metre high on Earth), and with a
/// direction held, the push that way on top of walking (running) speed.
const JUMP: f64 = 4.43;
const LEAP: f64 = 2.0;
/// The fastest it falls (m/s for a person).
const FALL: f64 = 60.0;
/// In water: the share of the body under the surface when it floats (its
/// buoyancy then equals its weight: a person floats chest deep, the eyes
/// out), how fast the water stops it rising and sinking and moving across
/// (per second, at that depth), swimming speed and how quickly it gets there
/// (m/s and per second, for a person), a kick up (Space), how fast it
/// scrambles up what it swims into (m/s for a person), and how high, of its
/// height above where it began.
const FLOAT: f64 = 0.75;
const WATER_DRAG: f64 = 4.0;
const WATER_DRAG_ACROSS: f64 = 1.5;
const SWIM: f64 = 1.3;
const SWIM_RUN: f64 = 2.2;
const SWIM_GRIP: f64 = 2.5;
const KICK: f64 = 3.0;
/// A dive: Space while swimming at the surface looking further down than
/// this (radians); and under water by more than PLUNGE of its height (fallen
/// in, walked off the bottom's edge) it is diving too.
const DIVE_LOOK: f64 = -0.25;
const PLUNGE: f64 = 0.15;
const SCRAMBLE: f64 = 1.5;
const CLIMB_OUT: f64 = 1.25;
/// The highest ledge it steps up onto, of its height.
const STEP: f64 = 0.35;
/// R / F: times bigger or smaller a second (with Shift).
const GROW: f64 = 2.5;
const GROW_FAST: f64 = 12.0;

/// What the keys say this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// -1..1: back / forward, left / right
    pub forward: f64,
    pub right: f64,
    pub run: bool,
    /// the rocket pack held (right mouse button)
    pub jet: bool,
    /// jumps if standing (Space, as it goes down)
    pub jump: bool,
    /// 1 grows, -1 shrinks
    pub grow: f64,
    /// grow and shrink faster
    pub fast: bool,
}

pub struct Game {
    /// the bottom of the body
    pub feet: D3,
    pub vel: D3,
    pub height: f64,
    pub grounded: bool,
    /// the rocket is pushing (building up or dying away)
    pub jetting: bool,
    /// the rocket's power, 0..1 (SPOOL_UP, SPOOL_DOWN), and the power at which
    /// it holds the walker up at this lean under this gravity (above 1: it
    /// cannot)
    pub power: f64,
    pub lift: f64,
    /// how far it leans forward (radians; negative: back): with the look
    pub lean: f64,
    /// swimming under water: where it looks, neither sinking nor floating up,
    /// until its head is out again
    pub diving: bool,
    /// the share of the body under water (0: dry); whether it is scrambling
    /// out of the water (the water neither holds it back nor up), and the
    /// height of its feet when it began
    pub wet: f64,
    climbing: bool,
    climb_from: f64,
    /// a scramble got no further: none again until the keys are let go
    climb_spent: bool,
    /// when it started (the hint shows for a while)
    pub since: Instant,
}

/// The scene's distance near the camera, as the frame draws it.
pub struct Probe<'a, 'c> {
    ctx: &'a Ctx<'c>,
    regs: &'a mut Regs,
}

impl<'a, 'c> Probe<'a, 'c> {
    pub fn new(ctx: &'a Ctx<'c>, regs: &'a mut Regs) -> Self {
        Probe { ctx, regs }
    }

    /// The distance at up to eight points (at most `radius`): as world
    /// points, and as offsets from the frame's anchor, which f32 holds finely
    /// near the camera (anchored fractals use those).
    fn dist(&mut self, pts: &[D3], radius: f64) -> [f64; 8] {
        let a = self.ctx.anchor;
        let anchor = [a.x as f64, a.y as f64, a.z as f64];
        let lanes = |c: usize, rel: bool| {
            F8::from_array(std::array::from_fn(|l| {
                let p = pts[l.min(pts.len() - 1)];
                if rel { (p[c] - anchor[c]) as f32 } else { p[c] as f32 }
            }))
        };
        let p = V8::new(lanes(0, false), lanes(1, false), lanes(2, false));
        let dp = V8::new(lanes(0, true), lanes(1, true), lanes(2, true));
        let (d, _) = dist_query_at(self.ctx, self.regs, p, dp, radius as f32, M8::none_set(), &[]);
        d.to_array().map(f64::from)
    }

    /// Which way the distance grows at `p` (unit), from taps `eps` either
    /// side along each axis: where two surfaces meet (a floor and a step's
    /// face) an axis neither changes along gets exactly nothing, where four
    /// tetrahedral taps slid the walker sideways along the step.
    /// (A shape not anchored at the camera is f32 in world coordinates: for
    /// a tiny walker far from the origin, taps closer than a float step meet
    /// at one point and say nothing. Then wider taps, until they do.)
    fn normal(&mut self, p: D3, eps: f64, radius: f64) -> D3 {
        let mut eps = eps;
        for _ in 0..8 {
            let taps: [D3; 6] = std::array::from_fn(|i| {
                let mut q = p;
                q[i / 2] += if i % 2 == 0 { eps } else { -eps };
                q
            });
            let d = self.dist(&taps, radius);
            let g = [d[0] - d[1], d[2] - d[3], d[4] - d[5]];
            if len(g) > 0.0 {
                return unit(g);
            }
            eps *= 8.0;
        }
        [0.0; 3]
    }
}

impl Game {
    /// A walker whose eye is at `eye` (the camera): a person's height, or
    /// standing on what is below when that is nearer, and smaller still where
    /// the space around is tight. The eye stays where it is, unless the
    /// camera is inside something: then the body is pushed out.
    pub fn enter(probe: &mut Probe, eye: D3) -> Game {
        // what is below, sphere-traced to a person's eye height
        let mut down = None;
        let mut t = 0.0;
        for _ in 0..400 {
            let d = probe.dist(&[[eye[0], eye[1] - t, eye[2]]], HUMAN)[0];
            if d < (t * 1e-4).max(1e-13) {
                down = Some(t);
                break;
            }
            t += d;
            if t > HUMAN * EYE {
                break;
            }
        }
        let mut h = down.map_or(HUMAN, |t| (t / EYE).min(HUMAN)).clamp(MIN_HEIGHT, MAX_HEIGHT);
        for _ in 0..120 {
            let r = RADIUS * h;
            let d = probe.dist(&centres(feet_below(eye, h), h), 4.0 * r);
            if d[..3].iter().all(|&d| d >= 0.98 * r) || h <= MIN_HEIGHT {
                break;
            }
            h = (h * 0.8).max(MIN_HEIGHT);
        }
        let feet = resolve(probe, feet_below(eye, h), h, &mut Vec::new());
        Game::stand(probe, feet, h)
    }

    /// A walker of `height` at rest with its feet at `feet` (where one stood
    /// in a screenshot: put back as it was, not guessed again).
    pub fn stand(probe: &mut Probe, feet: D3, height: f64) -> Game {
        let height = height.clamp(MIN_HEIGHT, MAX_HEIGHT);
        let grounded = ground(probe, feet, height);
        Game { feet, vel: [0.0; 3], height, grounded, jetting: false, power: 0.0, lift: 0.0, lean: 0.0, diving: false, wet: 0.0, climbing: false, climb_from: 0.0, climb_spent: false, since: Instant::now() }
    }

    pub fn eye(&self) -> D3 {
        [self.feet[0], self.feet[1] + EYE * self.height, self.feet[2]]
    }

    /// Out of what it is stuck in (where `resolve` finds no way out: inside
    /// a fractal): to the nearest place its body fits, at rest, as tall as
    /// it was. How far that was (nothing when it fits where it is), or None
    /// when nowhere within a kilometre (or ten thousand heights). (One that
    /// came into game mode inside a fractal is 10 nm tall, its way out
    /// perhaps a metre off.)
    pub fn get_out(&mut self, probe: &mut Probe) -> Option<f64> {
        let h = self.height;
        let feet = open_space(probe, self.feet, &centres([0.0; 3], h), 0.98 * RADIUS * h, (1e4 * h).max(1e3))?;
        let moved = len(sub(feet, self.feet));
        if moved > 0.0 {
            let since = self.since;
            *self = Game::stand(probe, feet, h);
            self.since = since;
        }
        Some(moved)
    }

    /// `dt` seconds of the walker looking along `yaw` and `pitch` (radians,
    /// up positive), doing what `input` says, under `gravity` (m/s², down:
    /// the scene's; a body of another size feels it scaled with it), with
    /// `water`'s surface height at (x, z) when the scene has water.
    pub fn step(&mut self, probe: &mut Probe, (yaw, pitch): (f32, f32), input: &Input, dt: f64, gravity: f64, water: Option<&dyn Fn(f64, f64) -> f64>) {
        self.resize(probe, input, dt);
        let (h, s) = (self.height, self.height / HUMAN);
        let r = RADIUS * h;
        let g = gravity.max(0.0) * s;
        let (sy, cy) = (yaw as f64).sin_cos();
        let ahead = [-sy, 0.0, -cy];
        let mut wish = add(scale(ahead, input.forward), scale([cy, 0.0, -sy], input.right));
        let l = len(wish);
        if l > 1.0 {
            wish = scale(wish, 1.0 / l);
        }
        // how much of it is under water (from the feet up), and whether its
        // eye is, by more than `by` of its height
        let wet_at = |f: D3| water.map_or(0.0, |w| ((w(f[0], f[2]) - f[1]) / h).clamp(0.0, 1.0));
        let eye_under = |f: D3, by: f64| water.is_some_and(|w| w(f[0], f[2]) > f[1] + (EYE + by) * h);
        self.wet = wet_at(self.feet);
        // under water it swims along the look, up and down too
        let (sp, cp) = (pitch as f64).sin_cos();
        let look = [-sy * cp, sp, -cy * cp];
        let mut dive_wish = add(scale(look, input.forward), scale([cy, 0.0, -sy], input.right));
        if len(dive_wish) > 1.0 {
            dive_wish = unit(dive_wish);
        }
        // wading is slower, the deeper the slower
        let speed = if input.run { RUN } else { WALK } * s * (1.0 - 0.6 * self.wet);
        let target = scale(wish, speed);
        let swim = if input.run { SWIM_RUN } else { SWIM } * s;
        // the rocket is on its back: looking down it leans forward and the
        // push tips forward, looking up back; never past LEAN
        self.lean = (-pitch as f64).clamp(-LEAN, LEAN);
        let push = add(scale(ahead, self.lean.sin()), [0.0, self.lean.cos(), 0.0]);
        self.lift = gravity.max(0.0) / (THRUST * G * self.lean.cos());
        // in the water, Space kicks up (out at the surface, onto a low edge;
        // under water, toward the surface); at the surface looking down, it
        // dives: head first, under
        if input.jump && !self.grounded && (self.diving || self.wet > 0.2) {
            if !self.diving && (pitch as f64) < DIVE_LOOK {
                self.diving = true;
                self.climbing = false;
                self.vel = [look[0] * KICK * s, -KICK * s, look[2] * KICK * s];
            } else {
                self.vel[1] = self.vel[1].max(KICK * s);
            }
        }
        if input.jump && self.grounded {
            self.vel[1] = JUMP * s;
            // with a direction held, a leap that way: at least the speed the
            // keys walk (run) at, and a push more
            if len(wish) > 0.1 {
                let d = unit(wish);
                let want = speed * len(wish) + LEAP * s;
                let along = dot(self.vel, d);
                if along < want {
                    self.vel = add(self.vel, scale(d, want - along));
                }
            }
            self.grounded = false;
        }
        // in steps of at most half a sphere's radius: a wall or a floor
        // thinner than that is never passed through
        let most = len(self.vel) + speed.max(swim) + (CLIMB + JUMP + LEAP + (G + g / s.max(1e-300)) * dt) * s;
        let n = (most * dt / (0.5 * r)).ceil().clamp(1.0, 256.0) as usize;
        let ds = dt / n as f64;
        for _ in 0..n {
            let wet = wet_at(self.feet);
            // diving: until the head comes up out of the water (not on the
            // way down from the surface), it stands on the bottom, or the
            // rocket fires; under water with nothing to stand on (fallen in,
            // off the bottom's edge, jumped up from it), diving again
            if self.diving && (self.grounded || self.power > 0.0 || (self.vel[1] >= 0.0 && !eye_under(self.feet, 0.0))) {
                self.diving = false;
            }
            if !self.diving && !self.grounded && !self.climbing && self.power == 0.0 && eye_under(self.feet, PLUNGE) {
                self.diving = true;
            }
            let swimming = !self.grounded && !self.diving && (wet > 0.3 || self.climbing);
            if self.grounded {
                // across: toward what the keys ask, quickly
                let k = 1.0 - (-12.0 * ds).exp();
                self.vel[0] += (target[0] - self.vel[0]) * k;
                self.vel[2] += (target[2] - self.vel[2]) * k;
            } else if self.diving {
                // under water: toward what the keys ask along the look, as
                // fast as one swims; still when they ask nothing
                let k = 1.0 - (-SWIM_GRIP * ds).exp();
                for (c, v) in self.vel.iter_mut().enumerate() {
                    *v += (dive_wish[c] * swim - *v) * k;
                }
            } else if swimming {
                // swimming (or scrambling out): toward what the keys ask, as
                // fast as one swims
                let k = 1.0 - (-SWIM_GRIP * ds).exp();
                self.vel[0] += (wish[0] * swim - self.vel[0]) * k;
                self.vel[2] += (wish[2] * swim - self.vel[2]) * k;
            } else {
                // in the air: the keys nudge it, the air slows it (and the
                // water, as much as it is in it)
                let k = (-(DRAG + WATER_DRAG_ACROSS * wet / FLOAT) * ds).exp();
                self.vel[0] = self.vel[0] * k + wish[0] * AIR * s * ds;
                self.vel[2] = self.vel[2] * k + wish[2] * AIR * s * ds;
            }
            // (standing, no gravity: it would sink by less than the push out
            // of the floor, then be pushed back, and the view would never
            // settle; rising off it, gravity again. Buoyancy: the water's
            // push, its weight at FLOAT under. Standing, it keeps to the
            // bottom however deep: it walks into deep water; diving, neither)
            let buoy = if self.climbing { 0.0 } else { g * wet / FLOAT };
            if !self.diving && (!self.grounded || self.vel[1] > 0.0) {
                self.vel[1] += (buoy - g) * ds;
                if wet > 0.0 && !self.climbing {
                    // the water stops its bobbing within a second or two
                    self.vel[1] *= (-WATER_DRAG * (wet / FLOAT).min(1.0) * ds).exp();
                }
            }
            // the rocket builds up while held and dies away when let go
            self.power = if input.jet { (self.power + ds / SPOOL_UP).min(1.0) } else { (self.power - ds / SPOOL_DOWN).max(0.0) };
            if self.power > 0.0 {
                let a = THRUST * G * self.power * s * ds;
                self.vel[0] += push[0] * a;
                self.vel[2] += push[2] * a;
                // standing, the ground carries the weight: only a push past
                // gravity lifts it (less had it hop off and back every other
                // step, the keys working only half the time)
                let up = if self.grounded { (push[1] * a - g * ds).max(0.0) } else { push[1] * a };
                if self.vel[1] < CLIMB * s {
                    self.vel[1] = (self.vel[1] + up).min(CLIMB * s);
                }
            }
            self.vel[1] = self.vel[1].max(-FALL * s);
            self.jetting = self.power > 0.0;
            let was = self.grounded;
            let blocked = self.walk(probe, scale(self.vel, ds));
            // walked into a ledge no higher than STEP: a hop just onto it, at
            // walking speed, so it clears the edge
            if was && blocked && len(wish) > 0.1
                && let Some(rise) = ledge(probe, self.feet, h, unit(wish))
            {
                self.vel = [target[0], (2.0 * g * (rise + 0.15 * r)).sqrt(), target[2]];
                self.grounded = false;
            }
            if self.climbing && (self.grounded || self.vel[1] <= 0.0) {
                // (ended without standing: spent, whatever stopped it)
                self.climb_spent = !self.grounded;
                self.climbing = false;
            }
            // swam into something it cannot stand on (an island's rock, a
            // ledge): it scrambles up it while the keys push into it, until it
            // is over the top, at most CLIMB_OUT of its height above where it
            // began (not up a cliff; an overhang or a branch stops it)
            // One try per push: one that ends without it standing (a cliff
            // higher than CLIMB_OUT, a branch overhead) falls back and waits
            // for the keys to be let go (re-armed, it climbed a cliff, fell
            // and climbed again for ever). The rocket takes over from it.
            if len(wish) < 0.1 || self.grounded {
                self.climb_spent = false;
            }
            if self.power > 0.0 {
                self.climbing = false;
            }
            if swimming && blocked && len(wish) > 0.1 && self.power == 0.0 && (self.climbing || !self.climb_spent) {
                if !self.climbing {
                    self.climbing = true;
                    self.climb_from = self.feet[1];
                }
                if self.feet[1] < self.climb_from + CLIMB_OUT * h {
                    self.vel[1] = self.vel[1].max(SCRAMBLE * s);
                }
            } else if self.climbing && !blocked && len(wish) > 0.1 {
                // over the edge: onto the top at walking pace (at swimming
                // pace it slid back off the lip)
                let on = scale(unit(wish), WALK * s);
                self.vel[0] = on[0];
                self.vel[2] = on[2];
            }
        }
    }

    /// Moves the body by `delta`, sliding along what it meets; whether
    /// something stopped most of its move across.
    fn walk(&mut self, probe: &mut Probe, delta: D3) -> bool {
        let h = self.height;
        let r = RADIUS * h;
        let start = self.feet;
        let mut met = Vec::new();
        let end = resolve(probe, add(start, delta), h, &mut met);
        let across = [delta[0], 0.0, delta[2]];
        let want = len(across);
        let blocked = want > 1e-3 * r && dot(sub(end, start), across) / want < 0.5 * want;
        self.feet = end;
        // nothing of the velocity goes on into what it pushed against: a
        // floor stops the fall, a ceiling the rise, a wall the walk into it
        // (across only: where a floor meets a wall the normal leans, and the
        // walk into the corner turned into a leap up)
        for n in met {
            if n[1] > 0.7 {
                self.vel[1] = self.vel[1].max(0.0);
            } else if n[1] < -0.7 {
                self.vel[1] = self.vel[1].min(0.0);
            } else if self.vel[1] < 0.0 && n[1] > 0.0 && dot(self.vel, n) < 0.0 {
                // falling onto a slope too steep to stand on: it slides down
                // it, its fall into it stopped (else that built up while the
                // slope held it, and it bounced on it: a swimmer against an
                // island's rock below the waterline)
                // (and never makes a rise of it: a running leap onto a steep
                // slope turned its speed across into a launch up)
                self.vel = sub(self.vel, scale(n, dot(self.vel, n)));
                self.vel[1] = self.vel[1].min(0.0);
            } else {
                let w = unit([n[0], 0.0, n[2]]);
                let vn = dot(self.vel, w);
                if vn < 0.0 {
                    self.vel = sub(self.vel, scale(w, vn));
                }
            }
        }
        // (rising off it, as a jump starts, it is not standing: the ground's
        // grip took a leap's push away)
        self.grounded = self.vel[1] <= 0.0 && ground(probe, self.feet, h);
        if self.grounded {
            self.vel[1] = 0.0;
            // down to the ground it stands on (walking down a slope or off a
            // low step, without gravity it would float off it): the gap
            // under its lowest point, where a wall beside is a radius away
            let c = centres(self.feet, h)[0];
            let gap = probe.dist(&[[c[0], c[1] - r, c[2]]], 4.0 * r)[0];
            if gap > 2e-3 * r && gap < 0.5 * r {
                self.feet[1] -= gap - 1e-3 * r;
            }
        }
        blocked
    }

    /// R / F: grows or shrinks the body with its feet where they are; it
    /// grows only where it fits (pushed aside a little at most).
    fn resize(&mut self, probe: &mut Probe, input: &Input, dt: f64) {
        if input.grow == 0.0 {
            return;
        }
        let rate = if input.fast { GROW_FAST } else { GROW };
        let h = (self.height * rate.powf(input.grow * dt)).clamp(MIN_HEIGHT, MAX_HEIGHT);
        if h <= self.height {
            self.set_height(h);
            return;
        }
        let r = RADIUS * h;
        let f = resolve(probe, self.feet, h, &mut Vec::new());
        let d = probe.dist(&centres(f, h), 4.0 * r);
        if d[..3].iter().all(|&d| d >= 0.97 * r) && len(sub(f, self.feet)) < 0.5 * r {
            self.feet = f;
            self.set_height(h);
        }
    }

    /// A new height, its speed scaled with it: so many heights a second at
    /// any size (shrunk in mid-flight at full speed, it shot away and through
    /// walls, its gravity shrunk with it).
    fn set_height(&mut self, h: f64) {
        self.vel = scale(self.vel, h / self.height);
        self.height = h;
    }
}

/// How high a ledge is just past the front of a body (feet `f`, `h` tall,
/// walking along `dir`), when it is one to hop onto: higher than a crumb,
/// no higher than STEP, with the space above it open and room for the body
/// on it. Traced down from STEP above the feet, half a radius past the
/// body's front.
fn ledge(probe: &mut Probe, f: D3, h: f64, dir: D3) -> Option<f64> {
    let r = RADIUS * h;
    let top = STEP * h + 0.1 * r;
    let p = add(add(f, scale(dir, 1.5 * r)), [0.0, top, 0.0]);
    let mut t = 0.0;
    for _ in 0..64 {
        let d = probe.dist(&[[p[0], p[1] - t, p[2]]], 4.0 * r)[0];
        if t == 0.0 && d < 0.5 * r {
            // a wall up there: not a ledge
            return None;
        }
        if d < 1e-3 * r {
            let rise = top - t;
            if !(rise > 0.05 * r && rise <= STEP * h) {
                return None;
            }
            // and the body fits up there, where it is and on the ledge (else
            // a branch overhead stops the hop halfway, again and again)
            let up = [0.0, rise + 0.15 * r, 0.0];
            for at in [add(f, up), add(add(f, scale(dir, 1.5 * r)), up)] {
                if probe.dist(&centres(at, h), 4.0 * r)[..3].iter().any(|&d| d < 0.9 * r) {
                    return None;
                }
            }
            return Some(rise);
        }
        t += d;
        if t > top {
            return None;
        }
    }
    None
}

/// The feet of a body `h` tall whose eye is at `eye`.
fn feet_below(eye: D3, h: f64) -> D3 {
    [eye[0], eye[1] - EYE * h, eye[2]]
}

/// The centres of the body's spheres: feet `f`, `h` tall.
fn centres(f: D3, h: f64) -> [D3; 3] {
    let r = RADIUS * h;
    [0.0, 0.5, 1.0].map(|k| [f[0], f[1] + r + k * (h - 2.0 * r), f[2]])
}

/// The feet once the body (`h` tall) is pushed out of what it overlaps,
/// deepest overlap first; the directions it was pushed in go into `met`.
fn resolve(probe: &mut Probe, mut f: D3, h: f64, met: &mut Vec<D3>) -> D3 {
    let r = RADIUS * h;
    for _ in 0..6 {
        let c = centres(f, h);
        let d = probe.dist(&c, 4.0 * r);
        let (i, di) = (0..3).map(|i| (i, d[i])).fold((0, f64::INFINITY), |a, b| if b.1 < a.1 { b } else { a });
        if di >= r {
            break;
        }
        let n = probe.normal(c[i], 0.05 * r, 4.0 * r);
        if !n.iter().all(|c| c.is_finite()) || len(n) < 0.5 {
            break;
        }
        // (out of a floor it can stand on, straight up: standing it does not
        // creep down a gentle slope, or sideways off an uneven floor)
        let push = if i == 0 && n[1] > 0.7 { [0.0, (r - di + 1e-3 * r) / n[1], 0.0] } else { scale(n, r - di + 1e-3 * r) };
        f = add(f, push);
        met.push(n);
    }
    f
}

/// Whether the body stands on something: half a radius below its lowest
/// sphere's centre, a surface it could stand on (up to about 50 degrees)
/// is nearer than a wall beside it would be. (The nearest surface's normal
/// said "wall" at the foot of a step, where the step's face is nearer than
/// the floor.)
fn ground(probe: &mut Probe, f: D3, h: f64) -> bool {
    let r = RADIUS * h;
    let c = centres(f, h)[0];
    probe.dist(&[[c[0], c[1] - 0.5 * r, c[2]]], 4.0 * r)[0] < 0.73 * r
}

/// The nearest place to `at` where a body has room: each of its points
/// (`body`, offsets from it) at least `room` from anything solid. `at`
/// itself when it has; None when nowhere within `reach` has. Looked for on
/// spheres round `at`, growing outward; on the first with an open
/// direction, each open one is halved back toward the sphere before, and
/// the nearest kept. Only whether a point has room counts: inside a
/// fractal the distance is nothing or noise, no guide to the way out.
pub fn open_space(probe: &mut Probe, at: D3, body: &[D3], room: f64, reach: f64) -> Option<D3> {
    if fits(probe, &[at], body, room)[0] {
        return Some(at);
    }
    let dirs = all_round();
    let (mut inner, mut r) = (0.0, room);
    while r <= reach {
        let sphere: Vec<D3> = dirs.iter().map(|&u| add(at, scale(u, r))).collect();
        let open = fits(probe, &sphere, body, room);
        if open.contains(&true) {
            let mut best = (f64::INFINITY, at);
            for (&u, _) in dirs.iter().zip(&open).filter(|(_, o)| **o) {
                let (mut lo, mut hi) = (inner, r);
                for _ in 0..10 {
                    let mid = 0.5 * (lo + hi);
                    if fits(probe, &[add(at, scale(u, mid))], body, room)[0] {
                        hi = mid;
                    } else {
                        lo = mid;
                    }
                }
                if hi < best.0 {
                    best = (hi, add(at, scale(u, hi)));
                }
            }
            return Some(best.1);
        }
        inner = r;
        r *= 1.15;
    }
    None
}

/// Whether a body (its points `body`, offsets) at each of `at` has `room`
/// round every point.
fn fits(probe: &mut Probe, at: &[D3], body: &[D3], room: f64) -> Vec<bool> {
    let pts: Vec<D3> = at.iter().flat_map(|&a| body.iter().map(move |&b| add(a, b))).collect();
    let mut ok = vec![true; at.len()];
    for (k, chunk) in pts.chunks(8).enumerate() {
        let d = probe.dist(chunk, 2.0 * room);
        for (j, &d) in d[..chunk.len()].iter().enumerate() {
            if d.is_nan() || d < room {
                ok[(8 * k + j) / body.len()] = false;
            }
        }
    }
    ok
}

/// 128 directions spread evenly over the sphere (a Fibonacci spiral).
fn all_round() -> [D3; 128] {
    let turn = std::f64::consts::PI * (3.0 - 5f64.sqrt());
    std::array::from_fn(|i| {
        let y = 1.0 - (i as f64 + 0.5) / 64.0;
        let s = (1.0 - y * y).sqrt();
        let a = turn * i as f64;
        [s * a.cos(), y, s * a.sin()]
    })
}

fn add(a: D3, b: D3) -> D3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: D3, b: D3) -> D3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: D3, k: f64) -> D3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: D3, b: D3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn len(a: D3) -> f64 {
    dot(a, a).sqrt()
}

fn unit(a: D3) -> D3 {
    scale(a, 1.0 / len(a).max(1e-300))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::scene::{Scene, TerrainCache};
    use crate::settings::Settings;

    fn scene(objects: serde_json::Value) -> Scene {
        Scene::from_value(&serde_json::json!({ "objects": objects }), &mut TerrainCache::default()).unwrap()
    }

    /// The camera at `eye` looking along `yaw`, set up for a 640 x 360 view.
    fn camera(eye: D3, yaw: f32) -> Camera {
        let mut c = Camera::new(crate::math::Vec3::ZERO, yaw, 0.0, 60.0);
        c.set_exact(eye);
        c.setup(640, 360);
        c
    }

    fn spawn(sc: &Scene, eye: D3) -> Game {
        crate::render::view_query(sc, &camera(eye, 0.0), &Settings::default(), |ctx, regs| Game::enter(&mut Probe::new(ctx, regs), eye))
    }

    /// `secs` of 60 Hz frames doing `input`, looking along `yaw` (level),
    /// on Earth.
    fn run(sc: &Scene, g: &mut Game, yaw: f32, input: Input, secs: f64) {
        run_at(sc, g, (yaw, 0.0), input, secs, G);
    }

    /// `secs` of 60 Hz frames doing `input`, looking along `(yaw, pitch)`,
    /// under `gravity`.
    fn run_at(sc: &Scene, g: &mut Game, (yaw, pitch): (f32, f32), input: Input, secs: f64, gravity: f64) {
        run_in(sc, g, (yaw, pitch), input, secs, gravity, None);
    }

    /// As run_at, with water whose surface is at `water`'s height.
    fn run_in(sc: &Scene, g: &mut Game, (yaw, pitch): (f32, f32), input: Input, secs: f64, gravity: f64, water: Option<&dyn Fn(f64, f64) -> f64>) {
        let s = Settings::default();
        let dt = 1.0 / 60.0;
        for _ in 0..(secs / dt).round() as usize {
            let cam = camera(g.eye(), yaw);
            crate::render::view_query(sc, &cam, &s, |ctx, regs| g.step(&mut Probe::new(ctx, regs), (yaw, pitch), &input, dt, gravity, water));
        }
    }

    const IDLE: Input = Input { forward: 0.0, right: 0.0, run: false, jet: false, jump: false, grow: 0.0, fast: false };

    /// On a floor in front of a wall: it stands a person's height with its
    /// eye where the camera was, walks to the wall and stops, rockets up,
    /// and shrinks with its feet on the floor.
    #[test]
    fn walks_stops_flies_and_shrinks() {
        let sc = scene(serde_json::json!([
            { "type": "plane" },
            { "type": "box", "size": [10, 4, 0.4], "position": [0, 2, -3] }
        ]));
        let eye = [0.0, 1.656, 0.0];
        let mut g = spawn(&sc, eye);
        assert!((g.height - 1.8).abs() < 0.01, "height {}", g.height);
        assert!(sub(g.eye(), eye).iter().all(|c| c.abs() < 1e-9), "the eye moved: {:?}", g.eye());
        run(&sc, &mut g, 0.0, IDLE, 0.5);
        assert!(g.grounded && g.feet[1].abs() < 0.01, "not standing: {:?}", g.feet);
        run(&sc, &mut g, 0.0, Input { forward: 1.0, ..IDLE }, 3.0);
        let face = -2.8 + RADIUS * g.height;
        assert!(g.feet[2] > face - 0.02 && g.feet[2] < face + 0.3, "should stop at the wall: z {}", g.feet[2]);
        run(&sc, &mut g, 0.0, Input { jet: true, ..IDLE }, 0.8);
        assert!(g.feet[1] < 0.01 && g.power > 0.3, "it should still be building up: y {}, power {}", g.feet[1], g.power);
        run(&sc, &mut g, 0.0, Input { jet: true, ..IDLE }, 1.7);
        assert!(g.feet[1] > 1.0, "the rocket should lift it: y {}", g.feet[1]);
        run(&sc, &mut g, 0.0, IDLE, 4.0);
        assert!(g.grounded, "should land");
        let before = g.eye()[1];
        run(&sc, &mut g, 0.0, Input { grow: -1.0, ..IDLE }, 2.0);
        assert!((g.height - 1.8 / 6.25).abs() < 0.02, "height {}", g.height);
        assert!(g.feet[1].abs() < 0.01 && g.eye()[1] < before * 0.3, "feet {:?}, eye {}", g.feet, g.eye()[1]);
    }

    /// In deep water it floats chest deep with the eyes out, still after a
    /// few seconds; it swims where the keys say, and swimming into a ledge
    /// half a metre out of the water climbs out onto it. In water shallower
    /// than that it wades, standing.
    #[test]
    fn floats_swims_and_climbs_out() {
        let sc = scene(serde_json::json!([
            { "type": "water" },
            { "type": "box", "size": [4, 3, 4], "position": [0, -1, -6] }
        ]));
        let calm = |_: f64, _: f64| 0.0;
        let mut g = spawn(&sc, [0.0, 1.0, 0.0]);
        run_in(&sc, &mut g, (0.0, 0.0), IDLE, 8.0, G, Some(&calm));
        let under = -g.feet[1] / g.height;
        assert!((under - FLOAT).abs() < 0.03, "should float {FLOAT} under: {under}");
        assert!(g.eye()[1] > 0.1 && len(g.vel) < 0.05, "eye {}, speed {}", g.eye()[1], len(g.vel));
        // two seconds' swim toward the ledge (its face at z -4)
        run_in(&sc, &mut g, (0.0, 0.0), Input { forward: 1.0, ..IDLE }, 2.0, G, Some(&calm));
        assert!(g.feet[2] < -1.8 && g.feet[2] > -3.0, "should swim about 2.4 m: z {}", g.feet[2]);
        run_in(&sc, &mut g, (0.0, 0.0), Input { forward: 1.0, ..IDLE }, 3.0, G, Some(&calm));
        run_in(&sc, &mut g, (0.0, 0.0), IDLE, 1.0, G, Some(&calm));
        assert!(g.grounded && (g.feet[1] - 0.5).abs() < 0.05 && g.feet[2] < -4.0, "should be out on the ledge: {:?}", g.feet);
        // wading: the floor 0.5 m under the water
        let shallow = scene(serde_json::json!([{ "type": "plane", "offset": -0.5 }]));
        let mut g = spawn(&shallow, [0.0, 1.0, 0.0]);
        run_in(&shallow, &mut g, (0.0, 0.0), Input { forward: 1.0, ..IDLE }, 2.0, G, Some(&calm));
        assert!(g.grounded && (g.feet[1] + 0.5).abs() < 0.02, "should wade: {:?}", g.feet);
    }

    /// On the bottom of deep water it stays on the bottom and walks it (the
    /// water does not lift it off). Floating, Space looking down dives: it
    /// goes under and stays there, neither sinking nor rising, while the
    /// keys ask nothing; swimming up it comes out and floats again.
    #[test]
    fn walks_the_bottom_and_dives() {
        let calm = |_: f64, _: f64| 0.0;
        let deep = scene(serde_json::json!([{ "type": "water" }, { "type": "plane", "offset": -3.0 }]));
        let mut g = spawn(&deep, [0.0, -3.0 + EYE * HUMAN, 0.0]);
        run_in(&deep, &mut g, (0.0, 0.0), IDLE, 2.0, G, Some(&calm));
        assert!(g.grounded && (g.feet[1] + 3.0).abs() < 0.02 && !g.diving, "should stand on the bottom: {:?}", g.feet);
        run_in(&deep, &mut g, (0.0, 0.0), Input { forward: 1.0, ..IDLE }, 2.0, G, Some(&calm));
        assert!(g.grounded && g.feet[2] < -1.5, "should walk the bottom: {:?}", g.feet);

        let sc = scene(serde_json::json!([{ "type": "water" }]));
        let mut g = spawn(&sc, [0.0, 1.0, 0.0]);
        run_in(&sc, &mut g, (0.0, 0.0), IDLE, 6.0, G, Some(&calm));
        assert!(!g.diving && (g.wet - FLOAT).abs() < 0.03, "should float: wet {}", g.wet);
        let down = (0.0, -0.6);
        run_in(&sc, &mut g, down, Input { jump: true, ..IDLE }, 1.0 / 60.0, G, Some(&calm));
        run_in(&sc, &mut g, down, IDLE, 2.0, G, Some(&calm));
        let y = g.eye()[1];
        assert!(g.diving && y < -0.3, "should be under: eye {y}");
        run_in(&sc, &mut g, down, IDLE, 3.0, G, Some(&calm));
        assert!(g.diving && (g.eye()[1] - y).abs() < 0.05, "should hold its depth: {} then {}", y, g.eye()[1]);
        run_in(&sc, &mut g, (0.0, 0.8), Input { forward: 1.0, ..IDLE }, 4.0, G, Some(&calm));
        run_in(&sc, &mut g, (0.0, 0.0), IDLE, 4.0, G, Some(&calm));
        assert!(!g.diving && (g.wet - FLOAT).abs() < 0.05, "should be floating again: wet {}, diving {}", g.wet, g.diving);
    }

    /// Swimming into a cliff too high to climb, it tries once and floats
    /// there (it climbed, fell back and climbed again for ever). Shrunk in
    /// mid-flight, its speed shrinks with it: it does not shoot off.
    #[test]
    fn one_scramble_and_shrinking_in_flight() {
        let sc = scene(serde_json::json!([
            { "type": "water" },
            { "type": "box", "size": [8, 8, 4], "position": [0, -1, -6] }
        ]));
        let calm = |_: f64, _: f64| 0.0;
        let mut g = spawn(&sc, [0.0, 1.0, 0.0]);
        run_in(&sc, &mut g, (0.0, 0.0), IDLE, 6.0, G, Some(&calm));
        let (mut climbs, mut was_up) = (0, false);
        for _ in 0..600 {
            run_in(&sc, &mut g, (0.0, 0.0), Input { forward: 1.0, ..IDLE }, 1.0 / 60.0, G, Some(&calm));
            let up = g.feet[1] > -0.8;
            if up && !was_up {
                climbs += 1;
            }
            was_up = up;
        }
        assert!(climbs <= 1, "climbed {climbs} times");
        assert!(g.feet[1] < -1.0 && !g.grounded, "should float at the cliff: {:?}", g.feet);

        let sc = scene(serde_json::json!([{ "type": "plane" }]));
        let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
        run(&sc, &mut g, 0.0, Input { jet: true, forward: 1.0, ..IDLE }, 2.5);
        run(&sc, &mut g, 0.0, Input { grow: -1.0, fast: true, ..IDLE }, 3.0);
        let speed = len(g.vel) / g.height;
        assert!(speed < 20.0, "{speed} heights a second at {} m", g.height);
    }

    /// A jump is a person's push: a metre on Earth, six times as high on
    /// the Moon. The rocket leans with the look: looking down it carries
    /// the walker forward, looking up back, never further than at LEAN.
    #[test]
    fn jumps_with_gravity_and_leans_the_rocket() {
        let sc = scene(serde_json::json!([{ "type": "plane" }]));
        for (gravity, want) in [(G, 1.0), (1.62, G / 1.62)] {
            let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
            run_at(&sc, &mut g, (0.0, 0.0), IDLE, 0.2, gravity);
            run_at(&sc, &mut g, (0.0, 0.0), Input { jump: true, ..IDLE }, 1.0 / 60.0, gravity);
            let mut top = 0.0f64;
            for _ in 0..1500 {
                run_at(&sc, &mut g, (0.0, 0.0), IDLE, 1.0 / 60.0, gravity);
                top = top.max(g.feet[1]);
            }
            assert!((top - want).abs() < 0.08 * want, "jumped {top} m under {gravity} m/s² (want {want})");
            assert!(g.grounded, "should land again");
        }
        let fly = |pitch: f64| {
            let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
            run_at(&sc, &mut g, (0.0, pitch.to_radians() as f32), Input { jet: true, ..IDLE }, 3.0, G);
            g.feet[2]
        };
        let (level, down20, down40, down80, up20) = (fly(0.0), fly(-20.0), fly(-40.0), fly(-80.0), fly(20.0));
        assert!(level.abs() < 1e-6, "level should go straight up: z {level}");
        assert!(down20 < -1.0 && down40 < down20, "looking down should carry it forward: {down20}, {down40}");
        assert!((down80 - down40).abs() < 1e-6, "past LEAN it should lean no further: {down80} vs {down40}");
        assert!(up20 > 1.0, "looking up should carry it back: {up20}");
    }

    /// A jump with a direction held leaps that way, from a standstill too
    /// (W and Space at once): forward, back, to either side; without one it
    /// comes down where it went up.
    #[test]
    fn leaps_the_way_the_keys_point() {
        let sc = scene(serde_json::json!([{ "type": "plane" }]));
        // where a jump from standing lands, with `forward` and `right` held
        let leap = |forward: f64, right: f64| {
            let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
            run(&sc, &mut g, 0.0, IDLE, 0.2);
            let held = Input { forward, right, ..IDLE };
            run(&sc, &mut g, 0.0, Input { jump: true, ..held }, 1.0 / 60.0);
            let mut frames = 0;
            while !g.grounded && frames < 300 {
                run(&sc, &mut g, 0.0, held, 1.0 / 60.0);
                frames += 1;
            }
            assert!(g.grounded, "should land");
            [g.feet[0], g.feet[2]]
        };
        let still = leap(0.0, 0.0);
        assert!(still[0].abs() < 1e-6 && still[1].abs() < 1e-6, "a jump with no direction should come straight down: {still:?}");
        // (looking along -z: forward is -z, right is +x)
        let ahead = leap(1.0, 0.0);
        assert!(ahead[1] < -3.0 && ahead[0].abs() < 1e-6, "W should leap forward: {ahead:?}");
        let back = leap(-1.0, 0.0);
        assert!(back[1] > 3.0, "S should leap back: {back:?}");
        let side = leap(0.0, 1.0);
        assert!(side[0] > 3.0 && side[1].abs() < 1e-6, "D should leap right: {side:?}");
    }

    /// Under a low ceiling it does not grow past it.
    #[test]
    fn grows_only_where_it_fits() {
        let sc = scene(serde_json::json!([
            { "type": "plane" },
            { "type": "box", "size": [10, 1, 10], "position": [0, 3.0, 0] }
        ]));
        let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
        run(&sc, &mut g, 0.0, IDLE, 0.3);
        run(&sc, &mut g, 0.0, Input { grow: 1.0, ..IDLE }, 2.0);
        assert!(g.feet[1] + g.height <= 2.5 + 0.01, "grew into the ceiling: {} tall", g.height);
        assert!(g.height > 2.0, "should grow up to it: {} tall", g.height);
    }

    /// Falling fast onto a thin floor, and running at a thin wall, it goes
    /// through neither.
    #[test]
    fn does_not_tunnel() {
        let sc = scene(serde_json::json!([
            { "type": "box", "size": [20, 0.02, 20], "position": [0, 1, 0] },
            { "type": "box", "size": [20, 4, 0.02], "position": [0, 3, -4] }
        ]));
        let mut g = spawn(&sc, [0.0, 400.0, 0.0]);
        g.vel = [0.0, -FALL * g.height / HUMAN, 0.0];
        run(&sc, &mut g, 0.0, IDLE, 12.0);
        assert!(g.grounded && (g.feet[1] - 1.01).abs() < 0.02, "fell through: {:?}", g.feet);
        run(&sc, &mut g, 0.0, Input { forward: 1.0, run: true, ..IDLE }, 3.0);
        assert!(g.feet[2] > -4.0, "ran through the wall: {:?}", g.feet);
    }

    /// Steps up onto a ledge lower than STEP, not onto a higher one.
    #[test]
    fn steps_up_ledges() {
        let sc = scene(serde_json::json!([
            { "type": "plane" },
            { "type": "box", "size": [10, 0.4, 4], "position": [0, 0.2, -4] },
            { "type": "box", "size": [10, 1.5, 4], "position": [0, 0.75, -8] }
        ]));
        let mut g = spawn(&sc, [0.0, 1.656, 0.0]);
        for i in 0..40 {
            run(&sc, &mut g, 0.0, Input { forward: 1.0, ..IDLE }, 0.1);
            if std::env::var_os("GAME_DEBUG").is_some() {
                eprintln!("{i}: feet {:.4?} vel {:.4?} grounded {}", g.feet, g.vel, g.grounded);
            }
        }
        assert!((g.feet[1] - 0.4).abs() < 0.02, "should be up the 40 cm step: {:?}", g.feet);
        assert!(g.feet[2] > -6.0 + RADIUS * g.height - 0.05, "climbed the 1.5 m wall: {:?}", g.feet);
    }

    /// Small steps of the camera add up below what f32 holds at six metres.
    #[test]
    fn camera_keeps_small_steps() {
        let mut c = Camera::default();
        c.set_exact([6.4375, 2.24, -1.48]);
        for _ in 0..1000 {
            c.translate(crate::math::Vec3::new(1e-8, 0.0, 0.0));
        }
        assert!((c.exact()[0] - (6.4375 + 1e-5)).abs() < 1e-10, "{}", c.exact()[0]);
    }

    /// Deep in a Menger sponge a metre from its centre, in a tunnel tens of
    /// micrometres wide (only there at the camera's detail): it stands in it
    /// without the eye moving, overlapping nothing.
    #[test]
    fn fits_in_a_sponge_tunnel() {
        let sc = scene(serde_json::json!([
            { "type": "menger", "iterations": 5, "position": [6.6, 1.5, -2.2], "rotation": [10, 73, 5], "scale": 0.75 }
        ]));
        let s = Settings::default();
        let dist = |p: D3| crate::render::view_query(&sc, &camera(p, 0.0), &s, |ctx, regs| Probe::new(ctx, regs).dist(&[p], 1.0)[0]);
        let normal = |p: D3, eps: f64| crate::render::view_query(&sc, &camera(p, 0.0), &s, |ctx, regs| Probe::new(ctx, regs).normal(p, eps, 1.0));
        // from open space in the sponge, toward a wall until it is ~20 µm away
        let mut eye = [0.0; 3];
        let mut found = false;
        let mut seed = 0x2545f4914f6cdd1du64;
        for _ in 0..200 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let u = |k: u32| ((seed >> (k * 20)) & 0xfffff) as f64 / 1048575.0 - 0.5;
            let mut p = [6.6 + u(0) * 1.2, 1.5 + u(1) * 1.2, -2.2 + u(2) * 1.2];
            if dist(p) <= 0.0 {
                continue;
            }
            for _ in 0..40 {
                let d = dist(p);
                if (1e-5..4e-5).contains(&d) {
                    found = true;
                    break;
                }
                p = add(p, scale(normal(p, (d * 0.01).max(1e-9)), -(d - 2e-5)));
            }
            if found {
                eye = p;
                break;
            }
        }
        assert!(found, "no tunnel found");
        let mut g = spawn(&sc, eye);
        assert!(sub(g.eye(), eye).iter().all(|c| c.abs() < 1e-12), "the eye moved: {:?} from {:?}", g.eye(), eye);
        assert!(g.height < 1e-4, "{} tall in a tunnel of 20 µm", g.height);
        run(&sc, &mut g, 0.0, IDLE, 1.0);
        let r = RADIUS * g.height;
        let cam = camera(g.eye(), 0.0);
        let d = crate::render::view_query(&sc, &cam, &s, |ctx, regs| Probe::new(ctx, regs).dist(&centres(g.feet, g.height), 1.0));
        assert!(d[..3].iter().all(|&d| d > 0.9 * r), "overlaps: {:?} (radius {r:e})", &d[..3]);
    }

    /// Get out, for the camera: a metre deep in a block, out through the
    /// nearest face; in the Mandelbulb where a camera was stuck (flown in
    /// through the surface), out of it nearby; with room already, it stays.
    #[test]
    fn camera_gets_out() {
        let s = Settings::default();
        let out = |sc: &Scene, eye: D3, room: f64| crate::render::view_query(sc, &camera(eye, 0.0), &s, |ctx, regs| open_space(&mut Probe::new(ctx, regs), eye, &[[0.0; 3]], room, 1e4 * room));
        let dist = |sc: &Scene, p: D3| crate::render::view_query(sc, &camera(p, 0.0), &s, |ctx, regs| Probe::new(ctx, regs).dist(&[p], 1.0)[0]);
        let block = scene(serde_json::json!([{ "type": "box", "size": [4, 4, 4] }]));
        let p = out(&block, [1.0, 0.0, 0.0], 0.1).expect("no way out of the block");
        assert!(p[0] > 2.099 && len(sub(p, [1.0, 0.0, 0.0])) < 1.15, "out of the block at {p:?}");
        assert_eq!(out(&block, [3.0, 0.5, 0.0], 0.1), Some([3.0, 0.5, 0.0]));
        let bulb = scene(serde_json::json!([
            { "type": "mandelbulb", "power": 8, "iterations": 10, "scale": 1.25, "rotation": [28.7, 20.0, -5.9], "position": [0.213, 2.762, -6.448] }
        ]));
        let stuck = [-0.28012135890030176, 3.468377677295294, -6.244910577893236];
        assert!(dist(&bulb, stuck) < 0.3, "not stuck: {}", dist(&bulb, stuck));
        let p = out(&bulb, stuck, 0.3).expect("no way out of the bulb");
        let moved = len(sub(p, stuck));
        assert!(dist(&bulb, p) >= 0.3 && moved < 1.5, "out of the bulb at {p:?}, {moved} m away, {} from it", dist(&bulb, p));
    }

    /// Get out, for the walker: stuck in the Mandelbulb (no way out for
    /// resolve), out to where its body fits (on top of the bulb), as tall;
    /// there it comes to rest overlapping nothing, with room already.
    #[test]
    fn walker_gets_out() {
        let sc = scene(serde_json::json!([
            { "type": "plane" },
            { "type": "mandelbulb", "power": 8, "iterations": 10, "scale": 1.25, "rotation": [28.7, 20.0, -5.9], "position": [0.213, 2.762, -6.448] }
        ]));
        let s = Settings::default();
        let h = 0.3;
        let feet = [-0.28, 3.47 - EYE * h, -6.24];
        let r = RADIUS * h;
        let body = |g: &Game| crate::render::view_query(&sc, &camera(g.eye(), 0.0), &s, |ctx, regs| Probe::new(ctx, regs).dist(&centres(g.feet, g.height), 1.0));
        let mut g = crate::render::view_query(&sc, &camera(feet, 0.0), &s, |ctx, regs| Game::stand(&mut Probe::new(ctx, regs), feet, h));
        assert!(body(&g)[..3].iter().any(|&d| d < 0.9 * r), "not stuck: {:?}", &body(&g)[..3]);
        let moved = crate::render::view_query(&sc, &camera(g.eye(), 0.0), &s, |ctx, regs| g.get_out(&mut Probe::new(ctx, regs))).expect("no way out");
        assert!(moved > 0.0 && moved < 2.0, "moved {moved} m");
        assert_eq!(g.height, h);
        assert!(body(&g)[..3].iter().all(|&d| d >= 0.98 * r * (1.0 - 1e-6)), "still stuck: {:?} (radius {r})", &body(&g)[..3]);
        run(&sc, &mut g, 0.0, IDLE, 3.0);
        assert!(len(g.vel) < 1e-3 && body(&g)[..3].iter().all(|&d| d > 0.9 * r), "after 3 s: {:?}, moving at {:?}", &body(&g)[..3], g.vel);
        let again = crate::render::view_query(&sc, &camera(g.eye(), 0.0), &s, |ctx, regs| g.get_out(&mut Probe::new(ctx, regs)));
        assert_eq!(again, Some(0.0));
        // into game mode inside the bulb: no room, so it is the least height,
        // stuck; out, still that tall, to the bulb's surface
        let mut g = spawn(&sc, [-0.28, 3.47, -6.24]);
        assert_eq!(g.height, MIN_HEIGHT);
        let moved = crate::render::view_query(&sc, &camera(g.eye(), 0.0), &s, |ctx, regs| g.get_out(&mut Probe::new(ctx, regs))).expect("no way out");
        let r = RADIUS * g.height;
        assert!(moved > 1e-3 && moved < 2.0 && g.height == MIN_HEIGHT, "moved {moved} m, {} tall", g.height);
        assert!(body(&g)[..3].iter().all(|&d| d >= 0.98 * r * (1.0 - 1e-3)), "still stuck: {:?} (radius {r:e})", &body(&g)[..3]);
    }
}

#[cfg(test)]
mod quicksave {
    use super::*;

    /// The quick save's camera (saves/quicksave.json: kept out of the
    /// repository, so run by hand with --ignored): the walker stands where
    /// the camera is, eye unmoved, overlapping nothing, and stays standing.
    #[test]
    #[ignore]
    fn spawns_at_the_quick_save() {
        let text = std::fs::read_to_string("saves/quicksave.json").expect("no quick save");
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let sc = crate::scene::Scene::from_value(&v, &mut crate::scene::TerrainCache::default()).unwrap();
        let s: crate::settings::Settings = serde_json::from_value(v["render"].clone()).unwrap_or_default();
        let mut cam = crate::start_camera(&sc);
        cam.setup(1015, 825);
        let eye = cam.exact();
        let mut g = crate::render::view_query(&sc, &cam, &s, |ctx, regs| Game::enter(&mut Probe::new(ctx, regs), eye));
        let moved = sub(g.eye(), eye);
        eprintln!("height {:e} m, grounded {}, eye moved {:?}", g.height, g.grounded, moved);
        assert!(moved.iter().all(|c| c.abs() < 1e-12));
        for _ in 0..120 {
            let mut c = cam;
            c.set_exact(g.eye());
            crate::render::view_query(&sc, &c, &s, |ctx, regs| g.step(&mut Probe::new(ctx, regs), (cam.yaw, cam.pitch), &Input::default(), 1.0 / 60.0, G, None));
        }
        let mut c = cam;
        c.set_exact(g.eye());
        let r = RADIUS * g.height;
        let d = crate::render::view_query(&sc, &c, &s, |ctx, regs| Probe::new(ctx, regs).dist(&centres(g.feet, g.height), 1.0));
        eprintln!("after 2 s: feet moved {:?}, grounded {}, sphere clearances {:?} (radius {r:e})", sub(g.feet, feet_below(eye, g.height)), g.grounded, &d[..3]);
        assert!(d[..3].iter().all(|&d| d > 0.9 * r));
    }
}
