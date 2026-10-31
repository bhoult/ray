[← Riemann Sphere](riemann_sphere.md) · [All fractals](../FRACTALS.md#bulbs) · [Hopfbrot →](hopfbrot.md)

# Hobold Riemann Bulb (`riemann_hobold`)

*Bulbs*

![Hobold Riemann Bulb](../images/fractals/riemann_hobold.jpg)

A round, quilted ball caged in a curved lattice of ribs, with a star of ribbed lobes around one pole. It is a Mandelbulb that turns its angles with no trigonometry at all: points on the Riemann sphere are moved by squaring points on two circles. The Add example is turned to show both the cage and the star.

**Formula.** Start from $`z = p`$ and repeat: put the direction on a sphere of radius $`\rho`$, $`u = \rho\,z/|z|`$, and with $`v = 1 - u_y`$ project it from the pole $`(0, 1, 0)`$ onto the unit circle in the $`xy`$ plane:

```math
K = \frac{(2u_x v,\ u_x^2 - v^2)}{u_x^2 + v^2}.
```

$`K`$ sits at the point's central angle. Double that angle $`n`$ times by complex squaring, $`K \leftarrow (-2K_x K_y,\ K_x^2 - K_y^2)`$, and take the normal $`n_1 = (K_y - 1, -K_x)`$ of the plane through the pole at the new angle. Do the same with $`u_z`$ in place of $`u_x`$ for a second plane, normal $`n_2`$. The two planes meet the unit sphere at the new direction $`u'`$, which has a closed form in $`n_1`$ and $`n_2`$ with no division by zero. Then

```math
z \leftarrow |z|^{q/2}\,u' + p + c.
```

Optional offsets tweak the normals, and an angle turns them into each other. Distance estimate: $`d = \tfrac12|z|\ln|z|/\partial`$ with $`\partial \leftarrow \tfrac q2 |z|^{q/2-1}\partial + 1`$, and an escape radius as large as the power allows.

**Origin.** hobold introduced the "Riemandelettuce", a Mandelbulb in Riemann sphere coordinates, on [Fractal Forums](https://www.fractalforums.com/the-3d-mandelbulb/riemandelettuce) in February 2010. He later reworked it without trigonometry in the fractalforums.org thread "Riemandelettuce without trigonometry". Ported from Mandelbulber 2's `riemann_sphere_hobold_pow8.cl` and `riemann_sphere_hobold_multi.cl`.

**Parameters.** `radius` (0.8): the sphere the direction is put on before the projection; 1 is the exact construction, and Mandelbulber's 0.8 bends it into the ribbed lattice. `offset` ([0, 0]): tweaks to the two planes' normals that skew the lattice. `angle` (0): turns the two normals into each other, in degrees, and twists the lattice. `c` ([0, 0, 0]): a constant added at every step besides the point, which shifts it off its symmetric form. `doublings` (3): how many times the angles are doubled; 3 multiplies them by 8, 2 (by 4) gives a simpler shape with fewer lobes, and 4 (by 16) a finer one. `power` (16): the radius is raised to half this each step, so 16 keeps pace with the eightfold angles and gives the round, quilted ball. Mandelbulber's pow8 uses 8, a spikier ball of hair and long lobes whose distance estimate is rougher (lower the node's `step` if it frays). `iterations` (8): more give finer ribs.

---

[← Riemann Sphere](riemann_sphere.md) · [All fractals](../FRACTALS.md#bulbs) · [Hopfbrot →](hopfbrot.md)
