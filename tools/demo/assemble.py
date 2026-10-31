#!/usr/bin/env python3
"""Puts the demo video together: a title card (the README's banner), the
editor (its README picture, pushed in slowly), the clips make_demo.py
recorded in its order, each with its caption, and an end card; half-second
crossfades between them; exactly LENGTH seconds (the piece named FLEX gives
or takes the difference), 1920x1080 at 30 frames a second, no sound.
    python3 tools/demo/assemble.py WORKDIR OUT.mp4
Needs ffmpeg, PIL and Lato (fonts-lato)."""
import os
import subprocess
import sys

from PIL import Image, ImageDraw, ImageFilter, ImageFont

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from make_demo import SEGMENTS  # noqa: E402

W, H, FPS = 1920, 1080, 30
LENGTH = 120.0
FADE = 0.5
# the piece that gives or takes the difference (the editor's still: any length)
FLEX = "editor"
# seconds cut off a clip's start (a walker settling, a first frame of noise)
TRIM = {"kleinian": 2.4, "bulb": 0.8}
ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "../.."))
LATO = "/usr/share/fonts/truetype/lato/Lato-{}.ttf"
EDITOR = ("editor", "The editor: every object's parameters and its share of the frame's cost; the CPU and GPU load below", 7.0)


def duration(path):
    out = subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path], capture_output=True, text=True, check=True)
    return float(out.stdout)


def caption(text, path):
    """The caption: white words on a dark, rounded box at the bottom left,
    a copper bar at its edge (the icon's colour)."""
    font = ImageFont.truetype(LATO.format("Medium"), 40)
    pad, bar = 24, 7
    w = int(font.getlength(text))
    box = (0, 0, w + 2 * pad + bar, 40 + 2 * pad)
    img = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    x0, y0 = 64, H - 64 - box[3]
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle((x0, y0 + 4, x0 + box[2], y0 + box[3] + 4), 14, fill=(0, 0, 0, 110))
    img = Image.alpha_composite(img, shadow.filter(ImageFilter.GaussianBlur(8)))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((x0, y0, x0 + box[2], y0 + box[3]), 14, fill=(12, 12, 16, 168))
    d.rounded_rectangle((x0, y0, x0 + bar + 14, y0 + box[3]), 14, fill=(232, 149, 90, 255))
    d.rectangle((x0 + bar, y0, x0 + bar + 14, y0 + box[3]), fill=(12, 12, 16, 168))
    d.text((x0 + bar + pad, y0 + pad - 4), text, font=font, fill=(255, 255, 255, 255))
    img.save(path)


