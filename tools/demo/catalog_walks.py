#!/usr/bin/env python3
"""Walks through the inside of every fractal in the catalog that
tools/demo/interiors.py and tools/demo/originals.py leave out, 10 s each,
into WORKDIR/clips/NAME.mp4; around the outside of those with no inside to
walk. Each is the catalog's node (saves/fractal-catalog.json: its settings;
of a fractal clipped by a box or a sphere, the fractal alone) by itself at
the origin, in a material of its own (MATERIALS) under the gallery's sky
(saves/fractals.json), the headlight on inside.
How each is shown is decided once, by raycasts (interiors.py's planner),
and kept with its path in WORKDIR/paths/NAME.json: made GROW times its size
in the catalog, a walk from the most open enclosed place found (four rays in
five hitting something, else three in five) if it is open enough; else a
slow orbit at the catalog's size.
    python3 tools/demo/catalog_walks.py WORKDIR [--scout] [--plan] [name ...]
(--scout: small and fast, into WORKDIR/scout; --plan: only decide and plan,
by `ray raycast`, without the viewer.)
"""
import json
import math
import os
import random
import subprocess
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import interiors as it  # noqa: E402
import make_demo as md  # noqa: E402
from viewer import Viewer  # noqa: E402

ROOT = it.ROOT
LENGTH = it.LENGTH
# how many times its catalog size a fractal is made to be walked through
# (most are about a metre across there: 15 to 30 m)
GROW = 18.0
# the share of rays from a start point that must hit, tried in turn
ENCLOSED = (0.8, 0.6)
# a walk needs this much room (its openness, as a share of the half-size
# searched): less is a crevice
ROOMY = 0.05
# and its views must be mostly the fractal: of the rays through each view
# along it, on average this share must hit it, and at least LEAST at any
# point (a path along the outside of a solid one looks out at the sky)
SEEN, LEAST = 0.6, 0.4
# how many of the best start points each try walks from
STARTS = 16
# gone round even so: the best walk found through benesi ends inside it,
# hypercomplex's is mostly sky
ORBIT = {"benesi", "hypercomplex"}
# Kleinian slabs: their walks climb or fall steeply, and looking along one a
# ceiling or a floor fills the view; these look level, along the walk's
# heading, down the rows of arches between the two
LEVEL = {"kleinian_mod", "kleinian4d"}
# the ones walked through or round already (interiors.py, originals.py)
DONE = set(it.FRACTALS) | {"labyrinth_sponge", "quasicrystal_rooms", "screw_sponge", "cusp_vaults", "cathedral", "octet_truss", "nil_stair", "mayer_julia", "dirac_frost", "horned_sphere"}

