"""Seam trees whose ball changes with the generation: Trionda -> Hat Trick, Hat Trick -> Trionda, alternating, three-stage."""
from _common import *
import json

cam = lambda d=5.5, az=35, el=25: dict(camera=fk.Camera(azimuth=az, elevation=el, distance=d), step=0.7, max_steps=250,
                                      palette=None, trap_weight=0.85)
T = lambda: fk.TriondaHorned(iters=1)
H = lambda: fk.HatTrickHorned(iters=1)
C = lambda: fk.PolyBall("cube", "vertex", False, iters=1)
M = fk.MixedSeamTree
specs = [
    ("Trionda -> Hat Trick (shrink 0.35)", M([T(), H()], shrink=0.35, iters=3)),
    ("Hat Trick -> Trionda (0.35)", M([H(), T()], shrink=0.35, iters=3)),
    ("Trionda / Hat Trick alternating (0.35)", M([T(), H()], shrink=0.35, iters=4, cyclic=True)),
    ("Trionda -> cube -> Hat Trick (0.35)", M([T(), C(), H()], shrink=0.35, iters=3)),
    ("Trionda -> Hat Trick, turn 36", M([T(), H()], shrink=0.35, turn=36, iters=3)),
    ("Trionda -> Hat Trick, shrink 0.45", M([T(), H()], shrink=0.45, iters=3)),
]
items = []
for i, (title, t) in enumerate(specs):
    kw = cam(6.0 if i != 2 else 6.5, az=35 if i != 1 else 200, el=25 if i != 1 else -15)
    kw["palette"] = t.cmap()
    items.append((title, t, kw))
grid(items, name="mixed_seamtrees", size=(220, 165))

rows = []
for title, t in specs:
    n = [sum(1 for p in t.networks(d) if p[3] == d) for d in range(3)]
    c, fused, pc = t.clearance(2)
    rows.append(dict(tree=title, networks=n, clearance=c, fused_pairs=fused, parent_clearance=pc))
    print(rows[-1], flush=True)
(OUT / "mixed_seamtrees.json").write_text(json.dumps(rows, indent=1, default=float))
