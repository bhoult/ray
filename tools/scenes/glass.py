"""Writes saves/glass.json: a glass chess set mid-game at sunset."""
import math
from common import write, r

SQ = 0.3                     # a square
BOARD_Y = 0.78               # the board's top

# profiles: [radius, height] from the foot up, in metres (closed on the axis: solid)
FOOT = [[0, 0], [0.105, 0], [0.11, 0.02], [0.1, 0.035], [0.08, 0.045]]
PROFILES = {
    "pawn": FOOT + [[0.05, 0.07], [0.035, 0.14], [0.06, 0.15], [0.03, 0.165], [0.055, 0.19], [0.06, 0.225], [0.03, 0.255], [0, 0.26]],
    "rook": FOOT + [[0.06, 0.07], [0.055, 0.2], [0.075, 0.22], [0.075, 0.28], [0.06, 0.28], [0.055, 0.26], [0, 0.26]],
    "bishop": FOOT + [[0.05, 0.07], [0.035, 0.2], [0.06, 0.21], [0.03, 0.225], [0.055, 0.26], [0.045, 0.3], [0.015, 0.33], [0.02, 0.345], [0, 0.35]],
    "queen": FOOT + [[0.055, 0.07], [0.035, 0.24], [0.07, 0.255], [0.04, 0.27], [0.06, 0.31], [0.075, 0.34], [0.05, 0.345], [0.02, 0.36], [0.025, 0.375], [0, 0.38]],
    "king": FOOT + [[0.055, 0.07], [0.038, 0.27], [0.072, 0.285], [0.042, 0.3], [0.065, 0.36], [0.03, 0.37], [0.012, 0.38], [0.012, 0.43], [0, 0.43]],
}


def piece(kind, file, rank, mat):
    x, z = (file - 3.5) * SQ, (3.5 - rank) * SQ
    if kind == "knight":
        # a revolve can not be a horse: a base and a head of rounded boxes (a union)
        face = 0 if mat == "clear" else 180
        return {"name": f"{mat} knight", "type": "union", "position": r([x, BOARD_Y, z], 3), "rotation": [0, face, 0], "material": mat, "children": [
            {"type": "cylinder", "radius": 0.1, "height": 0.05, "position": [0, 0.025, 0], "rounding": 0.01},
            {"type": "box", "size": [0.07, 0.17, 0.11], "rounding": 0.03, "position": [0, 0.13, -0.01], "rotation": [-12, 0, 0]},
            {"type": "box", "size": [0.065, 0.075, 0.14], "rounding": 0.03, "position": [0, 0.22, 0.04], "rotation": [25, 0, 0]},
            {"type": "cone", "radius": 0.016, "top_radius": 0.0, "height": 0.04, "position": [0.018, 0.27, 0.0]},
            {"type": "cone", "radius": 0.016, "top_radius": 0.0, "height": 0.04, "position": [-0.018, 0.27, 0.0]}]}
    return {"name": f"{mat} {kind}", "type": "revolve", "position": r([x, BOARD_Y, z], 3), "profile": PROFILES[kind], "degree": 2, "material": mat}


# a game a dozen moves in (file 0-7 = a-h, rank 0-7 = 1-8)
clear = [("rook", 0, 0), ("king", 6, 0), ("rook", 5, 0), ("queen", 3, 2), ("bishop", 2, 3), ("knight", 5, 2),
         ("pawn", 0, 1), ("pawn", 1, 1), ("pawn", 2, 2), ("pawn", 4, 3), ("pawn", 5, 1), ("pawn", 6, 1), ("pawn", 7, 2)]
smoked = [("rook", 0, 7), ("king", 6, 7), ("rook", 4, 7), ("queen", 3, 6), ("bishop", 6, 6), ("knight", 2, 5),
          ("pawn", 0, 5), ("pawn", 1, 6), ("pawn", 2, 6), ("pawn", 3, 4), ("pawn", 5, 6), ("pawn", 6, 5), ("pawn", 7, 6)]
taken = [("bishop", "smoked", [1.55, 0.0]), ("pawn", "clear", [-1.55, 0.25]), ("knight", "smoked", [1.62, 0.35])]

objects = [
    {"name": "floor", "type": "plane", "locked": True, "material": "floor"},
    {"name": "table", "type": "group", "locked": True, "children": [
        {"name": "top", "type": "cylinder", "radius": 1.9, "height": 0.06, "position": [0, BOARD_Y - 0.055, 0], "material": "walnut"},
        {"name": "stem", "type": "cylinder", "radius": 0.12, "height": BOARD_Y - 0.1, "position": [0, (BOARD_Y - 0.1) / 2, 0], "material": "brass"},
        {"name": "foot", "type": "cylinder", "radius": 0.6, "height": 0.04, "position": [0, 0.02, 0], "material": "brass"}]},
    {"name": "board", "type": "group", "locked": True, "children": [
        {"name": "squares", "type": "box", "size": [8 * SQ, 0.03, 8 * SQ], "position": [0, BOARD_Y - 0.015, 0], "material": "board"},
        {"name": "frame", "type": "difference", "position": [0, BOARD_Y - 0.02, 0], "material": "walnut", "children": [
            {"type": "box", "size": [8 * SQ + 0.16, 0.05, 8 * SQ + 0.16], "rounding": 0.01},
            {"type": "box", "size": [8 * SQ, 0.2, 8 * SQ]}]}]},
]
for (k, f, rk) in clear:
    objects.append(piece(k, f, rk, "clear"))
for (k, f, rk) in smoked:
    objects.append(piece(k, f, rk, "smoked"))
for (k, m, (x, z)) in taken:
    p = piece(k, 0, 0, m)
    p["position"] = r([x, BOARD_Y - 0.025, z], 3)
    p["name"] = f"taken {m} {k}"
    if k == "bishop":
        p["rotation"] = [0, 30, 90]
        p["position"][1] = round(BOARD_Y + 0.087, 3)
    objects.append(p)

write('glass', {
    "description": "A glass chess set mid-game on a walnut table at sunset: the pieces are NURBS surfaces of revolution in clear and smoked glass (the knights, which no lathe can make, are unions of rounded boxes); the low sun throws their refracted light and long shadows across the board. Try path tracing (2) for the light through the glass.",
    "camera": {"position": [1.7, 1.75, 2.4], "look_at": [-0.05, 0.86, -0.05], "fov": 42, "speed": 1.5},
    "environment": {"preset": "golden", "time_of_day": 17.6, "sun_azimuth": 200, "clouds": 0.3, "fog_density": 0.003},
    "render": {"bounces": 6},
    "materials": {
        "clear": {"color": [0.97, 0.98, 1.0], "transmission": 1, "ior": 1.52, "roughness": 0},
        "smoked": {"color": [0.42, 0.22, 0.1], "transmission": 1, "ior": 1.55, "absorption": 28, "roughness": 0.02},
        "board": {"color": [0.86, 0.84, 0.8], "color2": [0.08, 0.08, 0.09], "texture": "checker", "texture_scale": round(1 / SQ, 4), "roughness": 0.08, "roughness2": 0.05, "space": "world"},
        "walnut": {"color": [0.32, 0.18, 0.09], "color2": [0.18, 0.09, 0.04], "texture": "wood", "texture_scale": 2.5, "roughness": 0.25},
        "brass": {"color": [0.85, 0.65, 0.3], "metallic": 1, "roughness": 0.3},
        "floor": {"color": [0.5, 0.48, 0.45], "color2": [0.38, 0.36, 0.33], "texture": "hexagons", "texture_scale": 2, "roughness": 0.5, "space": "world"},
    },
    "objects": objects,
})
