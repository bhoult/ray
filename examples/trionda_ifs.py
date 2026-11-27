"""The Trionda as an IFS: a twisted Sierpinski tetrahedron with tetrahedral rotation symmetry."""
from _common import *

cam = lambda az=35, el=25, d=3.6: dict(camera=fk.Camera(azimuth=az, elevation=el, distance=d), step=0.7, max_steps=300,
                                       palette="turbo", trap_weight=0.9)
# twist sweep, straight edges (twist 0 = the Sierpinski tetrahedron frame) and the Trionda's S-curve seams
items = [(f"edges, twist {t}", fk.TriondaIFS(seed="edges", twist=t, tube=0.05, iters=4), cam()) for t in (0, 30, 70)]
items += [(f"seams, twist {t}", fk.TriondaIFS(seed="seams", twist=t, tube=0.05, iters=4), cam()) for t in (0, 30, 70)]
grid(items, name="trionda_ifs_twist", size=(220, 165))

# depth, shrink
items = [(f"seams, twist 30, depth {n}", fk.TriondaIFS(twist=30, tube=0.05, iters=n), cam()) for n in (1, 2, 3)]
items += [(f"shrink {s}, twist 30, depth 4", fk.TriondaIFS(shrink=s, twist=30, tube=0.05, iters=4), cam()) for s in (0.42, 0.5, 0.58)]
grid(items, name="trionda_ifs_depth", size=(220, 165))
