//! What every shape type and parameter means: the help screen, the tooltips
//! of the inspector and the Add menu all read this table.

use serde_json::Value;

pub struct ShapeInfo {
    pub ty: &'static str,
    pub category: &'static str,
    pub desc: &'static str,
    /// a new node of this type (with the fields serde cannot default)
    pub new: &'static str,
    pub fields: &'static [(&'static str, &'static str)],
}

pub const CATEGORIES: [&str; 8] = ["Primitives", "Surfaces", "Fractals", "Blobs", "Combine", "Modify", "Landscape", "Effects"];

pub const SHAPES: &[ShapeInfo] = &[
    ShapeInfo { ty: "sphere", category: "Primitives", desc: "A ball. Exact and cheap at the top level.", new: r#"{"type":"sphere","radius":0.5}"#,
        fields: &[("radius", "Radius in metres.")] },
    ShapeInfo { ty: "box", category: "Primitives", desc: "A rectangular block, optionally with rounded edges.", new: r#"{"type":"box","size":[1,1,1]}"#,
        fields: &[("size", "Full width, height and depth in metres (not half sizes)."), ("rounding", "Radius of the rounded edges in metres; 0 = sharp. A rounded box is marched rather than intersected exactly.")] },
    ShapeInfo { ty: "plane", category: "Primitives", desc: "An infinite flat ground or wall; everything behind it is solid. Planes are fixed (not draggable).", new: r#"{"type":"plane"}"#,
        fields: &[("normal", "The direction the plane faces; the solid side is the opposite one."), ("offset", "How far the plane sits from the origin along its normal, in metres.")] },
    ShapeInfo { ty: "torus", category: "Primitives", desc: "A ring lying flat in the xz plane.", new: r#"{"type":"torus","radius":0.5,"thickness":0.15}"#,
        fields: &[("radius", "From the centre of the ring to the middle of the tube, in metres."), ("thickness", "Radius of the tube in metres.")] },
    ShapeInfo { ty: "cylinder", category: "Primitives", desc: "An upright capped cylinder along y.", new: r#"{"type":"cylinder","radius":0.4,"height":1}"#,
        fields: &[("radius", "Radius in metres."), ("height", "Full height in metres.")] },
    ShapeInfo { ty: "cone", category: "Primitives", desc: "A capped cone or truncated cone along y.", new: r#"{"type":"cone","radius":0.5,"height":1}"#,
        fields: &[("radius", "Radius at the bottom in metres."), ("top_radius", "Radius at the top in metres; 0 = a point."), ("height", "Full height in metres.")] },
    ShapeInfo { ty: "capsule", category: "Primitives", desc: "A cylinder with round ends, between two points.", new: r#"{"type":"capsule"}"#,
        fields: &[("a", "One end, in the node's own coordinates."), ("b", "The other end."), ("radius", "Radius in metres.")] },
    ShapeInfo { ty: "ellipsoid", category: "Primitives", desc: "A sphere stretched along its axes.", new: r#"{"type":"ellipsoid"}"#,
        fields: &[("radii", "Half sizes along x, y and z in metres. Very flat ellipsoids march more slowly (their distance estimate is approximate).")] },
    ShapeInfo { ty: "octahedron", category: "Primitives", desc: "A double pyramid: a gem or crystal.", new: r#"{"type":"octahedron","size":0.6}"#,
        fields: &[("size", "From the centre to each of the six tips, in metres.")] },
    ShapeInfo { ty: "gyroid", category: "Primitives", desc: "The gyroid minimal surface as a sheet: an endless lattice of saddles. It is infinite, so intersect it with a sphere or box.", new: r#"{"type":"intersection","children":[{"type":"sphere","radius":0.8},{"type":"gyroid","frequency":1.5,"thickness":0.1}]}"#,
        fields: &[("frequency", "Lattice cells per unit length."), ("thickness", "Sheet thickness (approximate, in lattice units).")] },
    ShapeInfo { ty: "revolve", category: "Surfaces", desc: "A NURBS surface of revolution: a profile curve turned about the y axis, traced exactly (no mesh, no marching). Vases, bottles, goblets, bowls, lamp shades; with weights, an exact sphere or torus. A profile that starts and ends on the axis, or loops back to its start, is solid (glass refracts through it). Alone it is traced exactly; in a union, intersection or difference, or under a modifier (twist, shell...), it is measured by its distance instead: slower, but it blends and cuts like any shape.", new: r#"{"type":"revolve"}"#,
        fields: &[
            ("profile", "The curve's control points, [radius, height] in metres (or [radius, height, weight]), from bottom to top; the surface is pulled toward them but only passes through the first and last. Left out: a vase with 3 cm walls."),
            ("degree", "The profile curve's degree: 1 = straight segments between the points (a faceted lathe), 2 = smooth, 3 = smoother (default). Up to 3."),
            ("knots", "The profile's knot vector: where its pieces join (clamped: the first and last value repeated degree + 1 times). Left out: evenly spaced. Repeat an inside knot to make a sharp crease."),
            ("angle", "How far round it turns, in degrees (360 = all the way; less leaves a slice open)."),
            ("solid", "Whether it encloses a volume (glass refracts through it, the inside is inside). Left out: worked out from the profile."),
        ] },
    ShapeInfo { ty: "nurbs", category: "Surfaces", desc: "A free-form NURBS surface: a grid of control points that the surface follows smoothly, traced exactly (no mesh). Weights pull the surface toward a point (and make exact circles); knots set where its pieces join. An open sheet shows both sides. Alone it is traced exactly; in a union, intersection or difference, or under a modifier (twist, shell...), it is measured by its distance instead: slower, but it blends and cuts like any shape.", new: r#"{"type":"nurbs"}"#,
        fields: &[
            ("points", "The control net: rows along u, each a list of [x, y, z] points (or [x, y, z, weight]), in the node's own metres; every row the same length. Left out: a 2 m sheet with a bump and a dip."),
            ("degree", "Degree along u and along v: 1 = flat facets between the points, 2 = smooth, 3 = smoother (default). Up to 3, and at most the number of points minus 1."),
            ("knots_u", "The knot vector along u (rows): where the surface's pieces join, clamped (the first and last value repeated degree + 1 times). Left out: evenly spaced."),
            ("knots_v", "The knot vector along v (along each row), as knots_u."),
            ("solid", "On for a closed surface that encloses a volume (glass refracts through it); off for an open sheet, seen from both sides."),
        ] },
    ShapeInfo { ty: "menger", category: "Fractals", desc: "The Menger sponge: a cube with holes punched through it recursively. Fills [-1, 1] on every axis; size it with scale.", new: r#"{"type":"menger","iterations":4,"scale":0.6}"#,
        fields: &[("iterations", "Recursion levels as seen from afar (0 to 12). Each level adds holes a third the size; the cost grows by one fold per level. Close up more are added by themselves, as many as the pixels can show (up to 12), so zooming in keeps finding detail.")] },
    ShapeInfo { ty: "mandelbulb", category: "Fractals", desc: "The Mandelbulb: the Mandelbrot set in 3D, from powers in spherical coordinates. About 2.4 units across; size it with scale.", new: r#"{"type":"mandelbulb","scale":0.6}"#,
        fields: &[("power", "The exponent: 8 is the classic bulb (2 to 16). Higher powers give more, finer lobes."), ("iterations", "Escape-time iterations: more = finer detail. Each iteration costs a pow, two atan2, a sin and a cos per point.")] },
    ShapeInfo { ty: "mandelbox", category: "Fractals", desc: "The Mandelbox: box folds and sphere folds repeated. Negative fold scales give crumpled cubes, positive ones (2 to 3) boxy cities.", new: r#"{"type":"mandelbox","fold_scale":-1.5,"iterations":12,"scale":0.25}"#,
        fields: &[("fold_scale", "The scale applied after each fold (avoid -1 to 1). Sets the overall shape."), ("iterations", "Fold iterations: more = finer detail, linear cost."), ("min_radius", "Radius of the inner sphere fold: smaller makes sharper, deeper bulbs.")] },
    ShapeInfo { ty: "julia", category: "Fractals", desc: "A quaternion Julia set: a smooth, organic 4D fractal sliced in 3D.", new: r#"{"type":"julia","scale":0.6}"#,
        fields: &[("c", "The constant quaternion (x, y, z, w). Small changes reshape it completely."), ("iterations", "Escape-time iterations: more = finer detail.")] },
    ShapeInfo { ty: "sierpinski", category: "Fractals", desc: "The Sierpinski tetrahedron, built by folding space.", new: r#"{"type":"sierpinski","iterations":8,"scale":0.6}"#,
        fields: &[("iterations", "Levels of subdivision; each halves the size of the smallest tetrahedra. Close up more are added by themselves, as many as the pixels can show (up to 24), so zooming in keeps finding detail.")] },
    ShapeInfo { ty: "apollonian", category: "Fractals", desc: "An Apollonian packing (iq's): spheres within spheres, packed by repeated inversion, like foam or coral. It fills all of space in a lattice, so it is cut to a box here; try the orbit pattern on it.", new: r#"{"type":"intersection","scale":0.5,"children":[{"type":"box","size":[4,4,4]},{"type":"apollonian","fold":1.1,"iterations":8}]}"#,
        fields: &[("fold", "The inversion's strength (0.5 to 2): above 1 the spheres swell and merge into a sponge; below, they thin into a lace."), ("iterations", "Levels of packing: more = smaller spheres between the big ones.")] },
    ShapeInfo { ty: "kifs", category: "Fractals", desc: "A kaleidoscopic IFS (Knighty's): space folded into a mirror octant, its axes sorted, turned, then scaled away from the offset, again and again. With no turn it is the Menger sponge; small turns give spires, crystals and alien machinery.", new: r#"{"type":"kifs","fold_scale":3,"offset":[1,1,1],"angles":[12,8,0],"iterations":6,"scale":0.6}"#,
        fields: &[("fold_scale", "How much each step scales (3 = the Menger sponge; 2 to 4 is typical)."), ("offset", "The point each step scales away from: it sets where the copies sit."), ("angles", "Degrees turned about x, y and z at every step: 0 is the Menger sponge; a few degrees twist it into something new."), ("iterations", "Steps: more = finer detail, linear cost. Close up more are added by themselves, as many as the pixels can show (up to 24), so zooming in keeps finding detail.")] },
    ShapeInfo { ty: "kleinian", category: "Fractals", desc: "The pseudo-Kleinian (Knighty's): box folds and sphere inversions that build a limit set of rings and tunnels, like a cathedral of coral. It fills all of space, so it is cut to a box here; fly inside it.", new: r#"{"type":"intersection","scale":0.5,"children":[{"type":"box","size":[4,4,4]},{"type":"kleinian","iterations":8}]}"#,
        fields: &[("fold_box", "Half sizes of the box fold along x, y and z: small changes reshape the tunnels."), ("inversion", "The inversion sphere's radius squared: larger opens the structure up."), ("iterations", "Steps: more = finer detail.")] },
    ShapeInfo { ty: "metaballs", category: "Blobs", desc: "Balls that melt into each other where they come close (a smooth union of spheres).", new: r#"{"type":"metaballs","blend":0.4,"balls":[{"center":[0,0.4,0],"radius":0.4},{"center":[0.45,0.7,0],"radius":0.3},{"center":[-0.3,0.9,0.1],"radius":0.25}]}"#,
        fields: &[("balls", "The balls: each a centre (in the node's coordinates) and a radius."), ("blend", "How far apart the balls start to merge, in metres: 0 = separate balls, larger = gooier.")] },
    ShapeInfo { ty: "union", category: "Combine", desc: "All its children together. With smooth > 0 they blend into each other where they meet.", new: r#"{"type":"union","smooth":0.2,"children":[{"type":"sphere","radius":0.5,"position":[0,0.5,0]},{"type":"box","size":[0.8,0.4,0.8],"position":[0,0.2,0]}]}"#,
        fields: &[("children", "The shapes to combine."), ("smooth", "Blend radius in metres: 0 = sharp seams, larger = a fillet between the children.")] },
    ShapeInfo { ty: "intersection", category: "Combine", desc: "Only the space inside every child.", new: r#"{"type":"intersection","children":[{"type":"sphere","radius":0.6},{"type":"box","size":[0.9,0.9,0.9]}]}"#,
        fields: &[("children", "The shapes to intersect."), ("smooth", "Blend radius in metres: rounds the edges where the children cut each other.")] },
    ShapeInfo { ty: "difference", category: "Combine", desc: "The first child with all the others carved out of it.", new: r#"{"type":"difference","children":[{"type":"box","size":[1,1,1],"rounding":0.05},{"type":"sphere","radius":0.65}]}"#,
        fields: &[("children", "The first is carved; the rest are the cutters."), ("smooth", "Blend radius in metres: rounds the carved edges.")] },
    ShapeInfo { ty: "group", category: "Combine", desc: "Whole objects kept together: a building of several parts, a city block, a table set. Each member stays an object of its own (any kind, with its own material: it renders as fast as on its own), and the group moves, turns and scales them all; its material is theirs where they have none. Unless it is locked, the group falls, is carried and is thrown as one body (when every member is a distance shape: a NURBS surface, cloud, terrain or water member keeps it where it is). Click a member in the view to select it; the inspector's Into group / Out of group move objects in and out.", new: r#"{"type":"group","children":[{"type":"box","size":[0.8,1.6,0.8],"position":[-0.6,0.8,0]},{"type":"sphere","radius":0.4,"position":[0.6,0.4,0]}]}"#,
        fields: &[("children", "The member objects, placed in the group's frame.")] },
    ShapeInfo { ty: "repeat", category: "Modify", desc: "Repeats its child on a lattice: endless copies for the cost of one evaluation.", new: r#"{"type":"repeat","period":[2,0,2],"count":[2,0,2],"child":{"type":"sphere","radius":0.4,"position":[0,0.4,0]}}"#,
        fields: &[("period", "Spacing along x, y and z in metres; 0 = not repeated along that axis. Keep the child smaller than half the spacing, or copies cut into each other."), ("count", "Copies on each side of the original per axis (2 = five in a row). \"infinite\": no count, the copies go on forever, for the cost of one."), ("child", "The shape to repeat.")] },
    ShapeInfo { ty: "mirror", category: "Modify", desc: "Mirrors its child across the planes of the chosen axes (symmetry).", new: r#"{"type":"mirror","axes":"x","child":{"type":"sphere","radius":0.3,"position":[0.5,0.3,0]}}"#,
        fields: &[("axes", "Which axes to mirror across: any of x, y and z, e.g. \"xz\"."), ("child", "The shape to mirror.")] },
    ShapeInfo { ty: "twist", category: "Modify", desc: "Twists its child about the y axis.", new: r#"{"type":"twist","amount":1.5,"child":{"type":"box","size":[0.5,1.6,0.5]}}"#,
        fields: &[("amount", "Radians of twist per unit of height. Strong twists slow the march (smaller steps stay safe)."), ("child", "The shape to twist.")] },
    ShapeInfo { ty: "bend", category: "Modify", desc: "Bends its child in the xy plane.", new: r#"{"type":"bend","amount":0.8,"child":{"type":"box","size":[1.6,0.3,0.4]}}"#,
        fields: &[("amount", "Radians of bend per unit along x."), ("child", "The shape to bend.")] },
    ShapeInfo { ty: "round", category: "Modify", desc: "Rounds every edge of its child by growing it outward.", new: r#"{"type":"round","radius":0.08,"child":{"type":"box","size":[0.8,0.8,0.8]}}"#,
        fields: &[("radius", "How far to grow the surface, in metres."), ("child", "The shape to round.")] },
    ShapeInfo { ty: "shell", category: "Modify", desc: "Hollows its child into a thin shell (cut it open to see inside).", new: r#"{"type":"shell","thickness":0.03,"child":{"type":"sphere","radius":0.5}}"#,
        fields: &[("thickness", "Wall thickness in metres."), ("child", "The shape to hollow.")] },
    ShapeInfo { ty: "displace", category: "Modify", desc: "Roughens its child's surface with 3D Perlin noise: rocks, asteroids, bark.", new: r#"{"type":"displace","amplitude":0.08,"frequency":3,"octaves":4,"child":{"type":"sphere","radius":0.5}}"#,
        fields: &[("amplitude", "Height of the bumps in metres."), ("frequency", "Bumps per unit length."), ("octaves", "Noise layers (1 to 8): more = finer grit and more cost per step."), ("child", "The shape to roughen.")] },
    ShapeInfo { ty: "slice", category: "Modify", desc: "Cuts its child with a plane: everything beyond the plane goes, so you can look inside (a fractal's interior, a shell's wall). With a thickness, only a slab that thick stays: a cross-section. Drag the offset to sweep the cut through it.", new: r#"{"type":"slice","normal":[0,0,1],"offset":0,"child":{"type":"sphere","radius":0.5}}"#,
        fields: &[
            ("normal", "The direction the cut faces (pointing to the part that is removed): [0, 0, 1] cuts away what is toward +z."),
            ("offset", "Where the plane is along the normal, in the child's own metres: 0 through its middle; drag it to sweep through."),
            ("thickness", "0: keep everything behind the plane. Above 0: keep only a slab this thick around it (a cross-section)."),
            ("child", "The shape to cut."),
        ] },
    ShapeInfo { ty: "relief", category: "Modify", desc: "Raises a pattern out of its child's surface (or carves it in, with a negative depth): ripples, waves, bricks, hexagons, scales, weave, cells... The surface itself moves, so the outline changes too (a material's bump only shades). Steep patterns (bricks, grid) march more slowly.", new: r#"{"type":"relief","pattern":"ripples","frequency":4,"depth":0.03,"child":{"type":"sphere","radius":0.5}}"#,
        fields: &[
            ("pattern", "Which pattern moves the surface (the same patterns as materials: ripples, waves, bricks, hexagons, scales, weave, cells, noise...)."),
            ("frequency", "Pattern cells per metre: higher = smaller, closer ripples or tiles."),
            ("depth", "How far the pattern raises the surface, in metres; negative carves it in (grooves instead of ridges)."),
            ("warp", "Noise distortion of the pattern (wobbly ripples, irregular tiles)."),
            ("sharpness", "Contrast of the pattern: higher = crisper edges (and slower marching)."),
            ("child", "The shape to emboss."),
        ] },
    ShapeInfo { ty: "terrain", category: "Landscape", desc: "A noise heightfield landscape (fraktal256's), marched on a precomputed height map. Top level only; fixed in place.", new: r#"{"type":"terrain","size":256,"resolution":512,"height":30,"base":-8}"#,
        fields: &[
            ("size", "Width of the square map in metres, centred on the position."),
            ("resolution", "Samples per side: more = finer detail, more memory, slower to build."),
            ("noise", "eroded: fraktal's slope-damped noise (valleys and ridges); perlin: rolling hills; ridged: sharp mountain ridges; billow: puffy hills; flat: level ground at `base` (a floor; give it a material such as a checkerboard)."),
            ("height", "Height scale in metres."),
            ("base", "Height added everywhere, in metres."),
            ("frequency", "Cycles per metre of the largest noise layer: smaller = bigger features."),
            ("octaves", "Noise layers: more = finer detail."),
            ("seed", "Random seed: each number gives a different landscape."),
            ("rim", "Raise a bowl rim beyond this distance from the centre, in metres; 0 = none."),
            ("rim_height", "With a rim: a ring of mountains this high instead of the bowl, rising from the rim and falling away again over rim_width (its crest halfway); 0 = the bowl."),
            ("rim_width", "The ring of mountains' width, in metres (its crest halfway across)."),
            ("island", "An island: the ground sinks under the sea toward this distance from the centre, in metres, the coast following the noise; 0 = none."),
            ("cap", "No terrain above this height (null = no cap)."),
            ("relax", "March relaxation: lower is slower but safer on steep cliffs (0.05 to 1)."),
            ("rock", "Rock colour."), ("grass", "Grass colour."), ("snow", "Snow colour."), ("sand", "Sand colour."),
            ("snow_line", "Snow above this height (null = automatic)."),
            ("sand_line", "Sand below this height (null = 0.6 m above the water, or none)."),
        ] },
    ShapeInfo { ty: "water", category: "Landscape", desc: "An infinite water plane with waves, reflection, refraction, absorption and caustics. Top level only.", new: r#"{"type":"water","level":0}"#,
        fields: &[
            ("level", "Height of the water surface in metres."),
            ("waves", "Size of the waves, as a multiple (0 = flat as a mirror). Floating objects ride them."),
            ("wave_speed", "How fast the waves move while the water is animated (render setting), as a multiple."),
        ] },
    ShapeInfo { ty: "emitter", category: "Effects", desc: "Sends out copies of an object, like a fountain, a dispenser or a spray: each grows in, lives for its lifespan (or for ever) and shrinks away. With physics they bounce and pile up; without, they fly through everything. The copies are not saved with the scene. The emitter itself is a small glowing nozzle pointing along its own +y (rotate it to aim); untick visible to hide it. Its emitted object shows as its child: select that to change what it emits.", new: r#"{"type":"emitter","rate":3,"lifespan":10,"speed":3,"spread":15,"emit":{"type":"sphere","radius":0.12,"material":{"color":[0.85,0.3,0.2],"roughness":0.4}}}"#,
        fields: &[
            ("rate", "Copies sent out a second."),
            ("lifespan", "Seconds each copy lives; 0 = for ever (they stay, up to max)."),
            ("grow", "Seconds a copy takes to grow from nothing to full size."),
            ("fade", "Seconds a copy takes to shrink away at the end of its life."),
            ("speed", "Launch speed in metres a second, along the emitter's +y."),
            ("spread", "How far from straight the copies may fly, in degrees (0 = all the same way, 180 = every way)."),
            ("physics", "Copies are solid: they bounce, roll and pile up (with the emitted object's mass, friction and bounce). Off: they fly through everything, under gravity unless the emitted object says gravity off."),
            ("max", "The most copies at once: when there are more, the oldest goes."),
            ("visible", "Show the emitter itself (a small glowing nozzle). Hidden, it still emits; select it in the object list."),
            ("seed", "Random seed of the directions."),
            ("emit", "The object sent out: any shape, with its own material, size, mass and gravity (select it below the emitter to edit it)."),
        ] },
    ShapeInfo { ty: "cloud", category: "Landscape", desc: "A volume cloud, as Blender makes them with a Principled Volume and a noise texture: scattering droplets filling a smooth union of puffs, broken up by turbulence into billows and wisps. Light passes through it and scatters (a silver lining toward the sun, pink from below at sunset), and it casts a soft shadow. It floats where it is put. Top level only.", new: r#"{"type":"cloud"}"#,
        fields: &[
            ("puffs", "The cloud's rough shape: balls it fills, each a centre (in the cloud's own metres) and a radius. Left out: a cumulus heap about 13 m across."),
            ("blend", "How the puffs join up. 0: separate round balls with creases between them. Larger: they melt into one smooth, rounded mass (the width of the melting zone, in metres; 0.5 to 3 is typical)."),
            ("density", "How thick the cloud is. Higher: solid white with darker undersides. Lower: thin and see-through, so its inside shows (try 0.3 with wisps). 1 takes out 60% of the light per metre at the core."),
            ("turbulence", "How much noise breaks up the shape. 0: smooth round puffs. 0.75 (default): billowy cumulus. 2 to 4: torn, ragged clouds."),
            ("wisps", "0 to 1: carves the inside of the cloud into wisps and filaments with a finer noise, so its inner structure shows (smoky, cirrus-like). Strongest with a low density."),
            ("frequency", "The size of the billows: billows per metre of the noise's largest layer. Smaller = fewer, bigger billows; larger = many small ones."),
            ("detail", "Noise layers (1 to 8): more = finer billows and wisps on top of the big ones (best seen with a low softness), and more cost."),
            ("softness", "How deep inside, in metres, the density reaches full: larger = softer, hazier edges that also blur the finer detail."),
            ("seed", "Random seed of the noise: each number gives other billows."),
            ("color", "The droplets' colour (white for clouds; tinted for smoke or dust)."),
            ("stretch", "How much longer it is along its own x, y and z, on top of scale (the Scale handles with Shift set this)."),
        ] },
];

