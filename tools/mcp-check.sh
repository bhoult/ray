#!/usr/bin/env bash
# Drives a viewer through the MCP server (`ray mcp`), as an assistant would,
# and checks every answer: a private headless GNOME Shell and viewer on a
# socket of their own, with preferences of their own (the quick save slot,
# screenshots and videos in a temporary folder: the user's are never
# touched), a copy of a saved scene opened, then edits, undo, a refused edit,
# commands by name, the quick save, a screenshot, saving (and failing to).
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
python3 - "$tmp" > "$tmp/requests.jsonl" <<'PY'
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
]
print(json.dumps({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": reqs[0][1]}))
print(json.dumps({"jsonrpc": "2.0", "method": "notifications/initialized"}))
for i, (name, args) in enumerate(reqs[1:], 1):
    if name == "tools/list":
        print(json.dumps({"jsonrpc": "2.0", "id": i, "method": "tools/list", "params": {}}))
    else:
        print(json.dumps({"jsonrpc": "2.0", "id": i, "method": "tools/call", "params": {"name": name, "arguments": args}}))
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
  kill $app 2>/dev/null; wait $app 2>/dev/null || true
  kill $shell 2>/dev/null; wait $shell 2>/dev/null || true
' _ "$display" "$tmp" >/dev/null 2>&1 || true
python3 - "$tmp" <<'PY'
import base64, json, os, sys
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
check(1, "tools/list has every tool", all(n in names for n in ["run_command", "quick_save", "add_objects", "screenshot", "save_scene"]))
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
if fails:
    print(f"mcp-check: {len(fails)} failed (the viewer's log: {t}/viewer.log)")
    sys.exit(1)
print("mcp-check: all passed")
PY
