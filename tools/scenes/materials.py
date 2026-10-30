"""Adds two rows to saves/materials.json: the newer patterns, bump maps, orbit traps and relief."""
import json, sys
from common import write, r
d = json.load(open(sys.argv[2]))
names = {"stand", "hexagon tiles", "sand ripples", "pond rings", "dragon scales", "wicker", "tower windows", "orbit copper", "orbit coral julia", "relief stone", "hammered gold", "knurled steel", "pebbled leather", "light 1"}
d['objects'] = [o for o in d['objects'] if o.get('name') not in names]
X = [-3.125, -1.875, -0.625, 0.625, 1.875, 3.125]
row6 = [
    ("hexagon tiles", "box", {"color": [0.85, 0.83, 0.78], "color2": [0.2, 0.2, 0.22], "texture": "hexagons", "texture_scale": 4, "roughness": 0.15, "bump_pattern": "hexagons", "bump": 0.6}),
    ("sand ripples", "sphere", {"color": [0.86, 0.72, 0.5], "color2": [0.7, 0.56, 0.36], "texture": "waves", "texture_scale": 5, "roughness": 0.9, "bump_pattern": "waves", "bump": 1.2}),
    ("pond rings", "sphere", {"color": [0.12, 0.3, 0.38], "color2": [0.3, 0.55, 0.6], "texture": "ripples", "texture_scale": 6, "roughness": 0.05, "bump_pattern": "ripples", "bump": 0.8}),
    ("dragon scales", "sphere", {"color": [0.15, 0.5, 0.25], "color2": [0.03, 0.12, 0.06], "metallic": 0.5, "texture": "scales", "texture_scale": 6, "roughness": 0.25, "bump_pattern": "scales", "bump": 1.5}),
    ("wicker", "box", {"color": [0.7, 0.52, 0.28], "color2": [0.15, 0.09, 0.04], "texture": "weave", "texture_scale": 6, "roughness": 0.8, "bump_pattern": "weave", "bump": 2}),
    ("tower windows", "tall", {"color": [0.3, 0.3, 0.32], "color2": [1.0, 0.82, 0.55], "emission2": [2.0, 1.5, 0.8], "texture": "windows", "texture_scale": 5, "roughness": 0.6}),
]
row7 = [
    ("orbit copper", {"type": "mandelbulb", "scale": 0.42}, {"color": [0.95, 0.55, 0.35], "color2": [0.15, 0.5, 0.5], "metallic": 1, "metallic2": 0.2, "texture": "orbit", "texture_scale": 2, "roughness": 0.25}),
    ("orbit coral julia", {"type": "julia", "c": [-0.2, 0.6, 0.2, 0.2], "iterations": 11, "scale": 0.36, "rotation": [0, 0, 90]}, {"color": [0.95, 0.82, 0.72], "color2": [0.7, 0.18, 0.15], "texture": "orbit", "texture_scale": 2, "roughness": 0.55}),
    ("relief stone", {"type": "relief", "pattern": "cells", "scale": 0.45, "depth": 0.06, "child": {"type": "sphere", "radius": 1.0}}, {"color": [0.6, 0.57, 0.52], "roughness": 0.85}),
    ("hammered gold", {"type": "sphere", "radius": 0.45}, {"color": [1.0, 0.76, 0.33], "metallic": 1, "roughness": 0.15, "bump_pattern": "cells", "bump_scale": 9, "bump": 1.5}),
    ("knurled steel", {"type": "relief", "pattern": "weave", "scale": 0.4, "depth": 0.03, "child": {"type": "cylinder", "radius": 1.0, "height": 2.2}}, {"color": [0.7, 0.71, 0.74], "metallic": 1, "roughness": 0.3}),
    ("pebbled leather", {"type": "sphere", "radius": 0.45}, {"color": [0.35, 0.16, 0.08], "roughness": 0.6, "bump_pattern": "spots", "bump_scale": 14, "bump": 1.2}),
]
# the two new rows stand on steps, so they show over the others
S6, S7 = 0.7, 1.4
d['objects'].append({"name": "stand", "type": "group", "locked": True, "children": [
    {"name": "step", "type": "box", "size": [7.6, S6, 1.4], "position": [0, S6 / 2, -7.0], "material": {"color": [0.3, 0.3, 0.31], "roughness": 0.7}},
    {"name": "step", "type": "box", "size": [7.6, S7, 1.4], "position": [0, S7 / 2, -8.4], "material": {"color": [0.27, 0.27, 0.28], "roughness": 0.7}}]})
for i, (name, kind, m) in enumerate(row6):
    z = -7.0
    if kind == "box":
        o = {"name": name, "type": "box", "size": [0.8, 0.8, 0.8], "position": [X[i], S6 + 0.4, z], "rotation": [0, 25, 0]}
    elif kind == "tall":
        o = {"name": name, "type": "box", "size": [0.6, 1.3, 0.6], "position": [X[i], S6 + 0.65, z], "rotation": [0, 30, 0]}
    else:
        o = {"name": name, "type": "sphere", "radius": 0.45, "position": [X[i], S6 + 0.45, z]}
    o["material"] = m
    d['objects'].append(o)
for i, (name, node, m) in enumerate(row7):
    o = dict(node)
    o["name"] = name
    o["position"] = [X[i], S7 + 0.48, -8.4]
    o["material"] = m
    d['objects'].append(o)
d['objects'].append({"name": "light 1", "type": "sphere", "radius": 0.12, "position": [0, 4.6, -7.6], "gravity": False, "material": {"color": [1.0, 0.9, 0.8], "emission": [700, 630, 560]}})
for o in d['objects']:
    if o.get('name') == 'light 0':
        o['position'] = [0, 4.2, -2.6]
        o['material']['emission'] = [900, 810, 720]
d["camera"] = {"position": [0, 4.4, 3.2], "look_at": [0, 0.5, -4.6], "fov": 58, "speed": 3}
d['description'] = "Materials on rows of objects: metals, plastics, glass, stone, wood and the procedural patterns; at the back the newer patterns (hexagons, waves, ripples, scales, weave, windows), bump maps, fractals coloured by orbit traps, and relief (a pattern carved into the surface)."
write('materials', d)
