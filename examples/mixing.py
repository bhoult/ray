"""Ways to mix fractals: hybrid iteration schedules, booleans, morphs."""
from _common import *

bulb, box, kifs = fk.Mandelbulb(8, iters=10), fk.Mandelbox(2.0, 0.5, iters=10), fk.KIFS(3.0, (1, 1, 1), iters=10)

# 1. iteration-level hybrids (Mandelbulber style). A Hybrid runs the *steps* of several
#    maps; a schedule entry that is a list runs several steps inside ONE iteration.
bulb2, bulb8 = fk.Mandelbulb(2), fk.Mandelbulb(8, iters=10)
box = fk.Mandelbox(2.0, 0.5, iters=10)
bulbbox = fk.Hybrid([fk.BoxFold(1.0), fk.SphereFold(0.5), fk.Scale(1.2), bulb2], [[0, 1, 2, 3]],
                    iters=14, finisher=3, bound=6.5, bail=30.0)          # ray's `bulbbox`, qualitatively
twisted = fk.Hybrid([fk.BoxFold(1.0), fk.SphereFold(0.5), fk.Scale(1.2), fk.Rotate(0, 0, 25), bulb2],
                    [[0, 1, 2, 3, 4]], iters=14, finisher=4, bound=6.5, bail=30.0)
kaleido = fk.Hybrid([fk.AbsFold(), fk.Rotate(0, 20, 0), bulb8], [[0, 1, 2]], iters=10, finisher=2, bound=1.6)
alt = fk.Hybrid([bulb8, fk.KIFS(3.0, (1, 1, 1), iters=10)], [0, 1], iters=10, finisher=0, bound=1.8)
far = dict(camera=fk.Camera(distance=24, elevation=15), step=0.5, max_steps=300)
hyb = [
    ("Mandelbulb n=8", bulb8),
    ("box+sphere fold, scale, bulb^2 (group)", bulbbox, far),
    ("same + 25 deg twist each iteration", twisted, far),
    ("abs fold + rotate + bulb n=8", kaleido, dict(step=0.6)),
    ("alternate bulb / KIFS steps", alt, dict(step=0.5, camera=fk.Camera(distance=4.2))),
    ("Mandelbox alone", box, dict(camera=fk.Camera(distance=24))),
]
grid(hyb, name="mix_hybrid", max_steps=200)

# 2. object-level: booleans and smooth blends of distance fields
menger = fk.Menger(4)
sph = fk.Sphere(1.3)
ops = [
    ("Menger & sphere", menger & sph),
    ("Menger - sphere(1.1)", menger - fk.Sphere(1.1)),
    ("Menger | torus", menger.scaled(0.8) | fk.Torus(1.2, 0.15).rotated(90, 0, 0)),
    ("smooth_union(0.15)", menger.scaled(0.8).smooth_union(fk.Torus(1.2, 0.15).rotated(90, 0, 0), 0.15)),
    ("Menger shell, one corner cut away", fk.Menger(3).shell(0.015) - fk.Box((1.2, 1.2, 1.2)).translated((1.0, -1.0, 1.0))),
    ("Menger repeated", fk.Menger(3).scaled(0.4).repeated(1.0) & fk.Sphere(1.6)),
]
grid(ops, name="mix_boolean", max_steps=200)

# 3. a morph between two fractals (linear blend of the estimates)
frames = [(f"t={t:.2f}", fk.Mandelbulb(8, iters=9).blend(fk.Menger(4), t)) for t in (0.0, 0.25, 0.5, 0.75, 1.0)]
grid(frames, ncols=5, size=(180, 135), name="mix_morph", max_steps=200, step=0.6)
