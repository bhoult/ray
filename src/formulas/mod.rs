//! The fractal formula catalog: escape-time, folding and IFS fractals beyond
//! the built-in eight (scene/sdf.rs), each a JSON shape type of its own
//! (`{"type": "kaliset", ...}`), one distance kernel for the CPU (eight
//! points at a time, render/vm.rs) and the same in WGSL for the GPU
//! (gpu/codegen.rs includes the kernels a scene uses). A formula is data:
//! its parameters, their defaults and docs (the inspector, help and Add menu
//! read them, ui/schema.rs), how it is bounded and marched, and its kernels.
//! The groups below are its families, a file each.
//!
//! A kernel takes the point (in the shape's own units: `scale` sizes it),
//! the parameters packed in order into twelve floats (`Formula::pack`), and
//! the iteration count; it returns the distance estimate and an orbit trap
//! in 0..1 (what the "orbit" texture colours by). The WGSL kernel is the
//! same function: `fn NAME(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters:
//! u32) -> vec2f` with the twelve floats in a, b, c; any helpers it needs
//! are in its `wgsl` too, named after it.

use crate::simd::{F8, V8};
use serde::de::Error as _;
use serde_json::Value;
use std::sync::OnceLock;

pub mod algebras;
pub mod algebras2;
pub mod algebras3;
pub mod assorted;
pub mod assorted2;
pub mod bulbs;
pub mod bulbs2;
pub mod cathedral;
pub mod clifford;
pub mod cubes;
pub mod folds;
pub mod horned;
pub mod hyperbolic;
pub mod kleinian;
pub mod nil;
pub mod originals;
pub mod polyhedral;
pub mod quasicrystal;
pub mod quasiregular;
pub mod riemann;
pub mod screw;
pub mod slices4d;
pub mod tori;
pub mod tpms;
pub mod truss;
pub mod util;

/// Floats a formula's parameters pack into.
pub const K: usize = 12;

/// One parameter: a number, or an array of up to four.
pub struct Param {
    pub name: &'static str,
    /// its default: one number (a scalar in the JSON) or two to four (an array)
    pub default: &'static [f32],
    /// every number is clamped to this range when the shape is compiled
    pub min: f32,
    pub max: f32,
    /// a whole number (written without a fraction, rounded when compiled)
    pub whole: bool,
    pub doc: &'static str,
}

impl Param {
    pub const fn num(name: &'static str, default: &'static [f32], min: f32, max: f32, doc: &'static str) -> Param {
        Param { name, default, min, max, whole: false, doc }
    }

    pub const fn whole(name: &'static str, default: f32, min: f32, max: f32, doc: &'static str) -> Param {
        Param { name, default: whole_default(default), min, max, whole: true, doc }
    }
}

const fn whole_default(v: f32) -> &'static [f32] {
    // (a const fn cannot borrow a temporary: the whole defaults there are)
    const WHOLE: [f32; 17] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0];
    let i = v as usize;
    assert!(i < WHOLE.len() && i as f32 == v, "Param::whole: a default from 0 to 16");
    WHOLE.split_at(i).1.split_at(1).0
}

/// A CPU kernel: (distance, orbit trap) at eight points.
pub type Cpu = fn(V8, &[f32; K], u32) -> (F8, F8);

