#!/usr/bin/env python3
"""Walks through the inside of each kind of fractal, and through a screenshot's
scene in haze. It drives the viewer of tools/demo/session.sh through its
control socket: each fractal alone in a scene of its own (saves/fractals.json's
look: its material, the sun and sky), made big, so its chambers and tunnels
are metres across. A path is found by raycasts: of points scattered through
the fractal, the most open that is enclosed (most rays from it hit), then on
from there the way that stays open, kept in the middle of the space, smoothed
and checked clear. The camera glides along it (looking ahead) while
make_demo's Take records 10 s at 1920 x 1080 into WORKDIR/clips/NAME.mp4.
The beams walk goes on from where the screenshot was taken, the way it looks.
    python3 tools/demo/interiors.py WORKDIR [--scout] [--plan] [--shot PNG] [name ...]
(names: the fractals' (menger-1, menger-2 ...) and "beams", which walks from
the screenshot --shot (a PNG ray saved: it holds its scene); default all.
--scout: small and fast, into WORKDIR/scout; --plan: only the paths, into
WORKDIR/paths/NAME.json. A path already planned is used again.)
"""
import json
import math
import os
import random
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import make_demo as md  # noqa: E402
from viewer import Viewer  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
LENGTH = 10.0
LEGS = 20

# each fractal: its node (saves/fractals.json's settings and material) at a
# scale that makes it tens of metres across, and the half-size of the box its
# start points are looked for in
FRACTALS = {
    "menger": ({"type": "menger", "iterations": 5, "material": "copper orbit"}, 12.0, 11.0),
    "mandelbulb": ({"type": "mandelbulb", "power": 8, "iterations": 10, "material": "copper orbit"}, 10.0, 11.0),
    # (fractals.json's fold of -1.5 is solid inside: 3 has rooms)
    "mandelbox": ({"type": "mandelbox", "fold_scale": 3.0, "iterations": 14}, 3.0, 11.0),
    # (fractals.json's c is solid inside: this one is a cage of ribbons round a hollow)
    "julia": ({"type": "julia", "c": [-0.125, -0.256, 0.847, 0.0895], "iterations": 11}, 10.0, 8.0),
    "sierpinski": ({"type": "sierpinski", "iterations": 9}, 14.0, 12.0),
    "apollonian": ({"type": "apollonian", "fold": 1.1, "iterations": 8, "material": "coral"}, 4.0, 8.0),
    "kifs": ({"type": "kifs", "fold_scale": 3, "offset": [1, 1, 1], "angles": [12, 8, 0], "iterations": 6}, 10.0, 12.0),
    "kleinian": ({"type": "kleinian", "iterations": 8, "material": "blue lace"}, 4.0, 8.0),
}

# the haze the beams walk is in (made for a screenshot on top of the
# gallery's Mandelbulb: shafts through its overhangs at 15:00)
BEAMS_ENV = {"time_of_day": 15.0, "haze_density": 6, "haze_top": 4.1}
# (the screenshot keeps the device it was taken with, GPU only: both trace it)
BEAMS_SETTINGS = {"haze_detail": 8, "haze_contrast": 1.6, "device": "both"}
# (its haze's shadow rays through the fractal cost a minute a frame at
# 1920 x 1080 and 32 samples: at 1280 x 720, 16 samples of 8 points each, a
# quarter of that; the reel scales it up)
BEAMS_SIZE, BEAMS_SAMPLES = (1280, 720), 16
# (it goes at most this far on, turning this many degrees left as it goes)
BEAMS_WALK, BEAMS_TURN = 0.08, 20.0


def fib(n):
    """n directions spread evenly over the sphere."""
    out, turn = [], math.pi * (3 - math.sqrt(5))
    for i in range(n):
        y = 1 - (i + 0.5) * 2 / n
        s = math.sqrt(max(0.0, 1 - y * y))
        out.append((s * math.cos(turn * i), y, s * math.sin(turn * i)))
    return out


def add(a, b, k=1.0):
    return [a[i] + b[i] * k for i in range(3)]


def sub(a, b):
    return [a[i] - b[i] for i in range(3)]


