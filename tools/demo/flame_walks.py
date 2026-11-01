#!/usr/bin/env python3
"""Flies into each fractal flame preset, 10 s each, into WORKDIR/clips/flame_NAME.mp4:
the flame alone at night (no sky: the flames' own glow is all there is),
big (SIZE metres across), the camera swinging round it and in close, among
its threads. Not all the way in: inside, the voxels' grid shows. The presets
and their palettes are assets/flame-presets.json's.
    python3 tools/demo/flame_walks.py WORKDIR [--scout] [name ...]
"""
import json
import math
import os
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import make_demo as md  # noqa: E402
from viewer import Viewer  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
LENGTH = 10.0
LEGS = 20
# the flame's longest side (m), and the camera's distance from its centre
# from start to end, its height and how far round it swings (degrees)
SIZE = 10.0
RADII = (0.85 * SIZE, 0.33 * SIZE)
HEIGHTS = (0.25 * SIZE, 0.04 * SIZE)
SWING = 75.0
# the dense ones, cloud-like, stop further out (closer in, they are fog)
CLOSEST = {"cumulus": 0.6, "snowflake": 0.6, "pearl": 0.55, "galaxy": 0.55, "starfish": 0.5, "supernova": 0.45}
# finer than the presets' picture: it is seen up close
FLAME = {"glow": 8, "density": 2, "resolution": 256, "points": 128}
NIGHT = {"preset": "golden", "time_of_day": 23, "sky": "off", "fog_density": 0}


def presets():
    """The presets in assets/flame-presets.json's order: name -> its node there."""
    sc = json.load(open(os.path.join(ROOT, "assets", "flame-presets.json")))
    return {o["preset"]: o for o in sc["objects"] if o.get("type") == "flame"}


def scene_for(node):
    node = {k: v for k, v in node.items() if k not in ("position", "name", "size", "scale")}
    node.update(FLAME, size=SIZE)
    return {"environment": NIGHT, "objects": [node], "camera": {"position": [0, 0, RADII[0]], "look_at": [0, 0, 0], "fov": 60}}


def main():
    args = sys.argv[1:]
    work = os.path.abspath(args[0])
    md.SCOUT = "--scout" in args
    every = presets()
    names = [a for a in args[1:] if not a.startswith("--")] or list(every)
    os.makedirs(os.path.join(work, "scenes"), exist_ok=True)
    v = Viewer(md.SOCKET)
    v.up()
    for name in names:
        clip = "flame_" + name
        done = os.path.join(work, "scout" if md.SCOUT else "clips", clip + ".mp4")
        if os.path.exists(done):
            print(f"{clip}: done already", flush=True)
            continue
        print(f"{clip}:", flush=True)
        scene_file = os.path.join(work, "scenes", clip + ".json")
        json.dump(scene_for(every[name]), open(scene_file, "w"), indent=1)
        md.setup(v, scene_file, settings={"headlight": 0.0})
        # (from an angle its name picks, round and in, easing in at the end)
        a0 = zlib.crc32(name.encode()) % 360
        r0, r1 = RADII[0], CLOSEST.get(name, RADII[1] / SIZE) * SIZE
        at = lambda k: (math.radians(a0 + SWING * k), r0 + (r1 - r0) * (1 - (1 - k) ** 2), HEIGHTS[0] + (HEIGHTS[1] - HEIGHTS[0]) * k)
        pos = lambda k: (lambda a, r, h: [r * math.sin(a), h, r * math.cos(a)])(*at(k))
        v("camera", position=pos(0), look_at=[0, 0, 0])
        with md.Take(v, work, clip, LENGTH) as t:
            for i in range(1, LEGS + 1):
                v("glide", position=pos(i / LEGS), look_at=[0, 0, 0], seconds=LENGTH / LEGS, even=True, queue=i > 1)
            t.end()


if __name__ == "__main__":
    main()
