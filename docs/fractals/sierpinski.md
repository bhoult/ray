[← Quaternion Julia](julia.md) · [All fractals](../FRACTALS.md#the-classics) · [Apollonian →](apollonian.md)

# Sierpinski Tetrahedron (`sierpinski`)

*The classics*

![Sierpinski Tetrahedron](../images/fractals/sierpinski.jpg)

The 3D analog of Sierpiński's triangle: a tetrahedron made of four half-size copies of itself, each made of four more, and so on, so that it is mostly air. ray builds it by folding space and adds levels by itself close up.

**Formula.** Start from $`q = p`$ and repeat `iterations` times: reflect in three of the tetrahedron's mirror planes,

```math
\text{if } q_x + q_y \lt  0:\ (q_x, q_y) \leftarrow (-q_y, -q_x); \quad \text{if } q_x + q_z \lt  0:\ (q_x, q_z) \leftarrow (-q_z, -q_x); \quad \text{if } q_y + q_z \lt  0:\ (q_y, q_z) \leftarrow (-q_z, -q_y)
```

which brings points toward the copy at the vertex $`(1, 1, 1)`$, then scale by 2 about that vertex: $`q \leftarrow 2q - (1, 1, 1)`$. After $`n`$ steps, measure the base tetrahedron with vertices $`(1,1,1)`$, $`(-1,-1,1)`$, $`(-1,1,-1)`$, $`(1,-1,-1)`$ by its four face planes, and scale back:

```math
d = \frac{\max\bigl(|q_x + q_y| - q_z,\ |q_x - q_y| + q_z\bigr) - 1}{\sqrt3}\; 2^{-n}
```

Orbit trap: the share of the reflections that were taken, relative to `iterations`.

**Origin.** Wacław Sierpiński described the triangle in 1915 ([Sierpiński triangle](https://en.wikipedia.org/wiki/Sierpi%C5%84ski_triangle)); the tetrahedron, sometimes called the tetrix, is its standard 3D analog. The folding construction, with exactly these three folds and a scale of 2 about $`(1, 1, 1)`$, is Knighty's: it is the first algorithm in his post [Kaleidoscopic (escape time) IFS](https://www.fractalforums.com/sierpinski-gasket/kaleidoscopic-(escape-time-ifs)/) on Fractal Forums (1 May 2010), and Mikael Hvidtfeldt Christensen explains it in [Distance Estimated 3D Fractals (III): Folding Space](https://blog.hvidtfeldts.net/index.php/2011/08/distance-estimated-3d-fractals-iii-folding-space/) (2011). Knighty measured the result as $`(|q| - 2)\,2^{-n}`$, a ball; ray measures the base tetrahedron instead, so the finest copies are sharp tetrahedra.

**Parameters.** `iterations` (8): levels of subdivision, 1 to 20; each halves the size of the smallest tetrahedra. Close up, ray adds levels as the pixels can resolve them, up to 24.

---

[← Quaternion Julia](julia.md) · [All fractals](../FRACTALS.md#the-classics) · [Apollonian →](apollonian.md)
