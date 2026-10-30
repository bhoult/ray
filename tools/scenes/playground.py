"""Adds a ball cannon (an emitter) aimed at the crate stack to saves/playground.json."""
import json, sys
from common import write
d = json.load(open(sys.argv[2]))
d['objects'] = [o for o in d['objects'] if o.get('name') not in ('cannon', 'cannon balls')]
d['objects'].append({"name": "cannon", "type": "group", "position": [-3.0, 0, 5.55], "rotation": [0, -90, 0], "locked": True, "children": [
    {"name": "barrel", "type": "cylinder", "radius": 0.2, "height": 1.4, "rounding": 0.03, "position": [-0.2, 0.75, 0], "rotation": [0, 0, 84], "material": "cannon iron"},
    {"name": "muzzle", "type": "torus", "radius": 0.2, "thickness": 0.05, "position": [-0.9, 0.82, 0], "rotation": [0, 0, 84], "material": "cannon iron"},
    {"name": "wheel", "type": "torus", "radius": 0.42, "thickness": 0.07, "position": [0.1, 0.45, 0.32], "rotation": [90, 0, 0], "material": "cannon wood"},
    {"name": "wheel", "type": "torus", "radius": 0.42, "thickness": 0.07, "position": [0.1, 0.45, -0.32], "rotation": [90, 0, 0], "material": "cannon wood"},
    {"name": "carriage", "type": "box", "size": [1.2, 0.25, 0.5], "position": [0.25, 0.35, 0], "rotation": [0, 0, -10], "material": "cannon wood"}]})
d['objects'].append({"name": "cannon balls", "type": "emitter", "position": [-3.0, 0.83, 4.62], "rotation": [-80, 0, 0], "visible": False,
                     "rate": 0.4, "lifespan": 9, "grow": 0.05, "fade": 0.8, "speed": 9, "spread": 1.5, "physics": True, "max": 5, "seed": 4,
                     "emit": {"type": "sphere", "radius": 0.17, "material": "cannon ball", "bounce": 0.35}})
d.setdefault('materials', {}).update({
    "cannon iron": {"color": [0.12, 0.12, 0.13], "metallic": 1, "roughness": 0.45},
    "cannon wood": {"color": [0.45, 0.28, 0.14], "color2": [0.28, 0.16, 0.07], "texture": "wood", "texture_scale": 3, "roughness": 0.7},
    "cannon ball": {"color": [0.85, 0.15, 0.1], "roughness": 0.45, "density": 900},
})
d['description'] = "Physics toys on a tiled floor at dusk: a crate stack, dominoes, balls, a ramp, and a cannon (an emitter) that fires a rubber ball at the crates every few seconds. Carry, throw, or Drop all."
d['camera'] = {"position": [-0.3, 3.3, 9.6], "look_at": [-0.2, 0.6, 0.6], "fov": 60, "speed": 4}
write('playground', d)