/// Fields every node may carry (and the top-level-only ones).
pub const COMMON: &[(&str, &str)] = &[
    ("name", "A label for the outliner and the cost table."),
    ("position", "Where the node sits, in its parent's coordinates (metres)."),
    ("rotation", "Rotation in degrees: about x, then y, then z."),
    ("scale", "Uniform size multiplier."),
    ("material", "The node's material; children without one inherit it."),
    ("disabled", "Hide this node: it is left out when the scene compiles."),
    ("shadow", "Top level: whether it casts shadows (glass and water default to no)."),
    ("step", "Top level: march step multiplier, below 1 for shapes whose distance estimate overshoots (twisted, displaced). Empty = automatic."),
    ("bound", "Top level: a bounding radius around the position, for fractals whose size the compiler cannot work out."),
    ("friction", "Top level: sliding friction when dropped. 0 = ice, 0.5 = wood (default), 1 or more = rubber."),
    ("bounce", "Top level: bounciness when dropped. 0 = no bounce, 1 = perfectly elastic (default 0.15)."),
    ("locked", "Top level: a locked object cannot be dragged. Planes, terrain and water are always locked."),
    ("gravity", "Top level: whether it falls when dropped (how hard: the scene's environment gravity, Environment > World). Off: it floats where you let go of it, and a knock moves it only a little way (lights are made this way)."),
    ("mass", "Top level: mass in kg, for collisions (a heavy ball knocks light boxes aside, not the other way round) and throws (heavier flies slower). Empty = from its size: 300 kg per cubic metre of its bounding box, half filled."),
];

