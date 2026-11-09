"""Writes saves/sunbeams.json: a stone hall in hazy air, the low sun falling
through its tall windows in shafts.

The hall's air is haze (Environment > Haze): lit only where the sunlight
reaches, so each window's light shows as a beam, crossed by the shadows of
its tracery and the columns. A Menger sponge hangs in one of the beams,
turned so that its tunnels run along the sunlight: the light through them
goes on as a lattice of small beams.
"""
import math
from common import write, r

# the hall inside: x across, z along (the back wall at -33), up to the ceiling
W, H, BACK, FRONT = 7.0, 14.0, -33.0, 5.0
MID = (BACK + FRONT) / 2
LONG = FRONT - BACK
# the windows on the left wall (every 6 m about z -15) and the columns
# between them (about z -18); a window's sill, its arch's spring and width
SILL, SPRING, WIDE = 3.0, 10.0, 3.0
# toward the sun: low in the west, ahead of the camera
SUN = [-0.74, 0.5, -0.45]
s = [c / math.hypot(*SUN) for c in SUN]

window = {"type": "union", "children": [
    {"type": "box", "size": [3, SPRING - SILL, WIDE], "position": [0, (SILL + SPRING) / 2, 0]},
    {"type": "cylinder", "radius": WIDE / 2, "height": 3, "position": [0, SPRING, 0], "rotation": [0, 0, 90]}]}
tracery = {"type": "union", "children": [
    {"type": "box", "size": [0.22, SPRING - SILL + 0.9, 0.14], "position": [0, (SILL + SPRING) / 2 + 0.45, 0]},
    {"type": "box", "size": [0.22, 0.12, WIDE], "position": [0, SILL + (SPRING - SILL) * 0.55, 0]},
    {"type": "torus", "radius": 0.62, "thickness": 0.07, "position": [0, SPRING + 0.25, 0], "rotation": [0, 0, 90]}]}
column = {"type": "union", "children": [
    {"type": "box", "size": [1.5, 0.5, 1.5], "position": [0, 0.25, 0]},
    {"type": "cylinder", "radius": 0.55, "height": H - 1.1, "position": [0, H / 2, 0]},
    {"type": "box", "size": [1.5, 0.6, 1.5], "position": [0, H - 0.3, 0]}]}

# the sponge: where the light through the upper window (z -15) crosses the
# middle of the hall, its x axis (and so a set of tunnels) along the light
# (turned about x for the look, which keeps that)
HALF = 1.4
t = (0.3 + W) / -s[0]
centre = [-W, 9.3, -15.0]
centre = [centre[i] - s[i] * t for i in range(3)]
turn = [20.0, math.degrees(math.asin(s[2])), math.degrees(math.atan2(-s[1], -s[0]))]


def rotate(v, a):
    x, y, z = v
    ax, ay, az = (math.radians(d) for d in a)
    y, z = y * math.cos(ax) - z * math.sin(ax), y * math.sin(ax) + z * math.cos(ax)
    x, z = x * math.cos(ay) + z * math.sin(ay), -x * math.sin(ay) + z * math.cos(ay)
    x, y = x * math.cos(az) - y * math.sin(az), x * math.sin(az) + y * math.cos(az)
    return [x, y, z]


# (hung from the ceiling by its highest corner)
corners = [rotate([HALF * a, HALF * b, HALF * c], turn) for a in (-1, 1) for b in (-1, 1) for c in (-1, 1)]
top = max(corners, key=lambda p: p[1])
top = [centre[i] + top[i] for i in range(3)]

objects = [
    {"name": "floor", "type": "plane", "material": "marble", "locked": True},
    {"name": "window wall", "type": "difference", "position": [-W - 0.5, 0, MID], "material": "limestone", "locked": True, "children": [
        {"type": "box", "size": [1, H, LONG + 2], "position": [0, H / 2, 0]},
        {"type": "repeat", "period": [0, 0, 6], "count": [0, 0, 2], "position": [0, 0, -15 - MID], "child": window}]},
    {"name": "tracery", "type": "repeat", "period": [0, 0, 6], "count": [0, 0, 2], "position": [-W - 0.5, 0, -15], "material": "iron", "locked": True, "child": tracery},
    {"name": "wall", "type": "box", "size": [1, H, LONG + 2], "position": [W + 0.5, H / 2, MID], "material": "limestone", "locked": True},
    {"name": "back wall", "type": "box", "size": [2 * W + 2, H, 1], "position": [0, H / 2, BACK - 0.5], "material": "limestone", "locked": True},
    {"name": "front wall", "type": "box", "size": [2 * W + 2, H, 1], "position": [0, H / 2, FRONT + 0.5], "material": "limestone", "locked": True},
    {"name": "ceiling", "type": "box", "size": [2 * W + 2, 1, LONG + 2], "position": [0, H + 0.5, MID], "material": "limestone", "locked": True},
    {"name": "columns", "type": "mirror", "axes": "x", "position": [0, 0, -18], "material": "column stone", "locked": True, "child":
        {"type": "repeat", "period": [0, 0, 6], "count": [0, 0, 2], "position": [4.5, 0, 0], "child": column}},
    {"name": "sponge", "type": "menger", "iterations": 4, "scale": HALF, "position": r(centre, 3), "rotation": r(turn, 2), "material": "bronze", "locked": True},
    {"name": "rod", "type": "capsule", "a": r(top, 3), "b": r([top[0], H, top[2]], 3), "radius": 0.04, "material": "iron", "locked": True},
]

write('sunbeams', {
    "description": "Volumetric light: a stone hall full of hazy air, the low sun falling through its tall windows. The haze is lit only where the sunlight reaches it, so each window's light shows as a beam, crossed by the shadows of its tracery and the columns. The Menger sponge hanging in one of the beams is turned so that its tunnels run along the light, which goes on through them as a lattice of small beams. Environment > Haze sets the air's density; the Render panel's haze strength, detail and contrast change how it looks.",
    "camera": {"position": [4.0, 1.6, -2.5], "look_at": [-1.5, 5.5, -12], "fov": 70, "speed": 3},
    "environment": {"preset": "golden", "sun_direction": SUN, "ambient_level": 0.2, "fog_density": 0, "haze_density": 0.1, "haze_top": H},
    "render": {"haze_detail": 4, "haze_contrast": 1.6},
    "materials": {
        "limestone": {"color": [0.7, 0.64, 0.55], "color2": [0.6, 0.55, 0.47], "texture": "bricks", "texture_scale": 0.9, "roughness": 0.9, "bump_pattern": "bricks", "bump": 0.6, "space": "world"},
        "column stone": {"color": [0.74, 0.69, 0.6], "color2": [0.62, 0.57, 0.5], "texture": "noise", "texture_scale": 2, "roughness": 0.8},
        "marble": {"color": [0.78, 0.75, 0.69], "color2": [0.2, 0.19, 0.18], "texture": "checker", "texture_scale": 1, "roughness": 0.3, "space": "world"},
        "iron": {"color": [0.06, 0.06, 0.06], "metallic": 0.8, "roughness": 0.5},
        "bronze": {"color": [0.8, 0.55, 0.3], "metallic": 1, "roughness": 0.35},
    },
    "objects": objects,
})
