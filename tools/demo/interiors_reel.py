#!/usr/bin/env python3
"""Joins tools/demo/interiors.py's walks into one reel: each clip with its
caption (assemble.py's style) for its first few seconds, half-second
crossfades between them, 1920x1080 at 30 frames a second, no sound.
    python3 tools/demo/interiors_reel.py WORKDIR OUT.mp4 [name ...]
(default: every clip in WORKDIR/clips, fractals in interiors.py's order)"""
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from assemble import FADE, caption, duration  # noqa: E402
from interiors import FRACTALS  # noqa: E402

# what each walk is called on screen
TITLES = {
    "menger": "Inside the Menger sponge",
    "mandelbulb": "Inside the Mandelbulb",
    "mandelbox": "Inside the Mandelbox",
    "julia": "Inside a quaternion Julia set",
    "sierpinski": "Inside the Sierpinski tetrahedron",
    "apollonian": "Inside an Apollonian packing",
    "kifs": "Inside a kaleidoscopic IFS",
    "kleinian": "Inside the pseudo-Kleinian",
    "beams": "Light through haze, from a screenshot",
}
# seconds each caption shows
SHOWN = 3.5


def main():
    work, out = sys.argv[1], sys.argv[2]
    clips = os.path.join(work, "clips")
    order = [f"{f}-{k}" for f in FRACTALS for k in (1, 2)] + ["beams"]
    names = sys.argv[3:] or [n for n in order if os.path.exists(os.path.join(clips, n + ".mp4"))]
    tmp = tempfile.mkdtemp()
    inputs, chains, last, offset = [], [], None, 0.0
    for k, name in enumerate(names):
        clip = os.path.join(clips, name + ".mp4")
        cap = os.path.join(tmp, f"{k}.png")
        caption(TITLES[name.split("-")[0]], cap)
        inputs += ["-i", clip, "-loop", "1", "-t", f"{SHOWN}", "-i", cap]
        a, c = 2 * k, 2 * k + 1
        chains.append(f"[{c}:v]format=rgba,fade=t=in:st=0.3:d=0.5:alpha=1,fade=t=out:st={SHOWN - 0.6}:d=0.5:alpha=1[c{k}]")
        chains.append(f"[{a}:v]setpts=PTS-STARTPTS,fps=30,scale=1920:1080:flags=lanczos,format=yuv420p[b{k}]")
        chains.append(f"[b{k}][c{k}]overlay=0:0:eof_action=pass,format=yuv420p[p{k}]")
        length = duration(clip)
        if last is None:
            last, offset = f"p{k}", length
            continue
        offset -= FADE
        chains.append(f"[{last}][p{k}]xfade=transition=fade:duration={FADE}:offset={offset:.3f}[x{k}]")
        last, offset = f"x{k}", offset + length
    cmd = ["ffmpeg", "-y", "-v", "error", *inputs, "-filter_complex", ";".join(chains), "-map", f"[{last}]", "-c:v", "libx264", "-crf", "18", "-preset", "slow", "-pix_fmt", "yuv420p", "-movflags", "+faststart", out]
    subprocess.run(cmd, check=True)
    print(f"{out}: {len(names)} walks, {duration(out):.1f} s")


if __name__ == "__main__":
    main()