# materials for them: the orbit trap's two colours (its bands follow the
# fractal's own detail at any size), glowing seams to light the insides,
# metals with relief, stones and patterns
MATERIALS = {
    "molten seams": {"description": "Black basalt split by glowing orange seams (orbit).", "color": [0.05, 0.045, 0.045], "roughness": 0.8, "color2": [1.0, 0.42, 0.08], "emission2": [1.2, 0.4, 0.05], "texture": "orbit", "texture_scale": 2.5},
    "abyssal glow": {"description": "Deep-sea navy with cyan light in the bands (orbit).", "color": [0.03, 0.05, 0.12], "roughness": 0.35, "color2": [0.2, 0.9, 1.0], "emission2": [0.1, 0.6, 0.9], "texture": "orbit", "texture_scale": 2},
    "bioluminescence": {"description": "Wet dark flesh with green glowing cells.", "color": [0.06, 0.1, 0.09], "roughness": 0.25, "color2": [0.4, 1.0, 0.5], "emission2": [0.15, 0.9, 0.3], "texture": "cells", "texture_scale": 3, "sharpness": 2.5, "bump": 0.4},
    "anodized titanium": {"description": "Metal from blue to gold, as titanium colours when anodized (orbit).", "color": [0.2, 0.35, 0.85], "metallic": 1, "roughness": 0.2, "color2": [0.95, 0.75, 0.3], "texture": "orbit", "texture_scale": 3},
    "obsidian gold": {"description": "Polished black glass veined with gold (orbit).", "color": [0.02, 0.02, 0.025], "roughness": 0.05, "color2": [1.0, 0.76, 0.33], "metallic2": 1, "roughness2": 0.2, "texture": "orbit", "texture_scale": 2},
    "opal": {"description": "Milky white to pale sky blue, pearly (orbit).", "color": [0.93, 0.9, 0.95], "roughness": 0.12, "color2": [0.4, 0.7, 0.95], "metallic2": 0.5, "texture": "orbit", "texture_scale": 3},
    "amethyst": {"description": "Violet to deep purple metal (orbit).", "color": [0.72, 0.5, 0.95], "metallic": 0.6, "roughness": 0.2, "color2": [0.2, 0.05, 0.35], "texture": "orbit", "texture_scale": 2},
    "porcelain": {"description": "Glazed white porcelain painted cobalt blue (orbit).", "color": [0.95, 0.94, 0.9], "roughness": 0.04, "color2": [0.08, 0.18, 0.6], "texture": "orbit", "texture_scale": 2.5},
    "terracotta": {"description": "Terracotta with turquoise glaze in the folds (orbit).", "color": [0.78, 0.38, 0.22], "roughness": 0.75, "color2": [0.15, 0.65, 0.6], "roughness2": 0.1, "texture": "orbit", "texture_scale": 2},
    "rust chrome": {"description": "Chrome eaten to rust along the bands (orbit).", "color": [0.95, 0.95, 0.97], "metallic": 1, "roughness": 0.05, "color2": [0.45, 0.17, 0.06], "metallic2": 0, "roughness2": 0.9, "texture": "orbit", "texture_scale": 2},
    "candy": {"description": "Glossy pink and mint (orbit).", "color": [0.98, 0.55, 0.7], "roughness": 0.15, "color2": [0.55, 0.95, 0.8], "texture": "orbit", "texture_scale": 3},
    "glacier": {"description": "Snow white to glacier blue, wet (orbit).", "color": [0.92, 0.95, 1.0], "roughness": 0.1, "color2": [0.2, 0.5, 0.8], "texture": "orbit", "texture_scale": 2},
    "malachite": {"description": "Banded green malachite, polished.", "color": [0.05, 0.35, 0.2], "color2": [0.45, 0.85, 0.6], "texture": "marble", "texture_scale": 3, "warp": 1.4, "sharpness": 2.0, "roughness": 0.1},
    "lapis": {"description": "Lapis lazuli: deep blue flecked with gold.", "color": [0.08, 0.12, 0.5], "roughness": 0.2, "color2": [1.0, 0.8, 0.35], "metallic2": 1, "texture": "spots", "texture_scale": 14, "sharpness": 3},
    "hammered gold": {"description": "Gold beaten into dimples.", "color": [1.0, 0.76, 0.33], "metallic": 1, "roughness": 0.15, "bump_pattern": "cells", "bump_scale": 9, "bump": 1.5},
    "dragon scales": {"description": "Green scales with a metallic sheen, raised.", "color": [0.15, 0.5, 0.25], "color2": [0.03, 0.12, 0.06], "metallic": 0.5, "texture": "scales", "texture_scale": 6, "roughness": 0.25, "bump_pattern": "scales", "bump": 1.5},
    "lava": {"description": "Cooled black crust over glowing cracks.", "color": [0.08, 0.06, 0.05], "roughness": 0.9, "color2": [1.0, 0.35, 0.05], "emission2": [6, 1.75, 0.2], "texture": "cracks", "texture_scale": 1.5, "warp": 0.8, "sharpness": 0.7, "bump": -0.4},
    "lit windows": {"description": "Dark stone with warm lit windows, like a city at night.", "color": [0.3, 0.3, 0.32], "roughness": 0.6, "color2": [1.0, 0.82, 0.55], "emission2": [2.0, 1.5, 0.8], "texture": "windows", "texture_scale": 5},
    "neon grid": {"description": "Black with a glowing cyan grid a metre square.", "color": [0.03, 0.03, 0.05], "roughness": 0.4, "color2": [0.2, 0.9, 1.0], "emission2": [0.6, 4, 6], "texture": "grid", "texture_scale": 1, "space": "world"},
    "verdigris": {"description": "Bronze gone green in the hollows (orbit).", "color": [0.7, 0.45, 0.2], "metallic": 1, "roughness": 0.3, "color2": [0.3, 0.62, 0.52], "metallic2": 0, "roughness2": 0.7, "texture": "orbit", "texture_scale": 1.5},
    "ember": {"description": "Charcoal grey glowing red-hot in the bands (orbit).", "color": [0.12, 0.11, 0.1], "roughness": 0.7, "color2": [0.9, 0.12, 0.04], "emission2": [0.8, 0.08, 0.01], "texture": "orbit", "texture_scale": 3},
    "sandstone": {"description": "Red and cream sandstone in warped layers.", "color": [0.78, 0.42, 0.25], "color2": [0.92, 0.8, 0.62], "texture": "stripes", "texture_scale": 4, "warp": 1.6, "sharpness": 1.2, "roughness": 0.85, "bump": 0.2},
}