def norm(a):
    return math.sqrt(sum(c * c for c in a))


def unit(a):
    n = norm(a) or 1.0
    return [c / n for c in a]


def dot(a, b):
    return sum(a[i] * b[i] for i in range(3))


class Space:
    """Raycasts through the viewer's scene: how far is free from a point."""

    def __init__(self, v, cap):
        self.v, self.cap, self.casts = v, cap, 0

    def free(self, p, d):
        self.casts += 1
        r = self.v("raycast", origin=list(p), direction=list(d))
        return min(r["distance"], self.cap) if r.get("hit") else math.inf

    def around(self, p, dirs):
        return [self.free(p, d) for d in dirs]


# the share of rays from a start point that must hit something (the solid
# ones' open spaces are crevices, half open to the sky)
ENCLOSED = {"menger": 0.8, "mandelbulb": 0.6, "mandelbox": 0.7, "julia": 0.6, "sierpinski": 0.6, "apollonian": 0.8, "kifs": 0.55, "kleinian": 0.8}


def start_points(sp, center, half, rng, n=150, enclosed=0.8):
    """Enclosed points, as open as a room in it (about a sixth of its
    half-size: the big central voids are empty, the finest crevices tight):
    (openness, point), best first, where openness is the median distance to
    what is around, among points from which at least four rays in five hit
    something within the cap."""
    dirs = fib(20)
    axes = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)]
    found = []
    for _ in range(n):
        p = [center[i] + rng.uniform(-half, half) for i in range(3)]
        # (inside the solid: the rays start on it)
        if min(sp.around(p, axes)) < 0.02 * half:
            continue
        ds = sp.around(p, dirs)
        hits = [d for d in ds if d < math.inf]
        if len(hits) < enclosed * len(ds) or min(ds) < 0.03 * half:
            continue
        found.append((sorted(ds)[len(ds) // 2], p))
    target = half / 6
    found.sort(key=lambda f: abs(math.log(f[0] / target)))
    return found


def center_in(sp, p, open_, k=0.35):
    """`p` moved toward the middle of the space around it."""
    dirs = fib(14)
    ds = [min(d, 2.5 * open_) for d in sp.around(p, dirs)]
    mean = sum(ds) / len(ds)
    push = [0.0, 0.0, 0.0]
    for d, u in zip(ds, dirs):
        push = add(push, u, (d - mean) / len(ds))
    return add(p, push, k)


def grow(sp, p, heading, open_, length, rng, box=None):
    """A path from `p` on along the most open way, close to `heading`,
    `length` metres long (or as far as it gets), inside `box` (center,
    half-size) when given."""
    inside = lambda q: box is None or max(abs(q[i] - box[0][i]) for i in range(3)) <= 0.92 * box[1]
    pts, done = [p], 0.0
    heading = unit(heading)
    for _ in range(200):
        if done >= length:
            break
        best = None
        for _ in range(24):
            # (a random direction within 50 degrees of the heading)
            r = unit([rng.gauss(0, 1) for _ in range(3)])
            cand = unit(add(heading, unit(sub(r, [heading[i] * dot(r, heading) for i in range(3)])), rng.uniform(0, math.tan(math.radians(50)))))
            f = sp.free(pts[-1], cand)
            # (out of the fractal: no better than the room it is in)
            f = min(f, 3.0 * open_) if f < math.inf else open_
            if not inside(add(pts[-1], cand, min(0.3 * f, 0.6 * open_))):
                continue
            score = f * (0.5 + 0.5 * dot(cand, heading)) ** 2
            if best is None or score > best[0]:
                best = (score, cand, f)
        if best is None:
            break
        _, d, f = best
        step = min(0.3 * f, 0.6 * open_)
        if step < 0.02 * open_:
            break
        q = center_in(sp, add(pts[-1], d, step), open_)
        # (still clear from the last point to the centred one)
        seg = sub(q, pts[-1])
        if sp.free(pts[-1], unit(seg)) < norm(seg) * 1.05:
            q = add(pts[-1], d, step)
        done += norm(sub(q, pts[-1]))
        heading = unit(add(heading, unit(sub(q, pts[-1])), 1.5))
        pts.append(q)
    return pts


def resample(pts, n):
    """`n` + 1 points evenly spaced along the polyline."""
    cum = [0.0]
    for a, b in zip(pts, pts[1:]):
        cum.append(cum[-1] + norm(sub(b, a)))
    out, j = [], 0
    for i in range(n + 1):
        s = cum[-1] * i / n
        while j < len(cum) - 2 and cum[j + 1] < s:
            j += 1
        seg = cum[j + 1] - cum[j] or 1.0
        k = (s - cum[j]) / seg
        out.append(add(pts[j], sub(pts[j + 1], pts[j]), k))
    return out


def smooth(pts, rounds=3):
    for _ in range(rounds):
        pts = [pts[0]] + [[(pts[i - 1][c] + 2 * pts[i][c] + pts[i + 1][c]) / 4 for c in range(3)] for i in range(1, len(pts) - 1)] + [pts[-1]]
    return pts


def clear(sp, pts, margin):
    """Whether every leg is free, with `margin` to spare beyond its end."""
    for a, b in zip(pts, pts[1:]):
        seg = sub(b, a)
        if norm(seg) > 0 and sp.free(a, unit(seg)) < norm(seg) + margin:
            return False
    return True


def plan(sp, center, half, seed, avoid=None, enclosed=0.8):
    """A path: LEGS + 1 points, clear, and the openness it was planned for."""
    rng = random.Random(seed)
    starts = start_points(sp, center, half, rng, enclosed=enclosed)
    if avoid is not None:
        starts = [s for s in starts if norm(sub(s[1], avoid)) > 0.4 * half] or starts
    if not starts:
        raise RuntimeError("no open space found")
    open_, p = starts[0]
    dirs = fib(32)
    ds = [min(d, 3.0 * open_) if d < math.inf else 0.0 for d in sp.around(p, dirs)]
    heading = dirs[max(range(len(dirs)), key=lambda i: ds[i])]
    length = 0.4 * open_ * LENGTH
    raw = grow(sp, p, heading, open_, length, rng, (center, half))
    path = resample(raw, LEGS)
    for rounds in (3, 1, 0):
        cand = smooth(path, rounds)
        if clear(sp, cand, 0.05 * open_):
            return cand, open_
    return path, open_


def scene_for(name):
    """A scene with the fractal alone, big, at the origin: saves/fractals.json's
    environment and materials."""
    base = json.load(open(os.path.join(ROOT, "saves", "fractals.json")))
    node, scale, _ = FRACTALS[name.split("-")[0]]
    node = dict(node, scale=scale, name=name.split("-")[0])
    node.setdefault("material", "copper orbit")
    out = {k: base[k] for k in ("environment", "materials", "render") if k in base}
    out["objects"] = [node]
    out["camera"] = {"position": [0, 0, scale * 3], "yaw": 0, "pitch": 0, "fov": 70, "speed": scale * 0.3}
    return out


def dolly(sp, start, yaw, pitch):
    """The beams walk: on from where the screenshot was taken along the way it
    looks (its walker stood in a pocket a few of its heights across: the open
    way is up into the light), half way to what is there or BEAMS_WALK,
    looking where it looked and turning a little: a path as plan()'s, with
    what each point looks at."""
    yaw, pitch = math.radians(yaw), math.radians(pitch)
    look = lambda a: [-math.sin(a) * math.cos(pitch), math.sin(pitch), -math.cos(a) * math.cos(pitch)]
    free = sp.free(start, look(yaw))
    length = min(BEAMS_WALK, 0.5 * free)
    path = [add(start, look(yaw), length * i / LEGS) for i in range(LEGS + 1)]
    looks = [add(q, look(yaw + math.radians(BEAMS_TURN) * i / LEGS), length) for i, q in enumerate(path)]
    return {"path": path, "open": free, "looks": looks}


def walk(v, take, path, open_, looks=None, look_ahead=3):
    """The take: the camera on the path's first point looking along it (or
    at `looks`), then the legs, queued at once."""
    look = (lambda i: looks[i]) if looks else lambda i: path[min(i + look_ahead, len(path) - 1)] if i + look_ahead < len(path) else add(path[-1], sub(path[-1], path[-2]), look_ahead)
    v("camera", position=path[0], look_at=look(0))
    with take(LENGTH) as t:
        for i in range(1, len(path)):
            v("glide", position=path[i], look_at=look(i), seconds=LENGTH / LEGS, even=True, queue=i > 1)
        t.end()


def main():
    args = sys.argv[1:]
    work = os.path.abspath(args[0])
    md.SCOUT = "--scout" in args
    only_plan = "--plan" in args
    names = [a for a in args[1:] if not a.startswith("--")]
    shot = args[args.index("--shot") + 1] if "--shot" in args else None
    names = [a for a in names if a != shot]
    every = [f"{f}-{k}" for f in FRACTALS for k in (1, 2)] + (["beams"] if shot else [])
    names = names or every
    if "beams" in names and not shot:
        sys.exit("beams: which screenshot? (--shot PNG)")
    v = Viewer(md.SOCKET)
    v.up()
    os.makedirs(os.path.join(work, "paths"), exist_ok=True)
    os.makedirs(os.path.join(work, "scenes"), exist_ok=True)
    take = lambda length: md.Take(v, work, name, length, samples=BEAMS_SAMPLES if name == "beams" else md.SAMPLES)
    for name in names:
        done = os.path.join(work, "scout" if md.SCOUT else "clips", name + ".mp4")
        if os.path.exists(done) and not only_plan:
            print(f"{name}: done already", flush=True)
            continue
        print(f"{name}:", flush=True)
        path_file = os.path.join(work, "paths", name + ".json")
        if name == "beams":
            md.setup(v, os.path.abspath(os.path.expanduser(shot)))
            v("environment", set=BEAMS_ENV)
            v("settings", set=BEAMS_SETTINGS)
            v.ready(samples=1, timeout=120)
            cam = v("scene")["camera"]
            if os.path.exists(path_file):
                plan_ = json.load(open(path_file))
            else:
                plan_ = dolly(Space(v, 1.0), cam["position"], cam["yaw_degrees"], cam["pitch_degrees"])
                json.dump(plan_, open(path_file, "w"))
        else:
            fractal = name.split("-")[0]
            scene_file = os.path.join(work, "scenes", fractal + ".json")
            sc = scene_for(name)
            json.dump(sc, open(scene_file, "w"), indent=1)
            _, scale, half = FRACTALS[fractal]
            md.setup(v, scene_file, settings={"headlight": 1.0, "headlight_shadows": False})
            if os.path.exists(path_file):
                plan_ = json.load(open(path_file))
            else:
                # (each raycast waits for a frame: small ones while planning)
                v("settings", set={"scale": 0.1})
                sp = Space(v, 2.0 * half)
                # (each walk starts away from where its other one did)
                avoid = None
                other = os.path.join(work, "paths", fractal + ("-1.json" if name.endswith("-2") else "-2.json"))
                if os.path.exists(other):
                    avoid = json.load(open(other))["path"][0]
                try:
                    path, open_ = plan(sp, [0, 0, 0], half, zlib.crc32(name.encode()), avoid=avoid, enclosed=ENCLOSED[fractal])
                except RuntimeError as e:
                    v("settings", set={"scale": 1.0})
                    print(f"  {name}: {e}, skipped", flush=True)
                    continue
                plan_ = {"path": path, "open": open_}
                json.dump(plan_, open(path_file, "w"))
                v("settings", set={"scale": 1.0})
                print(f"  planned {norm(sub(path[-1], path[0])):.1f} m end to end, openness {open_:.2f} m, {sp.casts} raycasts", flush=True)
            v("settings", set={"headlight_reach": max(4.0 * plan_["open"], 1.0)})
        if only_plan:
            continue
        md.SIZE = BEAMS_SIZE if name == "beams" else (1920, 1080)
        walk(v, take, plan_["path"], plan_["open"], plan_.get("looks"))


if __name__ == "__main__":
    main()
