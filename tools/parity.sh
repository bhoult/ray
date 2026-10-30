#!/usr/bin/env bash
# Renders every scene in saves/ on the CPU and on the GPU (320 x 180, 8
# frames) and compares them: the two devices with each other (the renderers
# are separate code, render/*.rs and gpu/lib.wgsl, so this is what catches one
# drifting from the other), and each image with the same one in a baseline
# folder when one is named (what a change did to every scene). Run it into
# one folder before a change and into another after, naming the first.
#   tools/parity.sh out-dir [baseline-dir]
# Fails (exit 1) when a scene does not render, leaves an object out, or more
# than LIMIT of its pixels differ visibly (by more than 0.1) between CPU and
# GPU (PT_LIMIT for path-traced scenes, noisier at 8 frames). Visibly
# different pixels, not the mean: in fractal detail finer than a pixel the
# two devices' rounding scatters single pixels (fractals2's mean is 0.006 from
# 0.4% of them), where a broken shader changes whole areas. Differences from
# the baseline are listed, not failed (a change may mean them).
set -euo pipefail
cd "$(dirname "$0")/.."
out=${1:?usage: tools/parity.sh out-dir [baseline-dir]}
base=${2:-}
mkdir -p "$out"
status=0
for f in saves/*.json; do
  n=$(basename "$f" .json)
  [ "$n" = quicksave ] && continue
  for d in cpu gpu; do
    if ! ./target/release/ray "$f" --device "$d" --size 320x180 --frames 8 -o "$out/$n-$d.png" > "$out/$n-$d.log" 2>&1; then
      echo "$n ($d): did not render (see $out/$n-$d.log)"
      status=1
    elif grep -qi "left out" "$out/$n-$d.log"; then
      echo "$n ($d): $(grep -i -m1 'left out' "$out/$n-$d.log")"
      status=1
    fi
  done
done
python3 - "$out" "$base" <<'EOF' || status=1
import json, os, sys
from PIL import Image
import numpy as np
LIMIT, PT_LIMIT = 0.01, 0.05
out, base = sys.argv[1], sys.argv[2]
load = lambda p: np.asarray(Image.open(p)).astype(float) / 255
bad = 0
for f in sorted(os.listdir('saves')):
    n, ext = os.path.splitext(f)
    if ext != '.json' or n == 'quicksave' or not os.path.exists(f'{out}/{n}-cpu.png') or not os.path.exists(f'{out}/{n}-gpu.png'):
        continue
    cpu, gpu = load(f'{out}/{n}-cpu.png'), load(f'{out}/{n}-gpu.png')
    pt = (json.load(open(f'saves/{f}')).get('render') or {}).get('mode') == 'path_trace'
    d = abs(cpu - gpu)
    visible = (d.max(axis=2) > 0.1).mean()
    limit = PT_LIMIT if pt else LIMIT
    line = f'{n:14} cpu-gpu {d.mean():.5f}, visibly {visible * 100:5.2f}% of pixels' + (' (path traced)' if pt else '')
    if visible > limit:
        line += f'  ** over {limit * 100:.0f}%'
        bad += 1
    if base:
        for dev, img in (('cpu', cpu), ('gpu', gpu)):
            p = f'{base}/{n}-{dev}.png'
            if os.path.exists(p):
                c = abs(img - load(p)).mean()
                line += f'  {dev} vs base {c:.6f}' if c > 0 else f'  {dev} same'
            else:
                line += f'  {dev}: not in base'
    print(line)
print(f'parity: {bad} of {sum(1 for f in os.listdir("saves") if f.endswith(".json") and f != "quicksave.json")} scenes over the limit' if bad else 'parity: all passed')
sys.exit(1 if bad else 0)
EOF
exit $status