# each fractal's material: neighbours in the catalog's order differ
MATERIAL_OF = {
    "amazing_surf": "anodized titanium", "kalibox": "rust chrome", "mandalay": "porcelain", "kaliset": "abyssal glow",
    "bulbbox": "verdigris", "juliabulb": "opal", "mandelbar": "ember", "burning_ship": "lava", "power2_bulb": "candy",
    "benesi": "obsidian gold", "kosalos": "glacier", "lambda_bulb": "amethyst", "sincos_bulb": "terracotta",
    "riemann_sphere": "rust chrome", "riemann_hobold": "malachite", "hopfbrot": "abyssal glow", "quaternion": "hammered gold",
    "tetrabrot": "opal", "hypercomplex": "molten seams", "aexion": "dragon scales", "bristorbrot": "porcelain",
    "buffalo": "sandstone", "mandelcup": "anodized titanium", "quick_dudley": "ember", "collatz": "lapis",
    "collatz_mod": "candy", "newton": "verdigris", "jos_kleinian": "lit windows", "schottky": "glacier",
    "kleinian_mod": "terracotta", "msltoe_donut": "amethyst", "mandeltorus": "terracotta", "spheretree": "obsidian gold",
    "sphere_cluster": "bioluminescence", "mosely": "neon grid", "vicsek": "lit windows", "cantor_dust": "anodized titanium",
    "jerusalem": "sandstone", "polyhedral": "rust chrome", "koch": "malachite", "knot": "hammered gold",
    "menger4d": "molten seams", "sierpinski4d": "porcelain", "mandelbox4d": "abyssal glow", "kleinian4d": "ember",
    "xenodreambuie": "dragon scales", "pine_tree": "lava", "makin": "opal", "ides": "lapis", "difs": "candy",
    "msltoe_sym": "verdigris", "eiffie": "amethyst", "mandelnest": "terracotta", "seifert_gasket": "glacier",
    "indra_cells": "neon grid", "zorich_pagodas": "lit windows",
}


def catalog():
    """Each fractal of the catalog not walked yet: (the node to walk through,
    the node to go round, its half-height there), each at the origin: of a
    clipped one, the fractal alone to walk through (at the clip's scale, and
    searched as if at least a metre across: the clip cuts it smaller), the
    clipped one to go round."""
    out = {}
    for o in json.load(open(os.path.join(ROOT, "saves", "fractal-catalog.json")))["objects"]:
        if not str(o.get("material", "")).startswith("row"):
            continue
        node = outside = {k: v for k, v in o.items() if k not in ("position", "material", "name", "locked")}
        half = o["position"][1] - 0.9
        if node["type"] == "intersection":
            inner = dict(node["children"][-1])
            inner.pop("position", None)
            inner["scale"] = inner.get("scale", 1.0) * node["scale"]
            if "rotation" in node:
                inner["rotation"] = node["rotation"]
            node, half = inner, max(half, 0.6)
        if node["type"] not in DONE:
            out[node["type"]] = (node, outside, half)
    return out


