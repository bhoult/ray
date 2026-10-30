"""Writes saves/night.json: a campfire in a pine clearing under the moon.

Then let the kettle and the log seats settle onto the uneven ground:
    python3 settle.py ../../saves/night.json kettle "log seat"
"""
import math, random
from common import write, r
R = random.Random(11)

fire = [
    {"name": "flame", "type": "cone", "radius": 0.28, "top_radius": 0.0, "height": 0.8, "position": [0, 0.5, 0], "material": "flame"},
    {"name": "flame", "type": "cone", "radius": 0.18, "top_radius": 0.0, "height": 0.55, "position": [0.16, 0.38, 0.1], "rotation": [10, 0, -14], "material": "flame"},
    {"name": "flame", "type": "cone", "radius": 0.18, "top_radius": 0.0, "height": 0.6, "position": [-0.15, 0.38, -0.08], "rotation": [-8, 0, 12], "material": "flame"},
    {"name": "embers", "type": "cylinder", "radius": 0.45, "height": 0.08, "position": [0, 0.06, 0], "material": "embers"},
]
for k in range(4):
    a = k * 90 + 20
    fire.append({"name": "log", "type": "cylinder", "radius": 0.08, "height": 1.0, "position": r([0.18 * math.cos(math.radians(a)), 0.25, 0.18 * math.sin(math.radians(a))]),
                 "rotation": r([0, -a, 62]), "material": "log"})
for k in range(9):
    a = math.tau * k / 9
    s = R.uniform(0.8, 1.2)
    fire.append({"name": "stone", "type": "ellipsoid", "radii": r([0.2 * s, 0.13 * s, 0.17 * s]), "position": r([0.72 * math.cos(a), 0.08, 0.72 * math.sin(a)]), "rotation": [0, round(-math.degrees(a)), 0], "material": "stone"})

tent = {"name": "tent", "type": "group", "position": [-2.3, 0.05, -2.0], "rotation": [0, 62, 0], "locked": True, "children": [
    {"name": "side", "type": "box", "size": [1.55, 0.03, 2.6], "position": [-0.5, 0.65, 0], "rotation": [0, 0, 55], "material": "canvas"},
    {"name": "side", "type": "box", "size": [1.55, 0.03, 2.6], "position": [0.5, 0.65, 0], "rotation": [0, 0, -55], "material": "canvas"},
    {"name": "back", "type": "intersection", "position": [0, 0.64, -1.28], "material": "canvas", "children": [
        {"type": "box", "size": [1.9, 1.3, 0.03]},
        {"type": "box", "size": [1.6, 1.6, 0.04], "position": [0, -0.9, 0], "rotation": [0, 0, 45]}]},
    {"name": "lantern", "type": "sphere", "radius": 0.07, "position": [0, 0.95, -0.3], "material": "lantern"},
]}

