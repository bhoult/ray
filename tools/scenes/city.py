"""Writes night-city.json: a city at the blue hour, one group per block.

python3 city.py [out-dir] [seed]   (default: saves, seed 7)
"""
import json, math, random, sys
OUT = sys.argv[1] if len(sys.argv) > 1 else 'saves'
R = random.Random(int(sys.argv[2]) if len(sys.argv) > 2 else 7)


def write(name, d):
    lines = ['{']
    for k in [k for k in d if k != 'objects']:
        if k == 'materials':
            lines.append('  "materials": {')
            ms = list(d[k].items())
            for i, (mk, mv) in enumerate(ms):
                lines.append(f'    {json.dumps(mk)}: {json.dumps(mv)}' + (',' if i < len(ms) - 1 else ''))
            lines.append('  },')
        else:
            lines.append(f'  {json.dumps(k)}: {json.dumps(d[k])},')
    lines.append('  "objects": [')
    for i, o in enumerate(d['objects']):
        lines.append('    ' + json.dumps(o) + (',' if i < len(d['objects']) - 1 else ''))
    lines.append('  ]')
    lines.append('}')
    open(f'{OUT}/{name}.json', 'w').write('\n'.join(lines) + '\n')


def r(v, n=2):
    return [round(x, n) for x in v] if isinstance(v, (list, tuple)) else round(v, n)


def box(size, pos, mat=None, **kw):
    o = {"type": "box", "size": r(size), "position": r(pos)}
    if mat:
        o["material"] = mat
    o.update(kw)
    return o


# ---- the street grid: avenues run along z, streets along x
AVES = [-130, -65, 0, 65, 130]        # avenue centre lines (x)
AVE_W = {0: 22}                        # road widths (the main avenue is wider)
STREETS = [150, 95, 40, -15, -70, -125, -180, -235, -290, -345]  # street centre lines (z)
STREET_W = 13
WALK = 4.0                             # sidewalk width
CELL = 2.0                             # facade window bay (texture scale 0.5)
STOREY = 2.8                           # 1.4 / 0.5


def ave_w(x):
    return AVE_W.get(x, 15)


def snap(v, s=CELL):
    return round(v / s) * s


# the tallest towers stand downtown, around here
DOWNTOWN = (0.0, -200.0)


def tall(x, z):
    d2 = ((x - DOWNTOWN[0]) / 120) ** 2 + ((z - DOWNTOWN[1]) / 150) ** 2
    return math.exp(-d2)


FACADES = ["office", "glass", "apartment", "concrete", "brick"]


def building(x0, x1, z0, z1, base, block_tall):
    """One building on the lot [x0,x1] x [z0,z1] (world): its parts."""
    w, d = x1 - x0, z1 - z0
    cx, cz = (x0 + x1) / 2, (z0 + z1) / 2
    k = block_tall
    h = 10 + R.random() * 16 + k * (40 + R.random() * 130) * (R.random() ** 0.6)
    h = max(STOREY * 3, round(h / STOREY) * STOREY)
    if k > 0.5 and h > 60:
        fac = R.choice(["glass", "office", "glass", "concrete"])
    elif h < 30:
        fac = R.choice(["apartment", "brick", "concrete", "apartment"])
    else:
        fac = R.choice(FACADES)
    parts = []
    # setbacks: tall ones step in as they rise
    tiers = 1 if h < 45 else (2 if h < 110 else 3)
    y = base
    tw, td = w, d
    heights = [h] if tiers == 1 else ([h * 0.55, h * 0.45] if tiers == 2 else [h * 0.45, h * 0.33, h * 0.22])
    for i, th in enumerate(heights):
        th = max(STOREY * 2, round(th / STOREY) * STOREY)
        parts.append(box([tw, th, td], [cx, y + th / 2, cz], fac))
        y += th
        tw, td = max(snap(tw * 0.72), 6), max(snap(td * 0.72), 6)
    top = y
    # roof: a parapet slab, plant boxes, a water tank on the low ones, a spire
    # with a red beacon on the tallest
    parts.append(box([min(w, tw / 0.72) * 0.96, 0.6, min(d, td / 0.72) * 0.96], [cx, top + 0.3, cz], "roof"))
    if h < 40 and R.random() < 0.55:
        tx, tz = cx + R.uniform(-w, w) * 0.25, cz + R.uniform(-d, d) * 0.25
        parts.append({"type": "cylinder", "radius": 1.4, "height": 3.2, "position": r([tx, top + 2.2, tz]), "material": "tank"})
        parts.append({"type": "cone", "radius": 1.5, "top_radius": 0.1, "height": 1.2, "position": r([tx, top + 4.4, tz]), "material": "tank"})
    elif R.random() < 0.7:
        parts.append(box([R.uniform(3, 6), 2.2, R.uniform(3, 5)], [cx + R.uniform(-1, 1) * w * 0.2, top + 1.1, cz + R.uniform(-1, 1) * d * 0.2], "roof"))
    if h > 120:
        sh = 12 + h * 0.12
        parts.append({"type": "cylinder", "radius": 0.35, "height": sh, "position": r([cx, top + sh / 2, cz]), "material": "steel"})
        parts.append({"type": "sphere", "radius": 0.6, "position": r([cx, top + sh + 0.4, cz]), "material": "beacon"})
    return parts, h


