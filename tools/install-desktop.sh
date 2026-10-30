#!/bin/sh
# Installs ray.desktop for this user: the desktop matches it to ray's window
# by its app id ("ray") for the name and icon; without it GNOME shows
# "Unknown". It runs this checkout's release build, from the checkout (the
# default scene and the quick save slot are relative paths).
# Remove: rm ~/.local/share/applications/ray.desktop
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
apps=${XDG_DATA_HOME:-$HOME/.local/share}/applications
mkdir -p "$apps"
cat > "$apps/ray.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=ray
Comment=Ray tracer and scene editor for distance fields and NURBS
Exec=$root/target/release/ray %f
Path=$root
Icon=$root/assets/icon.png
Terminal=false
Categories=Graphics;3DGraphics;
StartupWMClass=ray
EOF
update-desktop-database "$apps" 2>/dev/null || true
echo "installed $apps/ray.desktop"
