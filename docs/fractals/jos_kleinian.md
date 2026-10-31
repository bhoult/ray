[← Sphere Cluster](sphere_cluster.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Schottky group →](schottky.md)

# Jos Leys's Kleinian (`jos_kleinian`)

*Spheres and tori*

![Jos Leys's Kleinian](../images/fractals/jos_kleinian.jpg)

The limit set of a true Kleinian group: floors of great domes with necklaces of ever smaller spheres strung between them into columns, layer upon layer. It is what the pseudo-Kleinian imitates, done with the group itself.

**Formula.** Start from $`z = p`$ and $`\partial = 1`$ (or from $`p`$ inverted in the sphere `inversion_centre`, `inversion_radius`, with $`\partial`$ that inversion's scale) and repeat: wrap $`z`$ into the cell $`[-1, 1) \times [0, a) \times [-1, 1)`$ (with `box` at 1); above the separation line, $`z_y \ge a\left(\tfrac12 + \tfrac15 \sin\left(\pi\,\mathrm{sgn}(b)\,(z_x + \tfrac b2)\right)\right)`$, turn it half round, $`z \leftarrow (-b, a, 0) - z`$; then apply the generator

```math
z \leftarrow \left(\frac{z_x}{|z|^2} - b,\; a - \frac{z_y}{|z|^2},\; -\frac{z_z}{|z|^2}\right), \qquad \partial \leftarrow \frac{\partial}{|z|^2},
```

an inversion in the unit sphere and a half turn. In the plane $`w = z_x + i z_y`$ it is Maskit's generator $`w \mapsto t + 1/w`$ with $`t = -b + ai`$, and the wrap in $`x`$ is the other generator, $`w \mapsto w + 2`$. The loop stops when $`|z| \gt  10`$ or the orbit closes a 2-cycle. Distance estimate (Knighty's): $`d = \min(h, c_y) / \max(\partial, c_\partial)`$, where $`h = \min(z_y, a - z_y)`$ is the height above the floor or below the ceiling (just $`z_y`$ with `spheres` at 0), taking the smaller of its values at the last two steps.

**Origin.** Jos Leys's escape-time algorithm for the limit sets of Kleinian groups in Maskit's slice, from his Fractal Forums thread [An escape time algorithm for Kleinian group limit sets](http://www.fractalforums.com/3d-fractal-generation/an-escape-tim-algorithm-for-kleinian-group-limit-sets/) (2016-17) and his paper [A fast algorithm for limit sets of Kleinian groups with the Maskit parametrisation](http://www.josleys.com/articles/Kleinian%20escape-time_3.pdf) (January 2017), with the distance estimate by Knighty; more of his pictures are at [josleys.com](https://www.josleys.com/galleries.php). Ported from Mandelbulber 2's `jos_kleinian.cl` (added as JosLeys-Kleinian in v2.12, November 2017), with two changes. Like Jos Leys's original, it stops once the orbit closes a 2-cycle. It also takes the smaller estimate of the cycle's two phases, so neighboring points that stop in different phases agree. The defaults differ too: $`a = 1.9`$ rather than 2 (open necklaces rather than cusps), and a step clamp of 0.2 rather than 0.05. At 0.05 the estimate is 5 to 10 times too small near the cusps, and that shows as fuzz. With $`b \ne 0`$, Mandelbulber's unsheared wrap makes the estimate jump in places; lower `clamp` if that shows.

**Parameters.** `klein` ([1.9, 0]): Jos Leys's KleinR and KleinI. The first, $`a`$, is the height of a layer (1.8 to 2.1, at least 0.5): below 2 the necklaces open into columns of beads, at 2 they pinch to points. The second, $`b`$ (-0.2 to 0.2), curls them into seahorse tails, though the estimate gets rougher. `box` ([1, 1, 1]): half sizes of the wrap cell along x, y and z. At 1 it is the group's own period; other values cut and repeat it. y also spaces the layers: 5 or more leaves a single layer between a solid floor and ceiling. `spheres` (1): 1 draws the solid spheres on both faces of each layer, the classic look; 0 draws only the floor's images, a finer and dustier lattice. `clamp` ([0.2, 1]): the most one step of the estimate may be, and the least the maps' scale may count as. Smaller is safer and slower; neither changes the shape. `inversion_centre` ([0, 1, 0]) and `inversion_radius` (0): an inversion applied before anything else, off at radius 0. About a point inside a layer it bends the stacked layers into a tower of nested bubbles. With a single layer (`box` y of 5 or more), `spheres` at 0 and the center above the layer, it folds the whole infinite layer into one bounded cluster. `iterations` (20): steps of the maps, up to 64; more resolves smaller spheres in the necklaces, at linear cost.

---

[← Sphere Cluster](sphere_cluster.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Schottky group →](schottky.md)
