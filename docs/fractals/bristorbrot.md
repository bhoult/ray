[← Aexion](aexion.md) · [All fractals](../FRACTALS.md#algebras) · [Buffalo →](buffalo.md)

# Bristorbrot (`bristorbrot`)

*Algebras*

![Bristorbrot](../images/fractals/bristorbrot.jpg)

Doug Bristor's 3D Mandelbrot set: the Mandelbrot set's outline along x, swept around into a swirled, lopsided solid with a long antenna. Squaring in his algebra mixes the two imaginary axes, which gives the surface its twisted bands.

**Formula.** Start from $`z = c = p`$ (shifted so the solid sits centered) and repeat

```math
z \leftarrow \big(x^2 - y^2 - z^2,\; y\,(2x - t z),\; z\,(2x + t y)\big) + c
```

with $`t`$ the twist ($`t = 1`$ is Bristor's algebra). Escape at $`|z| = 100`$.
Distance estimate: $`d = \tfrac12 |z| \ln|z| / \partial`$ with $`\partial \leftarrow 2\,\frac{|f(z)|}{|z|}\,\partial + 1`$, $`f`$ the squaring alone (its stretch varies with direction, up to about $`3|z|`$, which the march's step allows for), blended across the escape as for `aexion`.

**Origin.** Doug Bristor's "rotational" complex numbers, two imaginary units whose products depend on their order (according to his site, [bristorbrot.org](https://bristorbrot.org/about/), he worked them out in the mid-1990s). He posted "Bristorbrot 3D" on Fractal Forums on November 28, 2009, with his original 3D iteration and renders by Jos Leys ([thread](http://www.fractalforums.com/theory/bristorbrot-3d/)). Ported from Mandelbulber 2's Bristorbrot. The twist parameter is this port's addition.

**Parameters.** `twist` (1): how much the two imaginary axes mix, 0 to 2. At 1 you get Bristor's algebra; 0 spins the Mandelbrot set about its axis into a smooth solid of revolution; above 1 it shears further. `iterations` (12): more iterations give finer bands and antenna detail.

---

[← Aexion](aexion.md) · [All fractals](../FRACTALS.md#algebras) · [Buffalo →](buffalo.md)