NEON = [[4, 0.3, 2.2], [0.3, 2.6, 4], [3.6, 0.5, 0.2], [0.5, 3.4, 1.2], [3.2, 2.4, 0.2], [3.8, 1.2, 0.3]]


def block(x0, x1, z0, z1, name, avenue=0):
    """A city block between the kerbs [x0,x1] x [z0,z1]: the sidewalk slab and
    its buildings, as one group (positions relative to its centre)."""
    cx, cz = (x0 + x1) / 2, (z0 + z1) / 2
    kids = [box([x1 - x0, 0.18, z1 - z0], [0, 0.09, 0], "pavement")]
    ix0, ix1, iz0, iz1 = x0 + WALK, x1 - WALK, z0 + WALK, z1 - WALK
    k = tall(cx, cz)
    # lots: split the block into 2-3 along x and 1-2 along z
    nx = R.choice([2, 2, 3]) if ix1 - ix0 > 36 else 1 + (ix1 - ix0 > 22)
    nz = R.choice([1, 2, 2]) if iz1 - iz0 > 28 else 1
    xs = [ix0] + sorted(snap(ix0 + (ix1 - ix0) * (i + R.uniform(-0.12, 0.12)) / nx) for i in range(1, nx)) + [ix1]
    zs = [iz0] + sorted(snap(iz0 + (iz1 - iz0) * (j + R.uniform(-0.12, 0.12)) / nz) for j in range(1, nz)) + [iz1]
    tallest = 0
    for i in range(nx):
        for j in range(nz):
            gap = 0.8
            parts, h = building(xs[i] + gap, xs[i + 1] - gap, zs[j] + gap, zs[j + 1] - gap, 0.18, k)
            tallest = max(tallest, h)
            # shop signs on the fronts that face the main avenue
            front = (avenue > 0 and i == nx - 1) or (avenue < 0 and i == 0)
            if front:
                fx = (xs[i + 1] - gap if avenue > 0 else xs[i] + gap) + avenue * 0.12
                zl = zs[j + 1] - zs[j] - 2 * gap
                col = R.choice(NEON)
                parts.append({"type": "box", "size": r([0.2, 0.8, zl * R.uniform(0.4, 0.7)]), "position": r([fx, 3.9, (zs[j] + zs[j + 1]) / 2]), "material": {"color": [0.9, 0.9, 0.9], "emission": col, "light_range": 4}})
                if R.random() < 0.5:
                    col = R.choice(NEON)
                    parts.append({"type": "box", "size": r([1.0, R.uniform(4, 7), 0.25]), "position": r([fx + avenue * 0.5, R.uniform(8, 11), zs[j] + gap + 2.0]), "material": {"color": [0.9, 0.9, 0.9], "emission": col, "light_range": 4}})
            for p in parts:
                p["position"] = r([p["position"][0] - cx, p["position"][1], p["position"][2] - cz])
            kids += parts
    return {"name": name, "type": "group", "position": r([cx, 0, cz]), "locked": True, "children": kids}