pub struct Formula {
    /// its JSON type
    pub name: &'static str,
    /// the Add menu's and help's category: one of CATEGORIES
    pub category: &'static str,
    /// what it is and where it comes from (whose formula), for the help and
    /// the Add menu's tooltip
    pub desc: &'static str,
    pub params: &'static [Param],
    /// (default, most) iterations, and what more of them does
    pub iterations: (u32, u32, &'static str),
    /// the half size of the cube it fits in, at scale 1; INFINITY: it fills
    /// space (intersect it with a box, as its `example` does)
    pub bound: f32,
    /// iterates further close up (sdf.rs Lod): (how many times finer each
    /// iteration's copies are, the most iterations); None: as set (escape
    /// time and inversion fractals, whose further iterations reshape them)
    pub lod: Option<(f32, u32)>,
    /// the march's step factor (at most 1): below 1 for estimates that
    /// overshoot
    pub step: f32,
    /// what Add puts in the scene: a node of this type at a good size (an
    /// infinite one cut to a box)
    pub example: &'static str,
    /// the parameters as packed, worked on before they reach the kernels
    /// (say angles into a rotation): None leaves them
    pub prep: Option<fn(&mut [f32; K])>,
    pub cpu: Cpu,
    /// the WGSL kernel `wgsl_fn` and its helpers
    pub wgsl: &'static str,
    pub wgsl_fn: &'static str,
}

/// The families, in the order the Add menu and help list them.
pub const CATEGORIES: [&str; 8] = [
    "Fractals: box folds",
    "Fractals: bulbs",
    "Fractals: algebras",
    "Fractals: spheres and tori",
    "Fractals: geometric",
    "Fractals: 4D slices",
    "Fractals: more",
    "Fractals: originals",
];

/// Every family's formulas, in order (a formula's index into `all` is its
/// id in compiled programs).
const GROUPS: &[&[Formula]] = &[
    folds::FORMULAS,
    bulbs::FORMULAS,
    bulbs2::FORMULAS,
    riemann::FORMULAS,
    algebras::FORMULAS,
    algebras2::FORMULAS,
    algebras3::FORMULAS,
    tori::FORMULAS,
    kleinian::FORMULAS,
    cubes::FORMULAS,
    polyhedral::FORMULAS,
    slices4d::FORMULAS,
    assorted::FORMULAS,
    assorted2::FORMULAS,
    originals::FORMULAS,
    nil::FORMULAS,
    quasicrystal::FORMULAS,
    screw::FORMULAS,
    tpms::FORMULAS,
    quasiregular::FORMULAS,
    clifford::FORMULAS,
    cathedral::FORMULAS,
    truss::FORMULAS,
    hyperbolic::FORMULAS,
    horned::FORMULAS,
];

/// Every formula.
pub fn all() -> &'static [&'static Formula] {
    static ALL: OnceLock<Vec<&'static Formula>> = OnceLock::new();
    ALL.get_or_init(|| GROUPS.iter().flat_map(|g| g.iter()).collect())
}

/// The formula with id `i`.
pub fn get(i: u16) -> &'static Formula {
    all()[i as usize]
}

/// A formula's id by its JSON type.
pub fn find(name: &str) -> Option<u16> {
    all().iter().position(|f| f.name == name).map(|i| i as u16)
}

impl Formula {
    /// Its parameters' defaults, packed.
    pub fn defaults(&self) -> [f32; K] {
        let mut k = [0.0; K];
        let mut at = 0;
        for p in self.params {
            for &v in p.default {
                k[at] = v;
                at += 1;
            }
        }
        k
    }

    /// Floats its parameters take.
    pub fn width(&self) -> usize {
        self.params.iter().map(|p| p.default.len()).sum()
    }
}

/// A formula shape as the scene file has it: its type, its parameters
/// packed (as written, unclamped: the file keeps what was typed) and its
/// iterations. In the JSON it is flat like every other shape:
/// `{"type": "kaliset", "c": [..], "iterations": 12}`; parameters left out
/// take their defaults, and other fields (the node's position, scale and so
/// on) belong to the node.
#[derive(Clone, Debug, PartialEq)]
pub struct FormulaNode {
    pub id: u16,
    pub values: [f32; K],
    pub iterations: u32,
}

