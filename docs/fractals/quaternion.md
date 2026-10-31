[All fractals](../FRACTALS.md#algebras) · [Tetrabrot →](tetrabrot.md)

# Quaternion Mandelbrot (`quaternion`)

*Algebras*

![Quaternion Mandelbrot](../images/fractals/quaternion.jpg)

The Mandelbrot set iterated with quaternions and sliced in 3D. Squaring a quaternion never takes it out of the plane spanned by the real axis and its own imaginary part, so the set is the Mandelbrot set spun about its real axis: a lathe-turned ornament of discs and rings with the antenna for a spire. Moving the slice along the fourth axis cuts it down to a dish.

**Formula.** With the point $`p`$, let $`c = (p_x - 0.75) + p_y\,i + p_z\,j + w\,k`$, start from $`q = c`$ and repeat:

```math
q \leftarrow q^2 + c, \qquad (a + \mathbf{v})^2 = a^2 - |\mathbf{v}|^2 + 2a\,\mathbf{v}
```

where $`a`$ is the real part and $`\mathbf{v}`$ the imaginary part. Distance estimate (Inigo Quilez's, as for ray's quaternion Julia set, with the point as $`c`$): $`d = |q| \ln|q| / (2|q'|)`$ with $`|q'| \leftarrow 2|q|\,|q'| + 1`$, and $`d = 0`$ for an orbit still bounded after the last iteration.

**Origin.** Quaternion Julia sets were first drawn by Alan Norton (1982), and Hart, Sandin and Kauffman (1989) gave them a distance estimate for ray tracing. The quaternion Mandelbrot set was studied by Gomatam, Doyle, Steves and McFarlane ("Generalization of the Mandelbrot set: quaternionic quadratic maps", *Chaos, Solitons & Fractals*, 1995) and by Bedding and Briggs ("Iteration of quaternion maps", *International Journal of Bifurcation and Chaos*, 1995), who both found it to be a surface of revolution of the Mandelbrot set. Mandelbulber 2's "Quaternion" formula is its $`w = 0`$ slice.

**Parameters.** `w` (0): the slice's fourth ($`k`$) coordinate. At 0 it is the spun Mandelbrot set; away from 0 only the parts of the set farther than $`|w|`$ from its real axis are left, pulled in to the axis, so the antenna and the left-hand bulbs go first and the rest thins to a dish and rings (nothing is left past about 1.1). `iterations` (14, up to 40): escape-time iterations; more give sharper bulbs and finer rings.

---

[All fractals](../FRACTALS.md#algebras) · [Tetrabrot →](tetrabrot.md)