/// The scene's `environment` block (scene/json.rs EnvDef).
pub const ENVIRONMENT: &[(&str, &str)] = &[
    ("preset", "A palette for sun, sky, fog and water: golden (default), dusk, frozen, storm. The fields below override parts of it."),
    ("sky", "day (default), stars (a dim night sky) or off (black)."),
    ("time_of_day", "Hours, 0 to 24: the sun rises at 6, is highest at 12 (at sun_azimuth / sun_elevation) and sets at 18; the moon lights the night."),
    ("sun_azimuth", "Compass direction of the sun, degrees from -z toward +x."),
    ("sun_elevation", "Height of the sun above the horizon, degrees."),
    ("sun_level", "Strength of the sunlight (1: the preset's)."),
    ("ambient_level", "Strength of the sky light filling shadows (1: the preset's)."),
    ("fog_density", "Haze per metre: 0 is clear air."),
    ("clouds", "The sky's cloud cover, 0 to 1 (cloud objects are separate)."),
    ("sun_color", "[r, g, b] of the sunlight."),
    ("sky_zenith", "[r, g, b] of the sky straight up."),
    ("sky_horizon", "[r, g, b] of the sky at the horizon."),
    ("fog_color", "[r, g, b] of the haze."),
    ("ambient", "[r, g, b] of the sky light in shadows."),
    ("gravity", "m/s² straight down: 9.81 (Earth, default), 3.71 Mars, 1.62 the Moon, 0 none. Dropped objects, emitted copies, floating in water and game mode's walker feel it."),
];

