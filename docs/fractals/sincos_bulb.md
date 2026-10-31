[← Lambdabulb](lambda_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Riemann Sphere →](riemann_sphere.md)

# Sine/cosine bulb (`sincos_bulb`)

*Bulbs*

![Sine/cosine bulb](../images/fractals/sincos_bulb.jpg)

A Mandelbulb whose polar angle is measured with acos, asin, or a blend of the two, then put back as a latitude, so sine and cosine trade places. The default blend halves the polar angle's power against the longitude's, which turns the bulb into stacked onion domes draped in long folds, with star-centered buds around them.

**Formula.** With $`z`$ up, start from $`z = p`$ and repeat:

```math
\theta = n\left((1-m)\arccos\tfrac{z}{r} + m\arcsin\tfrac{z}{r} + \beta\right), \qquad \varphi = n\left(\mathrm{atan2}(y, x) + \alpha\right)
```

```math
z \leftarrow r^n\left(\cos\theta\cos\varphi,\; \cos\theta\sin\varphi,\; \sin\theta\right) + o + p, \qquad z_z \leftarrow s\,z_z
```

with the `mix` $`m`$, the `angles` $`(\alpha, \beta)`$, the `offset` $`o`$ and the `z_scale` $`s`$. Since $`\arcsin = \pi/2 - \arccos`$, the polar angle's multiplier is $`n(1 - 2m)`$: 0.25 halves it. Distance estimate: $`d = \tfrac12 r\ln r/\partial z`$ with $`\partial z \leftarrow (n r^{n-1}\partial z + 1)\max(1, s)`$; the orbit trap is the nearest the orbit comes to the pole axis ($`\sqrt{x^2+y^2}/r`$).

**Origin.** Graeme McLaren added `mandelbulb_sin_cos.cl` to Mandelbulber 2 in December 2021, as a variation on the Mandelbulb of Daniel White and Paul Nylander ([Fractal Forums, 2009](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/)). With `mix` 0 (pure acos) it differs from the ordinary bulb in detail but looks much like it, so the default is a quarter of the way to asin, which Mandelbulber's blend control also allows. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `power` (9): the exponent (3 to 16); higher powers give more, finer lobes. `mix` (0.25): blends the polar angle from acos (0) to asin (1, the ordinary bulb); 0.15 to 0.35 is the useful range, and at 0.5 the bulb collapses. `z_scale` (1): scales the vertical component after each step; below 1 squashes the domes, above 1 stretches them (0.75 to 1.5). `offset` (0, 0, 0): added after each step, which shifts the orbit and makes the bulb lopsided (-0.5 to 0.5). `angles` (0, 0): degrees added to the longitude and the polar angle before the power, which twist and tilt the lobes. `iterations` (10): more give finer detail.

---

[← Lambdabulb](lambda_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Riemann Sphere →](riemann_sphere.md)
