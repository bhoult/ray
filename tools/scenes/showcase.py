"""Updates saves/showcase.json: orbit gold on the bulb, a fountain (an emitter), bricks relief on the columns, hexagon floor."""
import json, sys
from common import write, r
d = json.load(open(sys.argv[2]))
d['objects'] = [o for o in d['objects'] if o.get('name') not in ('fountain', 'fountain jet')]
for o in d['objects']:
    if o.get('name') == 'bulb':
        o['material'] = 'gold orbit'
        o['iterations'] = 10
    if o.get('name') == 'columns' and o['child'].get('type') != 'relief':
        # carved stone: the bricks pattern pressed into the shafts
        o['child'] = {"type": "relief", "pattern": "bricks", "frequency": 3, "depth": 0.02, "child": o["child"]}
d['materials']['gold orbit'] = {"color": [1.0, 0.76, 0.33], "metallic": 1, "roughness": 0.15, "color2": [0.55, 0.12, 0.06], "metallic2": 0.6, "texture": "orbit", "texture_scale": 2.5}
d['materials']['basin'] = {"color": [0.78, 0.76, 0.72], "color2": [0.5, 0.48, 0.45], "texture": "hexagons", "texture_scale": 5, "roughness": 0.4, "bump_pattern": "hexagons", "bump": 0.6}
d['materials']['pool'] = {"color": [0.1, 0.35, 0.45], "roughness": 0.02, "bump_pattern": "ripples", "bump_scale": 4, "bump": 0.6}
d['materials']['drop'] = {"color": [0.7, 0.9, 1.0], "roughness": 0.05, "metallic": 0.6, "emission": [0.15, 0.3, 0.4]}
fx, fz = 1.4, 4.4
d['objects'].append({"name": "fountain", "type": "group", "position": [fx, 0, fz], "locked": True, "children": [
    {"name": "basin", "type": "difference", "position": [0, 0.25, 0], "material": "basin", "children": [
        {"type": "cylinder", "radius": 1.0, "height": 0.5, "rounding": 0.04},
        {"type": "cylinder", "radius": 0.88, "height": 0.6, "position": [0, 0.15, 0]}]},
    {"name": "water", "type": "cylinder", "radius": 0.9, "height": 0.05, "position": [0, 0.38, 0], "material": "pool"},
    {"name": "spout", "type": "cylinder", "radius": 0.07, "height": 0.6, "position": [0, 0.5, 0], "material": "gold"}]})
d['objects'].append({"name": "fountain jet", "type": "emitter", "position": [fx, 0.82, fz], "visible": False,
                     "rate": 28, "lifespan": 1.25, "grow": 0.05, "fade": 0.3, "speed": 4.2, "spread": 7, "physics": False, "max": 40, "seed": 9,
                     "emit": {"type": "sphere", "radius": 0.035, "shadow": False, "material": "drop"}})
d['description'] = "The tour: CSG, metaballs, twist, displacement, a gyroid, a Menger sponge, a Mandelbulb coloured by its orbit trap, glass and chrome on a checkerboard; columns with bricks pressed in (relief), and a fountain whose jet is an emitter."
write('showcase', d)
