[← Menger Sponge](menger.md) · [All fractals](../FRACTALS.md#the-classics) · [Mandelbox →](mandelbox.md)

# Mandelbulb (`mandelbulb`)

*The classics*

![Mandelbulb](../images/fractals/mandelbulb.jpg)

The best-known 3D relative of the Mandelbrot set: a round bulb covered in buds, ridges and cauliflower-like growths, made by raising a point to a power in spherical coordinates. ray stands it upright, with the y axis as its pole.

**Formula.** Start from $`z = p`$ and $`dr = 1`$, and while $`r = |z| \lt  2`$, for at most `iterations` steps, repeat:

```math
\theta = \mathrm{atan2}\!\bigl(\sqrt{z_x^2 + z_z^2},\ z_y\bigr), \qquad \phi = \mathrm{atan2}(z_z,\ z_x)
```

```math
dr \leftarrow n\,r^{\,n-1}\,dr + 1, \qquad z \leftarrow r^n\bigl(\sin n\theta\,\cos n\phi,\ \cos n\theta,\ \sin n\theta\,\sin n\phi\bigr) + p
```

with $`n`$ the `power`. This is White and Nylander's formula with $`\theta`$ measured from the $`+y`$ axis instead of $`+z`$; starting from $`z = p`$ is the same as starting from 0 and taking one more step. Distance estimate: $`d = \tfrac12\, r \ln r / dr`$, from the last radius and the running derivative. Orbit trap: the smallest $`|z|`$ along the orbit, halved.

**Origin.** Daniel White (twinbee) started the Fractal Forums thread [True 3D mandelbrot type fractal](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/) in September 2007, and that November proposed doubling a point's angles in spherical coordinates, as complex squaring does in the plane. In 2009 Paul Nylander raised the power instead of squaring, and power 8 gave the Mandelbulb. White tells the story in [The Unravelling of the Real 3D Mandelbrot Fractal](https://www.skytopia.com/project/fractal/mandelbulb.html) (8 November 2009); see also [Mandelbulb](https://en.wikipedia.org/wiki/Mandelbulb). The distance estimate is the standard running-derivative estimate for escape-time fractals, as in Inigo Quilez's [Mandelbulb](https://iquilezles.org/articles/mandelbulb/) article and Mikael Hvidtfeldt Christensen's [Distance Estimated 3D Fractals (V)](https://blog.hvidtfeldts.net/index.php/2011/09/distance-estimated-3d-fractals-v-the-mandelbulb-different-de-approximations/).

**Parameters.** `power` (8): the exponent, 2 to 16. 8 is the classic bulb; higher powers give more, finer lobes, lower ones fewer and broader. `iterations` (8): escape-time steps, 1 to 32. More steps give finer detail and a crisper surface; each step costs a pow, two atan2, a sin and a cos per point. Iterations are not added close up.

---

[← Menger Sponge](menger.md) · [All fractals](../FRACTALS.md#the-classics) · [Mandelbox →](mandelbox.md)
