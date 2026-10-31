[← Hypercomplex Mandelbrot](hypercomplex.md) · [All fractals](../FRACTALS.md#algebras) · [Bristorbrot →](bristorbrot.md)

# Aexion (`aexion`)

*Algebras*

![Aexion](../images/fractals/aexion.jpg)

Aexion's quadray Mandelbrot set: the point is written in four tetrahedral coordinates and squared in four dimensions. At the default shift the set falls apart into six blade-like lobes along the axes, four around the middle and two at the poles, with Mandelbrot-style bulbs along their edges.

**Formula.** Write the point in folded quadray coordinates, each shifted by $`s`$,

```math
c = \big(|x+y+z|,\; |z-x-y|,\; |y-x-z|,\; |x-y-z|\big) + s,
```

start from $`z = c`$ and repeat

```math
z \leftarrow (A + B,\; B - A,\; C + D,\; D - C) + c
```

with $`A = z_1^2 - z_2^2`$, $`B = 2 z_3 z_4`$, $`C = z_3^2 - z_4^2`$ and $`D = 2 z_1 z_2`$. In complex pairs $`u = z_1 + i z_2`$, $`v = z_3 + i z_4`$ this is two complex squares that trade imaginary parts: $`u \leftarrow (1-i)(\mathrm{Re}\,u^2 + i\,\mathrm{Im}\,v^2) + c_u`$ and $`v \leftarrow (1-i)(\mathrm{Re}\,v^2 + i\,\mathrm{Im}\,u^2) + c_v`$. Escape at $`|z| = 10^4`$.
Distance estimate: $`d = \tfrac12 |z| \ln|z| / \partial`$ with $`\partial \leftarrow 2\,\frac{|f(z)|}{|z|}\,\partial + 2`$ from $`\partial = 2`$, where $`f`$ is the squaring alone and $`|f(z)| = \sqrt{2(|u|^4 + |v|^4)}`$ lies between $`|z|^2`$ and $`\sqrt2\,|z|^2`$. The estimate is blended with the previous iteration's as $`|z|`$ runs from the bailout to its square, so it doesn't jump where an orbit escapes one step later.

**Origin.** Aexion (the author of the Incendia renderer) posted his "Quadray Sets" on Fractal Forums on April 26, 2011: the OctoMandelbrot (no shift), an eight-dimensional mirrored variant, and the "Mandelbrot Six", with every quadray coordinate shifted by −1.35 ([thread](http://www.fractalforums.com/the-3d-mandelbulb/quadray-sets/msg31458/#msg31458)). Syntopia posted a GLSL distance estimate there the same day. Ported from Mandelbulber 2's Aexion formula, which starts the orbit at $`c`$ rather than 0 (one step ahead; the same set) and defaults to a shift of −1.3.

**Parameters.** `shift` (−1.3): added to each of the four quadray coordinates, −1.6 to 0. Around −1.3 you get the Mandelbrot Six; lower values part the lobes further, and toward 0 they merge into one small octahedral star (the OctoMandelbrot, about a fifth the size). `iterations` (20): few iterations fatten the lobes and join them in the middle; past about 20 little changes.

---

[← Hypercomplex Mandelbrot](hypercomplex.md) · [All fractals](../FRACTALS.md#algebras) · [Bristorbrot →](bristorbrot.md)
