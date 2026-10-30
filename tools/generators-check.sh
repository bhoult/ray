#!/usr/bin/env bash
# Runs every scene generator in tools/scenes into a temporary folder (the
# ones that add to a scene, onto a copy of it; saves/ is never written) and
# loads each scene it writes: exit 1 when a generator fails or a scene does
# not load whole (an object left out, a parse error).
#   tools/generators-check.sh
set -uo pipefail
cd "$(dirname "$0")/.."
root=$(pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fails=0
gen() {
  local what=$1
  shift
  if (cd tools/scenes && python3 "$@" > "$tmp/$what.log" 2>&1); then
    echo "ok   $what"
  else
    echo "FAIL $what: $(tail -1 "$tmp/$what.log")"
    fails=$((fails + 1))
  fi
}
# the ones that write a scene of their own (bonsai with another seed too)
for g in bonsai city clouds fractals glass night nurbs; do
  gen "$g" "$g.py" "$tmp"
done
mkdir -p "$tmp/seed3"
gen "bonsai, seed 3" bonsai.py "$tmp/seed3" 3
# the ones that add to a scene: a copy of it
for g in materials physics playground showcase vehicles water; do
  cp "saves/$g.json" "$tmp/$g-in.json"
  gen "$g" "$g.py" "$tmp" "$tmp/$g-in.json"
done
mkdir -p "$tmp/touched"
cp saves/landscape.json saves/infinite.json "$tmp/touched/"
gen touches touches.py "$tmp/touched" "$tmp/touched"
# settling: the kettle by the campfire falls onto what is under it
cp saves/night.json "$tmp/settle.json"
gen settle settle.py "$tmp/settle.json" kettle
# every scene they wrote loads whole
for f in "$tmp"/*.json "$tmp"/seed3/*.json "$tmp"/touched/*.json; do
  case "$f" in *-in.json) continue ;; esac
  log=$(./target/release/ray "$f" --device cpu --size 64x36 --frames 1 -o "$tmp/x.png" 2>&1)
  if [ $? -ne 0 ] || echo "$log" | grep -qi "left out\|panicked\|^error"; then
    echo "FAIL ${f#$tmp/}: $(echo "$log" | grep -i -m1 'left out\|panicked\|error')"
    fails=$((fails + 1))
  else
    echo "ok   ${f#$tmp/} loads"
  fi
done
if [ $fails -gt 0 ]; then
  echo "generators-check: $fails failed"
  exit 1
fi
echo "generators-check: all passed"