objects = [
    {"name": "ground", "type": "terrain", "noise": "perlin", "size": 400, "resolution": 256, "height": 0.6, "base": 0, "frequency": 0.05, "octaves": 4, "seed": 3, "locked": True, "material": "forest floor"},
    {"name": "campfire", "type": "group", "position": [0, 0.15, 0], "locked": True, "children": fire},
    {"name": "sparks", "type": "emitter", "position": [0, 0.75, 0], "visible": False,
     "rate": 11, "lifespan": 2.2, "grow": 0.05, "fade": 0.9, "speed": 1.8, "spread": 22, "physics": False, "max": 26, "seed": 5,
     "emit": {"type": "sphere", "radius": 0.018, "gravity": False, "shadow": False, "material": "spark"}},
    {"name": "smoke", "type": "emitter", "position": [0, 1.3, 0], "rotation": [0, 0, -8], "visible": False,
     "rate": 0.7, "lifespan": 9, "grow": 6, "fade": 3, "speed": 0.7, "spread": 10, "physics": False, "max": 8,
     "emit": {"type": "cloud", "scale": 0.12, "density": 2.5, "color": [0.32, 0.3, 0.29], "turbulence": 1.2, "softness": 1.5, "gravity": False}},
    tent,
    {"name": "log seat", "type": "cylinder", "radius": 0.2, "height": 1.8, "position": [1.7, 0.38, 0.9], "rotation": [0, -30, 90], "material": "log"},
    {"name": "log seat", "type": "cylinder", "radius": 0.18, "height": 1.5, "position": [-1.2, 0.36, 1.7], "rotation": [0, 50, 90], "material": "log"},
    {"name": "kettle", "type": "union", "position": [1.08, 0.23, -0.81], "material": "iron", "children": [
        {"type": "cylinder", "radius": 0.1, "height": 0.03, "position": [0, -0.065, 0]},
        {"type": "sphere", "radius": 0.16, "position": [0, 0.08, 0]},
        {"type": "torus", "radius": 0.11, "thickness": 0.012, "position": [0, 0.26, 0], "rotation": [90, 0, 0]}]},
]
pines = []
for k in range(40):
    a = R.uniform(0, math.tau)
    d = R.uniform(6.5, 22)
    x, z = d * math.cos(a), d * math.sin(a)
    if z > 4 and abs(x) < 6:  # keep the view open
        continue
    h = R.uniform(4, 8)
    pines.append({"name": "pine", "type": "union", "position": r([x, 0, z]), "rotation": [0, R.randint(0, 359), 0], "children": [
        {"type": "cylinder", "radius": 0.14 * h / 6, "height": h * 0.35, "position": r([0, h * 0.17, 0]), "material": "bark"},
        {"type": "cone", "radius": h * 0.24, "top_radius": 0.0, "height": h * 0.5, "position": r([0, h * 0.45, 0]), "material": "needles"},
        {"type": "cone", "radius": h * 0.19, "top_radius": 0.0, "height": h * 0.42, "position": r([0, h * 0.66, 0]), "material": "needles"},
        {"type": "cone", "radius": h * 0.13, "top_radius": 0.0, "height": h * 0.34, "position": r([0, h * 0.85, 0]), "material": "needles"}]})
objects.append({"name": "pines", "type": "group", "locked": True, "children": pines})

write('night', {
    "description": "A campfire in a pine clearing under the moon and the stars. The fire's flames light the scene; two hidden emitters send up sparks (each a small light that flickers over the ground as it flies and fades) and smoke; the tent glows from the lantern inside (its canvas lets light through). Press L for a headlight to look around in the dark.",
    "camera": {"position": [2.2, 1.7, 5.4], "look_at": [-0.6, 0.7, -0.6], "fov": 55, "speed": 3},
    "environment": {"preset": "golden", "time_of_day": 23.2, "sun_azimuth": 200, "clouds": 0.1, "fog_density": 0.01},
    "render": {"bounces": 3},
    "materials": {
        "forest floor": {"color": [0.16, 0.12, 0.08], "color2": [0.1, 0.13, 0.06], "texture": "noise", "texture_scale": 0.6, "warp": 0.5, "roughness": 0.95, "bump_pattern": "noise", "bump_scale": 4, "bump": 1.2, "space": "world"},
        "flame": {"color": [1, 0.55, 0.15], "emission": [26, 9, 1.6], "light_range": 14},
        "embers": {"color": [0.1, 0.05, 0.03], "color2": [1, 0.35, 0.05], "emission2": [6, 1.5, 0.2], "texture": "cracks", "texture_scale": 6, "roughness": 0.9},
        "log": {"color": [0.3, 0.2, 0.12], "color2": [0.12, 0.08, 0.05], "texture": "wood", "texture_scale": 3, "roughness": 0.85, "bump_pattern": "wood", "bump": 1.5},
        "stone": {"color": [0.42, 0.4, 0.38], "color2": [0.25, 0.24, 0.23], "texture": "noise", "texture_scale": 4, "roughness": 0.8, "bump_pattern": "cells", "bump_scale": 6, "bump": 1.5},
        "spark": {"color": [1, 0.6, 0.2], "emission": [260, 100, 18], "light_range": 3},
        "canvas": {"color": [0.7, 0.42, 0.16], "color2": [0.55, 0.32, 0.1], "texture": "weave", "texture_scale": 14, "roughness": 0.9, "translucency": 0.7},
        "lantern": {"color": [1, 0.85, 0.55], "emission": [80, 60, 30], "light_range": 4},
        "iron": {"color": [0.08, 0.08, 0.08], "metallic": 0.9, "roughness": 0.45},
        "bark": {"color": [0.16, 0.11, 0.07], "roughness": 0.95},
        "needles": {"color": [0.05, 0.12, 0.06], "color2": [0.03, 0.07, 0.04], "texture": "scales", "texture_scale": 3, "roughness": 0.9, "bump_pattern": "scales", "bump": 2},
    },
    "objects": objects,
})
