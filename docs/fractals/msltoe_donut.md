[All fractals](../FRACTALS.md#spheres-and-tori) · [Mandeltorus →](mandeltorus.md)

# Msltoe Donut (`msltoe_donut`)

*Spheres and tori*

![Msltoe Donut](../images/fractals/msltoe_donut.jpg)

A torus with nine smaller tori looped around its tube like chain links, each of those looped with nine smaller ones, and so on, so the ring sprouts bushy clusters of links. Unlike the escape-time bulbs, it is built from exact tori, so every generation is crisp.

**Formula.** In a frame with the ring about the third axis, start from $`(x, y, z) = p`$ and repeat for generation $`n = 0, 1, \dots`$:

```math
d_n = \frac{\sqrt{(\sqrt{x^2+y^2} - 1)^2 + z^2} - r}{f^{\,n}}, \qquad \theta_k = \frac{2\pi}{N}\,\mathrm{round}\!\left(\frac{N\,\mathrm{atan2}(y, x)}{2\pi}\right)
```

```math
(x', y') = R_{-\theta_k}(x, y), \qquad z \leftarrow f\,(x' - 1,\ z,\ y')
```

where $`d_n`$ is the distance to that generation's torus (ring radius 1, tube radius $`r`$), $`\theta_k`$ is the nearest of the $`N`$ sectors, and the map moves that sector's point on the ring to the center and turns the torus's cross-section plane into the next torus's ring plane. Distance estimate: $`d = \min_n d_n`$. A point stops early once no finer generation can come closer, because they all lie within $`\max\!\left(\frac{1}{f-1}, \frac{1+r}{f}\right)`$ of the ring line, scaled by $`f^{-n}`$.

**Origin.** Posted by msltoe on Fractal Forums in January 2016 ("Low-hanging dessert: an escape-time donut fractal", [thread](http://www.fractalforums.com/new-theories-and-research/low-hanging-dessert-an-escape-time-donut-fractal/msg90171/#msg90171)) and added to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by Krzysztof Marczak the same month. Mandelbulber runs it as an escape-time formula: a point inside the torus is divided by the torus function so that it escapes, and the distance comes from a numerical gradient. This port draws the same union of tori with an exact distance.

**Parameters.** `thickness` (0.1): the tube radius of every torus as a share of its ring radius; useful from 0.01 (wire rings) to 0.3 (fat doughnuts that crowd their links). `factor` (3): how many times smaller each generation is, 2 to 5; low values tangle the links into dense clusters, high ones leave them sparse. `number` (9): how many tori loop around each torus, 3 to 12. `iterations` (5): generations drawn; the renderer adds more close up (up to 12), assuming a factor of about 3.

---

[All fractals](../FRACTALS.md#spheres-and-tori) · [Mandeltorus →](mandeltorus.md)
