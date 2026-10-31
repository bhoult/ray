[All fractals](../FRACTALS.md#bulbs) · [Mandelbar →](mandelbar.md)

# Juliabulb (`juliabulb`)

*Bulbs*

![Juliabulb](../images/fractals/juliabulb.jpg)

The Mandelbulb's Julia sets: the bulb's power-8 map with a fixed constant added at every step in place of the point. Every constant sculpts something different; the default stacks knobbed, ringed tiers studded with small rosettes, like an ornate urn.

**Formula.** Start from $`\mathbf z = p`$ and repeat, in spherical coordinates about the vertical axis ($`r = |\mathbf z|`$, $`\theta = \mathrm{atan2}(\sqrt{x^2+z^2},\, y)`$, $`\phi = \mathrm{atan2}(z, x)`$):

```math
\mathbf z \leftarrow r^n\,(\sin n\theta\,\cos n\phi,\ \cos n\theta,\ \sin n\theta\,\sin n\phi) + c
```

until $`r \ge 2`$. Distance estimate: $`d = \tfrac12\, r \ln r / r'`$, with $`r' \leftarrow n\, r^{n-1}\, r'`$ from $`r' = 1`$ (no $`+1`$: the constant does not depend on the point).

**Origin.** Julia sets arrived with the Mandelbulb itself. Daniel White published his "triplex" squaring formula on Fractal Forums in November 2007, and Paul Nylander rendered its Julia set, a "3D Juliabulb", in July 2009, along with a 4th-order one, a month before his 8th-power Mandelbulb. Krzysztof Marczak (who wrote Mandelbulber), David Makin, Tom Beddard and Iñigo Quílez followed with Juliabulbs of other powers and with animations ([Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/), [White's Mandelbulb history](https://www.skytopia.com/project/fractal/mandelbulb.html)). This kernel is ray's built-in Mandelbulb with $`c`$ held fixed.

**Parameters.** `power` (8): the exponent, 2 to 16; 8 is the classic bulb's, and higher powers give more, finer lobes. `c` ([0.2, -0.64, 0.2]): the constant added at every step. Small changes reshape the set completely: near the origin it is a lumpy ball, with a large vertical part (like the default) it stacks into ringed tiers, and much past 0.8 from the origin it breaks into islands, where the estimate can overshoot in the gaps (lower the node's `step` if holes appear). `iterations` (10, up to 32): finer detail, at a pow, two atan2, a sine and a cosine per point each.

---

[All fractals](../FRACTALS.md#bulbs) · [Mandelbar →](mandelbar.md)
