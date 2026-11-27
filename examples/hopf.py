"""Hopf fibration, fibre networks, and fractals living on them."""
from _common import *

# nested tori of fibres: three latitudes, 14 fibres each
base = fk.ring_base([-0.6, 0.0, 0.6], 14)
hf = fk.HopfFibers(base, radius=0.05, max_circle=8.0)
fib_items = [
    ("Hopf fibres on 3 nested tori", hf),
    ("Fibres in the Clifford ring only", fk.HopfFibers(fk.ring_base([0.0], 18), radius=0.06)),
    ("Torus knot (2,3) fibre", fk.FiberNetwork.torus_knot(2, 3, radius=0.08)),
    ("Cubic lattice network", fk.FiberNetwork.lattice(3, 0.9, 0.07, diagonals=True)),
]
grid(fib_items, ncols=2, size=(300, 225), name="hopf_fibres", max_steps=250)

# fractal x fibration: carve / blend / intersect
mix = [
    ("Menger & fibre tube (fib shell)", fk.Menger(4).scaled(1.2) & hf.shell(0.12)),
    ("Mandelbulb threaded by fibres", fk.Mandelbulb(8, iters=9) | hf.scaled(0.3)),
    ("fibres smooth-merged into Menger", fk.Menger(3).smooth_union(hf.scaled(0.35), 0.06)),
    ("Hopfbrot tilt 90", fk.Hopfbrot(3, 90.0, 0.0)),
    ("Hopfbrot tilt 45", fk.Hopfbrot(3, 45.0, 0.0)),
    ("Hopfbrot tilt 0, slice .2", fk.Hopfbrot(3, 0.0, 0.2)),
]
grid(mix, name="hopf_mix", max_steps=200, step=0.6)

# 3D line plot of individual fibres, coloured by base point, with the base S^2
fig = plt.figure(figsize=(10, 4.6))
a = fig.add_subplot(1, 2, 1, projection="3d")
t = np.linspace(0, 2 * np.pi, 400)
from fractalkit.geometry import hopf_fiber_points, stereographic
cols = plt.cm.viridis(np.linspace(0, 1, len(base)))
for (x, y, z), c in zip(base, cols):
    eta, phi = 0.5 * np.arccos(x), np.arctan2(z, y)
    q = stereographic(hopf_fiber_points(eta, phi, t))
    if np.abs(q).max() < 6:
        a.plot(*q.T, color=c, lw=1)
a.set_title("Hopf fibres (stereographic)", fontsize=9); a.set_axis_off(); a.set_box_aspect((1, 1, 1))
b = fig.add_subplot(1, 2, 2, projection="3d")
b.scatter(*base.T, c=cols, s=30); b.set_title("their base points on S$^2$", fontsize=9); b.set_axis_off()
b.set_box_aspect((1, 1, 1))
fig.tight_layout(); fig.savefig(OUT / "hopf_lines.png", dpi=130); print("wrote", OUT / "hopf_lines.png")