def park(x0, x1, z0, z1):
    cx, cz = (x0 + x1) / 2, (z0 + z1) / 2
    w, d = x1 - x0, z1 - z0
    kids = [box([w, 0.18, d], [0, 0.09, 0], "pavement"),
            box([w - 2 * WALK, 0.3, d - 2 * WALK], [0, 0.15, 0], "grass")]
    # paths crossing it, a pond with a ring of stone in the middle
    kids.append(box([w - 2 * WALK, 0.32, 3], [0, 0.16, 0], "path"))
    kids.append(box([3, 0.32, d - 2 * WALK], [0, 0.16, 0], "path"))
    kids.append({"type": "cylinder", "radius": 7.5, "height": 0.6, "position": [0, 0.3, 0], "material": "stone"})
    kids.append({"type": "cylinder", "radius": 6.8, "height": 0.1, "position": [0, 0.58, 0], "material": "pool"})
    trees = 0
    for _ in range(80):
        if trees >= 26:
            break
        tx, tz = R.uniform(-w / 2 + WALK + 2, w / 2 - WALK - 2), R.uniform(-d / 2 + WALK + 2, d / 2 - WALK - 2)
        if abs(tx) < 3.5 or abs(tz) < 3.5 or math.hypot(tx, tz) < 10:
            continue
        s = R.uniform(0.8, 1.3)
        kids.append({"type": "union", "position": r([tx, 0.3, tz]), "scale": r(s), "children": [
            {"type": "cylinder", "radius": 0.25, "height": 3.0, "position": [0, 1.5, 0], "material": "bark"},
            {"type": "ellipsoid", "radii": r([2.2, 2.6, 2.2]), "position": [0, 4.4, 0], "material": "leaves"},
        ]})
        trees += 1
    # lamps along the paths
    for (lx, lz) in [(-12, 2.6), (12, -2.6), (2.6, 13), (-2.6, -13)]:
        kids.append({"type": "union", "position": [lx, 0.3, lz], "children": [
            {"type": "cylinder", "radius": 0.06, "height": 3.2, "position": [0, 1.6, 0], "material": "iron"},
            {"type": "sphere", "radius": 0.24, "position": [0, 3.35, 0], "material": "globe"},
        ]})
    return {"name": "park", "type": "group", "position": r([cx, 0, cz]), "locked": True, "children": kids}


def lamp(x, z, side):
    """A street lamp at the kerb: a pole and an arm over the road (side = +1:
    the arm reaches toward +x)."""
    return {"name": "street lamp", "type": "union", "position": r([x, 0.18, z]), "locked": True, "children": [
        {"type": "cylinder", "radius": 0.11, "height": 8.0, "position": [0, 4.0, 0], "material": "pole"},
        box([2.4, 0.12, 0.16], [side * 1.1, 7.9, 0], "pole"),
        box([0.9, 0.18, 0.4], [side * 2.2, 7.82, 0], "pole"),
        box([0.7, 0.06, 0.3], [side * 2.2, 7.7, 0], "sodium"),
    ]}


