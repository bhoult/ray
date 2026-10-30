#!/usr/bin/env bash
# Runs the viewer inside a private headless GNOME Shell (its own D-Bus session,
# settings in memory; nothing appears on the desktop) and saves a capture of
# the whole window after N frames. Extra RAY_TEST_* variables pass through to
# the viewer (each acts at a frame of the run; src/ui/mod.rs has them all):
#   SELECT=n | n/a/b | env  select object n, or its part a's part b, or
#                     the environment
#   HELP=1 | controls | materials, PREFS=1, OPEN=1, LIBRARY=1, MODIFIERS=1
#                     open that window
#   HEAT=1            the cost heat map;  DROP=n  object n falls
#   DRAG=tool,axis,dx,dy[,shift]  a handle drag;  CHARGE=1  a throw charging
#   GAME=1            game mode;  TAB=n,m  Tab at those frames
#   HOLD=key:from-to,...  keys held (w a s d r f space shift; lmb rmb)
#   SCREENSHOT=n      P at frame n;  RECORD=n,m  F10 at those frames (MIC=1:
#                     with the microphone)
# and to this script: CONFIG=dir keeps that preferences folder (its own
# config.json if it has one), MONITOR=WxH the screen (1600x900),
# TIMEOUT=s how long the viewer may run (90).
#   tools/gui-test.sh [scene.json] [out.png] [frames]
set -euo pipefail
cd "$(dirname "$0")/.."
scene=${1:-saves/showcase.json}
out=${2:-shots/gui.png}
frames=${3:-40}
mkdir -p "$(dirname "$out")"
export GSETTINGS_BACKEND=memory
# preferences of its own (the user's ~/.config/ray stays untouched, and test
# recordings go to a temporary folder, not ~/Videos): RAY_TEST_CONFIG names
# the folder to keep (to look at what was recorded), else one is made and
# removed afterwards
if [ -n "${RAY_TEST_CONFIG:-}" ]; then
  export XDG_CONFIG_HOME=$RAY_TEST_CONFIG
else
  export XDG_CONFIG_HOME=$(mktemp -d)
  trap 'rm -rf "$XDG_CONFIG_HOME"' EXIT
fi
mkdir -p "$XDG_CONFIG_HOME/ray"
# (a kept folder's own preferences stay, to test one: screenshot_interface...)
[ -f "$XDG_CONFIG_HOME/ray/config.json" ] || printf '{ "videos": "%s/videos", "screenshots": "%s/shots", "quick_save": "%s/quicksave.json" }\n' "$XDG_CONFIG_HOME" "$XDG_CONFIG_HOME" "$XDG_CONFIG_HOME" > "$XDG_CONFIG_HOME/ray/config.json"
# a display of its own, so that several tests can run at once; and no
# control socket unless RAY_SOCKET names one (the user's viewer has the default)
display=ray-test-$$
[ -n "${RAY_SOCKET:-}" ] || export RAY_NO_SOCKET=1
dbus-run-session -- bash -c '
  gnome-shell --headless --wayland --no-x11 --wayland-display "$3" --virtual-monitor "${RAY_TEST_MONITOR:-1600x900}" >/dev/null 2>&1 &
  shell=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$3" ] && break; sleep 0.1; done
  status=0
  WAYLAND_DISPLAY="$3" RAY_TEST_SHOT="$1" RAY_TEST_FRAMES="$2" timeout ${RAY_TEST_TIMEOUT:-90} ./target/release/ray "$0" 2>&1 | grep -E "^test|panicked|^ray:|error" || status=$?
  kill "$shell" 2>/dev/null || true
  wait "$shell" 2>/dev/null || true
  exit 0
' "$scene" "$out" "$frames" "$display" 2>/dev/null
[ -f "$out" ] || { echo "no capture written"; exit 1; }
