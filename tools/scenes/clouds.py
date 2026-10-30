"""Writes saves/clouds.json: balloons over a valley and its lake, cumulus clouds, a smoking steamer."""
import math
from common import write, r


def balloon(name, pos, scale, rot, paint):
    """A hot-air balloon: a NURBS envelope (revolve), a wicker basket, ropes and
    the burner's flame, as one group (a revolve cannot go in a union)."""
    kids = [
        {"name": "envelope", "type": "revolve", "position": [0, 2.6, 0], "material": paint,
         "profile": [[0, 0], [0.55, 0], [0.7, 0.4], [1.9, 2.0], [3.7, 4.2], [4.4, 6.2], [4.1, 8.0], [2.6, 9.6], [0, 10.0]]},
        {"name": "basket", "type": "box", "size": [1.3, 1.0, 1.3], "rounding": 0.06, "position": [0, 0.5, 0], "material": "wicker"},
        {"name": "flame", "type": "cone", "radius": 0.18, "top_radius": 0.02, "height": 0.55, "position": [0, 2.35, 0], "material": "flame"},
    ]
    a = math.degrees(math.atan2(0.15, 1.6))
    for sx, sz in [(-1, -1), (-1, 1), (1, -1), (1, 1)]:
        kids.append({"name": "rope", "type": "cylinder", "radius": 0.025, "height": 1.75, "position": [sx * 0.55, 1.85, sz * 0.55], "rotation": r([sz * a, 0, -sx * a], 1), "material": "rope"})
    # (a NURBS envelope has no physics body: the balloon stays where it is put)
    return {"name": name, "type": "group", "position": r(pos), "rotation": [0, rot, 0], "scale": scale, "locked": True, "children": kids}


def steamer(name, x, z, heading, level):
    """A paddle steamer afloat at `level`: hull, deck house, funnel and two
    paddle wheels, one group; and where the top of its funnel is."""
    kids = [
        {"name": "hull", "type": "box", "size": [3.2, 1.4, 11], "rounding": 0.5, "position": [0, 0.2, 0], "material": "hull"},
        {"name": "bow", "type": "cylinder", "radius": 1.6, "height": 1.4, "position": [0, 0.2, 5.0], "material": "hull"},
        {"name": "deck", "type": "box", "size": [3.3, 0.12, 11.2], "position": [0, 0.95, 0], "material": "deck"},
        {"name": "deck house", "type": "box", "size": [2.4, 1.6, 4.5], "rounding": 0.08, "position": [0, 1.8, -1.0], "material": "whitewash"},
        {"name": "windows", "type": "box", "size": [2.45, 0.45, 4.0], "position": [0, 2.0, -1.0], "material": "window"},
        {"name": "roof", "type": "box", "size": [2.8, 0.1, 5.0], "position": [0, 2.65, -1.0], "material": "deck"},
        {"name": "funnel", "type": "cylinder", "radius": 0.42, "height": 3.2, "position": [0, 3.6, 0.6], "material": "funnel"},
        {"name": "funnel top", "type": "cylinder", "radius": 0.46, "height": 0.5, "position": [0, 5.25, 0.6], "material": "soot"},
    ]
    for sx in (-1, 1):
        kids.append({"name": "paddle wheel", "type": "torus", "radius": 1.25, "thickness": 0.1, "position": [sx * 1.85, 0.8, -0.5], "rotation": [0, 0, 90], "material": "paddle"})
        kids.append({"name": "wheel box", "type": "box", "size": [0.5, 1.0, 2.8], "rounding": 0.2, "position": [sx * 1.85, 1.4, -0.5], "material": "funnel"})
    a = math.radians(heading)
    top = [x + 0.6 * math.sin(a), level + 5.55, z + 0.6 * math.cos(a)]
    return {"name": name, "type": "group", "position": r([x, level, z]), "rotation": [0, heading, 0], "locked": True, "children": kids}, top


LEVEL = -0.8
boat, top = steamer("paddle steamer", -7.0, 8.0, 65, LEVEL)
smoke = {"name": "funnel smoke", "type": "emitter", "position": r(top), "rotation": [0, 0, 18], "visible": False,
         "rate": 1.2, "lifespan": 10, "grow": 7, "fade": 3, "speed": 1.4, "spread": 10, "physics": False, "max": 14,
         "emit": {"type": "cloud", "scale": 0.24, "density": 3, "color": [0.5, 0.5, 0.52], "turbulence": 1.2, "softness": 1.5, "gravity": False}}

