"""Adds a Galton board (an emitter dropping balls through pegs into bins) to saves/physics.json."""
import json, sys
from common import write, r
src = sys.argv[2] if len(sys.argv) > 2 else 'saves/physics.json'
d = json.load(open(src))
d['objects'] = [o for o in d['objects'] if o.get('name') not in ('galton board', 'ball dropper')]

X, Z = -0.6, -3.4          # the board's foot, facing +z
W, H, GAP = 2.6, 3.5, 0.24  # inside width, height, and the gap the balls run in
# (walls thick enough that a fast marble cannot step through one)
T = 0.1
kids = [
    {"name": "back", "type": "box", "size": [W + 2 * T, H, T], "position": [0, H / 2, -T / 2], "material": "board"},
    {"name": "glass", "type": "box", "size": [W + 2 * T, H, T], "position": [0, H / 2, GAP + T / 2], "material": "pane"},
    {"name": "side", "type": "box", "size": [T, H, GAP], "position": [-W / 2 - T / 2, H / 2, GAP / 2], "material": "board"},
    {"name": "side", "type": "box", "size": [T, H, GAP], "position": [W / 2 + T / 2, H / 2, GAP / 2], "material": "board"},
    {"name": "floor", "type": "box", "size": [W, T, GAP], "position": [0, T / 2, GAP / 2], "material": "board"},
    {"name": "lid", "type": "box", "size": [W + 2 * T, T, GAP + 2 * T], "position": [0, H + T / 2, GAP / 2], "material": "board"},
    # a hopper that gathers the balls over the first peg
    {"name": "hopper", "type": "box", "size": [0.9, 0.04, GAP], "position": [-0.52, 3.15, GAP / 2], "rotation": [0, 0, -28], "material": "board"},
    {"name": "hopper", "type": "box", "size": [0.9, 0.04, GAP], "position": [0.52, 3.15, GAP / 2], "rotation": [0, 0, 28], "material": "board"},
]
# staggered rows across the whole width, close enough that a marble meets a
# peg in each (with a triangle of pegs they ran down its outside edges)
STEP, RISE, ROWS, TOP = 0.2, 0.15, 11, 2.85
for i in range(ROWS):
    for j in range(-7, 8):
        x = (j + 0.5 * (i % 2)) * STEP
        if abs(x) <= W / 2 - 0.1:
            kids.append({"name": "peg", "type": "cylinder", "radius": 0.03, "height": GAP, "position": r([x, TOP - i * RISE, GAP / 2], 3), "rotation": [90, 0, 0], "material": "brass"})
last = TOP - (ROWS - 1) * RISE
# the last row (even: pegs on multiples of STEP) leaves the gaps between them: bins under those
for j in range(-6, 7):
    kids.append({"name": "bin wall", "type": "box", "size": [0.02, last - 0.3, GAP], "position": r([j * STEP, (last - 0.3) / 2 + T, GAP / 2], 3), "material": "brass"})
board = {"name": "galton board", "type": "group", "position": [X, 0, Z], "bounce": 0.05, "friction": 0.1, "locked": True, "children": kids}
dropper = {"name": "ball dropper", "type": "emitter", "position": r([X, 3.4, Z + GAP / 2], 3), "rotation": [180, 0, 0], "visible": False,
           "rate": 3, "lifespan": 0, "grow": 0.1, "fade": 0.5, "speed": 0.4, "spread": 3, "physics": True, "max": 90,
           "emit": {"type": "sphere", "radius": 0.05, "material": "marble ball", "bounce": 0.05, "friction": 0.1}}
d['objects'] += [board, dropper]
d['materials'].update({
    "board": {"color": [0.35, 0.22, 0.12], "color2": [0.22, 0.13, 0.06], "texture": "wood", "texture_scale": 1.5, "roughness": 0.5},
    "pane": {"color": [0.92, 0.95, 0.97], "transmission": 1, "ior": 1.5, "roughness": 0},
    "brass": {"color": [0.9, 0.72, 0.35], "metallic": 1, "roughness": 0.25},
    "marble ball": {"color": [0.15, 0.35, 0.85], "color2": [0.9, 0.9, 0.95], "texture": "marble", "texture_scale": 8, "roughness": 0.08, "density": 2500},
})
d['description'] = "Things to knock over: a pyramid of crates, a ball stack, a ring of dominoes, a seesaw with a steel ball above it and a ramp; at the back a Galton board, where an emitter drops marbles through rows of brass pegs into bins. Press Drop all, or carry and throw."
write('physics', d)
