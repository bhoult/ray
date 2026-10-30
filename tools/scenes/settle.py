"""Lets the named objects of a scene fall onto what is under them (the
viewer's physics, headless) and writes where they come to rest, so that the
scene opens settled and Drop all leaves it as it is.

python3 settle.py scene.json name ...
"""
import json, os, re, subprocess, sys, tempfile
import common
from common import write

ray = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../target/release/ray')
path, names = sys.argv[1], set(sys.argv[2:])
d = json.load(open(path))
for i, o in enumerate(d['objects']):
    if o.get('name') not in names:
        continue
    with tempfile.TemporaryDirectory() as t:
        out = subprocess.run([ray, path, '--drop', f'{i},0', '--size', '16x16', '--frames', '1', '-o', f'{t}/x.png'], capture_output=True, text=True)
    m = re.search(rf'object {i} rests at \[([^\]]*)\], rotation \[([^\]]*)\]', out.stdout + out.stderr)
    if not m:
        print(f"{o.get('name')}: no rest ({(out.stdout + out.stderr).strip()[-200:]})")
        continue
    o['position'] = [round(float(v), 3) for v in m.group(1).split(',')]
    o['rotation'] = [round(float(v), 2) for v in m.group(2).split(',')]
    print(o['name'], o['position'], o['rotation'])
    json.dump(d, open(path, 'w'))  # (the next drop sees this one settled)
common.OUT = os.path.dirname(path) or '.'
write(os.path.splitext(os.path.basename(path))[0], d)
