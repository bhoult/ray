#!/usr/bin/env bash
# A private headless desktop (GNOME Shell) with the viewer on a socket and
# preferences of its own, for the demo director: it runs until killed.
#   tools/demo/session.sh WORKDIR [WxH] [scene]
# The socket is $XDG_RUNTIME_DIR/ray-demo.sock ($RAY_DEMO_SOCKET when set: a second session beside it); WORKDIR gets the preferences (ray/config.json:
# recordings into WORKDIR/videos, no microphone) and the viewer's log. The viewer runs
# at a lower priority (nice 10): the desktop's own programs come first.
set -euo pipefail
cd "$(dirname "$0")/../.."
work=$1
size=${2:-1920x1080}
scene=${3:-saves/fractals.json}
mkdir -p "$work/ray" "$work/videos"
printf '{ "videos": "%s/videos", "screenshots": "%s/shots", "quick_save": "%s/quicksave.json", "microphone": false, "video_fps": 30, "video_quality": 18 }\n' "$work" "$work" "$work" > "$work/ray/config.json"
# (a short socket path: a Unix socket's may be at most 108 bytes)
export XDG_CONFIG_HOME=$work GSETTINGS_BACKEND=memory RAY_SOCKET=${RAY_DEMO_SOCKET:-${XDG_RUNTIME_DIR:-/tmp}/ray-demo.sock}
rm -f "$RAY_SOCKET"
exec dbus-run-session -- bash -c '
  gnome-shell --headless --wayland --no-x11 --wayland-display "$4" --virtual-monitor "$1" >/dev/null 2>&1 &
  shell=$!
  for _ in $(seq 100); do [ -S "$XDG_RUNTIME_DIR/$4" ] && break; sleep 0.1; done
  WAYLAND_DISPLAY="$4" nice -n 10 ./target/release/ray "$2" > "$3/viewer.log" 2>&1
  kill $shell 2>/dev/null
' _ "$size" "$scene" "$work" "ray-demo-$$"
