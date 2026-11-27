"""Hopfbrot as an IterMap: its slices, and hybrids that interleave it with other maps."""
from _common import *

H = fk.Hopfbrot(3, 90.0, 0.0, 10)

# the z = 0 plane of the standalone map holds the cubic Mandelbrot set; the hybrid bends it
fig, axs = plt.subplots(1, 3, figsize=(11, 3.8))
fk.plot_slice(fk.Hopfbrot(3, iters=40), "z", 0.0, 1.6, 400, mode="mask", ax=axs[0], title="alone: z=0 (cubic Mandelbrot)")
mix = fk.Hybrid([fk.Hopfbrot(3, iters=10), fk.Mandelbulb(8.0)], schedule=[0, 0, 1], iters=12, bail=8.0, bound=2.0)
fk.plot_slice(mix, "z", 0.0, 1.6, 400, mode="mask", ax=axs[1], title="Hopfbrot,Hopfbrot,bulb: z=0")
fk.plot_slice(mix, "x", 0.0, 1.6, 400, mode="mask", ax=axs[2], title="same hybrid: x=0")
fig.tight_layout(); p = OUT / "hopfbrot_slices.png"; fig.savefig(p, dpi=130); plt.close(fig); print("wrote", p)

items = [
    ("Hopfbrot alone", H),
    ("Hopfbrot + Mandelbulb 8  [0,0,1]", mix),
    ("Hopfbrot + Mandelbox  [0,1]", fk.Hybrid([H, fk.Mandelbox(-1.8, iters=10)], schedule=[0, 1], iters=12, bail=30.0, bound=2.5)),
    ("Hopfbrot + KIFS Menger  [0,1]", fk.Hybrid([H, fk.Menger_kifs(iters=10)], schedule=[0, 1], iters=12, bail=30.0, bound=2.0)),
    ("Hopfbrot + Riemann sphere  [0,1]", fk.Hybrid([H, fk.RiemannSphere(1.0, iters=8)], schedule=[0, 1], iters=10, bail=30.0, bound=2.0)),
    ("Riemann sphere, finished by Hopfbrot", fk.Hybrid([fk.RiemannSphere(1.0, iters=8), H], schedule=[0, 1], iters=10, finisher=1, bail=30.0, bound=2.0)),
]
grid(items, name="hopfbrot_hybrid", max_steps=220, step=0.5, palette="plasma", trap_weight=1.2)
