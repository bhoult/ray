[← Mandelbar](mandelbar.md) · [All fractals](../FRACTALS.md#bulbs) · [Quadratic Mandelbulb →](power2_bulb.md)

# Burning Ship (`burning_ship`)

*Bulbs*

![Burning Ship](../images/fractals/burning_ship.jpg)

The Burning Ship in 3D: seen from the front, its silhouette is the familiar ship, masts and flames on top and the keel's needle trailing to the left, while in depth the hull rounds out into a swirled, glassy bowl.

**Formula.** Take the absolute value of every component, then apply the Mandelbulb's power map in its latitude form, about a pole running front to back ($`r = |\mathbf z|`$, latitude $`\theta = \mathrm{atan2}(z, \sqrt{x^2+y^2})`$, longitude $`\phi = \mathrm{atan2}(y, x)`$, with $`x, y, z`$ the absolute values of the components):

```math
\mathbf z \leftarrow r^n\,(\cos n\theta\,\cos n\phi,\ \cos n\theta\,\sin n\phi,\ \sin n\theta) + c
```

from $`\mathbf z = c`$, where $`c`$ is the point with $`y`$ flipped (so the masts point up, as the 2D ship is always shown) and, for $`n = 2`$, shifted to center the hull. In the plane $`z = 0`$ this is exactly $`(|x| + i|y|)^2 + c`$. With `polar` set, the map works in polar angles about the $`x`$ axis instead ($`\theta = \mathrm{atan2}(\sqrt{y^2+z^2}, x)`$, $`\phi = \mathrm{atan2}(z, y)`$, $`\mathbf z \leftarrow r^n(\cos n\theta,\ \sin n\theta\cos n\phi,\ \sin n\theta\sin n\phi) + c`$), which leaves the same 2D ship in the plane $`z = 0`$. Distance estimate: $`d = \tfrac12\,r\ln r/r'`$ with $`r' \leftarrow n\,r^{n-1}r' + 1`$ (absolute values are reflections, which keep lengths). The latitude map jumps across its pole, so the estimate overshoots near it, and the march takes 0.6 of each step.

**Origin.** The Burning Ship, $`(|\mathrm{Re} z| + i\,|\mathrm{Im} z|)^2 + c`$, was introduced by Michael Michelitsch and Otto E. Rössler ("The 'Burning Ship' and its quasi-Julia sets", *Computers & Graphics* 16(4), 1992). Its 3D form, the standard Mandelbulb with absolute values taken on all three axes before the power, was posted with a Fragmentarium script by laser blaster on Fractal Forums in October 2014 (["3D Burning Ship formula and .frag file"](https://www.fractalforums.com/fragmentarium/3d-burning-ship-formula-and-frag-file/)); Mandelbulber 2 offers the same as an option of its Mandelbulb Abs formula (`mandelbulb_abs.cl`). The polar variant is this port's addition: its map is continuous at the pole, so its estimate is close to exact and its hull comes out crisper and more crystalline.

**Parameters.** `power` (2): the exponent, 2 to 8. 2 is the Burning Ship; 3 and 4 fold it into a rounder hull with more points, and higher powers into a knobbly ball. `polar` (0): 0 is the original latitude form; 1 the polar form, crisper and with a truer estimate. `iterations` (12, up to 32): finer spires and smaller ships along the keel, at a pow, two atan2, a sine and a cosine per point each.

---

[← Mandelbar](mandelbar.md) · [All fractals](../FRACTALS.md#bulbs) · [Quadratic Mandelbulb →](power2_bulb.md)
