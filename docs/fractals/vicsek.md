[← Mosely Snowflake](mosely.md) · [All fractals](../FRACTALS.md#geometric) · [Cantor Dust →](cantor_dust.md)

# Vicsek Fractal (`vicsek`)

*Geometric*

![Vicsek Fractal](../images/fractals/vicsek.jpg)

A 3D cross whose six arms are themselves crosses, and so on down: a branching, loop-free tree of beams. Seen face on, it is the flat Vicsek cross. From any other angle, its arms reach out in all six directions.

**Formula.** Of the cube's $`3\times3\times3`$ cells keep only the center and the six face cells (7 of 27), and repeat in each. Using the same periodic fold as the Mosely snowflake, the cut cells are those where at least two of $`u`$ pass $`1/3`$. With $`a \ge b`$ the two largest of $`u`$, its distance is

```math
h_i = \bigl\lVert \max(\tfrac13 - (a, b),\, 0) \bigr\rVert - \max(b - \tfrac13,\, 0)
```

and $`d = \max\bigl(\mathrm{box}(p),\ \max_i (-h_i / 3^i)\bigr)`$. Dimension $`\log 7/\log 3 \approx 1.77`$, so it thins toward lines as the levels go up.

**Origin.** Tamás Vicsek introduced the flat version in 1983 as a deterministic model of diffusion-limited aggregation ("Fractal models for diffusion controlled aggregation", *J. Phys. A* 16, L647). This is its natural 3D form, also called the box fractal ([Wikipedia](https://en.wikipedia.org/wiki/Vicsek_fractal)).

**Parameters.** `iterations` (4): levels of crosses as seen from afar (1 to 12). Each level builds the arms out of crosses a third the size. Close up, more levels are added on their own.

---

[← Mosely Snowflake](mosely.md) · [All fractals](../FRACTALS.md#geometric) · [Cantor Dust →](cantor_dust.md)
