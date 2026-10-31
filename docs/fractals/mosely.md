[All fractals](../FRACTALS.md#geometric) · [Vicsek Fractal →](vicsek.md)

# Mosely Snowflake (`mosely`)

*Geometric*

![Mosely Snowflake](../images/fractals/mosely.jpg)

A cube whose corners are notched off at every scale, leaving a crinkled block that looks like a six-pointed snowflake when you view it along a long diagonal. The light variant is hollow and can be seen straight through along its diagonals (pictured: `light` 1, looking down one of them).

**Formula.** Split the cube $`[-1,1]^3`$ into $`3\times3\times3`$ cells and drop the 8 corner cells, keeping 19 of 27 (the light variant also drops the center cell, keeping 18), then do the same to every cell that is left. Level $`i`$ uses cells $`2/3^i`$ across. One periodic fold covers all of them at once: $`u = 1 - \lvert 2\,\mathrm{fract}(c\,3^i/2) - 1\rvert`$ on each axis is 0 at a cell's center and 1 at its faces. In those coordinates the corners are the cells with $`\min(u) \gt  1/3`$, and their signed distance is

```math
h_i = \bigl\lVert \max(\tfrac13 - u,\, 0) \bigr\rVert - \max(\min(u) - \tfrac13,\, 0)
```

(for the center, a box of half size 1/3). The solid is the cube minus every level's cut, $`d = \max\bigl(\mathrm{box}(p),\ \max_i (-h_i / 3^i)\bigr)`$, as in ray's Menger sponge. The estimate is exact at each level and conservative overall. Dimension $`\log 19/\log 3 \approx 2.68`$ (light: $`\log 18/\log 3 \approx 2.63`$).

**Origin.** Named after Jeannine Mosely, the software engineer and origami artist known for her business-card Menger sponge. In 2012 she and Margaret Wertheim of the Institute For Figuring led the folding of a giant Mosely Snowflake Sponge from about 49,000 business cards, which was shown at USC's Doheny Library ([IFF](https://theiff.org/current/?p=1864), [Wikipedia](https://en.wikipedia.org/wiki/Mosely_snowflake)). It is the Menger sponge's opposite: the sponge removes the center and face cells, the snowflake removes the corners.

**Parameters.** `light` (0): 0 keeps 19 of 27 cells (the heavier snowflake), 1 also removes each center cell (the lighter one), which makes it hollow. `iterations` (4): levels of cutting as seen from afar (1 to 12). Close up, more levels are added on their own as the pixels allow, so it keeps finding detail.

---

[All fractals](../FRACTALS.md#geometric) · [Vicsek Fractal →](vicsek.md)
