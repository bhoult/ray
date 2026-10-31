#!/usr/bin/env python3
"""What a screenshot from the viewer (P / F12) carries: the scene as it was
(its unsaved edits, the camera exactly, every render setting: a document
that opens as a scene), and the moment (the scene file it was taken of,
when, the image's size, the walker's feet and height in game mode). A
director starts from one by opening it (make_demo.setup(v, "shot.png")) and
reading its pose here.
    python3 tools/demo/still.py SHOT.png [--scene OUT.json]
prints the moment and the camera (--scene: the scene into OUT.json too).
"""
import json
import struct
import sys
import zlib


def texts(path):
    """The PNG's iTXt chunks, keyword to text."""
    data = open(path, "rb").read()
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise ValueError(f"{path}: not a PNG")
    out, i = {}, 8
    while i + 8 <= len(data):
        n, kind = struct.unpack(">I4s", data[i:i + 8])
        body = data[i + 8:i + 8 + n]
        if kind == b"iTXt":
            key, rest = body.split(b"\0", 1)
            compressed, text = rest[0], rest[2:].split(b"\0", 2)[2]
            out[key.decode("latin-1")] = (zlib.decompress(text) if compressed else text).decode()
        if kind in (b"IDAT", b"IEND"):
            break
        i += 12 + n
    return out


def read(path):
    """The screenshot's scene document and its moment."""
    t = texts(path)
    if "ray-scene" not in t:
        raise ValueError(f"{path}: no scene in it (only ray's screenshots carry one)")
    return json.loads(t["ray-scene"]), json.loads(t.get("ray-still", "{}"))


def pose(path):
    """The camera's position, yaw and pitch (degrees) in the screenshot."""
    c = read(path)[0]["camera"]
    return c["position"], c["yaw"], c["pitch"]


def main():
    path = sys.argv[1]
    scene, moment = read(path)
    print(json.dumps({"moment": moment, "camera": scene["camera"]}, indent=2))
    if "--scene" in sys.argv:
        out = sys.argv[sys.argv.index("--scene") + 1]
        with open(out, "w") as f:
            json.dump(scene, f, indent=2)
            f.write("\n")
        print(f"scene into {out}")


if __name__ == "__main__":
    main()
