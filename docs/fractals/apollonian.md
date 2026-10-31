[← Sierpinski Tetrahedron](sierpinski.md) · [All fractals](../FRACTALS.md#the-classics) · [Kaleidoscopic IFS →](kifs.md)

# Apollonian (`apollonian`)

*The classics*

![Apollonian](../images/fractals/apollonian.jpg)

Inigo Quilez's Apollonian fractal: a lattice of spheres in which every gap holds smaller spheres, and every gap between those smaller ones still, like foam or coral. It fills all of space, so the Add example cuts a 2 m cube out of it; try the orbit texture on it.

**Formula.** Start from $`q = p`$ and $`S = 1`$, and repeat `iterations` times:

```math
q \leftarrow 2\,\mathrm{fract}\!\left(\tfrac12 q + \tfrac12\right) - 1 \quad \text{(wrap into the cell } [-1, 1)^3\text{)}
```

```math
k = f / |q|^2, \qquad q \leftarrow k\,q, \qquad S \leftarrow k\,S
```

with $`f`$ the `fold`: the second line inverts $`q`$ in the sphere of radius $`\sqrt f`$ about the cell's center. Distance estimate: $`d = \tfrac14\,|q_y| / S`$, the distance to the plane $`q_y = 0`$ through the result, scaled back; the inversions bend that plane into the spheres. Orbit trap: the smallest $`|q|^2`$ after the wraps, times 6.

**Origin.** The name comes from Apollonius of Perga's problem of circles tangent to three others, whose repeated solution packs circles inside circles ([Apollonian gasket](https://en.wikipedia.org/wiki/Apollonian_gasket)); spheres pack the same way in 3D ([Apollonian sphere packing](https://en.wikipedia.org/wiki/Apollonian_sphere_packing)). This particular formula, repeated cell wrapping and sphere inversion measured against a plane, is from Inigo Quilez's Shadertoy [Apollonian](https://www.shadertoy.com/view/4ds3zn) (2013), which he calls the "classic standard apollonian/kleinian fractal" and says he can't recall where he learned it. ray drops iq's lower limit on the inversion factor ($`\max(f/|q|^2, 0.1)`$) and exposes iq's $`s`$ as `fold`.

**Parameters.** `fold` (1.1): the inversion's strength, 0.5 to 2. Above 1 the spheres swell and merge into a sponge; below 1 they thin into a lace. `iterations` (8): levels of packing, 1 to 20; more add smaller spheres between the big ones. Iterations are not added close up.

---

[← Sierpinski Tetrahedron](sierpinski.md) · [All fractals](../FRACTALS.md#the-classics) · [Kaleidoscopic IFS →](kifs.md)