impl FormulaNode {
    pub fn formula(&self) -> &'static Formula {
        get(self.id)
    }

    /// The parameters for the kernels: clamped, whole ones rounded, then
    /// the formula's prep.
    pub fn pack(&self) -> [f32; K] {
        let f = self.formula();
        let mut k = self.values;
        let mut at = 0;
        for p in f.params {
            for _ in p.default {
                let v = k[at].clamp(p.min, p.max);
                k[at] = if p.whole { v.round() } else { v };
                at += 1;
            }
        }
        if let Some(prep) = f.prep {
            prep(&mut k);
        }
        k
    }

    /// Its iterations, within the formula's range.
    pub fn iters(&self) -> u32 {
        self.iterations.clamp(1, self.formula().iterations.1)
    }
}

impl serde::Serialize for FormulaNode {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let f = self.formula();
        let mut m = s.serialize_map(Some(f.params.len() + 2))?;
        m.serialize_entry("type", f.name)?;
        let mut at = 0;
        for p in f.params {
            let n = p.default.len();
            let vals = &self.values[at..at + n];
            at += n;
            let num = |v: f32| if p.whole { Value::from(v.round() as i64) } else { Value::from(v as f64) };
            if n == 1 {
                m.serialize_entry(p.name, &num(vals[0]))?;
            } else {
                m.serialize_entry(p.name, &Value::Array(vals.iter().map(|&v| num(v)).collect()))?;
            }
        }
        m.serialize_entry("iterations", &self.iterations)?;
        m.end()
    }
}

