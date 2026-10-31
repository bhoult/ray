#!/usr/bin/env python3
"""Walks through and around the fractals invented for ray (docs/FRACTALS.md,
Originals), 10 s each, into WORKDIR/clips/NAME.mp4: the walkable ones
along a path found by raycasts (tools/demo/interiors.py's planner), the
cathedral down its nave to a chapel door, the solid ones in a slow close
orbit. Each stands alone in a scene of its own: the gallery's environment
and materials (saves/fractals.json), the headlight on inside.
    python3 tools/demo/originals.py WORKDIR [--scout] [--plan] [name ...]
(--scout: small and fast, into WORKDIR/scout; --plan: only the paths, into
WORKDIR/paths/NAME.json; a path already planned is used again.)
"""
import json
import math
import os
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import interiors as it  # noqa: E402
import make_demo as md  # noqa: E402
from viewer import Viewer  # noqa: E402

ROOT = it.ROOT
LENGTH = it.LENGTH

# each: the node (at the origin), its scale, the material, and how it is
# shown: ("plan", half, enclosed) walks a planned path through the box of
# that half size (enclosed: the share of rays from the start that must hit);
# ("orbit", radius, (h0, h1), (a0, a1)) circles it, looking at its centre;
# ("path", points, looks) glides through those points looking at those
SHOTS = {
    "labyrinth_sponge": ({"type": "labyrinth_sponge"}, 1.2, "jade orbit", ("plan", 7.0, 0.8)),
    "quasicrystal_rooms": ({"type": "quasicrystal_rooms"}, 6.0, "bone orbit", ("plan", 6.0, 0.8)),
    "screw_sponge": ({"type": "screw_sponge", "radius": 0.16}, 20.0, "copper orbit", ("plan", 14.0, 0.8)),
    "cusp_vaults": ({"type": "cusp_vaults", "position": [0, 1.15, 0]}, 4.0, "steel orbit", ("plan", 4.0, 0.8)),
    # (scale 5: the nave 10 m wide, eye height 1.7 m; a chapel door at x = -14.1 m, z = 0)
    "cathedral": ({"type": "cathedral"}, 5.0, "bone orbit", ("path",
        [[0, 1.7, 30], [0, 1.7, 20], [0, 1.7, 10], [-1, 1.7, 3], [-5, 1.7, 0.2], [-10, 1.7, 0]],
        [[0, 3, 0], [0, 3, -5], [-3, 3, -6], [-14, 2.5, -2], [-24, 2, 0], [-30, 1.8, 0]])),
    "nil_stair": ({"type": "nil_stair"}, 2.0, "brass", ("orbit", 4.6, (0.4, 2.6), (20, 200))),
    "mayer_julia": ({"type": "mayer_julia", "rotation": [0, 0, 90]}, 0.55, "jade orbit", ("orbit", 1.75, (0.2, 1.2), (-30, 120))),
    "dirac_frost": ({"type": "dirac_frost"}, 0.6, "coral", ("orbit", 1.9, (1.0, -0.2), (-100, 40))),
    "horned_sphere": ({"type": "horned_sphere"}, 0.6, "copper orbit", ("orbit", 1.3, (0.9, 0.4), (-40, 100))),
    # (100 units across: at 0.25 a block 25 m wide, its cells 12 m)
    "octet_truss": ({"type": "octet_truss"}, 0.25, "steel orbit", ("plan", 9.0, 0.5)),
}


def scene_for(name):
    """The fractal alone at the origin: the gallery's environment and materials."""
    base = json.load(open(os.path.join(ROOT, "saves", "fractals.json")))
    node, scale, mat, _ = SHOTS[name]
    node = dict(node, scale=scale, name=name, material=mat)
    out = {k: base[k] for k in ("environment", "materials", "render") if k in base}
    out["objects"] = [node]
    out["camera"] = {"position": [0, 1, 4], "yaw": 0, "pitch": 0, "fov": 60, "speed": 2}
    return out


def main():
    args = sys.argv[1:]
    work = os.path.abspath(args[0])
    md.SCOUT = "--scout" in args
    only_plan = "--plan" in args
    names = [a for a in args[1:] if not a.startswith("--")] or list(SHOTS)
    v = Viewer(md.SOCKET)
    v.up()
    for d in ("paths", "scenes"):
        os.makedirs(os.path.join(work, d), exist_ok=True)
    take = lambda length: md.Take(v, work, name, length)
    for name in names:
        done = os.path.join(work, "scout" if md.SCOUT else "clips", name + ".mp4")
        if os.path.exists(done) and not only_plan:
            print(f"{name}: done already", flush=True)
            continue
        print(f"{name}:", flush=True)
        scene_file = os.path.join(work, "scenes", name + ".json")
        json.dump(scene_for(name), open(scene_file, "w"), indent=1)
        _, scale, _, how = SHOTS[name]
        inside = how[0] != "orbit"
        md.setup(v, scene_file, settings={"headlight": 1.0 if inside else 0.0, "headlight_shadows": False})
        if how[0] == "plan":
            path_file = os.path.join(work, "paths", name + ".json")
            if os.path.exists(path_file):
                plan_ = json.load(open(path_file))
            else:
                _, half, enclosed = how
                v("settings", set={"scale": 0.1})
                sp = it.Space(v, 2.0 * half)
                try:
                    path, open_ = it.plan(sp, [0, 0, 0], half, zlib.crc32(name.encode()), enclosed=enclosed)
                except RuntimeError as e:
                    v("settings", set={"scale": 1.0})
                    print(f"  {name}: {e}, skipped", flush=True)
                    continue
                v("settings", set={"scale": 1.0})
                plan_ = {"path": path, "open": open_}
                json.dump(plan_, open(path_file, "w"))
                print(f"  planned {it.norm(it.sub(path[-1], path[0])):.1f} m end to end, openness {open_:.2f} m, {sp.casts} raycasts", flush=True)
            v("settings", set={"headlight_reach": max(4.0 * plan_["open"], 1.0)})
            if only_plan:
                continue
            it.walk(v, take, plan_["path"], plan_["open"])
        elif only_plan:
            continue
        elif how[0] == "path":
            _, pts, looks = how
            path = it.resample(pts, it.LEGS)
            lk = it.resample(looks, it.LEGS)
            v("settings", set={"headlight_reach": 20.0})
            it.walk(v, take, path, 1.0, looks=lk)
        else:
            _, radius, heights, angles = how
            a0 = math.radians(angles[0])
            v("camera", position=[radius * math.sin(a0), heights[0], radius * math.cos(a0)], look_at=[0, 0, 0])
            with take(LENGTH) as t:
                md.orbit(t, [0, 0, 0], radius, heights, angles, (0.0, LENGTH), legs=it.LEGS)
                t.end()


if __name__ == "__main__":
    main()
