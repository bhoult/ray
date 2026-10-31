[← Vicsek Fractal](vicsek.md) · [All fractals](../FRACTALS.md#geometric) · [Jerusalem Cube →](jerusalem.md)

# Cantor Dust (`cantor_dust`)

*Geometric*

![Cantor Dust](../images/fractals/cantor_dust.jpg)

A cloud of cubes in clusters of eight, the clusters in clusters of eight, with none of them touching. It is the three-dimensional Cantor set: a set with no volume and no connected piece larger than a point.

**Formula.** Keep only the 8 corner cells of the $`3\times3\times3`$ split, and repeat. What a level keeps is exactly what the Mosely snowflake cuts. So with the same fold, the solid at level $`i`$ has the exact signed distance

```math
g_i = \bigl\lVert \max(\tfrac13 - u,\, 0) \bigr\rVert - \max(\min(u) - \tfrac13,\, 0)
```

and $`d = \max\bigl(\mathrm{box}(p),\ \max_i\, g_i / 3^i\bigr)`$. That makes it the product $`C\times C\times C`$ of the middle-thirds Cantor set $`C`$, with dimension $`\log 8/\log 3 \approx 1.89`$.

**Origin.** Georg Cantor described the middle-thirds set in 1883 (it had appeared earlier in work by Henry Smith). Benoit Mandelbrot popularized the name "Cantor dust" for it and its higher-dimensional versions in *The Fractal Geometry of Nature* (1982).

**Parameters.** `iterations` (4): levels of splitting as seen from afar (1 to 12). Each splits every cube into eight a third the size. Close up, more levels are added on their own.

---

[← Vicsek Fractal](vicsek.md) · [All fractals](../FRACTALS.md#geometric) · [Jerusalem Cube →](jerusalem.md)
