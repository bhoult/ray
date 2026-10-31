[← Kaleidoscopic IFS](kifs.md) · [All fractals](../FRACTALS.md#the-classics)

# Pseudo-Kleinian (`kleinian`)

*The classics*

![Pseudo-Kleinian](../images/fractals/kleinian.jpg)

Knighty's pseudo-Kleinian: box folds and sphere inversions, like a Mandelbox of scale 1, whose limit set is a lace of rings, arches and tunnels, a cathedral of coral that resembles the limit sets of Kleinian groups. It fills all of space, so the Add example cuts a 2 m cube out of it; it is at its best seen from inside (as here).

**Formula.** Start from $`q = p`$ and $`T = 1`$, and repeat `iterations` times:

```math
q \leftarrow 2\,\mathrm{clamp}(q, -b, b) - q, \qquad k = \max\!\bigl(r / |q|^2,\ 1\bigr), \qquad q \leftarrow k\,q, \qquad T \leftarrow k\,T
```

with $`b`$ the `fold_box` (a box fold with a different half size on each axis) and $`r`$ the `inversion`: points inside the sphere of radius $`\sqrt r`$ are inverted in it, and the rest are left alone. Then measure a ring-shaped set and scale back: with $`\rho = \sqrt{q_x^2 + q_y^2}`$,

```math
d = \max\!\Bigl(\rho - 0.92784,\ \frac{|\rho\, q_z|}{|q|}\Bigr) \Big/ T
```

Orbit trap: the smallest $`|q|`$ just after the box folds.

**Origin.** Theli-at found it while experimenting with the Amazing Box (the Mandelbox) and its folding parameters in Mandelbulb3D, and posted [Kleinian drops](https://www.deviantart.com/theli-at/art/Kleinian-drops-192676501) on DeviantArt on 8 January 2011, noting its likeness to Kleinian groups ([Kleinian group](https://en.wikipedia.org/wiki/Kleinian_group)). Knighty reduced it to a standalone formula, a scale-1 Julia Mandelbox plus a final shape, and shared it as a Fragmentarium script in May 2011 ([Fractal Forums](https://www.fractalforums.com/3d-fractal-generation/fragmentarium-an-ide-for-exploring-3d-fractals-and-other-systems-on-the-gpu/msg32270/#msg32270)). It ships with Fragmentarium as [`PseudoKleinian.frag`](https://github.com/Syntopia/Fragmentarium/blob/master/Fragmentarium-Source/Examples/Knighty%20Collection/PseudoKleinian.frag), whose "P3" preset is ray's default fold box with inversion 1. The final ring distance, with its 0.92784, is from eiffie's Shadertoy [Pseudo Knightyan](https://www.shadertoy.com/view/lls3Wf); it gives lacier rings than the thin "thingy" (thickness 0.01) in Knighty's script.

**Parameters.** `fold_box` ((0.92436, 0.90756, 0.92436)): half sizes of the box fold along x, y and z; small changes reshape the tunnels. `inversion` (1): the inversion sphere's radius squared, 0.1 to 4; larger opens the structure up. `iterations` (8): steps, 1 to 20; more give finer detail. Iterations are not added close up.

---

[← Kaleidoscopic IFS](kifs.md) · [All fractals](../FRACTALS.md#the-classics)
