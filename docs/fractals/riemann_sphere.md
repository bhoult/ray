[← Sine/cosine bulb](sincos_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Hobold Riemann Bulb →](riemann_hobold.md)

# Riemann Sphere (`riemann_sphere`)

*Bulbs*

![Riemann Sphere](../images/fractals/riemann_sphere.jpg)

A bell-shaped body covered in a curved grid of studs, with clusters of pyramids around its rim and on its face. Each step folds the point's direction on the Riemann sphere into a grid of cells, and the radius is raised to a power that grows toward the sphere's pole, so the shape is lopsided top to bottom.

**Formula.** Start from $`z = p`$ and repeat: put the direction on a sphere of radius $`\rho`$, $`u = \rho\,z/|z|`$, project it stereographically from the pole $`+y`$,

```math
s = \frac{u_x}{1 - u_y}, \qquad t = \frac{u_z}{1 - u_y}, \qquad k = \min(1 + s^2 + t^2,\ 36),
```

fold the plane with $`s' = |\sin(\pi s + a)|`$, $`t' = |\sin(\pi t + b)|`$, and project back with the new radius:

```math
z \leftarrow \left(|z|^{2k} - \tfrac14\right)\frac{(2s',\ s'^2 + t'^2 - 1,\ 2t')}{1 + s'^2 + t'^2} + c + m\,p.
```

The exponent $`k`$ is the projection's own stretch, 1 at the opposite pole and capped at 36 near this one. Distance estimate: $`d = \tfrac12 |z| \ln|z| / \partial`$ with $`\partial \leftarrow 2k\,(|z|^{2k} + \pi/8)\,\partial/|z| + m`$. The $`\pi/8`$ term covers how the angles still stretch where $`|z|^{2k}`$ is tiny and the $`-\tfrac14`$ holds the radius up. The escape radius is 1000; the last step's radius is worked out in log space so that it can't overflow.

**Origin.** Posted by msltoe on Fractal Forums on June 27, 2011, opening the thread [Riemann fractals](http://www.fractalforums.com/the-3d-mandelbulb/riemann-fractals/msg33500/). His code added the constant $`(0, 0, 1)`$ along the pole, which is this formula's default $`c`$. Ported from Mandelbulber 2's `riemann_sphere_msltoe.cl` (DarkBeam generalized it there as the M3D variant).

**Parameters.** `radius` (1): the sphere the direction is put on before the projection; 1 is the true Riemann sphere, and values down to 0.5 pull the grid of clusters away from the pole and stretch the body into a pointed heart. `offset` ([0, 0]): phases of the two sine folds in radians ($`a`$, $`b`$), which slide the grid of clusters across the surface. `c` ([0, 1, 0]): the constant added at every step; along the pole it gives the clustered bell, at zero a plain ball, and sideways values lean and melt it. `mandel` (0): how much of the point itself is added at every step, from msltoe's Julia-like form (0) to a lopsided Mandelbrot-like one (1). `iterations` (8): more give finer pyramids on the pyramids.

---

[← Sine/cosine bulb](sincos_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Hobold Riemann Bulb →](riemann_hobold.md)
