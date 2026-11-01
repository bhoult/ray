#!/usr/bin/env python3
"""The fractal pages' animations: each fractal's 10 s clip (catalog_walks.py,
flame_walks.py, originals.py, interiors.py) as a small looping preview,
docs/images/fractals/anim/NAME.avif (640 wide, 15 fps, AV1: a tenth of
what WebP takes for the same), and on its page, under its description, that
preview linked to the full 1080p clip on Google Drive.
    python3 tools/demo/previews.py CLIPS REMOTE [--encode] [name ...]
CLIPS is the clips' folder (~/Videos/ray-clips: catalog/, flames/, originals/
and interiors/ in it), REMOTE the same tree's rclone copy on Drive
(gdrive:ray-clips), shared by link; each file's link is its Drive id, kept in
CLIPS/drive-links.json too. --encode makes the previews missing; a page
whose clips are not on Drive yet is left alone.
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import originals  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
PAGES = os.path.join(ROOT, "docs", "fractals")
ANIM = os.path.join(ROOT, "docs", "images", "fractals", "anim")
# (lp 4: four threads, well under the load cap)
ENCODE = ["-vf", "fps=15,scale=640:-2:flags=lanczos,format=yuv420p", "-c:v", "libsvtav1", "-preset", "6",
          "-crf", "46", "-g", "150", "-svtav1-params", "lp=4", "-loop", "0", "-an"]


def clips_of(clips, name):
    """A page's clips (paths in CLIPS, the previewed one first) and how they move."""
    if name.startswith("flame_"):
        return [f"flames/clips/{name}.mp4"], "swinging in among its threads"
    if os.path.exists(os.path.join(clips, "catalog", "clips", name + ".mp4")):
        plan = json.load(open(os.path.join(clips, "catalog", "paths", name + ".json")))
        return [f"catalog/clips/{name}.mp4"], "circling it" if plan.get("orbit") else "inside it"
    if name in originals.SHOTS:
        return [f"originals/clips/{name}.mp4"], "circling it" if originals.SHOTS[name][3][0] == "orbit" else "inside it"
    if os.path.exists(os.path.join(clips, "interiors", "clips", name + "-1.mp4")):
        return [f"interiors/clips/{name}-{i}.mp4" for i in (1, 2)], "inside it"
    return [], None


def drive_links(clips, remote):
    """Each clip on Drive (its path in CLIPS) -> its link."""
    out = subprocess.run(["rclone", "lsjson", "-R", "--files-only", remote], capture_output=True, text=True, check=True)
    links = {e["Path"]: f"https://drive.google.com/file/d/{e['ID']}/view" for e in json.loads(out.stdout)}
    json.dump(links, open(os.path.join(clips, "drive-links.json"), "w"), indent=1, sort_keys=True)
    return links


def block(name, title, paths, how, links):
    """The page's Animation paragraph and its preview, as lines."""
    url = links[paths[0]]
    if len(paths) == 1:
        text = f"**Animation.** Ten seconds {how}, in full 1080p [on Google Drive]({url}); below, a small preview of it."
    else:
        text = (f"**Animation.** Two walks of ten seconds {how}, in full 1080p on Google Drive: "
                f"[the first]({url}), previewed below, and [the second]({links[paths[1]]}).")
    img = f'[<img src="../images/fractals/anim/{name}.avif" width="640" alt="{title}, animated">]({url})'
    return [text, "", img]


def place(lines, new):
    """The page with its Animation block replaced, or put after the paragraph under its picture."""
    if any(ln.startswith("**Animation.**") for ln in lines):
        i = next(i for i, ln in enumerate(lines) if ln.startswith("**Animation.**"))
        return lines[:i] + new + lines[i + 3:]
    i = next(i for i, ln in enumerate(lines) if ln.startswith("![") and "../images/fractals/" in ln)
    j = next(j for j in range(i + 1, len(lines)) if lines[j].strip())
    k = next((k for k in range(j, len(lines)) if not lines[k].strip()), len(lines))
    return lines[:k] + [""] + new + lines[k:]


def main():
    args = sys.argv[1:]
    clips, remote = os.path.abspath(os.path.expanduser(args[0])), args[1]
    names = [a for a in args[2:] if not a.startswith("--")] or sorted(
        f[:-3] for f in os.listdir(PAGES) if f.endswith(".md") and f != "flame.md")
    os.makedirs(ANIM, exist_ok=True)
    links = drive_links(clips, remote)
    for name in names:
        paths, how = clips_of(clips, name)
        if not paths:
            print(f"{name}: no clip", flush=True)
            continue
        preview = os.path.join(ANIM, name + ".avif")
        if "--encode" in args and not os.path.exists(preview):
            subprocess.run(["nice", "ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", os.path.join(clips, paths[0])]
                           + ENCODE + [preview], check=True, stderr=subprocess.DEVNULL)
        missing = [p for p in paths if p not in links]
        if missing or not os.path.exists(preview):
            print(f"{name}: waiting for {', '.join(missing) or 'its preview'}", flush=True)
            continue
        page = os.path.join(PAGES, name + ".md")
        lines = open(page).read().split("\n")
        title = next(ln for ln in lines if ln.startswith("# "))[2:].split(" (`")[0]
        open(page, "w").write("\n".join(place(lines, block(name, title, paths, how, links))))
        print(f"{name}: {how}", flush=True)


if __name__ == "__main__":
    main()