pub const MATERIAL: &[(&str, &str)] = &[
    ("description", "What the material is or is for, in your words: the material library shows it under the preview. It does not change how it looks."),
    ("color", "Base colour (albedo): how much light the surface reflects, per channel."),
    ("roughness", "0 = mirror-smooth, 1 = matte. Controls reflection blur and highlights."),
    ("metallic", "0 = plastic, stone or wood; 1 = metal (tinted reflections, no diffuse colour)."),
    ("emission", "Light the surface gives off (colour and strength). Any emission makes the object a light source in every mode: it lights what is around it and casts shadows, and the bigger the object, the softer its shadows. Above about 1 it glows and blooms. With the sun, the ambient light and the sky off, emitting objects are the only light."),
    ("transmission", "0 = opaque, 1 = clear glass or liquid: light passes through and refracts."),
    ("ior", "Index of refraction of transmissive materials: 1.0 air, 1.33 water, 1.5 glass, 2.4 diamond."),
    ("absorption", "How quickly light inside a transmissive material loses the colours it does not have, per metre."),
    ("texture", "The procedural pattern blending the first set of values (colour, roughness, metal, emission) into the second."),
    ("color2", "The pattern's second colour."),
    ("roughness2", "Roughness where the pattern shows its second colour (e.g. matte rust on shiny metal)."),
    ("metallic2", "Metalness where the pattern shows its second colour."),
    ("emission2", "Emission where the pattern shows its second colour (lava cracks, neon lines)."),
    ("density", "kg per cubic metre. An object of this material weighs its volume times this, unless it has a mass of its own; under water's 1000 it floats (ice 917, oak 750), over it sinks (stone 2600, iron 7870). Unticked: the mass comes from the object's size."),
    ("light_range", "How far the light of its emission reaches, in metres: it fades smoothly to nothing there (the light still falls off with distance before that). Unticked: automatic, where it grows too faint to see."),
    ("translucency", "0..1: sunlight scattered through the material, as in clouds: lit from behind it glows, most at its rims when you look toward the sun. Not shadowed by the object itself."),
    ("texture_scale", "Pattern frequency: cells, rings or veins per metre."),
    ("warp", "Noise distortion of the pattern: 0 = regular, 1-2 = organic."),
    ("sharpness", "Contrast of the pattern: below 1 soft and blended, above 1 hard-edged."),
    ("bump", "Surface relief from the pattern's edges: grout lines, veins and cracks look carved. Negative values raise them."),
    ("bump_pattern", "The pattern the relief follows, if not the colour's: a plain colour with a bricks or hexagons relief, marble with a ripples one. Left out: the colour pattern."),
    ("bump_scale", "The size of the relief's pattern (cells per metre), if not the colour pattern's scale. Left out: the pattern scale."),
    ("space", "Pattern coordinates: object (moves with the object) or world (fixed in space, e.g. for floors)."),
];

