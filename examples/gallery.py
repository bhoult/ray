"""Every fractal in the model, sphere-traced, plus a few 2D slices."""
from _common import *

items = [
    ("Mandelbulb n=8", fk.Mandelbulb(8, iters=10)),
    ("Juliabulb", fk.Mandelbulb(8, iters=10, julia=(0.2, -0.64, 0.2))),
    ("Mandelbox s=2", fk.Mandelbox(2.0, 0.5, iters=10), dict(camera=fk.Camera(distance=24))),
    ("Menger (closed form)", fk.Menger(4)),
    ("KIFS twisted", fk.KIFS(2.2, (1, 0.9, 0.6), (10, 20, 5), iters=9)),
    ("Sierpinski", fk.Sierpinski(8)),
    ("Apollonian", fk.Apollonian(1.0, iters=7), dict(camera=fk.Camera(distance=3.0, fov=55))),
    ("Quaternion Julia", fk.QuaternionJulia((-0.2, 0.8, 0.0, 0.0), iters=12)),
    ("Hopfbrot n=3", fk.Hopfbrot(3, tilt=90, iters=10)),
]
grid(items, name="gallery", max_steps=160)

fig, axs = plt.subplots(1, 3, figsize=(11, 3.8))
fk.plot_slice(fk.Mandelbulb(8, iters=30), "y", 0.0, 1.3, 500, ax=axs[0], title="Mandelbulb, y=0 slice (distance)")
fk.plot_slice(fk.Hopfbrot(3, iters=40), "z", 0.0, 1.6, 500, mode="mask", ax=axs[1], title="Hopfbrot z=0 (cubic Mandelbrot set)")
fk.plot_slice(fk.Mandelbox(2.0, 0.5, iters=10), "z", 0.0, 3.0, 500, mode="trap", ax=axs[2], title="Mandelbox z=0 (orbit trap)")
fig.tight_layout(); fig.savefig(OUT / "slices.png", dpi=130); print("wrote", OUT / "slices.png")
