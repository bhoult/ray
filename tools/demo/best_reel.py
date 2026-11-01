#!/usr/bin/env python3
"""The best of the fractals' 10 s clips (catalog_walks.py, flame_walks.py,
originals.py, interiors.py) in one reel: a title card, three seconds of each
with its name, quick crossfades, an end card; 1920x1080 at 30 frames a
second, no sound.
    python3 tools/demo/best_reel.py CLIPS OUT.mp4
CLIPS is the clips' folder (~/Videos/ray-clips). The captions are the
fractals' page titles (docs/fractals/NAME.md). Needs ffmpeg, PIL and Lato.
"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from assemble import FPS, H, ROOT, W, caption, card, duration  # noqa: E402

SEG = 3.0
FADE = 0.35
CARD = 3.0
# each: the clip (in CLIPS) and the second its three begin at; mostly late,
# where the walks are deep in and the orbits close, earlier where the end
# is weaker (burning_ship's turns dark, horned_sphere's edge-on, the
# cathedral's and mandalay's halls are longest near the start)
SHOTS = [
    ("interiors/clips/menger-1.mp4", 5.5),
    ("catalog/clips/indra_cells.mp4", 5.5),
    ("flames/clips/flame_wildfire.mp4", 5.5),
    ("catalog/clips/kleinian4d.mp4", 5.5),
    ("interiors/clips/mandelbox-2.mp4", 5.5),
    ("catalog/clips/burning_ship.mp4", 3.5),
    ("originals/clips/cathedral.mp4", 1.0),
    ("flames/clips/flame_sierpinski.mp4", 4.0),
    ("interiors/clips/kleinian-2.mp4", 5.5),
    ("catalog/clips/mosely.mp4", 4.0),
    ("catalog/clips/msltoe_donut.mp4", 4.0),
    ("originals/clips/octet_truss.mp4", 5.5),
    ("flames/clips/flame_starfish.mp4", 5.5),
    ("catalog/clips/mandalay.mp4", 1.0),
    ("catalog/clips/menger4d.mp4", 5.5),
    ("catalog/clips/zorich_pagodas.mp4", 5.5),
    ("flames/clips/flame_supernova.mp4", 5.5),
    ("interiors/clips/kifs-1.mp4", 5.5),
    ("originals/clips/cusp_vaults.mp4", 5.5),
    ("catalog/clips/mandeltorus.mp4", 3.0),
    ("catalog/clips/jos_kleinian.mp4", 5.5),
    ("flames/clips/flame_pagoda.mp4", 3.0),
    ("catalog/clips/kleinian_mod.mp4", 5.5),
    ("catalog/clips/sierpinski4d.mp4", 5.5),
    ("originals/clips/labyrinth_sponge.mp4", 5.5),
    ("flames/clips/flame_jellyfish.mp4", 4.0),
    ("catalog/clips/pine_tree.mp4", 5.5),
    ("originals/clips/horned_sphere.mp4", 1.5),
    ("catalog/clips/kaliset.mp4", 3.0),
    ("flames/clips/flame_galaxy.mp4", 5.5),
    ("catalog/clips/schottky.mp4", 4.0),
    ("interiors/clips/mandelbulb-2.mp4", 3.0),
    ("interiors/clips/sierpinski-1.mp4", 5.5),
]


def title(clip):
    """The fractal's name on screen: its page's title (a flame's said to be one)."""
    name = os.path.basename(clip)[:-4].removesuffix("-1").removesuffix("-2")
    page = open(os.path.join(ROOT, "docs", "fractals", name + ".md")).read().split("\n")
    text = next(ln for ln in page if ln.startswith("# "))[2:].split(" (`")[0]
    return text + ", a fractal flame" if name.startswith("flame_") else text


def main():
    clips, out = os.path.abspath(os.path.expanduser(sys.argv[1])), sys.argv[2]
    tmp = tempfile.mkdtemp()
    card(f"{tmp}/title.png", 1600)
    card(f"{tmp}/end.png", 1200, "github.com/bhoult/ray")
    args, chains, n = ["nice", "ffmpeg", "-v", "error", "-y"], [], 0
    # (each piece runs a fade longer than it shows: the next fades in over its end)
    pieces = [("card", f"{tmp}/title.png", 0.0, CARD + FADE)]
    pieces += [("clip", os.path.join(clips, c), start, SEG + FADE) for c, start in SHOTS]
    pieces.append(("card", f"{tmp}/end.png", 0.0, CARD))
    for k, (kind, src, start, secs) in enumerate(pieces):
        if kind == "clip":
            if start + secs > duration(src):
                raise SystemExit(f"{src}: {start + secs:.2f} s wanted, {duration(src):.2f} s there")
            # (seeking the input: only the three seconds are decoded)
            args += ["-ss", f"{start:.3f}", "-t", f"{secs:.3f}", "-i", src]
            v = f"[{n}:v]setpts=PTS-STARTPTS,fps={FPS},scale={W}:{H}:flags=lanczos,setsar=1"
        else:
            args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{secs:.3f}", "-i", src]
            v = f"[{n}:v]fps={FPS},scale={W}:{H},setsar=1"
        n += 1
        if kind == "clip":
            caption(title(src), f"{tmp}/caption{k}.png")
            args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{secs:.3f}", "-i", f"{tmp}/caption{k}.png"]
            chains.append(f"[{n}:v]format=rgba,fade=t=in:st=0.3:d=0.3:alpha=1,fade=t=out:st={secs - 0.75:.3f}:d=0.3:alpha=1[c{k}]")
            chains.append(f"{v}[b{k}]")
            chains.append(f"[b{k}][c{k}]overlay=0:0:shortest=1,format=yuv420p[p{k}]")
            n += 1
        else:
            chains.append(f"{v},format=yuv420p[p{k}]")
    length, last = pieces[0][3], "p0"
    for k in range(1, len(pieces)):
        offset = length - FADE
        chains.append(f"[{last}][p{k}]xfade=transition=fade:duration={FADE}:offset={offset:.3f}[x{k}]")
        last, length = f"x{k}", offset + pieces[k][3]
    chains.append(f"[{last}]fade=t=in:st=0:d=0.5,fade=t=out:st={length - 0.8:.3f}:d=0.8,scale=out_range=tv,format=yuv420p[out]")
    # (18 of the 24 threads: x264 takes them all otherwise)
    args += ["-filter_complex", ";".join(chains), "-map", "[out]", "-r", str(FPS), "-c:v", "libx264", "-preset", "slow", "-crf", "18",
             "-threads", "18", "-pix_fmt", "yuv420p", "-color_range", "tv", "-movflags", "+faststart", out]
    print(f"{len(SHOTS)} fractals, {length:.2f} s")
    subprocess.run(args, check=True)
    print(f"wrote {out}: {duration(out):.2f} s")


if __name__ == "__main__":
    main()
