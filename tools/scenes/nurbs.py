"""Writes saves/nurbs.json: a still life on a table, every curved thing a NURBS surface."""
import math
from common import write, r

H = round(math.sqrt(0.5), 6)


def ring(R, a):
    """A circle of radius a around (R, 0) as a rational quadratic profile (9 points)."""
    pts = []
    for k in range(9):
        ang = k * math.pi / 4
        rr = a if k % 2 == 0 else a / H
        pts.append([round(R + rr * math.cos(ang), 5), round(rr * math.sin(ang), 5)] + ([] if k % 2 == 0 else [H]))
    return pts


RING_KNOTS = [0, 0, 0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1, 1, 1]
TOP = 0.76  # the table top

# a cloth laid over the table, its front hanging over the edge in folds
cloth_pts = []
xs = [-0.55, -0.37, -0.18, 0.0, 0.18, 0.37, 0.55]
# (z, y) along the cloth: flat until past the table's front edge (z 0.475),
# then down; the folds only push the hanging part outward, so the curve (it
# is pulled toward its points, not through them) never cuts into the edge
# (the hanging part stays within one depth of the checker's cells, 0.5 to
# 0.57 m: crossing into the next draws arcs across it)
rows = [(-0.4, 0.0), (-0.1, 0.0), (0.25, 0.0), (0.46, 0.0), (0.51, 0.0), (0.52, -0.05), (0.525, -0.15), (0.53, -0.26)]
FOLD = [0, 0, 0, 0, 0, 0.01, 0.02, 0.03]
for i, x in enumerate(xs):
    col = []
    for j, (z, y) in enumerate(rows):
        fold = FOLD[j] * (0.5 + 0.5 * math.sin(i * 1.9))
        col.append(r([x, y, z + fold], 4))
    cloth_pts.append(col)

table = {"name": "table", "type": "group", "locked": True, "children": [
    {"name": "top", "type": "box", "size": [0.05, 1.8, 0.95], "rounding": 0.01, "position": [0, TOP - 0.025, 0], "rotation": [0, 0, 90], "material": "oak"},
] + [{"name": "leg", "type": "box", "size": [0.06, TOP - 0.05, 0.06], "position": r([sx * 0.82, (TOP - 0.05) / 2, sz * 0.4]), "material": "oak"} for sx in (-1, 1) for sz in (-1, 1)]}

room = {"name": "room", "type": "group", "locked": True, "children": [
    {"name": "back wall", "type": "difference", "position": [0, 1.6, -1.1], "material": "plaster", "children": [
        {"type": "box", "size": [8, 3.2, 0.15]},
        {"type": "box", "size": [1.3, 1.5, 0.4], "position": [-0.2, 0.25, 0]}]},
    {"name": "sill", "type": "box", "size": [1.45, 0.05, 0.3], "position": [-0.2, 1.1, -1.0], "material": "oak"},
    {"name": "mullion", "type": "box", "size": [0.04, 1.5, 0.05], "position": [-0.2, 1.85, -1.1], "material": "oak"},
    {"name": "transom", "type": "box", "size": [1.3, 0.04, 0.05], "position": [-0.2, 1.85, -1.1], "material": "oak"},
]}

objects = [
    {"name": "floor", "type": "plane", "locked": True, "material": "floorboards"},
    room, table,
    {"name": "tablecloth", "type": "nurbs", "position": [0.05, TOP + 0.004, 0], "degree": [3, 3], "points": cloth_pts, "locked": True, "material": "linen"},
    {"name": "vase", "type": "revolve", "position": [-0.6, TOP + 0.005, -0.22], "scale": 0.5, "material": "glaze"},
    {"name": "goblet", "type": "revolve", "position": [-0.15, TOP + 0.005, 0.1], "scale": 0.3, "material": "glass",
     "profile": [[0, 0], [0.3, 0], [0.32, 0.03], [0.05, 0.06], [0.04, 0.35], [0.05, 0.5], [0.3, 0.62], [0.34, 0.95], [0.32, 0.95], [0.28, 0.64], [0.05, 0.56], [0, 0.56]]},
    {"name": "bottle", "type": "revolve", "position": [0.12, TOP + 0.005, -0.18], "scale": 0.42, "material": "bottle glass",
     "profile": [[0, 0], [0.12, 0], [0.135, 0.02], [0.135, 0.42], [0.12, 0.52], [0.05, 0.62], [0.04, 0.78], [0.047, 0.84], [0.036, 0.85], [0.03, 0.62], [0.11, 0.51], [0.123, 0.42], [0.123, 0.03], [0, 0.03]]},
    {"name": "bowl", "type": "revolve", "position": [0.48, TOP + 0.005, 0.12], "scale": 0.5, "material": "clay",
     "profile": [[0, 0], [0.12, 0], [0.16, 0.01], [0.36, 0.12], [0.45, 0.28], [0.47, 0.31], [0.44, 0.31], [0.41, 0.27], [0.33, 0.15], [0.14, 0.05], [0, 0.05]]},
    {"name": "napkin ring", "type": "revolve", "position": [-0.42, TOP + 0.019, 0.28], "scale": 0.1, "material": "gold", "degree": 2, "profile": ring(0.42, 0.12), "knots": RING_KNOTS},
    {"name": "ornament", "type": "revolve", "position": [0.78, TOP + 0.06, -0.2], "scale": 0.13, "material": "chrome", "degree": 2,
     "profile": [[0, -0.45], [0.45, -0.45, H], [0.45, 0], [0.45, 0.45, H], [0, 0.45]], "knots": [0, 0, 0, 0.5, 0.5, 1, 1, 1]},
    {"name": "candle", "type": "union", "position": [-0.32, TOP + 0.005, -0.3], "locked": True, "children": [
        {"type": "cylinder", "radius": 0.035, "height": 0.2, "position": [0, 0.1, 0], "material": "wax"},
        {"type": "cylinder", "radius": 0.06, "height": 0.015, "position": [0, 0.008, 0], "material": "gold"},
        {"type": "cone", "radius": 0.012, "top_radius": 0.0, "height": 0.04, "position": [0, 0.225, 0], "material": "candle flame"}]},
]
# a lamp behind the painter: warm fill for the shade the window leaves
objects.append({"name": "room lamp", "type": "sphere", "radius": 0.12, "position": [1.3, 2.3, 1.9], "locked": True, "shadow": False, "material": "lamp"})
# fruit in and by the bowl
for (x, y, z, rad, m) in [(0.46, 0.09, 0.1, 0.045, "orange"), (0.52, 0.1, 0.17, 0.042, "apple"), (0.43, 0.1, 0.18, 0.04, "orange"), (0.68, 0.042, 0.32, 0.04, "apple")]:
    objects.append({"name": m, "type": "sphere", "radius": rad, "position": r([x, TOP + y, z], 3), "material": m})

