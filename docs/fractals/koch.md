[← Polyhedral IFS](polyhedral.md) · [All fractals](../FRACTALS.md#geometric) · [Knotted Rope →](knot.md)

# Koch Delta (`koch`)

*Geometric*

![Koch Delta](../images/fractals/koch.jpg)

Eric Baird's Koch curve in three dimensions, the "delta" fractal: a triangular body whose copies hinge up out of its plane at the Koch angle, again and again, so that its edges and profile are Koch curves and its body is a crumpled crystal. The angle runs from a flat Koch tree at 60° to a solid at 90°.

**Formula.** With the Koch angle $`\theta`$, let $`\beta = \theta/2`$, $`t = \tfrac14\sqrt{3\tan^2\beta - 1}`$, the hinge normal $`\mathbf{h} = (1, 0, -2t)/\sqrt{1 + 4t^2}`$ and the scale $`s = 4\cos^2\beta = 2(1 + \cos\theta)`$. Start from $`\mathbf{z} = \mathbf{p}`$ and repeat: mirror $`y \leftarrow |y|`$, $`z \leftarrow |z|`$; fold the plane into the triangle's 60° wedge,

```math
(x, y) \leftarrow (x, y) - 2\min\!\big(0, \tfrac{\sqrt3}{2}x - \tfrac12 y\big)\big(\tfrac{\sqrt3}{2}, -\tfrac12\big), \qquad y \leftarrow |y|;
```

fold the raised half of the Koch bump onto the flat one about their hinge through $`\mathbf{o} = (\tfrac12, 0, t)`$,

```math
\mathbf{z} \leftarrow \mathbf{z} - 2\min\!\big(0, (\mathbf{z} - \mathbf{o})\cdot\mathbf{h}\big)\,\mathbf{h};
```

and scale about the corner: $`\mathbf{z} \leftarrow s\,(\mathbf{z} - (1, 0, 0)) + (1, 0, 0)`$. The unit ball holds the fractal and every image of itself, so it ends as a ball: $`d = (|\mathbf{z}| - 1)\,s^{-N}`$. The orbit trap records which copies were hinged up, in the first generations more than the later ones.

**Origin.** Eric Baird, [The Koch curve in three dimensions](https://www.researchgate.net/publication/262600735_The_Koch_curve_in_three_dimensions) (2014); Wikipedia's [Koch snowflake](https://en.wikipedia.org/wiki/Koch_snowflake) article lists it as the "Koch curve in 3D". Knighty coded it as a kaleidoscopic IFS in Fragmentarium, and the Mandelbulber team ported that to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) as Koch V3 ("Baird Delta"), which this is ported from. ray ends it in the unit ball rather than Mandelbulber's radius 3, which keeps it within its bound at every iteration count.

**Parameters.** `angle` (70): the Koch angle in degrees, 60 to 90. At 60 the copies lie flat, a planar Koch tree; more raises them out of the plane into a crumpled delta; toward 90 they pack into a solid. Each generation is $`2(1 + \cos\theta)`$ times smaller: 3 at 60°, 2.68 at 70°, 2 at 90°. `iterations` (8): generations of copies; more are added close up.

---

[← Polyhedral IFS](polyhedral.md) · [All fractals](../FRACTALS.md#geometric) · [Knotted Rope →](knot.md)
