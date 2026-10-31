[← Menger 4D](menger4d.md) · [All fractals](../FRACTALS.md#4d-slices) · [Mandelbox 4D →](mandelbox4d.md)

# Sierpinski 4D (`sierpinski4d`)

*4D slices*

![Sierpinski 4D](../images/fractals/sierpinski4d.jpg)

A slice through a four-dimensional Sierpinski solid: eight half-size copies on the corners of a 16-cell, each made the same way. At `w` = 0 (pictured) the slice is a cuboctahedron walled with Sierpinski triangles; at ±1 it is the Sierpinski tetrahedron.

**Formula.** Start from $`z = (x, y, z, w)`$ and repeat $`n`$ times: for each pair of axes in turn, $`(x,y), (x,z), (y,z), (x,w), (y,w), (z,w)`$, reflect if their sum is negative,

```math
z_i + z_j \lt  0:\quad (z_i, z_j) \leftarrow (-z_j, -z_i),
```

then scale about the corner $`(1, 1, 1, 1)`$: $`z \leftarrow s\,z - (s-1)`$. The folds bring the point near the one corner of the 16-cell (the corners of the hypercube with an even number of −1s) it is closest to. Distance estimate: the distance to that 16-cell, scaled back,

```math
d = \max\Bigl(\max_i |z_i| - 1,\ \tfrac12 \max_i \bigl|\textstyle\sum_j z_j - 2 z_i\bigr| - 1\Bigr) / s^n.
```

Its eight cells $`|z_i| = 1`$ are tetrahedra, which is why the slices at ±1 are the 3D Sierpinski tetrahedron and its mirror image. Iterations add finer copies close up on their own.

**Origin.** Syntopia's and DarkBeam's Sierpinski 4D for Mandelbulb 3D (DarkBeam's [custom formulas thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/) on Fractal Forums), the 4D form of the folding Sierpinski tetrahedron in Syntopia's [Distance Estimated 3D Fractals III: Folding Space](http://blog.hvidtfeldts.net/index.php/2011/08/distance-estimated-3d-fractals-iii-folding-space/) (2011). Ported from Mandelbulber 2's Sierpinski 4D (`sierpinski4d.cl`). Mandelbulber's per-axis offset and six-plane rotation are left out, so the base shape is always the 16-cell.

**Parameters.** `w` (0): the slice's 4th coordinate, from −1 to 1. At 0 the slice is a cuboctahedron walled with Sierpinski triangles. At 0.5 it is four half-size cuboctahedra on the corners of a tetrahedron, at 0.75 sixteen quarter-size ones; in general $`w = 1 - 2^{-k}`$ gives $`4^k`$ of them. Between those values the slice is spiky lace. At 1 it is exactly the Sierpinski tetrahedron and at −1 its mirror image; past ±1 the slice is empty. `fold_scale` (2): how much smaller each level's copies are. At 2 they touch; from 1.5 to 2 they overlap into a solid, from 2 to 3 they part into dust. `iterations` (10, up to 20): levels of copies, each half the size.

---

[← Menger 4D](menger4d.md) · [All fractals](../FRACTALS.md#4d-slices) · [Mandelbox 4D →](mandelbox4d.md)
