[← Mandeltorus](mandeltorus.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Sphere Cluster →](sphere_cluster.md)

# Sphere Tree (`spheretree`)

*Spheres and tori*

![Sphere Tree](../images/fractals/spheretree.jpg)

A dome of spheres within spheres. Every ball is lined inside with a hexagonal layer of smaller balls, and each of those with smaller ones again, so the surface grows like a cauliflower of bubbles.

**Formula.** With $`z`$ up and the base sphere of diameter 1 resting on the origin, start from $`z = p`$ and repeat:

- Stop inside the solid inner sphere of diameter $`\sqrt3/(1+\sqrt3)`$ resting on the origin. After the first step, also stop where $`z_z \gt  0.4`$ and the point is outside the sphere of radius 0.6 about $`(0, 0, 0.6)`$; that region is empty.
- If $`z_z \lt  0.4`$ and the point is outside the base sphere, invert: $`z \leftarrow z/|z|^2`$.
- Otherwise, invert, which turns the base sphere into the plane $`w_z = 1`$, then flip and stretch below it and twist: $`z \leftarrow R_{30^\circ}\!\left(\sqrt3\,w_x,\ \sqrt3\,w_y,\ 1 + \sqrt3\,(1 - w_z)\right)`$ with $`w = z/|z|^2`$.
- Fold $`(z_x, z_y)`$ into the central hexagon of a unit triangular lattice, then into its kite, so a lattice of unit spheres resting on $`z = 0`$ stands for the base sphere's children.

Distance estimate: $`d = \left(|z - (0, 0, \tfrac12)| - \tfrac12\right)/(2.22\,\partial)`$, where $`\partial`$ multiplies the inversions' $`1/|z|^2`$ and the stretches' $`\sqrt3`$, and 2.22 is Mandelbulber's safety factor. At every step $`d`$ is also held below the inner sphere's distance and below $`\max(\text{base sphere distance}, -\text{distance into the stop region})`$, so it does not jump where a point stops one generation earlier.

**Origin.** By TGlad (Tom Lowe) on Fractal Forums in 2020 ("New sphere tree", [thread](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557/msg22100#msg22100)). Ported to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by mclarekin, with extras by sabine62, in June 2020. In Mandelbulber, the stops set the estimate to the current generation's sphere, which jumps between generations. The extra bounds here remove the jumps.

**Parameters.** `twist` (30): how far each generation's hexagonal layer turns against its parent's, 0 to 60 degrees. It moves where the small balls sit, a subtle change. `lattice` (2): the density of each hexagonal layer, 1.5 to 2.5. At 2 the balls touch; lower values leave fewer, bigger balls and higher ones crowd in more, smaller ones. `iterations` (4): generations of balls, each about five times finer; the renderer adds more close up, up to 16.

---

[← Mandeltorus](mandeltorus.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Sphere Cluster →](sphere_cluster.md)
