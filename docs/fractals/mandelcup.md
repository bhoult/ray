[← Buffalo](buffalo.md) · [All fractals](../FRACTALS.md#algebras) · [Quick Dudley →](quick_dudley.md)

# Mandelcup (`mandelcup`)

*Algebras*

![Mandelcup](../images/fractals/mandelcup.jpg)

A goblet: a smooth bowl on a stem, with layered ornament inside the bowl and at the foot. Each point's direction is projected onto a plane, squared there as a complex number and wrapped back onto the sphere, and its distance is squared.

**Formula.** Start from $`z = c = p`$. With $`r = |z|`$, shape $`s`$ and stretch $`k`$, let

```math
w = k\,(1 + s^2)\left(\frac{x + i y}{z + s r}\right)^2
```

(a gnomonic projection of the direction, squared) and repeat

```math
z \leftarrow r^2\,\frac{\big(2\,\mathrm{Re}\,w,\; 2\,\mathrm{Im}\,w,\; |w|^2 - 1\big)}{|w|^2 + 1} + c,
```

an inverse stereographic projection back onto the sphere, scaled by $`r^2`$. (Written over a common denominator so the equator, where the projection runs off to infinity, stays finite.) The formula's own z axis points down, so the cup stands upright. Escape at $`|z| = 100`$.
Distance estimate: $`d = \tfrac12 r \ln r / \partial`$ with $`\partial \leftarrow 2r\,\partial + 1`$, blended across the escape as for `aexion`. The projection stretches up to twice the radial rate, which the march's step (0.6) allows for.

**Origin.** pupukuusikko posted "A new 3d mandelbrot variant - Mandelcup" on Fractal Forums on November 12, 2015 ([thread](http://www.fractalforums.com/the-3d-mandelbulb/a-new-3d-mandelbrot-variant-mandelcup/)). He found it by varying Tglad's formulas from the "2D conformal formula suggestion" thread, and noted its grail-like shape. His later version's constants c1 and c2 are `shape` and `stretch` here. Ported from Mandelbulber 2's Mandelcup, which normalizes the direction and scales the result by $`r^2`$.

**Parameters.** `stretch` (2): multiplies the projected square, 0.25 to 4. At 2, the original's value, you get the goblet; 1 (Mandelbulber's default) gives a straighter vase; higher values give a rounder, shallower bowl. `shape` (0): added to the direction's height before the projection, −1 to 1. Positive values close the cup into a rounded pod; negative values open a wide, shallow bowl on a thin stem. `iterations` (12): more iterations give finer ornament in the bowl.

---

[← Buffalo](buffalo.md) · [All fractals](../FRACTALS.md#algebras) · [Quick Dudley →](quick_dudley.md)
