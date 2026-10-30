"""Small touches: orbit colours in the landscape, carved arches in the infinite scene, seagulls... (one object per line kept)."""
import json, sys
from common import write
src = sys.argv[2]
# landscape: the fractals coloured by their orbit traps
d = json.load(open(f'{src}/landscape.json'))
d['materials']['citadel'] = {"color": [0.9, 0.92, 0.95], "metallic": 1, "roughness": 0.08, "color2": [0.75, 0.45, 0.2], "metallic2": 1, "texture": "orbit", "texture_scale": 2}
d['materials']['bone'] = {"color": [0.78, 0.72, 0.62], "color2": [0.4, 0.2, 0.12], "texture": "orbit", "texture_scale": 1.5, "roughness": 0.7}
d['materials']['lichen'] = {"color": [0.5, 0.48, 0.42], "color2": [0.3, 0.42, 0.18], "texture": "orbit", "texture_scale": 2, "roughness": 0.85}
for o in d['objects']:
    if o.get('name') == 'citadel':
        o['material'] = 'citadel'
    if o.get('name') == 'tower':
        o['material'] = 'lichen'
    if o.get('name') == 'bulb':
        o['material'] = 'bone'
d['description'] = "A 512 m eroded landscape with a lake; Menger structures and a Mandelbulb in the distance coloured by their orbit traps; crystals of glass and amber."
write('landscape', d)
# infinite: the arches carved from stone blocks (bricks relief) and the snow drifted (waves bump)
d = json.load(open(f'{src}/infinite.json'))
d['materials']['iron'] = {"color": [0.55, 0.56, 0.6], "metallic": 1, "roughness": 0.35, "bump_pattern": "bricks", "bump_scale": 2.5, "bump": 1.2}
d['materials']['snow'] = {"color": [0.85, 0.88, 0.92], "roughness": 0.8, "bump_pattern": "waves", "bump_scale": 0.6, "bump": 0.8}
d['description'] = "Infinitely repeated CSG arches (iron, with bricks as a bump map) and glass orbs in drifted snow, for the cost of one of each."
write('infinite', d)
