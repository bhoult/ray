"""Seam-tree horned spheres: the horned-sphere recursion on the Trionda (T) and Hat Trick (I) balls."""
from _common import *

def view(tree, az=35, el=25, dist=6.0, **kw):
    return dict(camera=fk.Camera(azimuth=az, elevation=el, distance=dist), step=0.7, max_steps=250,
                palette=tree.cmap(), trap_weight=0.85, **kw)

tr = lambda **k: fk.TriondaHorned(body=0.95, **k)
a = [("Trionda depth 1", tr(iters=1)), ("Trionda depth 2", tr(iters=2)), ("Trionda depth 3", tr(iters=3)),
     ("Trionda depth 4", tr(iters=4)), ("depth 3, other side", tr(iters=3)), ("depth 3, turn 70", tr(iters=3, turn=70))]
a[4] = (a[4][0], a[4][1], dict(az=215, el=-20)); 
items = [(n, t, view(t, **(e[0] if e else {}))) for n, t, *e in a]
grid(items, name="trionda_horned_gallery", size=(220, 165))

h = lambda **k: fk.HatTrickHorned(body=0.97, **k)
b = [("Hat Trick depth 1", h(iters=1)), ("Hat Trick depth 2", h(iters=2)), ("Hat Trick depth 3", h(iters=3)),
     ("Hat Trick depth 2, other side", h(iters=2)), ("depth 2, shrink 0.4", h(iters=2, shrink=0.4)), ("depth 2, turn 36", h(iters=2, turn=36))]
b[3] = (b[3][0], b[3][1], dict(az=215, el=-20))
items = [(n, t, view(t, dist=5.5, **(e[0] if e else {}))) for n, t, *e in b]
grid(items, name="hattrick_horned_gallery", size=(220, 165))

# sibling clasp: a trio of children around every vertex, clasped pairwise (see tools/scan_clasp.py)
cl = lambda **k: fk.TriondaHorned.clasp(body=0.95, **k)
c = [("clasp depth 1", cl(iters=1)), ("clasp depth 2", cl(iters=2)), ("clasp depth 3", cl(iters=3)),
     ("clasp depth 2, other side", cl(iters=2)), ("clasp depth 2, seams only", fk.TriondaHorned.clasp(iters=2)), ("clasp depth 2, top", cl(iters=2))]
ev = {3: dict(az=215, el=-20), 5: dict(az=0, el=85)}
items = [(n, t, view(t, **ev.get(i, {}))) for i, (n, t) in enumerate(c)]
items[4] = (items[4][0], items[4][1], {**items[4][2], "palette": "viridis", "trap_weight": 0.3})
grid(items, name="trionda_clasp_gallery", size=(220, 165))