pub fn shape(ty: &str) -> Option<&'static ShapeInfo> {
    SHAPES.iter().find(|s| s.ty == ty)
}

/// The description of field `key` of a node of type `ty`.
pub fn field_desc(ty: &str, key: &str) -> &'static str {
    shape(ty)
        .and_then(|s| s.fields.iter().find(|f| f.0 == key))
        .or_else(|| COMMON.iter().find(|f| f.0 == key))
        .map(|f| f.1)
        .unwrap_or("")
}

pub fn material_desc(key: &str) -> &'static str {
    MATERIAL.iter().find(|f| f.0 == key).map(|f| f.1).unwrap_or("")
}

/// Allowed range of a numeric field (min, max).
pub fn field_range(key: &str) -> (f64, f64) {
    match key {
        "radius" | "thickness" | "height" | "size" | "radii" | "period" | "top_radius" | "rounding" | "amplitude" | "blend" | "smooth" => (0.0, 1e6),
        "frequency" => (0.0001, 1e6),
        "mass" => (0.01, 1e6),
        "scale" => (0.001, 1e6),
        "iterations" => (0.0, 40.0),
        "octaves" => (1.0, 16.0),
        "degree" => (1.0, 3.0),
        "angle" => (1.0, 360.0),
        "power" => (2.0, 16.0),
        "resolution" => (16.0, 8192.0),
        "relax" => (0.05, 1.0),
        "min_radius" => (0.01, 1.0),
        "friction" => (0.0, 3.0),
        "bounce" => (0.0, 1.0),
        "step" => (0.05, 2.0),
        // clouds
        "turbulence" => (0.0, 4.0),
        "wisps" => (0.0, 1.0),
        "detail" => (1.0, 8.0),
        "softness" => (0.05, 20.0),
        "density" => (0.0, 100.0),
        "depth" => (-0.5, 0.5),
        "sharpness" => (0.1, 8.0),
        "warp" => (0.0, 3.0),
        // emitters
        "rate" => (0.0, 30.0),
        "lifespan" => (0.0, 120.0),
        "grow" | "fade" => (0.0, 5.0),
        "speed" => (0.0, 20.0),
        "spread" => (0.0, 180.0),
        "max" => (1.0, 300.0),
        _ => (-1e9, 1e9),
    }
}