def car(x, z, heading, paint):
    """A car (one union, so it can be carried and thrown) facing `heading`
    degrees about y (0: toward +z)."""
    parts = [
        {"type": "box", "size": [1.8, 0.6, 4.3], "rounding": 0.14, "position": [0, 0.65, 0], "material": paint},
        {"type": "box", "size": [1.55, 0.52, 2.2], "rounding": 0.2, "position": [0, 1.15, -0.25], "material": "carglass"},
        {"type": "sphere", "radius": 0.13, "position": [0.62, 0.75, 2.12], "material": "headlight"},
        {"type": "sphere", "radius": 0.13, "position": [-0.62, 0.75, 2.12], "material": "headlight"},
        {"type": "box", "size": [0.4, 0.1, 0.06], "position": [0.62, 0.8, -2.16], "material": "taillight"},
        {"type": "box", "size": [0.4, 0.1, 0.06], "position": [-0.62, 0.8, -2.16], "material": "taillight"},
    ] + [{"type": "cylinder", "radius": 0.36, "height": 0.26, "position": [sx * 0.86, 0.36, sz], "rotation": [0, 0, 90], "material": "tyre"}
         for sx in (-1, 1) for sz in (-1.4, 1.45)]
    return {"name": "car", "type": "union", "position": r([x, 0.0, z]), "rotation": [0, heading, 0], "children": parts}


objects = []
objects.append({"name": "ground", "type": "plane", "locked": True, "material": "asphalt"})

