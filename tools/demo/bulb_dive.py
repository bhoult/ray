#!/usr/bin/env python3
"""Into the Mandelbulb, a video of its own: the camera dives into a hole in
the bulb's shoulder, shrinks into the basin at its bottom, turns into the
dark under an overhang and goes through by headlight, comes out into the sun
on the far side, looks back and pulls away until the whole bulb shows; the
bulb's material morphs on the way (orbit traps throughout, so they hold at
any scale). Drives the viewer of tools/demo/session.sh like make_demo.py and
records it with render_video in pieces of PIECE seconds into
WORKDIR/clips/dive<k>.mp4 (pieces already there are kept: run it again after
a failure), joined into dive.mp4 (--scout: a third of the size and 2
samples, into WORKDIR/scout), then captions it between a title and an end
card into OUT.
    python3 tools/demo/bulb_dive.py WORKDIR OUT.mp4 [--scout] [--assemble]
(--assemble: only put the recorded take together)
"""
import math
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import assemble as asm  # noqa: E402
import make_demo as md  # noqa: E402
from viewer import Viewer  # noqa: E402

FPS = 30
LENGTH = 42.0
# the hole: a well about 12 cm across and 22 cm deep between bud clusters on
# the bulb's shoulder, open to the sky; the bottom's height by raycast
FLOOR = [-0.367, 3.4333, -7.048]
# a centimetre above it the walls are about 5 mm off; from there, along this
# heading, a tunnel runs under an overhang with 6 to 10 mm of room all round
# and comes out on the bulb's far side about 25 cm on
EYE = [-0.367, 3.444, -7.048]
TUNNEL = (-48.0, -6.0)
CENTRE = [0.213, 2.762, -6.448]

# the materials the bulb morphs through ("bulb skin": its own, so the cut
# sponge, which shares "copper orbit", stays as it is); which of a
# material's two colours shows depends on the place: in the hole jade shows
# its gold, ruby (glowing) its red and porcelain its white
COPPER = {"color": [0.95, 0.55, 0.35], "color2": [0.15, 0.5, 0.5], "metallic": 1, "metallic2": 0.2, "texture": "orbit", "texture_scale": 2, "roughness": 0.25}
GOLD = {"color": [0.3, 0.65, 0.45], "color2": [0.95, 0.8, 0.35], "metallic": 0, "metallic2": 1, "texture": "orbit", "texture_scale": 2, "roughness": 0.3}
RUBY = {"color": [0.75, 0.05, 0.07], "emission": [0.5, 0.03, 0.02], "color2": [0.13, 0.13, 0.14], "metallic": 0, "metallic2": 1, "texture": "orbit", "texture_scale": 2.5, "roughness": 0.3}
PORCELAIN = {"color": [0.15, 0.25, 0.65], "color2": [0.92, 0.92, 0.96], "metallic": 0, "metallic2": 0, "texture": "orbit", "texture_scale": 6, "roughness": 0.45}

# (seconds into the clip, caption)
CAPTIONS = [
    (0.5, 6.3, "Into a hole in the Mandelbulb, 12 cm across"),
    (6.8, 11.3, "Shrinking to a few millimeters; the headlight comes on as it darkens"),
    (11.8, 17.0, "Its material morphs as we go: verdigris to gold"),
    (17.5, 22.0, "Through a tunnel under an overhang of the bulb"),
    (22.5, 27.0, "Glowing ruby: orbit-trap colors hold at any scale"),
    (27.5, 33.0, "Looking back, the bulb turns to porcelain: white, cobalt in the folds"),
    (33.5, 41.5, "Back out: the whole bulb, 1.4 m across, on its plinth"),
]


def smooth(u):
    u = min(max(u, 0.0), 1.0)
    return u * u * (3 - 2 * u)


def lerp(a, b, k):
    return a + (b - a) * k


def blend(a, b, k):
    """Material a eased toward b by k: numbers and colours in between (a
    field one lacks counts as 0), the rest b's from halfway."""
    out = {}
    for key in set(a) | set(b):
        x, y = a.get(key), b.get(key)
        if isinstance(x, list) or isinstance(y, list):
            x, y = x or [0, 0, 0], y or [0, 0, 0]
            out[key] = [round(lerp(p, q, k), 4) for p, q in zip(x, y)]
        elif isinstance(x, (int, float)) or isinstance(y, (int, float)):
            out[key] = round(lerp(x or 0, y or 0, k), 4)
        else:
            out[key] = y if k >= 0.5 and y is not None else (x if x is not None else y)
    return out


def heading(yaw, pitch):
    y, p = math.radians(yaw), math.radians(pitch)
    return [-math.sin(y) * math.cos(p), math.sin(p), -math.cos(y) * math.cos(p)]


