#!/usr/bin/env bash
# The README's pictures, into docs/images: the banner (assets/banner.json,
# with the name set on it by tools/logo.py), the editor with the Mandelbulb
# selected (headless, tools/gui-test.sh) and the gallery, each rendered
# headless at 1280x720 from a saved scene. A few minutes.
#   tools/readme-images.sh
set -euo pipefail
cd "$(dirname "$0")/.."
out=docs/images
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
cargo build --release -q
jpg() { python3 -c "import sys; from PIL import Image; Image.open(sys.argv[1]).convert('RGB').save(sys.argv[2], quality=88)" "$1" "$2"; }
shot() {
  local name=$1 frames=$2
  shift 2
  ./target/release/ray "$@" --size 1280x720 --frames "$frames" -o "$tmp/$name.png" > /dev/null
  jpg "$tmp/$name.png" "$out/$name.jpg"
  echo "$out/$name.jpg"
}
./target/release/ray assets/banner.json --size 1600x500 --frames 64 -o "$tmp/banner.png" > /dev/null
python3 tools/logo.py "$tmp/banner.png" "$out/logo.jpg"
echo "$out/logo.jpg"
RAY_TEST_MONITOR=1920x1080 RAY_TEST_SELECT=2 RAY_TEST_TIMEOUT=150 tools/gui-test.sh saves/showcase.json "$tmp/editor.png" 120 > /dev/null 2>&1
jpg "$tmp/editor.png" "$out/editor.jpg"
echo "$out/editor.jpg"
# the fractals: the gallery from its side, the view out of the sponge, its
# corridors and the Kleinian's inside (the headlight on: game mode's spots)
shot fractal-gallery 64 saves/fractals.json --camera -5.2,1.6,0.6,-35,-5
shot sponge-window 64 saves/fractals3.json
shot sponge-corridors 64 saves/fractals.json --camera -6.4375,2.2410,-1.4767,-49.09,-13.54 --set '{"headlight": 1.0}'
shot kleinian 64 saves/fractals.json --camera 4.9766,1.7750,-4.2966,-117.78,56.44 --set '{"headlight": 1.0}'
# every fractal type, a family to a row (docs/images/fractals/ are each one's own, from docs/FRACTALS.md's entries)
shot fractal-catalog 64 saves/fractal-catalog.json --camera -6,6.5,6,-25,-24
for s in bonsai night-city clouds island nurbs nurbs-booleans water materials; do
  shot "$s" 64 "saves/$s.json"
done
shot glass-chess 64 saves/glass.json
shot cornell 400 saves/cornell.json
