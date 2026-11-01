"""The fractal flame presets' pictures, for their pages in docs/fractals/:
each preset as assets/flame-presets.json shows it (its palette, and its
rotation and scale if it has them), alone over a lake just after sunset,
rendered headless at 960x600 into docs/images/fractals/flame_<preset>.jpg.
Run from the repository root after cargo build --release (tools/readme-images.sh
does both); a few seconds a preset on a GPU. Options (--device gpu...) are
passed to ray.
    python3 tools/flame-images.py [PRESET...] [--option value...]    (all presets by default)
"""
import copy, json, os, subprocess, sys, tempfile
from PIL import Image

OUT = 'docs/images/fractals'
SCENE = {
    'camera': {'position': [0, 1.5, 6.2], 'look_at': [0, 2.5, 0], 'fov': 40},
    'environment': {'preset': 'golden', 'time_of_day': 18.3, 'sun_azimuth': 235, 'clouds': 0.25, 'fog_density': 0.003},
    'objects': [{'name': 'lake', 'type': 'water', 'level': 0, 'waves': 0.04}],
}
# how each is drawn here: bigger and finer than in the grid of them all
FLAME = {'position': [0, 2.4, 0], 'size': 3.2, 'glow': 8, 'density': 3, 'resolution': 192, 'points': 24}

args = sys.argv[1:]
first = next((i for i, a in enumerate(args) if a.startswith('--')), len(args))
want, opts = args[:first], args[first:]
flames = [o for o in json.load(open('assets/flame-presets.json'))['objects'] if not want or o['preset'] in want]
with tempfile.TemporaryDirectory() as tmp:
    for o in flames:
        name = o['preset']
        scene = copy.deepcopy(SCENE)
        flame = {'name': name, 'type': 'flame', 'preset': name, 'palette': o['palette'], **FLAME}
        flame.update({k: o[k] for k in ('rotation', 'scale') if k in o})
        scene['objects'].append(flame)
        path = os.path.join(tmp, name + '.json')
        with open(path, 'w') as f:
            json.dump(scene, f)
        png = os.path.join(tmp, name + '.png')
        subprocess.run(['./target/release/ray', path, '--size', '960x600', '--frames', '64', '-o', png, *opts], check=True, stdout=subprocess.DEVNULL)
        jpg = os.path.join(OUT, f'flame_{name}.jpg')
        Image.open(png).convert('RGB').save(jpg, quality=88)
        print(jpg)
