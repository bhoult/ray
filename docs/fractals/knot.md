[← Koch Delta](koch.md) · [All fractals](../FRACTALS.md#geometric)

# Knotted Rope (`knot`)

*Geometric*

![Knotted Rope](../images/fractals/knot.jpg)

A torus knot whose tube is a rope: strands twisted about its core, each of them a rope of finer strands twisted the other way, and so on down, the way rope is laid. It is a fractal that keeps its shape: close up, every strand turns out to be a rope too.

**Formula.** With $`\theta = \mathrm{atan2}(y, x)`$ and $`\rho = \sqrt{x^2 + y^2}`$, the point's place in the meridian plane, $`(\rho - 1, z)`$, is turned by $`(q/p)\,\theta`$ and folded to the nearest of the $`p`$ strands at distance $`r_g`$ from the core circle: that is DarkBeam's torus knot of $`p`$ laps and $`q`$ turns. The strand leans across the plane by $`\tan\alpha = r_g\,(q/p)/\rho`$, so its round section is the plane's ellipse: the section is taken square to it, $`(u, v) = (u, v\cos\alpha)/r_t`$ for the tube radius $`r_t`$, and $`\theta`$ is moved along the strand to its nearest point, $`\varphi`$. Each rope level then repeats the knot's step about the strand's own axis:

```math
\arg(u, v) \leftarrow \arg(u, v) + w_l\,\varphi, \quad \text{folded to the nearest of } n \text{ strands at } 1 - f, \quad (u, v) \leftarrow \big((u - (1 - f)),\ v\cos\beta_l\big)/f
```

with $`f`$ the strands' radius (a little over touching), $`\beta_l`$ their lean, and $`w_l = \pm\mathrm{round}(w f^{1-l})`$ the turns per lap, alternating in sign. Distance: at each level $`l`$, $`(|(u, v)| - 1)\,r_t f^l`$, no further than the nearest sibling strand at that level or above; the result is the largest of these. Far from a strand, it is divided by $`\sqrt{1 + g^2}`$, with $`g`$ bounding how much the twists stretch space. The orbit trap is the strand's index at each level, so every strand has its own color.

**Origin.** The knot is DarkBeam's "Not fractal but funny: trefoil knot routine" on Fractal Forums, refined by Knighty in 2012 (the "knot thingy" in [Fragmentarium](https://github.com/Syntopia/Fragmentarium)'s examples); Mandelbulber 2 has both as Knot V1 and V2. As those names say, it was not a fractal: the rope of ropes, with each level laid against the one above it, and the section taken square to the leaning strands are ray's own.

**Parameters.** `laps` (2) and `turns` (3): the torus knot $`T(p, q)`$, 1 to 8 and 0 to 8; 2 and 3 is the trefoil, 2 and 5 the cinquefoil, 3 and 4 a knot of 8 crossings; when they share a factor the knot falls into that many linked loops. `radius` (0.45): how far the knot strays from its core circle of radius 1, 0.1 to 0.6. `tube` (0.18): the rope's radius, 0.02 to 0.3; keep it under half the gap between passes. `strands` (3): strands in each rope, at every level, 2 to 6. `twist` (6): turns the strands make about their rope's core each lap of the knot, -24 to 24; each finer level turns proportionally faster, the other way; negative is the other hand, 0 lays them straight. `iterations` (4): levels, 1 a plain tube and each more a rope of finer strands; more are added close up.

---

[← Koch Delta](koch.md) · [All fractals](../FRACTALS.md#geometric)