def facing(frm, to):
    """The yaw and pitch (degrees) from one point toward another."""
    d = [b - a for a, b in zip(frm, to)]
    n = math.sqrt(sum(c * c for c in d))
    return math.degrees(math.atan2(-d[0], -d[2])), math.degrees(math.asin(d[1] / n))


def along(t):
    """The point t metres along the tunnel from EYE."""
    h = heading(*TUNNEL)
    return [EYE[k] + h[k] * t for k in range(3)]


# the tunnel's distance between 13.5 and 29 s (eased in and out) and the
# point where it ends
T_IN, T_OUT, DEEP = 14.0, 29.0, 0.5
STOP = along(DEEP)


def pose(t):
    """Where the camera is at scene time t, and its yaw and pitch."""
    if t < 6.5:
        # the dive: straight down, the height shrinking by the same factor
        # each moment (a zoom), levelling out toward the basin's open side
        u = t / 6.5
        h = math.exp(lerp(math.log(1.6), math.log(0.08), smooth(u)))
        k = smooth((u - 0.55) / 0.45)
        return [FLOOR[0], FLOOR[1] + h, FLOOR[2]], lerp(0, 70, k), lerp(-88, -12, k)
    if t < 10.5:
        # shrinking: down toward the floor (as a walker shrinking with its
        # feet there sees it), looking round
        u = (t - 6.5) / 4.0
        h = math.exp(lerp(math.log(0.08), math.log(EYE[1] - FLOOR[1]), smooth(u)))
        return [FLOOR[0], FLOOR[1] + h, FLOOR[2]], lerp(70, 100, smooth(u)), lerp(-12, -4, smooth(u))
    if t < T_IN:
        # round to the dark under the overhang
        u = smooth((t - 10.5) / (T_IN - 10.5))
        return list(EYE), lerp(100, TUNNEL[0], u), lerp(-4, TUNNEL[1], u)
    if t < 33.0:
        # through, out into the sun, and round to look back up at the bulb
        s = DEEP * smooth((t - T_IN) / (T_OUT - T_IN))
        look_up = 8 * math.sin(math.pi * smooth((t - 15.0) / 6.0))
        k = smooth((t - 25.0) / 5.0)
        return along(s), lerp(TUNNEL[0], TUNNEL[0] - 180, k), lerp(TUNNEL[1] + look_up, 20, k)
    # away from the bulb's centre until all of it shows, turning to it
    u = (t - 33.0) / (LENGTH - 33.0)
    out = [s - c for s, c in zip(STOP, CENTRE)]
    r0 = math.sqrt(sum(c * c for c in out))
    r = math.exp(lerp(math.log(r0), math.log(3.6), smooth(u)))
    pos = [CENTRE[k] + out[k] / r0 * r for k in range(3)]
    yaw, pitch = facing(pos, CENTRE)
    # (the look-back's yaw, unwound, eased into the bulb's direction)
    yaw0 = TUNNEL[0] - 180
    yaw = yaw0 + ((yaw - yaw0 + 180) % 360 - 180)
    k = smooth(u / 0.4)
    return pos, lerp(yaw0, yaw, k), lerp(20, pitch, k)


def skin(t):
    """The bulb's material at scene time t."""
    for t0, t1, a, b in ((11.0, 14.0, COPPER, GOLD), (18.5, 21.5, GOLD, RUBY), (27.0, 31.0, RUBY, PORCELAIN)):
        if t < t0:
            return a
        if t < t1:
            return blend(a, b, smooth((t - t0) / (t1 - t0)))
    return PORCELAIN


def light(t):
    """The headlight at scene time t: on as the basin darkens, and on
    through the look back, which faces into the sun."""
    return round(smooth((t - 8.8) / 0.8) * (1 - smooth((t - 33.5) / 1.5)), 3)


# the video in pieces of PIECE seconds, each its own take (a failure costs
# one; those already recorded are kept)
PIECE = 7.0


def take_piece(v, work, k):
    """Piece k: its first frame where the last one's left off (the camera,
    the material and the headlight are functions of the time). Each frame
    n+1's material and headlight are sent while frame n is drawn (its job
    posted already, so they land in n+1); the path goes as glides of a fifth
    of a second, queued at once (each starts where and when the last ends,
    on the scene's clock)."""
    t0, t1 = k * PIECE, min(LENGTH, (k + 1) * PIECE)
    pos, yaw, pitch = pose(t0)
    md.setup(v, "saves/fractals.json", settings={"headlight": light(t0)}, position=pos, yaw_degrees=yaw, pitch_degrees=pitch)
    v("material", name="bulb skin", definition=skin(t0))
    v("update", object="mandelbulb", set={"material": "bulb skin"})
    frames = round((t1 - t0) * FPS)
    with md.Take(v, work, f"dive{k}", t1 - t0) as t:
        legs = round((t1 - t0) * 5)
        for i in range(1, legs + 1):
            p, y, pt = pose(t0 + (t1 - t0) * i / legs)
            v("glide", position=p, yaw_degrees=y, pitch_degrees=pt, seconds=(t1 - t0) / legs, even=True, queue=i > 1)
        for m in range(frames - 1):
            now, nxt = t0 + m / FPS, t0 + (m + 1) / FPS
            if skin(nxt) != skin(now) or light(nxt) != light(now):
                t.at(m / FPS)
                if skin(nxt) != skin(now):
                    v("material", name="bulb skin", definition=skin(nxt))
                if light(nxt) != light(now):
                    v("settings", set={"headlight": light(nxt)})
        t.end()


