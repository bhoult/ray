[← Indra's Cells](indra_cells.md) · [All fractals](../FRACTALS.md#originals) · [Labyrinth Sponge →](labyrinth_sponge.md)

# Zorich Pagodas (`zorich_pagodas`)

*Originals*

![Zorich Pagodas](../images/fractals/zorich_pagodas.jpg)

A floor of domed pagodas, each crowned with tiers of spiky pinnacles and ringed by smaller pinnacles that are ringed in turn, in a lattice that runs on forever. They are the points that escape upward under a three-dimensional analogue of the exponential function.

**Formula.** Zorich's map is the 3D counterpart of $`e^z`$. On the square beam $`|x_1|, |x_2| \le 1`$,

```math
Z(x) = e^{x_3}\,\frac{v}{|v|}, \qquad v = (x_1,\ x_2,\ 1 - \max(|x_1|, |x_2|)),
```

so the beam's cross-section goes radially onto the upper half of the unit sphere, and height becomes the logarithm of distance. Reflecting across the beam's sides extends $`Z`$ to all of space, period 4 in $`x_1`$ and $`x_2`$, with each reflection mirroring the image in the plane $`x_3 = 0`$. Space is mapped to $`(x_1, x_2, x_3) = (x, z, \text{depth} - y)`$. A point belongs to the shape if some iterate of $`x \mapsto \lambda Z(x)`$, the point itself included, reaches $`x_3 \ge \text{depth}`$. The floor $`y \le 0`$ has escaped already; the pagodas are the points that escape at the first, second, third... step.

Distance estimate. The radial projection's derivative is at most $`|Dv|/|v| \le \sqrt{2}/|v|`$, so $`|DZ| \le e^{x_3}\max(1, \sqrt{2}/|v|)`$, and $`L_n = \prod_{k\lt n} \lambda |DZ(x_k)|`$ bounds how fast the $`n`$th iterate moves. The estimate is

```math
d = \min\Big(\text{depth} - y,\ \min_n \frac{\min(\text{depth} - x_{3,n},\ 2)}{L_n}\Big).
```

This first-order estimate assumes $`x_3`$ climbs toward the escape level at a steady rate, but it climbs exponentially, so the estimate is off in both directions. Uncapped, it overshoots far below the escape level; capped too tightly (we tried 0.5), it creeps, and rays stall into grain around the pinnacles. The gap is capped at 2 map units and the step held at 0.25. The steepness test bounds the overshoot, at the defaults and at every corner of the parameter ranges we tried, and the renders show the creep is gone.

**Origin.** Invented for ray as a distance-estimated fractal by Claude (Anthropic's Claude Opus 5.5), October 2026; the map and its dynamics are mathematics' own. V. A. Zorich introduced the map in 1967 ("A theorem of M. A. Lavrent'ev on quasiconformal space maps"), as a quasiregular analogue of the exponential. Its iteration has been studied by W. Bergweiler ([*Karpińska's paradox in dimension three*](https://arxiv.org/abs/0902.2686), 2010), Bergweiler and D. Nicks ([*Foundations for an iteration theory of entire quasiregular maps*](https://arxiv.org/abs/1210.3972), 2014) and A. Tsantaris ([*Julia sets of Zorich maps*](https://www.cambridge.org/core/journals/ergodic-theory-and-dynamical-systems/article/julia-sets-of-zorich-maps/7E9F7B562CE5FA4A01D2420BDDB9BB8D), 2022). For small $`\lambda`$, its Julia sets are "Cantor bouquets" of hairs reaching up to infinity, of which these pagodas are the thickened, finitely iterated form. To our knowledge, it has not been rendered before. We searched the web, Fractal Forums and Shadertoy ("Zorich map fractal raymarch render shadertoy fractalforums", "Zorich map Julia set Cantor bouquet hairs three dimensions", "3D exponential Julia set quasiregular Cantor bouquet ray traced"), Mandelbulber 2's formula list and Fragmentarium's examples, and found only the papers. Nearest relatives: the 2D exponential Julia sets of $`\lambda e^z`$ (Devaney and Krych's Cantor bouquets), and, among 3D fractals, the Mandelbulb, which builds a 3D analogue of $`z^n`$ much as this builds one of $`e^z`$.

**Parameters.** `lambda` (0.42): the map's factor, 0.2 to 0.5. Small values grow low domes with a few spikes, values around 0.4 grow tall tiered pagodas, and toward 0.5 the pagodas branch into a tangled lace. (Beyond 0.5, and for `depth` beyond 4, the finest tiers turn to grain, so the ranges stop there.) `depth` (3): the escape level's height above the floor, in the map's units, 1.5 to 4; low values leave domes with thin spines, and high ones grow dense towers. `iterations` (6): steps of the map; each adds a tier of smaller pinnacles. The lattice is infinite, so Add cuts a 2.4 m square of it, holding five pagodas, with a box. The pagodas stand on a square lattice turned 45°, $`2\sqrt{2}`$ units apart.

---

[← Indra's Cells](indra_cells.md) · [All fractals](../FRACTALS.md#originals) · [Labyrinth Sponge →](labyrinth_sponge.md)