def scene_for(name, node, scale):
    """The fractal alone at the origin at `scale` times its catalog size: the
    gallery's environment and materials, and MATERIALS (patterns not in the
    world's space or the orbit's made as much finer, so they keep their size)."""
    base = json.load(open(os.path.join(ROOT, "saves", "fractals.json")))
    mats = dict(base["materials"])
    for k, m in MATERIALS.items():
        m = dict(m)
        if m.get("space") != "world" and m.get("texture") != "orbit":
            for f in ("texture_scale", "bump_scale"):
                if f in m:
                    m[f] *= scale
        mats[k] = m
    node = dict(node, scale=node.get("scale", 1.0) * scale, name=name, material=MATERIAL_OF[name])
    out = {k: base[k] for k in ("environment", "render") if k in base}
    out["materials"] = mats
    out["objects"] = [node]
    out["camera"] = {"position": [0, 1, 4], "yaw": 0, "pitch": 0, "fov": 60, "speed": 2}
    return out


class Casts:
    """interiors.py's Space, cast by `ray raycast` on the scene's file: no
    viewer, no frame to wait for at each ray (a plan's four thousand took
    twenty minutes that way)."""

    def __init__(self, scene_file, cap):
        self.cap, self.casts = cap, 0
        self.p = subprocess.Popen([os.path.join(ROOT, "target", "release", "ray"), "raycast", scene_file], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)

    def around(self, p, dirs):
        self.casts += len(dirs)
        self.p.stdin.write("".join(f"{p[0]} {p[1]} {p[2]} {d[0]} {d[1]} {d[2]}\n" for d in dirs))
        self.p.stdin.flush()
        out = []
        for _ in dirs:
            r = self.p.stdout.readline().split()
            if not r:
                raise RuntimeError("ray raycast stopped")
            out.append(math.inf if r[0] == "miss" else min(float(r[0]), self.cap))
        return out

    def free(self, p, d):
        return self.around(p, [d])[0]

    def close(self):
        self.p.stdin.close()
        self.p.wait()


def walk_from(sp, p, open_, half, rng):
    """interiors.py's plan from a start point it was given: on along the
    most open way, smoothed where that stays clear (None: it went nowhere)."""
    dirs = it.fib(32)
    ds = [min(d, 3.0 * open_) if d < math.inf else 0.0 for d in sp.around(p, dirs)]
    heading = dirs[max(range(len(dirs)), key=lambda i: ds[i])]
    raw = it.grow(sp, p, heading, open_, 0.4 * open_ * LENGTH, rng, ([0, 0, 0], half))
    if len(raw) < 2:
        return None
    path = it.resample(raw, it.LEGS)
    for rounds in (3, 1, 0):
        cand = it.smooth(path, rounds)
        if it.clear(sp, cand, 0.05 * open_):
            return cand
    return path


def seen(sp, path):
    """How much of each view along `path` (looking ahead, 60 degrees high at
    16:9) is the fractal: the mean share of rays through it that hit, and
    the least."""
    shares = []
    for i in range(len(path)):
        j = min(i + 3, len(path) - 1)
        f = it.unit(it.sub(path[j], path[i]) if j > i else it.sub(path[-1], path[-2]))
        r = it.unit([-f[2], 0.0, f[0]]) if abs(f[1]) < 0.99 else [1.0, 0.0, 0.0]
        u = [r[1] * f[2] - r[2] * f[1], r[2] * f[0] - r[0] * f[2], r[0] * f[1] - r[1] * f[0]]
        ty = math.tan(math.radians(30))
        dirs = [it.add(it.add(f, r, ty * 16 / 9 * (x / 2.5 - 1)), u, ty * (y / 1.5 - 1)) for x in range(6) for y in range(4)]
        ds = sp.around(path[i], [it.unit(d) for d in dirs])
        shares.append(sum(d < math.inf for d in ds) / len(ds))
    return sum(shares) / len(shares), min(shares)


