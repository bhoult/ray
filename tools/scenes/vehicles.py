"""Adds a car park to saves/vehicles.json: asphalt with painted bays, kerbs, cones and lamp posts."""
import json, sys
from common import write, r
d = json.load(open(sys.argv[2]))
d['objects'] = [o for o in d['objects'] if o.get('name') not in ('car park', 'cone', 'lamp posts')]
lot = [{"name": "asphalt", "type": "box", "size": [18, 0.04, 13], "position": [0, 0.02, 0], "material": "asphalt"}]
# bays along the back and front edges
for k in range(-3, 4):
    for z0, sgn in [(-6.5, 1), (6.5, -1)]:
        lot.append({"name": "bay line", "type": "box", "size": [0.12, 0.012, 4.6], "position": r([k * 2.6 - 1.3, 0.045, z0 + sgn * 2.3]), "material": "paint"})
lot.append({"name": "lane arrow", "type": "box", "size": [3.0, 0.012, 0.12], "position": [5.5, 0.045, 0], "material": "paint"})
for (x, z, w, dpt) in [(0, -6.6, 18.4, 0.3), (0, 6.6, 18.4, 0.3), (-9.1, 0, 0.3, 13.2)]:
    lot.append({"name": "kerb", "type": "box", "size": [w, 0.18, dpt], "position": [x, 0.09, z], "material": "kerb"})
cones = []
for (x, z) in [(6.5, -1.2), (7.3, -0.4), (8.1, 0.4), (6.0, 2.2)]:
    cones.append({"name": "cone", "type": "union", "position": [x, 0.04, z], "material": "cone", "children": [
        {"type": "box", "size": [0.4, 0.04, 0.4], "position": [0, 0.02, 0], "material": "cone base"},
        {"type": "cone", "radius": 0.15, "top_radius": 0.03, "height": 0.55, "position": [0, 0.3, 0]}]})
lamps = []
for (x, z) in [(-8.6, -6.0), (-8.6, 6.0), (8.6, -6.6)]:
    lamps.append({"name": "lamp post", "type": "union", "position": [x, 0, z], "children": [
        {"type": "cylinder", "radius": 0.08, "height": 5.0, "position": [0, 2.5, 0], "material": "post"},
        {"type": "box", "size": [0.5, 0.15, 0.3], "position": [0.18, 5.0, 0], "material": "post"},
        {"type": "box", "size": [0.4, 0.04, 0.22], "position": [0.18, 4.92, 0], "material": "lamp glass"}]})
d['objects'].append({"name": "car park", "type": "group", "locked": True, "children": lot})
d['objects'] += cones
d['objects'].append({"name": "lamp posts", "type": "group", "locked": True, "children": lamps})
for o in d['objects']:
    if o.get('name') == 'floor':
        o['material'] = "grass"
d['materials'].update({
    "asphalt": {"color": [0.12, 0.12, 0.13], "color2": [0.08, 0.08, 0.085], "texture": "noise", "texture_scale": 3, "roughness": 0.8, "bump_pattern": "cells", "bump_scale": 20, "bump": 0.4, "space": "world"},
    "paint": {"color": [0.9, 0.88, 0.8], "roughness": 0.6},
    "kerb": {"color": [0.6, 0.58, 0.55], "color2": [0.45, 0.44, 0.42], "texture": "noise", "texture_scale": 4, "roughness": 0.8},
    "cone": {"color": [0.95, 0.35, 0.05], "color2": [0.95, 0.95, 0.95], "texture": "stripes", "texture_scale": 5, "sharpness": 6, "roughness": 0.5, "density": 300},
    "cone base": {"color": [0.08, 0.08, 0.08], "roughness": 0.8},
    "post": {"color": [0.3, 0.32, 0.33], "metallic": 0.8, "roughness": 0.45},
    "lamp glass": {"color": [1, 0.95, 0.85], "emission": [3, 2.8, 2.4]},
    "grass": {"color": [0.26, 0.42, 0.13], "color2": [0.17, 0.3, 0.09], "texture": "noise", "texture_scale": 0.5, "warp": 0.6, "roughness": 0.92, "space": "world"},
})
d['description'] = "Vehicles built from primitives, each one union node (so the physics moves it whole): two cars, a pickup truck and a cart in a car park (a group: asphalt, painted bays, kerbs), with traffic cones to knock over and a ramp to jump. Carry them, throw them, or Drop all."
write('vehicles', d)
