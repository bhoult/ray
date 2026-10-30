"""Shared by the scene generators: writing a scene one object per line.

Run a generator from this directory with the output directory first, e.g.
    python3 glass.py ../../saves
The ones that add to an existing scene take that file second:
    python3 physics.py ../../saves ../../saves/physics.json
"""
import json, math, random, sys
OUT = sys.argv[1] if len(sys.argv) > 1 else 'saves'


def write(name, d):
    lines = ['{']
    for k in [k for k in d if k != 'objects']:
        if k == 'materials':
            lines.append('  "materials": {')
            ms = list(d[k].items())
            for i, (mk, mv) in enumerate(ms):
                lines.append(f'    {json.dumps(mk)}: {json.dumps(mv)}' + (',' if i < len(ms) - 1 else ''))
            lines.append('  },')
        else:
            lines.append(f'  {json.dumps(k)}: {json.dumps(d[k])},')
    lines.append('  "objects": [')
    for i, o in enumerate(d['objects']):
        lines.append('    ' + json.dumps(o) + (',' if i < len(d['objects']) - 1 else ''))
    lines.append('  ]')
    lines.append('}')
    open(f'{OUT}/{name}.json', 'w').write('\n'.join(lines) + '\n')


def r(v, n=2):
    return [round(x, n) for x in v] if isinstance(v, (list, tuple)) else round(v, n)