def decide(name, half, scene_file):
    """A walk ({"path", "open", "seen"}) if the raycasts find one with room
    and a view (from the most enclosed start points first), else None."""
    for enclosed in ENCLOSED:
        sp = Casts(scene_file, 2.0 * half)
        rng = random.Random(zlib.crc32(name.encode()))
        best = near = None
        try:
            starts = it.start_points(sp, [0, 0, 0], half, rng, n=300, enclosed=enclosed)
            for open_, p in [s for s in starts if s[0] >= ROOMY * half][:STARTS]:
                path = walk_from(sp, p, open_, half, rng)
                if path is None:
                    continue
                mean, least = seen(sp, path)
                length = sum(it.norm(it.sub(b, a)) for a, b in zip(path, path[1:]))
                score = mean * min(1.0, length / (0.24 * open_ * LENGTH))
                if near is None or mean + least > near[0] + near[1]:
                    near = (mean, least)
                if mean >= SEEN and least >= LEAST and (best is None or score > best[0]):
                    best = (score, {"path": path, "open": open_, "seen": round(mean, 3)}, length, least)
        finally:
            sp.close()
        if best:
            print(f"  {enclosed:.0%} enclosed: {best[2]:.1f} m long, openness {best[1]['open']:.2f} m of {half:.1f}, seen {best[1]['seen']:.0%} (least {best[3]:.0%}), {sp.casts} raycasts", flush=True)
            return best[1]
        print(f"  {enclosed:.0%} enclosed: no walk with room and a view ({len(starts)} start points" + (f"; the best seen {near[0]:.0%}, least {near[1]:.0%}" if near else "") + f", {sp.casts} raycasts)", flush=True)
    return None


def main():
    args = sys.argv[1:]
    work = os.path.abspath(args[0])
    md.SCOUT = "--scout" in args
    only_plan = "--plan" in args
    cat = catalog()
    names = [a for a in args[1:] if not a.startswith("--")] or list(cat)
    for d in ("paths", "scenes"):
        os.makedirs(os.path.join(work, d), exist_ok=True)
    for name in names:
        node, _, half_cat = cat[name]
        path_file = os.path.join(work, "paths", name + ".json")
        if not os.path.exists(path_file):
            print(f"{name}: planning", flush=True)
            scene_file = os.path.join(work, "scenes", name + ".json")
            json.dump(scene_for(name, node, GROW), open(scene_file, "w"), indent=1)
            plan_ = None if name in ORBIT else decide(name, GROW * half_cat, scene_file)
            json.dump(plan_ or {"orbit": True}, open(path_file, "w"))
    if only_plan:
        return
    v = Viewer(md.SOCKET)
    v.up()
    take = lambda length: md.Take(v, work, name, length)
    for name in names:
        done = os.path.join(work, "scout" if md.SCOUT else "clips", name + ".mp4")
        if os.path.exists(done):
            print(f"{name}: done already", flush=True)
            continue
        print(f"{name}:", flush=True)
        node, outside, half_cat = cat[name]
        # (read now: one changed while the others render is taken as it is)
        plan_ = json.load(open(os.path.join(work, "paths", name + ".json")))
        scene_file = os.path.join(work, "scenes", name + ".json")
        if plan_.get("path"):
            json.dump(scene_for(name, node, GROW), open(scene_file, "w"), indent=1)
            md.setup(v, scene_file, settings={"headlight": 1.0, "headlight_shadows": False, "headlight_reach": max(4.0 * plan_["open"], 1.0)})
            it.walk(v, take, plan_["path"], plan_["open"], looks=level(plan_["path"]) if name in LEVEL else None)
        else:
            # (round it at its catalog size, a little above, from an angle
            # its name picks)
            json.dump(scene_for(name, outside, 1.0), open(scene_file, "w"), indent=1)
            md.setup(v, scene_file, settings={"headlight": 0.0, "headlight_shadows": False})
            radius, heights, angles = orbit_of(name, half_cat)
            a0 = math.radians(angles[0])
            v("camera", position=[radius * math.sin(a0), heights[0], radius * math.cos(a0)], look_at=[0, 0, 0])
            with take(LENGTH) as t:
                md.orbit(t, [0, 0, 0], radius, heights, angles, (0.0, LENGTH), legs=it.LEGS)
                t.end()


def level(path):
    """Where the camera looks from each point of a walk to look level, along
    the walk's heading (its first point to its last, flattened)."""
    h = [path[-1][0] - path[0][0], 0.0, path[-1][2] - path[0][2]]
    n = math.hypot(h[0], h[2])
    return [[p[0] + 3.0 * h[0] / n, p[1], p[2] + 3.0 * h[2] / n] for p in path]


def orbit_of(name, half_cat):
    """The orbit round one with no inside to walk: its radius, its heights
    from start to end, its angles (degrees, from an angle its name picks)."""
    radius = 1.8 * half_cat + 0.2
    a0 = zlib.crc32(name.encode()) % 360
    return radius, (0.55 * radius, 0.15 * radius), (a0, a0 + 140)


if __name__ == "__main__":
    main()