def take_dive(v, work, scout):
    """The pieces not yet recorded, then all of them joined into dive.mp4."""
    d = os.path.join(work, "scout" if scout else "clips")
    pieces = [os.path.join(d, f"dive{k}.mp4") for k in range(math.ceil(LENGTH / PIECE - 1e-9))]
    for k, f in enumerate(pieces):
        if not os.path.exists(f):
            print(f"piece {k}:", flush=True)
            take_piece(v, work, k)
    # (the same encoder and settings throughout: joined without encoding again)
    with open(os.path.join(d, "dive.txt"), "w") as f:
        f.writelines(f"file '{os.path.abspath(p)}'\n" for p in pieces)
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", os.path.join(d, "dive.txt"), "-c", "copy", os.path.join(d, "dive.mp4")], check=True)
    print(f"joined {len(pieces)} pieces: {asm.duration(os.path.join(d, 'dive.mp4')):.2f} s", flush=True)


def assemble(work, out, scout):
    """The title, the take with its captions, the end card; half-second
    crossfades."""
    tmp = os.path.join(work, "assemble-dive")
    os.makedirs(tmp, exist_ok=True)
    asm.card(f"{tmp}/title.png", 1400, "Into the Mandelbulb")
    asm.card(f"{tmp}/end.png", 1200, "github.com/bhoult/ray")
    clip = os.path.join(work, "scout" if scout else "clips", "dive.mp4")
    secs = asm.duration(clip) - 0.05
    W, H, F = asm.W, asm.H, asm.FADE
    args = ["ffmpeg", "-v", "error", "-y",
            "-loop", "1", "-framerate", str(FPS), "-t", "4", "-i", f"{tmp}/title.png",
            "-i", clip,
            "-loop", "1", "-framerate", str(FPS), "-t", "4", "-i", f"{tmp}/end.png"]
    chains = [f"[0:v]fps={FPS},scale={W}:{H},setsar=1,format=yuv420p[a]",
              f"[1:v]trim=0:{secs:.3f},setpts=PTS-STARTPTS,fps={FPS},scale={W}:{H},setsar=1[c0]",
              f"[2:v]fps={FPS},scale={W}:{H},setsar=1,format=yuv420p[z]"]
    for k, (t0, t1, text) in enumerate(CAPTIONS):
        asm.caption(text, f"{tmp}/caption{k}.png")
        args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{t1 - t0:.3f}", "-i", f"{tmp}/caption{k}.png"]
        chains.append(f"[{3 + k}:v]format=rgba,fade=t=in:st=0:d=0.4:alpha=1,fade=t=out:st={t1 - t0 - 0.4:.3f}:d=0.4:alpha=1,setpts=PTS+{t0:.3f}/TB[k{k}]")
        chains.append(f"[c{k}][k{k}]overlay=0:0:eof_action=pass[c{k + 1}]")
    n = len(CAPTIONS)
    chains.append(f"[c{n}]format=yuv420p[b]")
    chains.append(f"[a][b]xfade=transition=fade:duration={F}:offset={4 - F:.3f}[ab]")
    total = 4 - F + secs - F + 4
    chains.append(f"[ab][z]xfade=transition=fade:duration={F}:offset={4 - F + secs - F:.3f}[x]")
    chains.append(f"[x]fade=t=in:st=0:d=0.6,fade=t=out:st={total - 0.8:.3f}:d=0.8,scale=out_range=tv,format=yuv420p[out]")
    args += ["-filter_complex", ";".join(chains), "-map", "[out]", "-r", str(FPS), "-c:v", "libx264", "-preset", "slow", "-crf", "18",
             "-pix_fmt", "yuv420p", "-color_range", "tv", "-movflags", "+faststart", out]
    subprocess.run(args, check=True)
    print(f"wrote {out}: {asm.duration(out):.2f} s")


def main():
    work, out = sys.argv[1], sys.argv[2]
    scout = "--scout" in sys.argv
    md.SCOUT = scout
    if "--assemble" not in sys.argv:
        v = Viewer(md.SOCKET)
        v.up()
        take_dive(v, work, scout)
    assemble(work, out, scout)


if __name__ == "__main__":
    main()