grass = {"color": [0.26, 0.42, 0.13], "color2": [0.17, 0.3, 0.09], "texture": "noise", "texture_scale": 0.15, "warp": 0.6, "roughness": 0.92, "space": "world"}
objects = [
    {"name": "hills", "type": "terrain", "noise": "perlin", "size": 600, "resolution": 512, "height": 14, "base": -2, "frequency": 0.008, "octaves": 5, "seed": 7, "locked": True, "material": grass},
    {"name": "lake", "type": "water", "level": LEVEL, "waves": 0.5},
    boat, smoke,
    balloon("red balloon", [4, 16, -6], 1.0, 20, "red stripes"),
    balloon("blue balloon", [-22, 24, -40], 1.15, -40, "blue stripes"),
    balloon("harlequin balloon", [30, 30, -70], 1.3, 70, "harlequin"),
]
for (p, s, rot) in [([-18, 22, -10], 1.5, 10), ([6, 30, -30], 1.95, -25), ([26, 20, 0], 1.27, 40), ([-30, 35, -55], 1.6, 70), ([40, 28, -45], 1.8, 5)]:
    objects.append({"name": "cloud", "type": "cloud", "position": p, "rotation": [0, rot, 0], "scale": s})

write('clouds', {
    "description": "Hot-air balloons drift among cumulus clouds over a valley and its lake. Each balloon is a group: a NURBS envelope, a wicker basket, ropes and the burner's flame. The paddle steamer's funnel smokes: a hidden emitter (in the object list) that sends out small clouds. Try the time of day slider.",
    "camera": {"position": [6, 9, 30], "look_at": [-2, 10, -6], "fov": 60, "speed": 8},
    "environment": {"preset": "golden", "time_of_day": 9.2, "sun_azimuth": 200, "sun_elevation": 55, "clouds": 0.12, "fog_density": 0.002},
    "materials": {
        "red stripes": {"color": [0.85, 0.12, 0.08], "color2": [0.95, 0.8, 0.2], "texture": "stripes", "texture_scale": 0.55, "sharpness": 4, "roughness": 0.6, "translucency": 0.3},
        "blue stripes": {"color": [0.1, 0.25, 0.7], "color2": [0.9, 0.9, 0.9], "texture": "stripes", "texture_scale": 0.4, "sharpness": 4, "roughness": 0.6, "translucency": 0.3},
        "harlequin": {"color": [0.95, 0.55, 0.1], "color2": [0.55, 0.1, 0.35], "texture": "checker", "texture_scale": 0.9, "roughness": 0.6, "translucency": 0.3},
        "wicker": {"color": [0.55, 0.4, 0.22], "color2": [0.2, 0.13, 0.06], "texture": "weave", "texture_scale": 6, "roughness": 0.85, "bump_pattern": "weave", "bump": 2},
        "rope": {"color": [0.6, 0.5, 0.35], "roughness": 0.9},
        "flame": {"color": [1, 0.7, 0.3], "emission": [12, 6, 1.5], "light_range": 4},
        "whitewash": {"color": [0.85, 0.82, 0.76], "roughness": 0.85},
        "hull": {"color": [0.12, 0.14, 0.2], "color2": [0.55, 0.08, 0.06], "texture": "stripes", "texture_scale": 0.35, "sharpness": 6, "roughness": 0.5},
        "deck": {"color": [0.5, 0.35, 0.2], "color2": [0.32, 0.2, 0.1], "texture": "wood", "texture_scale": 1.5, "roughness": 0.7},
        "funnel": {"color": [0.75, 0.1, 0.06], "roughness": 0.45},
        "soot": {"color": [0.03, 0.03, 0.03], "roughness": 0.8},
        "paddle": {"color": [0.85, 0.75, 0.3], "metallic": 0.6, "roughness": 0.4},
        "window": {"color": [0.08, 0.1, 0.12], "roughness": 0.05, "metallic": 0.3},
    },
    "objects": objects,
})
