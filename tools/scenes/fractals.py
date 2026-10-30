"""Writes saves/fractals.json: a gallery of fractals on plinths in a half circle.

Then let the pieces settle onto their plinths (the names are printed last):
    python3 settle.py ../../saves/fractals.json <names>
"""
import json, math, sys
from common import write, r


def euler(m):
    """Euler angles (degrees, applied x, then y, then z) of rotation matrix m: Mat3::to_euler_deg."""
    sb = max(-1.0, min(1.0, -m[2][0]))
    b = math.asin(sb)
    if abs(sb) < 0.9999:
        a, c = math.atan2(m[2][1], m[2][2]), math.atan2(m[1][0], m[0][0])
    else:
        a, c = math.atan2(-m[1][2], m[1][1]), 0.0
    return [round(math.degrees(v), 2) for v in (a, b, c)]


def matrix(e):
    """The rotation of Euler angles e (degrees): Rz Ry Rx."""
    x, y, z = (math.radians(v) for v in e)
    rx = [[1, 0, 0], [0, math.cos(x), -math.sin(x)], [0, math.sin(x), math.cos(x)]]
    ry = [[math.cos(y), 0, math.sin(y)], [0, 1, 0], [-math.sin(y), 0, math.cos(y)]]
    rz = [[math.cos(z), -math.sin(z), 0], [math.sin(z), math.cos(z), 0], [0, 0, 1]]
    mul = lambda a, b: [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
    return mul(rz, mul(ry, rx))


def turned(face, e):
    """Rotation e, then turned `face` degrees about the vertical."""
    a = math.radians(face)
    ry = [[math.cos(a), 0, math.sin(a)], [0, 1, 0], [-math.sin(a), 0, math.cos(a)]]
    m = matrix(e)
    return euler([[sum(ry[i][k] * m[k][j] for k in range(3)) for j in range(3)] for i in range(3)])

R = 7.0
# (name, node, plinth height, size above it): left to right
pieces = [
    ("sponge", {"type": "menger", "iterations": 5, "scale": 0.75, "rotation": [0, 35, 0], "material": "blue lace"}, 0.7, 0.75),
    ("mandelbox", {"type": "mandelbox", "fold_scale": -1.5, "iterations": 14, "scale": 0.36, "material": "bone orbit"}, 0.9, 0.85),
    ("kifs", {"type": "kifs", "fold_scale": 3, "offset": [1, 1, 1], "angles": [12, 8, 0], "iterations": 6, "scale": 0.7, "rotation": [0, 20, 0], "material": "steel orbit"}, 1.1, 0.75),
    ("julia", {"type": "julia", "c": [-0.2, 0.6, 0.2, 0.2], "iterations": 11, "scale": 0.95, "rotation": [0, 0, 90], "material": "jade orbit"}, 1.3, 0.9),
    ("mandelbulb", {"type": "mandelbulb", "power": 8, "iterations": 10, "scale": 1.25, "rotation": [0, 30, 0], "material": "copper orbit"}, 1.5, 1.3),
    # (on one of its square faces, the cut tilted up toward the middle)
    ("cut sponge", {"type": "slice", "normal": [1, 1, 1], "offset": 0.0, "scale": 0.62, "rotation": [0, -45, 0], "material": "copper orbit", "child": {"type": "menger", "iterations": 4}}, 1.3, 0.62),
    ("apollonian", {"type": "intersection", "scale": 0.4, "material": "coral", "children": [{"type": "box", "size": [4, 4, 4]}, {"type": "apollonian", "fold": 1.1, "iterations": 8}]}, 1.1, 0.85),
    ("kleinian", {"type": "intersection", "scale": 0.4, "rotation": [0, -35, 0], "material": "blue lace", "children": [{"type": "box", "size": [4, 4, 4]}, {"type": "kleinian", "iterations": 8}]}, 0.9, 0.85),
    ("sierpinski", {"type": "sierpinski", "iterations": 9, "scale": 0.85, "rotation": [0, 40, 0], "material": "bone orbit"}, 0.7, 0.6),
]
objects = [{"name": "floor", "type": "plane", "locked": True, "material": "floor"}]
n = len(pieces)
for i, (name, node, ph, half) in enumerate(pieces):
    a = math.radians(-72 + 144 * i / (n - 1))
    x, z = R * math.sin(a), -R * math.cos(a)
    face = -math.degrees(a)
    # each piece rests on its plinth (Drop all settles it there; carry it
    # off and throw it), turned to face the middle; the plinths are fixed
    node = dict(node)
    node["name"] = name
    # (a little above its plinth: settle.py then lets it fall onto it)
    node["position"] = r([x, ph + 0.08 + half + 0.25, z], 3)
    node["rotation"] = turned(face, node.get("rotation", [0, 0, 0]))
    objects.append({"name": f"{name} plinth", "type": "group", "position": r([x, 0, z], 3), "rotation": [0, round(face, 1), 0], "locked": True, "children": [
        {"name": "plinth", "type": "cylinder", "radius": 0.62, "height": ph, "rounding": 0.03, "position": [0, ph / 2, 0], "material": "marble"},
        {"name": "plinth top", "type": "cylinder", "radius": 0.72, "height": 0.08, "position": [0, ph + 0.04, 0], "material": "marble"},
        {"name": "plinth foot", "type": "cylinder", "radius": 0.72, "height": 0.12, "position": [0, 0.06, 0], "material": "marble"},
    ]})
    if name in ("julia", "mandelbulb"):
        # round pieces sit in a ring, or they roll off
        objects[-1]["children"].append({"name": "cradle", "type": "torus", "radius": 0.34, "thickness": 0.05, "position": [0, round(ph + 0.13, 3), 0], "material": "brass"})
    objects.append(node)
# in the middle, ice
objects.append({"name": "chrome bulb", "type": "mandelbulb", "power": 5, "iterations": 7, "scale": 0.5, "position": [0, 1.0, -0.8], "material": "chrome"})
objects.append({"name": "ice plinth", "type": "cylinder", "radius": 0.32, "height": 0.48, "position": [0, 0.24, -0.8], "locked": True, "material": "marble"})

write('fractals', {
    "description": "A gallery of fractals in a half circle, each coloured by its orbit trap: Menger sponge, Mandelbox, kaleidoscopic IFS, quaternion Julia, Mandelbulb, a Menger sponge cut along its diagonal (the slice modifier: six-pointed stars inside), Apollonian packing, pseudo-Kleinian, Sierpinski tetrahedron; a chrome bulb in the middle. The plinths are fixed groups; the pieces rest on them: carry one off and throw it, or Drop all. Fly in close (the wheel and WASD slow near surfaces; L lights the way inside).",
    "camera": {"position": [0, 2.7, 5.6], "look_at": [0, 1.35, -3.4], "fov": 56, "speed": 3},
    "environment": {"preset": "golden", "time_of_day": 16.6, "sun_azimuth": 150, "fog_density": 0.006, "clouds": 0.35},
    "render": {"exposure": 1.0, "max_steps": 220},
    "materials": {
        "floor": {"color": [0.11, 0.1, 0.1], "color2": [0.03, 0.03, 0.03], "texture": "hexagons", "texture_scale": 1.6, "roughness": 0.12, "roughness2": 0.5, "space": "world"},
        "marble": {"color": [0.86, 0.84, 0.8], "color2": [0.35, 0.33, 0.32], "texture": "marble", "texture_scale": 1.2, "roughness": 0.25},
        "copper orbit": {"description": "Copper turning to verdigris along the orbit trap.", "color": [0.95, 0.55, 0.35], "color2": [0.15, 0.5, 0.5], "metallic": 1, "metallic2": 0.2, "texture": "orbit", "texture_scale": 2, "roughness": 0.25},
        "bone orbit": {"description": "Bone, rust-red where the orbit trap is high.", "color": [0.82, 0.78, 0.7], "color2": [0.5, 0.18, 0.1], "texture": "orbit", "texture_scale": 1.5, "roughness": 0.6},
        "jade orbit": {"description": "Jade with gold bands along the orbit trap.", "color": [0.3, 0.65, 0.45], "color2": [0.95, 0.8, 0.35], "metallic2": 1, "texture": "orbit", "texture_scale": 2, "roughness": 0.3},
        "coral": {"description": "Pale coral with deep red in the folds (orbit).", "color": [0.95, 0.82, 0.72], "color2": [0.7, 0.18, 0.15], "texture": "orbit", "texture_scale": 2, "roughness": 0.55},
        "blue lace": {"description": "White lace on deep blue (orbit).", "color": [0.88, 0.88, 0.95], "color2": [0.15, 0.25, 0.65], "texture": "orbit", "texture_scale": 2, "roughness": 0.45},
        "steel orbit": {"description": "Steel with glowing amber seams (orbit).", "color": [0.75, 0.78, 0.82], "metallic": 1, "color2": [0.95, 0.5, 0.15], "metallic2": 0, "emission2": [0.6, 0.25, 0.05], "texture": "orbit", "texture_scale": 2.5, "roughness": 0.3},
        "brass": {"color": [0.85, 0.65, 0.3], "metallic": 1, "roughness": 0.3},
        "chrome": {"color": [0.95, 0.95, 0.97], "metallic": 1, "roughness": 0.04},
    },
    "objects": objects,
})
print(' '.join(f'"{p[0]}"' for p in pieces) + ' "chrome bulb"')
