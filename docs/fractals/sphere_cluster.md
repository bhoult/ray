[← Sphere Tree](spheretree.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Jos Leys's Kleinian →](jos_kleinian.md)

# Sphere Cluster (`sphere_cluster`)

*Spheres and tori*

![Sphere Cluster](../images/fractals/sphere_cluster.jpg)

A hollow ball of sphere clusters with icosahedral symmetry, spiked like a snowflake. Clusters crowd toward an inner sphere in ever smaller copies, and every ball in a cluster is a smaller cluster in turn.

**Formula.** Twenty face spheres of radius $`r`$, centered at distance $`l = \sqrt{1 + r^2}`$ along the icosahedron's face directions so that each meets the unit sphere at right angles, leave a hole of radius $`h = l - r`$ about the center. For each generation, starting from $`z = p`$:

```math
z \leftarrow h\,\frac{z}{|z|^2}
```

Then, up to three times, fold $`z`$ into the icosahedron's fundamental cell with its mirror planes and, if it lies inside the face sphere there, invert in that sphere:

```math
z \leftarrow c + r^2\,\frac{z - c}{|z - c|^2}
```

A point that was never inverted is in the hole itself, which is empty, so it is inverted once toward the nearest copy. Then $`z \leftarrow z/(h k)`$ with $`k`$ = `pack` for the next generation. Distance estimate, with $`\Delta = |z| - hk`$ after the last generation:

```math
d = \frac{\Delta}{\partial + S\,\Delta}
```

where $`\partial`$ multiplies every inversion's and scaling's factor and $`S = \sum_j \partial_j/|z_j|`$ over the inversions about the center. A sphere inversion can shrink a distance by no more than that term, so the estimate does not outgrow the true distance far from the center.

**Origin.** By TGlad on Fractal Forums in 2023 ("Fractal clusters", [thread](https://fractalforums.org/fractal-mathematics-and-new-theories/28/fractal-clusters/5107)). Ported to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by mclarekin in June 2023. Mandelbulber recurses only from inside the hole and measures a ball from outside it, which makes the estimate jump at the hole's edge. It also gives everything beyond radius 1.5 a fixed distance. This port always recurses and bounds the inversions instead, so the estimate is steady.

**Parameters.** `pack` (0.8): how much of its gap each sub-cluster fills, 0.5 to 0.95. Low values leave airy, separate clusters; high ones swell them until they touch. `dodecahedral` (0): 0 hangs the sub-clusters over the 20 faces of an icosahedron, and 1 hangs them over the 12 faces of a dodecahedron, for fewer, bigger spikes. `alternate` (0): 1 switches between the two arrangements at every generation. `iterations` (4): generations, each turning the balls of the last into clusters; the renderer adds more close up, up to 8.

---

[← Sphere Tree](spheretree.md) · [All fractals](../FRACTALS.md#spheres-and-tori) · [Jos Leys's Kleinian →](jos_kleinian.md)
