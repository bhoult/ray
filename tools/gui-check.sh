#!/usr/bin/env bash
# The viewer's interface, run headless (tools/gui-test.sh: a private GNOME
# Shell, preferences in a folder of their own, nothing in the user's home
# folders) and checked by what it writes and logs:
#   screenshots of the view, and of the whole window when the preference says
#   so; a recording; the Open, Preferences and Help windows; the object list's
#   tree opened on a part; game mode floating in deep water; the gravity gun
#   sending a crate off.
# Exit 1 when a check fails.
#   tools/gui-check.sh
set -uo pipefail
cd "$(dirname "$0")/.."
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fails=0
pass() { echo "ok   $1"; }
fail() { echo "FAIL $1"; fails=$((fails + 1)); }
# preferences in their own folder: tools/gui-test.sh keeps a RAY_TEST_CONFIG
# folder's own config.json
prefs() {
  mkdir -p "$1/ray"
  printf '{ "videos": "%s/videos", "screenshots": "%s/shots", "quick_save": "%s/quicksave.json"%s }\n' "$1" "$1" "$1" "${2:-}" > "$1/ray/config.json"
}
run() {
  local name=$1 scene=$2 frames=$3
  shift 3
  env "$@" tools/gui-test.sh "$scene" "$tmp/$name.png" "$frames" > "$tmp/$name.log" 2>&1
  if grep -q "panicked" "$tmp/$name.log"; then
    fail "$name: the viewer panicked ($(grep -m1 panicked "$tmp/$name.log"))"
    return 1
  fi
}
size() { python3 -c "from PIL import Image; import sys; print('%dx%d' % Image.open(sys.argv[1]).size)" "$1"; }

# screenshots: the view alone, then the whole window
# (the view rendered whole first: a few samples, so it is quick)
prefs "$tmp/view" ', "screenshot_samples": 8'
run shot-view saves/cornell.json 90 RAY_TEST_CONFIG="$tmp/view" RAY_TEST_SCREENSHOT=20
f=$(ls "$tmp"/view/shots/*.png 2>/dev/null | head -1)
if [ -n "$f" ] && [ "$(size "$f" | cut -dx -f1)" -lt 1500 ]; then pass "screenshot of the view ($(size "$f"))"; else fail "screenshot of the view: ${f:-none}"; fi
prefs "$tmp/window" ', "screenshot_interface": true'
# (long enough for the viewer to ask again: a headless compositor drops a
# frame, and the request with it, now and then)
run shot-window saves/cornell.json 45 RAY_TEST_CONFIG="$tmp/window" RAY_TEST_SCREENSHOT=20
f=$(ls "$tmp"/window/shots/*.png 2>/dev/null | head -1)
if [ -n "$f" ] && [ "$(size "$f" | cut -dx -f1)" -ge 1500 ]; then pass "screenshot of the window ($(size "$f"))"; else fail "screenshot of the window: ${f:-none}"; fi

# a recording: F10 at frames 10 and 50
prefs "$tmp/rec"
run record saves/showcase.json 70 RAY_TEST_CONFIG="$tmp/rec" RAY_TEST_RECORD=10,50 RAY_TEST_TIMEOUT=120
sleep 2
f=$(ls "$tmp"/rec/videos/*.mp4 2>/dev/null | head -1)
if [ -n "$f" ] && [ "$(stat -c %s "$f")" -gt 5000 ] && ! ls "$tmp"/rec/videos/*.log > /dev/null 2>&1; then pass "a recording ($(stat -c %s "$f") bytes)"; else fail "a recording: ${f:-none}"; fi

# windows: Open, Preferences, the help; the tree opened on a part
for w in "open RAY_TEST_OPEN=1" "prefs RAY_TEST_PREFS=1" "help RAY_TEST_HELP=controls" "tree RAY_TEST_SELECT=6/1"; do
  set -- $w
  prefs "$tmp/$1"
  if run "window-$1" saves/showcase.json 12 RAY_TEST_CONFIG="$tmp/$1" "$2" && [ -s "$tmp/window-$1.png" ]; then pass "the $1 window"; else fail "the $1 window"; fi
done

# game mode in deep water: it floats with three quarters of it under
printf '{ "camera": { "position": [0, 1.0, 0], "look_at": [0, 1.0, -5] }, "objects": [{ "type": "water", "waves": 0.2 }] }\n' > "$tmp/sea.json"
prefs "$tmp/sea"
run float "$tmp/sea.json" 160 RAY_TEST_CONFIG="$tmp/sea" RAY_TEST_GAME=1 RAY_TEST_TIMEOUT=120
wet=$(grep -o "in water [0-9.]*" "$tmp/float.log" | tail -1 | grep -o "[0-9.]*$")
if [ -n "$wet" ] && python3 -c "import sys; sys.exit(0 if abs(float('$wet') - 0.75) < 0.06 else 1)"; then pass "floats in deep water (in water $wet)"; else fail "floats in deep water (in water ${wet:-?})"; fi

# the gravity gun: held on a crate and let go, it sends the crate off
printf '{ "camera": { "position": [0, 1.656, 5], "look_at": [0, 0.4, 0] }, "objects": [{ "type": "plane", "locked": true }, { "type": "box", "size": [0.8, 0.8, 0.8], "position": [0, 0.4, 0] }] }\n' > "$tmp/crate.json"
prefs "$tmp/gun"
run gun "$tmp/crate.json" 90 RAY_TEST_CONFIG="$tmp/gun" RAY_TEST_GAME=1 RAY_TEST_HOLD=lmb:20-50 RAY_TEST_TIMEOUT=120
if grep -q "bodies 1" "$tmp/gun.log"; then pass "the gravity gun sends a crate off"; else fail "the gravity gun: no body set moving"; fi

if [ $fails -gt 0 ]; then
  echo "gui-check: $fails failed"
  exit 1
fi
echo "gui-check: all passed"
