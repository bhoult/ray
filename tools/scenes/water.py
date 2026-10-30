"""Writes saves/water.json: the sea with floaters, a pier, a boat and a duck dispenser."""
import json, sys
from common import write, r
d = json.load(open(sys.argv[2]))
keep = [o for o in d['objects'] if o.get('name') not in ('pier post', 'pier', 'duck dispenser', 'rowing boat', 'buoy')]

LEVEL = 1.2
deck_y = LEVEL + 0.9
pier = []
L = 14
for k in range(int(L / 0.32)):
    pier.append({"name": "plank", "type": "box", "size": [2.0, 0.08, 0.28], "position": r([0, deck_y, -k * 0.32]), "rotation": [0, 0, [0, 0.6, -0.4][k % 3]], "material": "plank"})
for k in range(0, 8):
    for sx in (-0.95, 0.95):
        pier.append({"name": "post", "type": "cylinder", "radius": 0.13, "height": 3.2, "position": r([sx, deck_y - 1.3, -k * 2.0]), "material": "post"})
for sx in (-1.0, 1.0):
    pier.append({"name": "rail", "type": "box", "size": [0.08, 0.08, L], "position": r([sx, deck_y + 0.9, -L / 2 + 0.2]), "material": "post"})
    for k in range(0, 8):
        pier.append({"name": "rail post", "type": "box", "size": [0.08, 0.9, 0.08], "position": r([sx, deck_y + 0.45, -k * 2.0]), "material": "post"})
pier.append({"name": "lamp", "type": "union", "position": r([0.85, deck_y, -L + 0.6]), "children": [
    {"type": "cylinder", "radius": 0.05, "height": 2.2, "position": [0, 1.1, 0], "material": "iron"},
    {"type": "sphere", "radius": 0.15, "position": [0, 2.3, 0], "material": "lamp"}]})
pier.append({"name": "duck hopper", "type": "box", "size": [0.6, 0.7, 0.6], "rounding": 0.04, "position": r([0.55, deck_y + 0.39, -L + 1.4]), "material": "hopper"})
pier_obj = {"name": "pier", "type": "group", "position": [-4.4, 0, 3.0], "rotation": [0, 12, 0], "locked": True, "children": pier}

duck = {"type": "union", "material": "duck", "children": [
    {"type": "ellipsoid", "radii": [0.15, 0.1, 0.19], "position": [0, 0.09, 0]},
    {"type": "ellipsoid", "radii": [0.1, 0.06, 0.07], "position": [0, 0.13, -0.15]},
    {"type": "sphere", "radius": 0.085, "position": [0, 0.24, 0.1]},
    {"type": "ellipsoid", "radii": [0.045, 0.018, 0.05], "position": [0, 0.225, 0.19], "material": "beak"},
    {"type": "sphere", "radius": 0.014, "position": [0.05, 0.27, 0.16], "material": "eye"},
    {"type": "sphere", "radius": 0.014, "position": [-0.05, 0.27, 0.16], "material": "eye"}]}
# the dispenser at the far end of the pier, aimed out over the water
import math
a = math.radians(12)
ex, ez = 0.55 * math.cos(a) + (-L + 1.4) * math.sin(a), -0.55 * math.sin(a) + (-L + 1.4) * math.cos(a)
dispenser = {"name": "duck dispenser", "type": "emitter", "position": r([-4.4 + ex, deck_y + 0.78, 3.0 + ez]), "rotation": [0, 0, -50],
             "rate": 0.8, "lifespan": 0, "grow": 0.3, "fade": 0.8, "speed": 3.0, "spread": 18, "physics": True, "max": 24, "seed": 2, "emit": duck}

boat = {"name": "rowing boat", "type": "difference", "position": [2.6, LEVEL + 0.15, -3.2], "rotation": [0, 30, 0], "material": "boat", "children": [
    {"type": "ellipsoid", "radii": [0.75, 0.45, 1.9]},
    {"type": "ellipsoid", "radii": [0.68, 0.45, 1.82], "position": [0, 0.08, 0]},
    {"type": "box", "size": [2, 1, 4.2], "position": [0, 0.62, 0]}]}
buoys = []
for (x, z, c) in [(5.5, -6, "buoy red"), (-1.0, -9.5, "buoy red"), (7.0, -12, "buoy green")]:
    buoys.append({"name": "buoy", "type": "union", "position": [x, LEVEL + 0.1, z], "material": c, "mass": 30, "children": [
        {"type": "capsule", "radius": 0.3, "height": 0.6, "position": [0, 0, 0]},
        {"type": "cone", "radius": 0.18, "top_radius": 0.02, "height": 0.6, "position": [0, 0.6, 0]}]})

d['objects'] = keep + [pier_obj, dispenser, boat] + buoys
d['materials'].update({
    "plank": {"color": [0.5, 0.38, 0.26], "color2": [0.32, 0.22, 0.13], "texture": "wood", "texture_scale": 2, "roughness": 0.8},
    "post": {"color": [0.3, 0.22, 0.15], "color2": [0.18, 0.13, 0.08], "texture": "wood", "texture_scale": 3, "roughness": 0.9},
    "iron": {"color": [0.1, 0.1, 0.1], "metallic": 1, "roughness": 0.5},
    "lamp": {"color": [1, 0.9, 0.7], "emission": [8, 7, 5], "light_range": 5},
    "hopper": {"color": [0.95, 0.75, 0.1], "color2": [0.1, 0.1, 0.1], "texture": "stripes", "texture_scale": 3, "sharpness": 6, "roughness": 0.5},
    "duck": {"color": [1.0, 0.82, 0.08], "roughness": 0.3, "density": 120},
    "beak": {"color": [0.95, 0.4, 0.05], "roughness": 0.35},
    "eye": {"color": [0.02, 0.02, 0.02], "roughness": 0.1},
    "boat": {"color": [0.85, 0.85, 0.8], "color2": [0.1, 0.3, 0.55], "texture": "stripes", "texture_scale": 2.2, "sharpness": 6, "roughness": 0.45, "density": 450},
    "buoy red": {"color": [0.85, 0.1, 0.06], "color2": [0.95, 0.95, 0.95], "texture": "stripes", "texture_scale": 2.5, "sharpness": 5, "roughness": 0.4, "density": 300},
    "buoy green": {"color": [0.1, 0.55, 0.2], "color2": [0.95, 0.95, 0.95], "texture": "stripes", "texture_scale": 2.5, "sharpness": 5, "roughness": 0.4, "density": 300},
})
d['description'] = "The sea: floating and sinking objects (wood, ice, stone, iron, a beach ball, a rowing boat, buoys), ripples and animated waves. At the end of the pier (a group of planks, posts and rails) an emitter tosses rubber ducks into the water, where they bob on the waves."
d['camera'] = {"position": [1.5, 4.6, 10.5], "look_at": [-0.5, 1.2, -3.0], "fov": 58, "speed": 3}
write('water', d)