write('nurbs', {
    "description": "A still life by a window, every curve a NURBS surface traced exactly (no mesh): a draped tablecloth (a free-form sheet), a glazed vase, a glass goblet and bottle lit from behind, a clay bowl of fruit, a gold napkin ring and a chrome ball (exact circles from weighted points), and a candle. The table and the room are groups. Carry the pieces, or Drop all.",
    "camera": {"position": [0.55, 1.22, 1.75], "look_at": [0.0, 0.86, -0.1], "fov": 45, "speed": 1.5},
    "environment": {"preset": "golden", "time_of_day": 16.3, "sun_azimuth": 268, "clouds": 0.25},
    "render": {"bounces": 4},
    "materials": {
        "floorboards": {"color": [0.45, 0.3, 0.17], "color2": [0.2, 0.12, 0.06], "texture": "bricks", "texture_scale": 2.5, "roughness": 0.4, "space": "world"},
        "plaster": {"color": [0.92, 0.82, 0.66], "color2": [0.84, 0.74, 0.58], "texture": "noise", "texture_scale": 2, "roughness": 0.9},
        "oak": {"color": [0.55, 0.38, 0.22], "color2": [0.36, 0.22, 0.11], "texture": "wood", "texture_scale": 3, "roughness": 0.45},
        "linen": {"color": [0.92, 0.9, 0.86], "color2": [0.7, 0.1, 0.08], "texture": "checker", "texture_scale": 14, "roughness": 0.85, "bump_pattern": "weave", "bump_scale": 60, "bump": 0.5, "translucency": 0.4},
        "glaze": {"color": [0.06, 0.3, 0.42], "color2": [0.02, 0.12, 0.2], "texture": "scales", "texture_scale": 14, "roughness": 0.1, "bump_pattern": "scales", "bump": 0.8},
        "clay": {"color": [0.72, 0.36, 0.2], "color2": [0.6, 0.28, 0.14], "texture": "ripples", "texture_scale": 20, "roughness": 0.55, "bump_pattern": "ripples", "bump": 0.6},
        "glass": {"color": [0.95, 0.97, 1.0], "roughness": 0, "transmission": 1, "ior": 1.5},
        "bottle glass": {"color": [0.35, 0.75, 0.45], "roughness": 0.02, "transmission": 1, "ior": 1.5, "absorption": 2},
        "chrome": {"color": [0.95, 0.95, 0.95], "metallic": 1, "roughness": 0.04},
        "gold": {"color": [1.0, 0.76, 0.33], "metallic": 1, "roughness": 0.18},
        "lamp": {"color": [1, 0.85, 0.65], "emission": [90, 66, 40], "light_range": 7},
        "wax": {"color": [0.95, 0.92, 0.85], "roughness": 0.4, "translucency": 0.8},
        "candle flame": {"color": [1, 0.75, 0.4], "emission": [40, 22, 7], "light_range": 2.5},
        "orange": {"color": [0.95, 0.45, 0.05], "roughness": 0.5, "bump_pattern": "spots", "bump_scale": 60, "bump": 0.5},
        "apple": {"color": [0.7, 0.08, 0.05], "color2": [0.85, 0.6, 0.1], "texture": "noise", "texture_scale": 15, "roughness": 0.3},
    },
    "objects": objects,
})