impl<'de> serde::Deserialize<'de> for FormulaNode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let ty = v.get("type").and_then(Value::as_str).ok_or_else(|| D::Error::custom("a shape needs a \"type\""))?;
        let id = find(ty).ok_or_else(|| D::Error::custom(format!("unknown shape type '{ty}'")))?;
        let f = get(id);
        let mut values = f.defaults();
        let mut at = 0;
        for p in f.params {
            let n = p.default.len();
            match v.get(p.name) {
                None | Some(Value::Null) => {}
                Some(Value::Number(x)) if n == 1 => values[at] = x.as_f64().unwrap_or(0.0) as f32,
                Some(Value::Array(a)) if a.len() == n && a.iter().all(Value::is_number) => {
                    for (i, x) in a.iter().enumerate() {
                        values[at + i] = x.as_f64().unwrap_or(0.0) as f32;
                    }
                }
                Some(_) => {
                    let want = if n == 1 { "a number".to_string() } else { format!("an array of {n} numbers") };
                    return Err(D::Error::custom(format!("{ty}: '{}' must be {want}", p.name)));
                }
            }
            at += n;
        }
        let iterations = match v.get("iterations") {
            None | Some(Value::Null) => f.iterations.0,
            Some(x) => match x.as_f64() {
                Some(n) if n >= 0.0 && n.fract() == 0.0 => n as u32,
                _ => return Err(D::Error::custom(format!("{ty}: 'iterations' must be a whole number"))),
            },
        };
        Ok(FormulaNode { id, values, iterations })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Names are unique and not a built-in type's; every formula's
    /// parameters fit, have defaults within their ranges, and are described;
    /// its WGSL defines its function with the kernel signature.
    #[test]
    fn every_formula_is_well_formed() {
        let builtin = ["menger", "mandelbulb", "mandelbox", "julia", "sierpinski", "apollonian", "kifs", "kleinian"];
        let mut seen = std::collections::HashSet::new();
        for f in all() {
            assert!(seen.insert(f.name), "{}: two formulas of that name", f.name);
            assert!(!builtin.contains(&f.name), "{}: a built-in type's name", f.name);
            assert!(CATEGORIES.contains(&f.category), "{}: unknown category {}", f.name, f.category);
            assert!(f.width() <= K, "{}: {} floats of parameters, at most {K}", f.name, f.width());
            assert!(!f.desc.is_empty() && !f.iterations.2.is_empty(), "{}: describe it and its iterations", f.name);
            assert!(f.iterations.0 >= 1 && f.iterations.0 <= f.iterations.1, "{}: default iterations out of range", f.name);
            assert!(f.step > 0.0 && f.step <= 1.0, "{}: step in (0, 1]", f.name);
            for p in f.params {
                assert!((1..=4).contains(&p.default.len()), "{}.{}: 1 to 4 numbers", f.name, p.name);
                assert!(!p.doc.is_empty(), "{}.{}: describe it", f.name, p.name);
                // (the node's own fields share the object: a parameter of the
                // same name would be read as both)
                let node = ["iterations", "type", "position", "rotation", "scale", "material", "name", "shadow", "step", "bound", "friction", "bounce", "locked", "mass", "gravity", "disabled", "children", "child"];
                assert!(!node.contains(&p.name), "{}.{}: a reserved name (a node field)", f.name, p.name);
                for &v in p.default {
                    assert!(v >= p.min && v <= p.max, "{}.{}: default {v} outside {}..{}", f.name, p.name, p.min, p.max);
                }
            }
            let sig = format!("fn {}(p: vec3f, a: vec4f, b: vec4f, c: vec4f, iters: u32) -> vec2f", f.wgsl_fn);
            assert!(f.wgsl.contains(&sig), "{}: its wgsl must define `{sig}`", f.name);
        }
    }

    /// A formula node reads with defaults for what is left out and writes
    /// every parameter back.
    #[test]
    fn nodes_round_trip() {
        for (i, f) in all().iter().enumerate() {
            let n: FormulaNode = serde_json::from_value(serde_json::json!({ "type": f.name })).unwrap();
            assert_eq!(n.id as usize, i);
            assert_eq!(n.values, f.defaults());
            let back: FormulaNode = serde_json::from_value(serde_json::to_value(&n).unwrap()).unwrap();
            assert_eq!(back, n, "{}", f.name);
        }
    }

    /// Every kernel gives finite distances that, outside, do not grow faster
    /// than the distance between the points (an estimate steeper than 1
    /// makes the march overshoot; `step` allows for it), and traps within
    /// 0..1. Each formula has its own random points (by its name), so adding
    /// one does not move another's; a rare steep point is allowed, a steep
    /// hundredth is not.
    #[test]
    fn kernels_are_finite_and_not_too_steep() {
        for f in all() {
            let mut seed = f.name.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ b as u32).wrapping_mul(0x0100_0193)) | 1;
            let mut rnd = || {
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
            };
            let node = FormulaNode { id: find(f.name).unwrap(), values: f.defaults(), iterations: f.iterations.0 };
            let k = node.pack();
            let reach = if f.bound.is_finite() { f.bound * 1.2 } else { 3.0 };
            let mut slopes = Vec::new();
            for _ in 0..256 {
                let pts: [[f32; 8]; 3] = std::array::from_fn(|_| std::array::from_fn(|_| rnd() * reach));
                let p = V8::new(F8::from_array(pts[0]), F8::from_array(pts[1]), F8::from_array(pts[2]));
                let e = 1e-3 * reach;
                let q = V8::new(p.x + e, p.y, p.z);
                let (d, t) = (f.cpu)(p, &k, node.iters());
                let (dq, _) = (f.cpu)(q, &k, node.iters());
                for i in 0..8 {
                    let (a, b, tr) = (d.lane(i), dq.lane(i), t.lane(i));
                    assert!(a.is_finite(), "{}: distance {a} at {:?}", f.name, p.lane(i));
                    assert!(tr.is_finite() && (-1e-4..=1.0001).contains(&tr), "{}: trap {tr} at {:?}", f.name, p.lane(i));
                    // (outside only: inside, an estimate only has to be negative)
                    if a > 10.0 * e && b.is_finite() {
                        slopes.push((b - a).abs() / e * f.step);
                    }
                }
            }
            slopes.sort_by(f32::total_cmp);
            let p99 = slopes.get(slopes.len() * 99 / 100).copied().unwrap_or(0.0);
            assert!(p99 < 1.25, "{}: a hundredth of its estimate changes {p99:.2} times as fast as the point moves (allowing for its step)", f.name);
        }
    }
}