# the road markings of the main avenue: a double yellow line and the lane edges
zr = (STREETS[-1] - 30, STREETS[0] + 60)
mark = []
for zs, ze in zip([zr[0]] + [s + STREET_W / 2 for s in STREETS[::-1]], [s - STREET_W / 2 for s in STREETS[::-1]] + [zr[1]]):
    if ze - zs < 1:
        continue
    zc, L = (zs + ze) / 2, ze - zs
    mark.append(box([0.15, 0.02, L], [-0.2, 0.01, zc], "yellow line"))
    mark.append(box([0.15, 0.02, L], [0.2, 0.01, zc], "yellow line"))
    # dashed lane dividers
    n = int(L // 9)
    for i in range(n):
        z = zs + 4.5 + i * 9
        for x in (-5.4, 5.4):
            mark.append(box([0.14, 0.02, 3.0], [x, 0.01, z], "white line"))
# crosswalks over the main avenue at each street
for s in STREETS:
    for zz in (s - STREET_W / 2 - 1.8, s + STREET_W / 2 + 1.8):
        mark.append({"type": "box", "size": [0.02, AVE_W[0] - 1, 2.8], "position": r([0, 0.01, zz]), "rotation": [0, 0, 90], "material": "crossing"})
objects.append({"name": "road markings", "type": "group", "locked": True, "children": mark})

# blocks between the avenues and streets
ax = sorted(AVES)
zs_ = sorted(STREETS)
for i in range(len(ax) - 1):
    for j in range(len(zs_) - 1):
        x0 = ax[i] + ave_w(ax[i]) / 2
        x1 = ax[i + 1] - ave_w(ax[i + 1]) / 2
        z0 = zs_[j] + STREET_W / 2
        z1 = zs_[j + 1] - STREET_W / 2
        name = f"block {chr(65 + i)}{len(zs_) - 1 - j}"
        if (i, j) == (2, len(zs_) - 3):
            objects.append(park(x0, x1, z0, z1))
        else:
            objects.append(block(x0, x1, z0, z1, name, 1 if ax[i + 1] == 0 else (-1 if ax[i] == 0 else 0)))
# the half blocks beyond the outer avenues
for x0, x1 in [(-190, ax[0] - ave_w(ax[0]) / 2), (ax[-1] + ave_w(ax[-1]) / 2, 190)]:
    for j in range(len(zs_) - 1):
        z0, z1 = zs_[j] + STREET_W / 2, zs_[j + 1] - STREET_W / 2
        objects.append(block(x0, x1, z0, z1, f"block {'W' if x0 < 0 else 'E'}{len(zs_) - 1 - j}"))

# street lamps along the main avenue, the nearest first (the 64 brightest
# glowing parts light the scene)
lamps = []
half = AVE_W[0] / 2 + 0.6
for z in range(60, -330, -26):
    if any(abs(z - s) < STREET_W / 2 + 3 for s in STREETS):
        z -= 9
    lamps.append(lamp(-half, z, 1))
    lamps.append(lamp(half, z - 13, -1))
objects.append({"name": "avenue lamps", "type": "group", "locked": True, "children": lamps})

# traffic: coming toward the camera on the left, going away on the right
paints = ["red paint", "blue paint", "white paint", "black paint", "yellow paint", "silver paint"]
z = 48
for i in range(9):
    objects.append(car(R.choice([-8.0, -2.7]), z, 0, R.choice(paints)))
    z -= R.uniform(14, 34)
z = 30
for i in range(9):
    objects.append(car(R.choice([2.7, 8.0]), z, 180, R.choice(paints)))
    z -= R.uniform(14, 34)
# a few on the cross streets
for s in STREETS[1:5]:
    objects.append(car(R.uniform(-60, -20), s + R.choice([-3, 3]), 90 if R.random() < 0.5 else -90, R.choice(paints)))
    objects.append(car(R.uniform(20, 60), s + R.choice([-3, 3]), 90 if R.random() < 0.5 else -90, R.choice(paints)))

# the far city: tall dark shapes to the horizon
far = []
for k in range(140):
    a = R.uniform(0, math.tau)
    dist = R.uniform(420, 900)
    x, z = math.sin(a) * dist, -abs(math.cos(a)) * dist - 150 if R.random() < 0.8 else math.cos(a) * dist
    if abs(x) < 200 and z > -420:
        continue
    w, d = snap(R.uniform(16, 40)), snap(R.uniform(16, 40))
    h = round((20 + R.random() ** 2 * 160 * (1.4 - dist / 900)) / STOREY) * STOREY
    far.append(box([w, h, d], [x, h / 2, z], R.choice(["office", "glass", "concrete", "apartment"])))
objects.append({"name": "far city", "type": "group", "locked": True, "children": far})

write('night-city', {
    "description": "A city at the blue hour: every block is a group of buildings (select one by clicking, or a whole block in the outliner), the windows pattern lights a share of each tower's windows, sodium lamps line a wet avenue, a park with a pond, and traffic you can pick up and throw. Slide the time of day for dawn or full night.",
    "camera": {"position": [7.5, 16, 92], "look_at": [-3, 26, -120], "fov": 55, "speed": 20},
    "environment": {"preset": "golden", "time_of_day": 18.1, "sun_azimuth": 215, "clouds": 0.25, "fog_density": 0.0022},
    "render": {"bounces": 3},
    "materials": {
        "asphalt": {"color": [0.05, 0.05, 0.055], "color2": [0.025, 0.025, 0.03], "texture": "noise", "texture_scale": 0.06, "sharpness": 4, "roughness": 0.5, "roughness2": 0.04, "space": "world"},
        "pavement": {"color": [0.3, 0.29, 0.28], "color2": [0.18, 0.18, 0.17], "texture": "grid", "texture_scale": 0.5, "roughness": 0.6, "space": "world"},
        "office": {"color": [0.07, 0.08, 0.1], "metallic": 0.4, "roughness": 0.3, "color2": [0.75, 0.86, 1.0], "emission2": [1.1, 1.35, 1.7], "roughness2": 0.2, "texture": "windows", "texture_scale": 0.5, "space": "world"},
        "glass": {"color": [0.05, 0.07, 0.09], "metallic": 0.7, "roughness": 0.12, "color2": [0.9, 0.92, 0.85], "emission2": [1.3, 1.3, 1.1], "roughness2": 0.15, "texture": "windows", "texture_scale": 0.5, "sharpness": 1.5, "space": "world"},
        "apartment": {"color": [0.36, 0.25, 0.19], "roughness": 0.85, "color2": [1.0, 0.78, 0.5], "emission2": [1.9, 1.2, 0.5], "roughness2": 0.3, "texture": "windows", "texture_scale": 0.5, "space": "world"},
        "concrete": {"color": [0.42, 0.41, 0.39], "roughness": 0.8, "color2": [1.0, 0.92, 0.75], "emission2": [1.6, 1.4, 0.95], "roughness2": 0.3, "texture": "windows", "texture_scale": 0.5, "space": "world"},
        "brick": {"color": [0.42, 0.17, 0.12], "roughness": 0.9, "color2": [1.0, 0.82, 0.55], "emission2": [1.8, 1.3, 0.7], "roughness2": 0.3, "texture": "windows", "texture_scale": 0.5, "space": "world"},
        "roof": {"color": [0.16, 0.16, 0.17], "roughness": 0.85},
        "tank": {"color": [0.32, 0.22, 0.14], "color2": [0.2, 0.13, 0.08], "texture": "stripes", "texture_scale": 2.5, "roughness": 0.9},
        "steel": {"color": [0.6, 0.6, 0.62], "metallic": 1, "roughness": 0.35},
        "beacon": {"color": [1, 0.1, 0.05], "emission": [30, 2, 1], "light_range": 0.5},
        "pole": {"color": [0.12, 0.12, 0.13], "metallic": 0.8, "roughness": 0.45},
        "sodium": {"color": [1, 0.62, 0.3], "emission": [120, 58, 16], "light_range": 26},
        "yellow line": {"color": [0.8, 0.6, 0.1], "roughness": 0.5},
        "white line": {"color": [0.8, 0.8, 0.78], "roughness": 0.5},
        "crossing": {"color": [0.05, 0.05, 0.055], "color2": [0.8, 0.8, 0.78], "texture": "stripes", "texture_scale": 0.8, "sharpness": 3, "roughness": 0.35},
        "grass": {"color": [0.08, 0.16, 0.05], "color2": [0.05, 0.1, 0.03], "texture": "noise", "texture_scale": 0.4, "roughness": 0.95, "space": "world"},
        "path": {"color": [0.36, 0.32, 0.26], "roughness": 0.9},
        "stone": {"color": [0.5, 0.48, 0.44], "roughness": 0.7},
        "pool": {"color": [0.02, 0.06, 0.08], "roughness": 0.02, "metallic": 0.2},
        "bark": {"color": [0.15, 0.1, 0.07], "roughness": 0.95},
        "leaves": {"color": [0.07, 0.17, 0.05], "color2": [0.04, 0.09, 0.03], "texture": "cells", "texture_scale": 2.0, "roughness": 0.9, "bump_pattern": "cells", "bump": 2.0},
        "iron": {"color": [0.08, 0.08, 0.08], "metallic": 1, "roughness": 0.5},
        "globe": {"color": [1, 0.95, 0.85], "emission": [14, 12.5, 10], "light_range": 9},
        "red paint": {"color": [0.6, 0.04, 0.03], "metallic": 0.5, "roughness": 0.25},
        "blue paint": {"color": [0.05, 0.14, 0.5], "metallic": 0.5, "roughness": 0.25},
        "white paint": {"color": [0.8, 0.8, 0.8], "metallic": 0.3, "roughness": 0.3},
        "black paint": {"color": [0.02, 0.02, 0.02], "metallic": 0.5, "roughness": 0.2},
        "yellow paint": {"color": [0.85, 0.6, 0.05], "metallic": 0.3, "roughness": 0.3},
        "silver paint": {"color": [0.55, 0.56, 0.58], "metallic": 0.9, "roughness": 0.3},
        "carglass": {"color": [0.03, 0.04, 0.05], "metallic": 0.5, "roughness": 0.05},
        "headlight": {"color": [1, 0.95, 0.85], "emission": [40, 38, 32], "light_range": 9},
        "taillight": {"color": [1, 0.05, 0.03], "emission": [18, 0.8, 0.4], "light_range": 2},
        "tyre": {"color": [0.03, 0.03, 0.03], "roughness": 0.9},
    },
    "objects": objects,
})
print(len(objects), 'top-level objects,', sum(len(o.get('children', [])) if o['type'] == 'group' else 1 for o in objects), 'members')