/// A full node (every field with its default) for display.
pub fn with_defaults(node: &Value) -> Option<Value> {
    let def: crate::scene::json::NodeDef = serde_json::from_value(node.clone()).ok()?;
    serde_json::to_value(def).ok()
}

/// The default value of `key` for type `ty` (for the help screen).
pub fn default_of(ty: &str, key: &str) -> String {
    let Some(s) = shape(ty) else { return String::new() };
    // the bare type when it parses alone, else the example without this key
    let bare: Value = serde_json::json!({ "type": ty });
    let full = with_defaults(&bare).or_else(|| {
        let mut v: Value = serde_json::from_str(s.new).ok()?;
        if let Some(o) = v.as_object_mut()
            && !matches!(key, "children" | "child" | "balls")
        {
            o.remove(key);
        }
        with_defaults(&v)
    });
    match full.and_then(|f| f.get(key).cloned()) {
        Some(Value::Null) => "auto".into(),
        Some(Value::Array(a)) if a.iter().any(|x| x.is_object()) => format!("({} items)", a.len()),
        Some(Value::Object(_)) => "(node)".into(),
        Some(x) => x.to_string(),
        None => "required".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What Add puts in the scene compiles, each kind on its own.
    #[test]
    fn every_new_shape_compiles() {
        for s in SHAPES {
            let node: Value = serde_json::from_str(s.new).unwrap();
            let doc = serde_json::json!({ "objects": [node] });
            let scene = crate::scene::Scene::from_value(&doc, &mut crate::scene::TerrainCache::default()).unwrap();
            assert!(scene.problems.is_empty(), "{}: {:?}", s.ty, scene.problems);
        }
    }

    /// Every field the inspector can show for every shape (its own, with the
    /// defaults filled in, and the common ones) has a description to hover.
    #[test]
    fn every_field_is_described() {
        let mut missing = Vec::new();
        for s in SHAPES {
            let node: Value = serde_json::from_str(s.new).unwrap();
            let full = with_defaults(&node).unwrap_or(node);
            let ty = full.get("type").and_then(Value::as_str).unwrap_or(s.ty).to_string();
            for key in full.as_object().unwrap().keys() {
                if key != "type" && key != "children" && field_desc(&ty, key).is_empty() {
                    missing.push(format!("{ty}.{key}"));
                }
            }
        }
        // every material field, with each optional one set
        let mut m = serde_json::to_value(crate::scene::json::MaterialDef::default()).unwrap();
        for k in ["density", "light_range", "translucency", "texture", "color2", "emission2", "roughness2", "metallic2", "space", "description", "bump_pattern", "bump_scale"] {
            m[k] = json_value_for(k);
        }
        let m: crate::scene::json::MaterialDef = serde_json::from_value(m).unwrap();
        for key in serde_json::to_value(m).unwrap().as_object().unwrap().keys() {
            if material_desc(key).is_empty() {
                missing.push(format!("material.{key}"));
            }
        }
        assert!(missing.is_empty(), "no description: {missing:?}");
    }

    fn json_value_for(k: &str) -> Value {
        match k {
            "texture" => Value::from("noise"),
            "description" => Value::from("a test"),
            "bump_pattern" => Value::from("bricks"),
            "bump_scale" => Value::from(2.0),
            "space" => Value::from("world"),
            "color2" | "emission2" => serde_json::json!([0.5, 0.5, 0.5]),
            _ => Value::from(1.0),
        }
    }
}
