"""Writes saves/bonsai.json: a glass bonsai on a rock in a lake at sunset.

Every part of the tree (trunk, roots, branches, twigs and a dead jin) is a
NURBS tube: around it the exact rational circle (as revolve makes it), along
it a cubic through rings that follow the branch's curve (parallel transport,
so a ring never spins about the branch) and close to a point at both ends, so
each part is a solid that refracts. The trunk's rings are ovals that turn as
it rises: a twisted trunk. The foliage pads are small leaves of translucent
green glass, each turned to lie across its pad, in layers.

    python3 bonsai.py ../../saves [seed]
"""
import math, random, sys
from common import write, r

SEED = int(sys.argv[2]) if len(sys.argv) > 2 else 7
rng = random.Random(SEED)
H = math.sqrt(0.5)
RING_KNOTS = [0, 0, 0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1, 1, 1]


def add(a, b): return [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
def sub(a, b): return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
def mul(a, s): return [a[0] * s, a[1] * s, a[2] * s]
def dot(a, b): return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
def cross(a, b): return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
def length(a): return math.sqrt(dot(a, a))
def norm(a): return mul(a, 1.0 / max(length(a), 1e-12))


def heading(deg, elev=0.0):
    """A direction: `deg` round from +x toward +z (toward the camera), `elev` up."""
    h, e = math.radians(deg), math.radians(elev)
    return [math.cos(h) * math.cos(e), math.sin(e), math.sin(h) * math.cos(e)]


def bspline(ctrl, t):
    """A clamped uniform cubic B-spline (the renderer's default knots) at t in 0..1."""
    n, p = len(ctrl), min(3, len(ctrl) - 1)
    inner = n - p - 1
    k = [0.0] * (p + 1) + [(i + 1) / (inner + 1) for i in range(inner)] + [1.0] * (p + 1)
    t = min(max(t, 0.0), 1.0 - 1e-9)
    span = p
    while span < n - 1 and t >= k[span + 1]:
        span += 1
    d = [list(ctrl[span - p + j]) if isinstance(ctrl[0], (list, tuple)) else [ctrl[span - p + j]] for j in range(p + 1)]
    for rr in range(1, p + 1):
        for j in range(p, rr - 1, -1):
            i = span - p + j
            a = (t - k[i]) / (k[i + p + 1 - rr] - k[i])
            d[j] = [d[j - 1][c] + (d[j][c] - d[j - 1][c]) * a for c in range(len(d[j]))]
    return d[p] if isinstance(ctrl[0], (list, tuple)) else d[p][0]


def tube(name, centre, radii, material, oval=0.0, twist=0.0, tip=True):
    """A closed NURBS tube along `centre` with `radii` at its points; closed
    by a point at the start (a flat cap: buried in the parent) and, with
    `tip`, a rounded point just past the end. `oval` flattens the rings
    (0: circles) and `twist` turns the flattening by that many degrees from
    start to end."""
    pts = list(centre)
    rad = list(radii)
    # the poles: the start's at its first point, the tip's half a radius past the end
    last_t = norm(sub(pts[-1], pts[-2]))
    end = add(pts[-1], mul(last_t, rad[-1] * 0.8)) if tip else pts[-1]
    rows_c = [pts[0]] + pts + [end]
    rows_r = [0.0] + rad + [0.0]
    m = len(rows_c)
    # frames by parallel transport (the tangent from the nearest distinct
    # neighbours: the pole and the first ring share a point)
    def tangent(j):
        a, b = max(j - 1, 0), min(j + 1, m - 1)
        while length(sub(rows_c[b], rows_c[a])) < 1e-9 and b < m - 1:
            b += 1
        return norm(sub(rows_c[b], rows_c[a]))
    ts = [tangent(j) for j in range(m)]
    up = [0, 1, 0] if abs(ts[0][1]) < 0.7 else [1, 0, 0]
    frames = []
    nn = norm(sub(up, mul(ts[0], dot(up, ts[0]))))
    for j in range(m):
        nn = norm(sub(nn, mul(ts[j], dot(nn, ts[j]))))
        frames.append((nn, cross(ts[j], nn)))
    net = []
    for k in range(9):
        ang = k * math.pi / 4
        rr = 1.0 if k % 2 == 0 else 1.0 / H
        w = 1.0 if k % 2 == 0 else H
        cx, cy = rr * math.cos(ang), rr * math.sin(ang)
        row = []
        for j in range(m):
            nv, bv = frames[j]
            s = j / (m - 1)
            phi = math.radians(twist * s)
            # the oval's axes, turned by phi in the ring's plane
            a1 = add(mul(nv, math.cos(phi)), mul(bv, math.sin(phi)))
            a2 = add(mul(nv, -math.sin(phi)), mul(bv, math.cos(phi)))
            rad_j = rows_r[j]
            p = add(rows_c[j], add(mul(a1, cx * rad_j * (1 + oval)), mul(a2, cy * rad_j * (1 - oval))))
            row.append(r(p, 4) + ([] if w == 1.0 else [round(w, 6)]))
        net.append(row)
    return {"name": name, "type": "nurbs", "degree": [2, 3], "knots_u": RING_KNOTS, "points": net, "solid": True,
            "material": material, "locked": True}


# ---------------------------------------------------------------- the rock
ROCKS = [  # centre, radii (an island of three boulders, the tree on the biggest)
    ([0.0, -0.15, 0.0], [1.55, 0.8, 1.35]),
    ([1.25, -0.25, 0.55], [0.85, 0.6, 0.75]),
    ([-1.05, -0.3, -0.65], [0.9, 0.55, 0.8]),
]


def rock_top(x, z):
    """The island's height at (x, z) (the boulders' tops), or None over water."""
    best = None
    for c, rad in ROCKS:
        q = 1 - ((x - c[0]) / rad[0]) ** 2 - ((z - c[2]) / rad[2]) ** 2
        if q > 0:
            y = c[1] + rad[1] * math.sqrt(q)
            best = y if best is None else max(best, y)
    return best


rock = {"name": "rock", "type": "displace", "amplitude": 0.07, "frequency": 2.2, "octaves": 3, "material": "stone", "locked": True,
        "child": {"type": "union", "smooth": 0.35, "children": [
            {"type": "ellipsoid", "radii": rad, "position": c} for c, rad in ROCKS]}}

# ---------------------------------------------------------------- the trunk
# an informal upright (moyogi): out to the right, back over to the left, and
# the apex turned back over the base; thick and flared at the rock
TRUNK = [
    ([0.0, 0.45, 0.0], 0.62),
    ([0.08, 0.75, 0.02], 0.5),
    ([0.32, 1.15, -0.04], 0.4),
    ([0.62, 1.6, -0.1], 0.34),
    ([0.45, 2.15, 0.02], 0.29),
    ([-0.1, 2.6, 0.1], 0.24),
    ([-0.5, 3.1, 0.0], 0.19),
    ([-0.35, 3.6, -0.08], 0.14),
    ([0.0, 3.95, 0.0], 0.1),
    ([0.15, 4.25, 0.03], 0.07),
]
trunk_c = [p for p, _ in TRUNK]
trunk_r = [q for _, q in TRUNK]
objects = [rock, tube("trunk", [[0.0, 0.2, 0.0]] + trunk_c, [0.55] + trunk_r, "glass", oval=0.16, twist=140)]


def on_trunk(t):
    return bspline(trunk_c, t), bspline(trunk_r, t)


# ---------------------------------------------------------------- roots
# over the rock (root-over-rock, ishizuki-like) and down into the lake
for i in range(5):
    ang = i * 72 + 36 + rng.uniform(-20, 20)
    d = heading(ang)
    pts, rad = [], []
    reach = rng.uniform(1.6, 2.5)
    steps = 7
    for j in range(steps):
        s = j / (steps - 1)
        dist = 0.15 + reach * s
        wob = math.radians(rng.uniform(-8, 8))
        dd = [d[0] * math.cos(wob) - d[2] * math.sin(wob), 0, d[0] * math.sin(wob) + d[2] * math.cos(wob)]
        x, z = dd[0] * dist, dd[2] * dist
        rr = 0.2 * (1 - s) + 0.035 * s
        top = rock_top(x, z)
        # sitting on the stone (a little sunk into it), else sliding into the water
        y = (top + rr * 0.35) if top is not None else -0.15 - (dist - 1.6) * 0.9
        if j == 0:
            y = 0.62
        pts.append([x, y, z])
        rad.append(rr)
    objects.append(tube("root", [[0.0, 0.55, 0.0]] + pts, [0.22] + rad, "glass"))

# ---------------------------------------------------------------- branches and pads
pads = []  # (centre, half width, half height, yaw)
limbs = 0


def limb(name, base, start, hdg, length, r0, elev0, elev1, depth):
    """A branch from `base` (inside its parent) out through `start` (on the
    parent's surface), turning at every bud as pruned bonsai branches do;
    while `depth` lasts, side branches fan out level from its outer part,
    alternating sides (three of them from a primary and from a branch, two
    from a twig); every sprig (depth 0) ends in a pad. Each level bends one
    time fewer than its parent (a sprig twice): fewer patches."""
    global limbs
    steps = 2 + depth
    pts, rad, hs = [start], [r0], [hdg]
    pos, h = start, hdg
    seg = length / steps
    for j in range(1, steps + 1):
        s = j / steps
        h += rng.uniform(-25, 25)
        pos = add(pos, mul(heading(h, elev0 + (elev1 - elev0) * s + rng.uniform(-8, 8)), seg))
        pts.append(pos)
        rad.append(max(r0 * (1 - 0.65 * s), 0.008))
        hs.append(h)
    objects.append(tube(name, [base] + pts, [r0 * 1.1] + rad, "glass"))
    limbs += 1
    if depth == 0:
        w = rng.uniform(0.17, 0.22)
        hh = w * rng.uniform(0.38, 0.46)
        pads.append((add(pts[-1], [0, hh * 0.5, 0]), w, hh, rng.uniform(0, 180)))
        return
    sides = 2 if depth == 1 else 3
    at = sorted(rng.sample(range(1, steps), sides))
    side = rng.choice((-1, 1))
    for j in at:
        side = -side
        sh = hs[j] + side * rng.uniform(40, 70)
        left = length * (1 - j / steps)
        # (shorter at each level down: a branch, a twig, a sprig)
        d = depth - 1
        sub_len = max((0.16, 0.22, 0.3)[d], left * rng.uniform(0.5, 0.8) + (0.06, 0.1, 0.15)[d])
        limb(("sprig", "twig", "branch")[d], pts[j], add(pts[j], mul(heading(sh, 0), rad[j] * 0.8)), sh, sub_len,
             max(rad[j] * 0.6, 0.009), rng.uniform(-4, 6), rng.uniform(12, 24), d)


def primary(name, t, hdg, length, r0, elev0, elev1):
    base, tr = on_trunk(t)
    limb(name, base, add(base, mul(heading(hdg, elev0), tr * 0.85)), hdg, length, r0, elev0, elev1, 3)


# (t on the trunk, heading: 0 right, 90 toward the camera, -90 away; length;
# radius; elevation at the start and at the end) - the lowest is the longest,
# on the outside of the trunk's first bend; then alternating sides, with back
# branches for depth, shorter as they rise, and a crown of two at the top;
# each has three side branches, each three twigs, each two sprigs
primary("first branch", 0.24, 10, 2.2, 0.16, -16, 10)
primary("second branch", 0.33, 185, 2.0, 0.15, -14, 10)
primary("back branch", 0.42, -100, 1.6, 0.12, -8, 12)
primary("third branch", 0.5, 40, 1.6, 0.12, -10, 12)
primary("fourth branch", 0.57, 160, 1.5, 0.11, -10, 12)
primary("fifth branch", 0.64, -40, 1.3, 0.1, -6, 14)
primary("sixth branch", 0.71, 215, 1.2, 0.09, -6, 14)
primary("seventh branch", 0.79, 75, 1.0, 0.08, -4, 16)
primary("eighth branch", 0.86, -150, 0.85, 0.065, 0, 18)
primary("crown", 0.94, 120, 0.6, 0.05, 10, 25)
primary("crown", 0.97, -30, 0.55, 0.045, 10, 25)
# the jin: a dead branch, stripped and bleached (frosted glass), twisting up
jb, jr = on_trunk(0.46)
jp, jrad = [], []
pos = add(jb, mul(heading(70, 30), jr * 0.6))
for j, (hd, el, ln) in enumerate([(60, 35, 0.25), (95, 50, 0.22), (70, 62, 0.2), (110, 70, 0.16), (80, 78, 0.12)]):
    pos = add(pos, mul(heading(hd, el), ln))
    jp.append(pos)
    jrad.append(0.07 * (1 - j / 5) + 0.006)
objects.append(tube("jin", [jb] + jp, [0.08] + jrad, "frosted"))


LEAVES = 13
SHADES = ["leaf", "leaf", "leaf bright", "leaf deep", "leaf lime", "leaf jade"]  # (leaf twice: the usual green)


def leaf_euler(nrm, out):
    """Euler angles (x, y, z, degrees; applied in that order) turning a leaf
    (long along x, thin along y) to lie across `nrm` pointing along `out`."""
    y = norm(nrm)
    x = norm(sub(out, mul(y, dot(out, y))))
    z = cross(x, y)
    # the rotation's rows (columns are the leaf's axes in the world)
    m = [[x[0], y[0], z[0]], [x[1], y[1], z[1]], [x[2], y[2], z[2]]]
    sb = max(-1.0, min(1.0, -m[2][0]))
    b = math.asin(sb)
    if abs(sb) < 0.9999:
        a, c = math.atan2(m[2][1], m[2][2]), math.atan2(m[1][0], m[0][0])
    else:
        a, c = math.atan2(-m[1][2], m[1][1]), 0.0
    return [round(math.degrees(a), 1), round(math.degrees(b), 1), round(math.degrees(c), 1)]


for i, (c, w, hh, yaw) in enumerate(pads):
    d = w * rng.uniform(0.8, 0.95)
    ya = math.radians(yaw)
    cy, sy = math.cos(ya), math.sin(ya)

    def turn(v):  # about y by the pad's yaw
        return [v[0] * cy + v[2] * sy, v[1], -v[0] * sy + v[2] * cy]

    kids = []
    # (every pad the same number of leaves: the GPU shares one function between
    # programs of the same shape, so the shader builds quickly)
    for _ in range(LEAVES):
        # mostly over the top and round the sides, in a shallow layer (a ray
        # through glass leaves spends two bounces on each, so a pad is not
        # many leaves deep; the gaps between them show the ones behind)
        while True:
            u = norm([rng.gauss(0, 1), rng.gauss(0, 1), rng.gauss(0, 1)])
            if u[1] > -0.3:
                break
        depth = rng.uniform(0.85, 1.05)
        p = turn([u[0] * w * depth, u[1] * hh * depth, u[2] * d * depth])
        n = norm(turn([u[0] / w, u[1] / hh, u[2] / d]))
        n = norm(add(n, [rng.uniform(-0.45, 0.45) for _ in range(3)]))
        out = norm(add([p[0], 0.0, p[2]], [rng.uniform(-0.4, 0.4), 0, rng.uniform(-0.4, 0.4)]))
        sz = rng.uniform(0.8, 1.2)
        kids.append({"type": "ellipsoid", "radii": r([0.052 * sz, 0.007 * sz, 0.017 * sz], 4), "position": r(p, 3),
                     "rotation": leaf_euler(n, out), "material": rng.choice(SHADES)})
    objects.append({"name": "foliage pad", "type": "union", "position": r(c, 3), "shadow": False, "locked": True, "children": kids})

# the sandbank the island stands on: the top of a sphere 60 m across (exact,
# so a walker stands and wades on it smoothly), out of the water in a narrow
# strip round the rock, knee deep 4 m out, a person's depth 9 m out (there
# they float), 2.4 m down at 12 m, then the deep lake
objects.append({"name": "sandbank", "type": "sphere", "radius": 30, "position": [0, -29.9, 0], "material": "sand", "locked": True})
objects.append({"name": "lake", "type": "water", "level": 0, "waves": 0.3, "wave_speed": 0.7})

# two groups: the tree (trunk, roots, the jin and every branch, twig and
# sprig) and its leaves, each selected, hidden and moved as one (the object
# list five entries, not five hundred and fifty)
tree = [o for o in objects if o["type"] == "nurbs"]
leaves = [o for o in objects if o.get("name") == "foliage pad"]
grouped = {id(o) for o in tree + leaves}
rest = [o for o in objects if id(o) not in grouped]
objects = rest[:1] + [
    {"name": "tree", "type": "group", "locked": True, "children": tree},
    {"name": "leaves", "type": "group", "locked": True, "shadow": False, "children": leaves},
] + rest[1:]

write("bonsai", {
    "description": f"A glass bonsai on a rock in a lake at sunset. Trunk, roots, {limbs} branches and twigs and the bleached jin are all NURBS tubes of glass: the trunk an informal upright with a twisted, oval section, its roots gripping the rock and reaching into the water, each branch splitting into side branches, twigs and sprigs. {LEAVES * len(pads)} small leaves of translucent green glass in five shades fill {len(pads)} pads at the branch ends. A sandbank rises under the island, so in game mode you can wade out of the lake onto it; the water moves (so the image does not settle: turn off animate water in the Render panel for a clean still). Look into the sun to see the glass catch it.",
    "camera": {"position": [1.6, 1.35, 8.0], "look_at": [0.15, 1.95, 0], "fov": 45, "speed": 3},
    "environment": {"preset": "golden", "time_of_day": 17.88, "sun_azimuth": -71, "clouds": 0.35, "fog_density": 0.0025},
    "render": {"bounces": 8, "animate_water": True, "exposure": 1.0},
    "materials": {
        "glass": {"color": [0.7, 0.95, 0.88], "transmission": 1, "ior": 1.5, "roughness": 0, "absorption": 1.2},
        "frosted": {"color": [0.95, 0.97, 1.0], "transmission": 1, "ior": 1.5, "roughness": 0.3, "absorption": 0.3},
        # the leaves: translucent green glass. Green glass takes the red and
        # blue out of the light through it and lets the green by: the green
        # channel is 1 (none of it absorbed), and the others go within a
        # centimetre or two, so a leaf colours the warm sunset light green
        # rather than darkening it to olive (lime lets a little red by)
        "leaf": {"color": [0.06, 1.0, 0.12], "transmission": 1, "ior": 1.5, "roughness": 0.04, "absorption": 110},
        "leaf bright": {"color": [0.2, 1.0, 0.15], "transmission": 1, "ior": 1.5, "roughness": 0.04, "absorption": 90},
        "leaf deep": {"color": [0.02, 0.85, 0.15], "transmission": 1, "ior": 1.5, "roughness": 0.04, "absorption": 130},
        "leaf lime": {"color": [0.4, 1.0, 0.04], "transmission": 1, "ior": 1.5, "roughness": 0.04, "absorption": 90},
        "leaf jade": {"color": [0.03, 1.0, 0.55], "transmission": 1, "ior": 1.5, "roughness": 0.04, "absorption": 110},
        "sand": {"color": [0.78, 0.68, 0.5], "color2": [0.6, 0.5, 0.36], "texture": "noise", "texture_scale": 1.5, "roughness": 0.85, "bump_pattern": "ripples", "bump_scale": 2.5, "bump": 0.35, "space": "world"},
        "stone": {"color": [0.2, 0.19, 0.18], "color2": [0.1, 0.12, 0.08], "texture": "noise", "texture_scale": 2.5, "roughness": 0.75, "space": "world"},
    },
    "objects": objects,
})
print(f"{len(objects)} objects: {[o['name'] for o in objects]}; the tree {len(tree)} NURBS tubes ({limbs} branches), "
      f"the leaves {len(leaves)} pads, {sum(len(o['children']) for o in leaves)} leaves")
