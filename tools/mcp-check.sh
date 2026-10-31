#!/usr/bin/env bash
# Drives a viewer through the MCP server (`ray mcp`), as an assistant would,
# and checks every answer: a private headless GNOME Shell and viewer on a
# socket of their own, with preferences of their own (the quick save slot,
# screenshots and videos in a temporary folder: the user's are never
# touched), a copy of a saved scene opened, then edits, undo, a refused edit,
# commands by name, the quick save, a screenshot, screenshots reopened as their moment, saving (and failing to), get_out,
# game mode walking on held keys, a steady recording (its frames as many
# as the scene's clock moved on: 30 a second, at the preferences' size), and
# render_video (a size of its own, ending by itself after its seconds), and
# raycast's precision from far off.
# Exit 1 when an answer is wrong.
#   tools/mcp-check.sh
set -euo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/ray"
printf '{ "videos": "%s/videos", "screenshots": "%s/shots", "quick_save": "%s/quicksave.json" }\n' "$tmp" "$tmp" "$tmp" > "$tmp/ray/config.json"
cp saves/playground.json "$tmp/first.json"
cp saves/cornell.json "$tmp/second.json"
export XDG_CONFIG_HOME=$tmp GSETTINGS_BACKEND=memory RAY_SOCKET=$tmp/viewer.sock TMPDIR_CHECK=$tmp
python3 - "$tmp" <<'PY'
import json, sys
t = sys.argv[1]
reqs = [
    ("initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "check", "version": "1"}}),
    ("tools/list", {}),
    ("viewer_status", {}),
    ("open_scene", {"path": f"{t}/second.json"}),
    ("list_objects", {}),
    ("add_object", {"node": {"type": "sphere", "radius": 0.2, "position": [0, 1.2, 0], "name": "check ball"}}),
    ("list_objects", {}),
    ("update_object", {"object": "check ball", "set": {"radius": 0.3}}),
    ("undo", {}),
    ("undo", {}),
    ("list_objects", {}),
    ("add_object", {"node": {"type": "no such shape"}}),
    ("run_command", {"name": "headlight"}),
    ("run_command", {"name": "fly"}),
    ("run_command", {"name": "duplicate"}),
    ("quick_save", {}),
    ("quick_load", {}),
    ("screenshot", {"width": 200, "samples": 2, "timeout": 20}),
    ("raycast", {"x": 0, "z": 0}),
    ("save_scene", {"path": "/proc/not/here.json"}),
    ("save_scene", {"path": f"{t}/saved.json"}),
    ("run_command", {"name": "game_mode"}),
    ("viewer_status", {}),
    ("hold_keys", {"keys": ["w"], "seconds": 1.0}),
    ("screenshot", {"width": 64, "samples": 100000, "timeout": 1.5}),
    ("viewer_status", {}),
    ("set_preferences", {"set": {"video_steady": True, "video_size": [320, 180], "video_steady_samples": 2, "screenshot_samples": 4, "screenshot_size": [640, 360]}}),
    ("run_command", {"name": "record_video"}),
    ("glide_camera", {"turn_degrees": 30, "seconds": 1.0}),
    ("screenshot", {"width": 64, "samples": 100000, "timeout": 4}),
    ("run_command", {"name": "record_video"}),
    ("screenshot", {"width": 64, "samples": 100000, "timeout": 2}),
    ("set_preferences", {"set": {"no_such_preference": 1}}),
    ("glide_camera", {"turn_degrees": 5, "seconds": 10.0}),
    ("glide_camera", {"turn_degrees": 5, "seconds": 1.0, "queue": True}),
    ("render_video", {"width": 256, "height": 144, "samples": 4, "seconds": 0.5}),
    ("render_video", {}),
    # (a second session, once the video is whole: a steady frame stops at its
    # samples, so no screenshot waits)
    ("--", {}),
    ("viewer_status", {}),
    ("render_video", {"stop": True}),
    # (a fractal's hit from 900 m up lands where one from 2 m up does: the
    # march stops within a pixel's footprint as seen from the ray's start)
    ("add_object", {"node": {"type": "mandelbulb", "power": 8, "iterations": 10, "scale": 1, "position": [0, 40, 0], "name": "check bulb"}}),
    ("raycast", {"x": 0.11, "z": 0.07}),
    ("raycast", {"origin": [0.11, 42, 0.07], "direction": [0, -1, 0]}),
    # (screenshots carry the moment: one with the free camera, one in game
    # mode; a third session opens them, their names known by then)
    ("run_command", {"name": "game_mode"}),
    ("set_camera", {"position": [0.123456789012, 1.5, 3.25], "yaw_degrees": 12.5, "pitch_degrees": -7.25}),
    ("run_command", {"name": "screenshot"}),
    ("get_scene", {}),
    ("run_command", {"name": "game_mode"}),
    ("run_command", {"name": "screenshot"}),
    ("--", {}),
    ("open_scene", {"path": f"{t}/first.json"}),
    ("open_scene", {"path": "@SHOT1@"}),
    ("get_scene", {}),
    ("open_scene", {"path": "@SHOT2@"}),
    # (get out: the walker, put inside the check bulb, comes into game mode
    # as small as it gets, stuck; out it goes, as tall. Then the camera,
    # inside it out of game mode)
    ("set_camera", {"position": [0, 40, 0]}),
    ("viewer_status", {}),
    ("run_command", {"name": "get_out"}),
    ("run_command", {"name": "get_out"}),
    ("run_command", {"name": "game_mode"}),
    ("set_camera", {"position": [0, 40, 0]}),
    ("raycast", {"origin": [0, 40, 0], "direction": [1, 0, 0]}),
    ("run_command", {"name": "get_out"}),
    ("get_scene", {}),
    ("run_command", {"name": "get_out"}),
]
out, sessions = open(f"{t}/requests.jsonl", "w"), 1
def start():
    print(json.dumps({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": reqs[0][1]}), file=out)
    print(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}), file=out)
start()
for i, (name, args) in enumerate(reqs[1:], 1):
    if name == "--":
        sessions += 1
        out = open(f"{t}/requests{sessions}.jsonl", "w")
        start()
    elif name == "tools/list":
        print(json.dumps({"jsonrpc": "2.0", "id": i, "method": "tools/list", "params": {}}), file=out)
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": name, "arguments": args}}), file=out)
PY
display=ray-mcp-$$
dbus-run-session -- bash -c '
  gnome-shell --headless --wayland --no-x11 --wayland-display "$1" --virtual-monitor 1600x900 >/dev/null 2>&1 &
  shell=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$1" ] && break; sleep 0.1; done
  WAYLAND_DISPLAY="$1" ./target/release/ray "$2/first.json" > "$2/viewer.log" 2>&1 &
  app=$!
  for _ in $(seq 120); do [ -S "$RAY_SOCKET" ] && break; sleep 0.5; done
  timeout 300 ./target/release/ray mcp < "$2/requests.jsonl" > "$2/answers.jsonl" 2> "$2/mcp.err" || true
  # (the second recording whole: two videos, no log left)
  for _ in $(seq 120); do [ "$(ls "$2"/videos/*.mp4 2>/dev/null | wc -l)" -ge 2 ] && ! ls "$2"/videos/*.log >/dev/null 2>&1 && break; sleep 0.5; done
  timeout 60 ./target/release/ray mcp < "$2/requests2.jsonl" >> "$2/answers.jsonl" 2>> "$2/mcp.err" || true
  # (the screenshots just taken, oldest first, opened by the third)
  s1=$(ls -tr "$2"/shots/*.png 2>/dev/null | sed -n 1p); s2=$(ls -tr "$2"/shots/*.png 2>/dev/null | sed -n 2p)
  sed -e "s#@SHOT1@#$s1#" -e "s#@SHOT2@#$s2#" "$2/requests3.jsonl" > "$2/requests3b.jsonl"
  timeout 60 ./target/release/ray mcp < "$2/requests3b.jsonl" >> "$2/answers.jsonl" 2>> "$2/mcp.err" || true
  kill $app 2>/dev/null; wait $app 2>/dev/null || true
  kill $shell 2>/dev/null; wait $shell 2>/dev/null || true
' _ "$display" "$tmp" >/dev/null 2>&1 || true
python3 - "$tmp" <<'PY'
import base64, json, math, os, sys
t = sys.argv[1]
ans = {}
for line in open(f"{t}/answers.jsonl"):
    d = json.loads(line)
    ans[d.get("id")] = d
fails = []
def check(i, what, ok):
    print(f"{'ok  ' if ok else 'FAIL'} {i:2} {what}")
    if not ok:
        fails.append(i)
def text(i):
    r = ans.get(i, {}).get("result", {})
    c = r.get("content", [{}])
    return next((x.get("text", "") for x in c if x.get("type") == "text"), "")
def err(i):
    return ans.get(i, {}).get("result", {}).get("isError") is True
def data(i):
    try:
        return json.loads(text(i))
    except Exception:
        return None
check(0, "initialize", "protocolVersion" in ans.get(0, {}).get("result", {}))
names = [x["name"] for x in ans.get(1, {}).get("result", {}).get("tools", [])]
check(1, "tools/list has every tool", all(n in names for n in ["run_command", "quick_save", "add_objects", "screenshot", "save_scene", "render_video"]))
check(2, "a viewer is running", (data(2) or {}).get("running") is True)
check(3, "open_scene opens the copy", (data(3) or {}).get("file", "").endswith("second.json"))
n0 = len(data(4) or [])
check(4, f"list_objects ({n0})", n0 > 3)
check(5, "add_object", not err(5) and (data(5) or {}).get("index") == n0)
check(6, "the object is there", len(data(6) or []) == n0 + 1 and "check ball" in text(6))
check(7, "update_object", not err(7))
check(8, "undo the update", not err(8))
check(9, "undo the add", not err(9))
check(10, "back to the objects there were", len(data(10) or []) == n0)
check(11, "a bad node is refused, with the reason", err(11) and "no such shape" in text(11))
check(12, "run_command headlight", not err(12) and (data(12) or {}).get("file") is not None)
check(13, "run_command of no such command says so", err(13) and "no command" in text(13))
check(14, "run_command that cannot run now says why", err(14) and "cannot run now" in text(14))
check(15, "quick_save into the private slot", not err(15) and os.path.exists(f"{t}/quicksave.json"))
check(16, "quick_load", not err(16))
img = next((x for x in ans.get(17, {}).get("result", {}).get("content", []) if x.get("type") == "image"), None)
check(17, "screenshot is a PNG", img is not None and base64.b64decode(img["data"])[:4] == b"\x89PNG")
check(18, "raycast hits the floor", not err(18) and "point" in text(18))
check(19, "saving where it cannot says so", err(19) and "save failed" in text(19))
check(20, "save_scene writes the file", not err(20) and os.path.exists(f"{t}/saved.json"))
check(21, "run_command game_mode", not err(21))
check(22, "game mode is on", (data(22) or {}).get("game") is not None)
check(23, "hold_keys", not err(23))
f0 = ((data(22) or {}).get("game") or {}).get("feet") or [0, 0, 0]
f1 = ((data(25) or {}).get("game") or {}).get("feet") or [0, 0, 0]
moved = sum((a - b) ** 2 for a, b in zip(f0, f1)) ** 0.5
check(25, f"holding w walks ({moved:.2f} m)", moved > 0.5)
check(26, "set_preferences", not err(26) and (data(26) or {}).get("video_steady") is True)
steady = ((data(27) or {}).get("recording") or {}).get("steady")
check(27, "a steady recording starts", steady is not None)
check(28, "glide_camera", not err(28))
import subprocess
def probe(path):
    """A video's width, height and frames (zeros: none there)."""
    if not path or not os.path.exists(path):
        return 0, 0, 0
    out = subprocess.run(["ffprobe", "-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", "stream=width,height,nb_read_frames", "-of", "csv=p=0", path], capture_output=True, text=True).stdout.strip()
    return tuple(int(x or 0) for x in (out.split(",") + ["", "", ""])[:3])
w, h, frames = probe(((data(27) or {}).get("recording") or {}).get("file"))
c0, c1 = (data(27) or {}).get("clock", 0), (data(30) or {}).get("clock", 0)
moved = round((c1 - c0) * 30)
check(30, f"the steady video has a frame for each 1/30 s the scene moved ({frames} frames, {moved} steps)", frames >= 3 and abs(frames - moved) <= 1)
check(30, f"at the preferences' size, whatever the window's ({w} x {h})", (w, h) == (320, 180))
check(32, "an unknown preference is refused", err(32) and "no_such_preference" in text(32))
check(34, "a queued glide starts as the one before ends", ((data(34) or {}).get("starts_in") or 0) > 5)
film = data(35) or {}
check(35, "render_video starts, at its own size, at most 90% busy", not err(35) and film.get("size") == [256, 144] and film.get("frames") == 15 and abs((film.get("max_load") or 0) - 0.9) < 1e-3)
check(36, "a second recording is refused while one runs", err(36) and "already recording" in text(36))
st = data(38) or {}
check(38, "render_video ends by itself, its file whole", st.get("recording") is None and st.get("last_video") == film.get("file") and st.get("recording_error") is None)
w, h, frames = probe(film.get("file"))
check(38, f"its video: 256 x 144, 15 frames ({w} x {h}, {frames})", (w, h, frames) == (256, 144, 15))
check(39, "stopping when not recording says so", err(39) and "not recording" in text(39))
p1, p2 = (data(41) or {}).get("point"), (data(42) or {}).get("point")
off = sum((a - b) ** 2 for a, b in zip(p1, p2)) ** 0.5 if p1 and p2 else 1.0
check(41, f"raycast from 900 m onto a fractal lands where one from 2 m does ({off * 1000:.3f} mm apart)", not err(40) and "check bulb" in text(41) and "check bulb" in text(42) and off < 2e-4)
import glob, struct, zlib
def still(path):
    """A screenshot's iTXt chunks (keyword: text) and whether they all come
    before the pixels."""
    d, i, out, seen = open(path, "rb").read(), 8, {}, []
    while i < len(d):
        n, kind = struct.unpack(">I4s", d[i:i + 8])
        seen.append(kind)
        if kind == b"iTXt":
            key, rest = d[i + 8:i + 8 + n].split(b"\0", 1)
            body = rest[2:].split(b"\0", 2)[2]
            out[key.decode()] = (zlib.decompress(body) if rest[0] else body).decode()
        i += 12 + n
    last = max((k for k, c in enumerate(seen) if c == b"iTXt"), default=len(seen))
    return out, b"IDAT" in seen and last < seen.index(b"IDAT")
shots = sorted(glob.glob(f"{t}/shots/*.png"), key=os.path.getmtime)
def size_of(path):
    """A PNG's width and height (its IHDR)."""
    try:
        return struct.unpack(">II", open(path, "rb").read()[16:24])
    except Exception:
        return None
def near(a, b):
    """Two positions alike but for the last bit (the MCP server reads the
    viewer's answers with serde_json, whose floats can come back an ulp off)."""
    return bool(a) and bool(b) and len(a) == len(b) and all(abs(x - y) <= 1e-12 for x, y in zip(a, b))
check(43, "out of game mode", not err(43) and (data(43) or {}).get("game") is None)
c46 = (data(46) or {}).get("camera") or {}
s1, ahead = still(shots[0]) if shots else ({}, False)
scene1 = json.loads(s1.get("ray-scene", "{}"))
p1 = (scene1.get("camera") or {}).get("position")
f45 = (data(45) or {}).get("file") or ""
check(45, f"run_command screenshot renders whole and answers with the file ({os.path.basename(f45)}, {size_of(f45)})", bool(shots) and os.path.exists(f45) and os.path.samefile(f45, shots[0]) and size_of(f45) == (640, 360))
check(45, "a screenshot carries the scene, the camera exactly, ahead of the pixels", ahead and near(p1, c46.get("position")) and near(p1, [0.123456789012, 1.5, 3.25]))
st = data(51) or {}
c52 = (data(52) or {}).get("camera") or {}
check(51, "open_scene with a screenshot: its scene file, edited since (the check bulb), the camera exactly", not err(51) and st.get("file", "").endswith("saved.json") and st.get("unsaved_edits") is True and "check bulb" in text(52) and near(c52.get("position"), p1))
s2, _ = still(shots[1]) if len(shots) > 1 else ({}, False)
g = (json.loads(s2.get("ray-still", "{}")) or {}).get("game") or {}
g53 = (data(53) or {}).get("game") or {}
check(53, f"a game-mode screenshot opens with the walker as it stood ({g.get('height')} m tall)", g.get("height") and g53.get("height_m") == g.get("height") and near(g53.get("feet"), g.get("feet")))
g55, s56 = (data(55) or {}).get("game") or {}, data(56) or {}
g56 = s56.get("game") or {}
moved = math.dist(g56.get("feet") or [0, 0, 0], g55.get("feet") or [0, 0, 0]) if g55 and g56 else 0
check(55, f"into game mode inside the check bulb: stuck, {g55.get('height_m')} m tall", g55.get("height_m") == 1e-8 and math.dist(g55.get("feet") or [9, 9, 9], [0, 40, 0]) < 1e-6)
check(56, f"get_out: the walker out of the bulb, {moved:.3f} m, as tall ({s56.get('said')})", 0.1 < moved < 2 and g56.get("height_m") == 1e-8 and s56.get("said", "").startswith("got out: the walker moved"))
check(57, "get_out again: room already", "room around it already" in (data(57) or {}).get("said", ""))
r60, s61 = data(60) or {}, data(61) or {}
p62 = ((data(62) or {}).get("camera") or {}).get("position") or [0, 40, 0]
check(60, "the camera inside the bulb (a ray from it hits at once)", (data(58) or {}).get("game") is None and r60.get("hit") and r60.get("distance", 1) < 1e-3)
check(61, f"get_out: the camera out of the bulb, {math.dist(p62, [0, 40, 0]):.3f} m ({s61.get('said')})", s61.get("said", "").startswith("got out: the camera moved") and 0.3 < math.dist(p62, [0, 40, 0]) < 3)
check(63, "get_out again: room already", "room around it already" in (data(63) or {}).get("said", ""))
if fails:
    print(f"mcp-check: {len(fails)} failed (the viewer's log: {t}/viewer.log)")
    sys.exit(1)
print("mcp-check: all passed")
PY