def card(path, logo_width, line=None):
    """A card: the banner, rounded and lifted by a shadow, on a dark, warm
    glow."""
    logo = Image.open(os.path.join(ROOT, "docs/images/logo.jpg")).convert("RGBA")
    # the glow: brightest behind the banner, dark at the edges
    small = Image.new("RGB", (W // 8, H // 8))
    px = small.load()
    for y in range(small.height):
        for x in range(small.width):
            r = min(1.0, (((x + 0.5) / small.width - 0.5) ** 2 * 2.2 + ((y + 0.5) / small.height - 0.46) ** 2 * 3.0) ** 0.5)
            k = (1 - r) ** 1.6
            px[x, y] = (int(8 + 52 * k), int(6 + 30 * k), int(5 + 16 * k))
    bg = small.resize((W, H), Image.BICUBIC).convert("RGBA")
    fg = logo.resize((logo_width, int(logo.height * logo_width / logo.width)), Image.LANCZOS)
    mask = Image.new("L", fg.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, fg.width - 1, fg.height - 1), 22, fill=255)
    x, y = (W - fg.width) // 2, (H - fg.height) // 2 - (40 if line else 0)
    shadow = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle((x, y + 14, x + fg.width, y + fg.height + 14), 22, fill=(0, 0, 0, 150))
    bg = Image.alpha_composite(bg, shadow.filter(ImageFilter.GaussianBlur(24)))
    bg.paste(fg, (x, y), mask)
    if line:
        font = ImageFont.truetype(LATO.format("Medium"), 44)
        ImageDraw.Draw(bg).text(((W - font.getlength(line)) / 2, y + fg.height + 48), line, font=font, fill=(255, 228, 194))
    bg.convert("RGB").save(path)


def main():
    work, out = sys.argv[1], sys.argv[2]
    tmp = os.path.join(work, "assemble")
    os.makedirs(tmp, exist_ok=True)
    card(f"{tmp}/title.png", 1600)
    card(f"{tmp}/end.png", 1200, "github.com/bhoult/ray")
    # (kind, source, seconds, caption)
    # (kind, source, seconds, caption, seconds cut off its start)
    pieces = [("card", f"{tmp}/title.png", 4.0, None, 0.0), ("still", os.path.join(ROOT, "docs/images/editor.jpg"), EDITOR[2], EDITOR[1], 0.0)]
    for name, text, _ in SEGMENTS:
        clip = os.path.join(work, "clips", name + ".mp4")
        if not os.path.exists(clip):
            print(f"no clip {clip}: left out")
            continue
        skip = TRIM.get(name, 0.0)
        pieces.append(("clip", clip, duration(clip) - 0.05 - skip, text, skip))
    pieces.append(("card", f"{tmp}/end.png", 4.0, None, 0.0))
    # exactly LENGTH: the flexible clip gives or takes the difference
    total = sum(p[2] for p in pieces) - FADE * (len(pieces) - 1)
    i = next(i for i, p in enumerate(pieces) if os.path.splitext(os.path.basename(p[1]))[0] == FLEX)
    want = pieces[i][2] - (total - LENGTH)
    longest = pieces[i][2] if pieces[i][0] == "clip" else 12.0
    if want > longest + 1e-6 or want < 3:
        raise SystemExit(f"{FLEX} would need {want:.2f} s (it can have 3 to {longest:.2f}): record more or less footage")
    pieces[i] = pieces[i][:2] + (want,) + pieces[i][3:]
    for k, p in enumerate(pieces):
        print(f"{k:2} {p[0]:5} {p[2]:6.2f} s  {os.path.basename(p[1])}")

    args = ["ffmpeg", "-v", "error", "-y"]
    chains = []
    n = 0
    for k, (kind, src, secs, text, skip) in enumerate(pieces):
        if kind == "clip":
            args += ["-i", src]
            v = f"[{n}:v]trim={skip:.3f}:{skip + secs:.3f},setpts=PTS-STARTPTS,fps={FPS},scale={W}:{H}:force_original_aspect_ratio=increase,crop={W}:{H},setsar=1"
        elif kind == "still":
            # (upscaled first: zoompan rounds to whole pixels, and a slow
            # push-in at the output's size shudders)
            args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{secs:.3f}", "-i", src]
            frames = int(secs * FPS)
            v = (f"[{n}:v]scale={2 * W}:{2 * H}:force_original_aspect_ratio=increase,crop={2 * W}:{2 * H},"
                 f"zoompan=z='1+0.07*on/{frames}':d=1:x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':s={W}x{H}:fps={FPS},setsar=1")
        else:
            args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{secs:.3f}", "-i", src]
            v = f"[{n}:v]fps={FPS},scale={W}:{H},setsar=1"
        n += 1
        if text:
            caption(text, f"{tmp}/caption{k}.png")
            args += ["-loop", "1", "-framerate", str(FPS), "-t", f"{secs:.3f}", "-i", f"{tmp}/caption{k}.png"]
            fade_out = max(secs - 0.9, 1.0)
            chains.append(f"[{n}:v]format=rgba,fade=t=in:st=0.5:d=0.4:alpha=1,fade=t=out:st={fade_out:.3f}:d=0.4:alpha=1[c{k}]")
            chains.append(f"{v}[b{k}]")
            chains.append(f"[b{k}][c{k}]overlay=0:0:shortest=1,format=yuv420p[p{k}]")
            n += 1
        else:
            chains.append(f"{v},format=yuv420p[p{k}]")
    # the crossfades, one after another
    length = pieces[0][2]
    last = "p0"
    for k in range(1, len(pieces)):
        offset = length - FADE
        chains.append(f"[{last}][p{k}]xfade=transition=fade:duration={FADE}:offset={offset:.3f}[x{k}]")
        last = f"x{k}"
        length = offset + pieces[k][2]
    chains.append(f"[{last}]fade=t=in:st=0:d=0.6,fade=t=out:st={length - 0.8:.3f}:d=0.8,trim=0:{LENGTH},setpts=PTS-STARTPTS,scale=out_range=tv,format=yuv420p[out]")
    args += ["-filter_complex", ";".join(chains), "-map", "[out]", "-r", str(FPS), "-c:v", "libx264", "-preset", "slow", "-crf", "18", "-pix_fmt", "yuv420p", "-color_range", "tv", "-movflags", "+faststart", out]
    print(f"{len(pieces)} pieces, {length:.2f} s")
    subprocess.run(args, check=True)
    print(f"wrote {out}: {duration(out):.2f} s")


if __name__ == "__main__":
    main()
