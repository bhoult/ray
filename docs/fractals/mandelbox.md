[← Mandelbulb](mandelbulb.md) · [All fractals](../FRACTALS.md#the-classics) · [Quaternion Julia →](julia.md)

# Mandelbox (`mandelbox`)

*The classics*

![Mandelbox](../images/fractals/mandelbox.jpg)

Box folds and sphere folds repeated with a scale. At the Add example's fold scale of -1.5 it is a cube whose faces are carved with symmetric relief and whose inside is a maze; positive scales from 2 to 3 give boxy cities.

**Formula.** Start from $`z = p`$ and $`dr = 1`$, and repeat `iterations` times (there is no bailout):

```math
z \leftarrow 2\,\mathrm{clamp}(z, -1, 1) - z \quad \text{(box fold, on each component)}
```

```math
k = \begin{cases} 1/m^2 & |z|^2 \lt  m^2 \\ 1/|z|^2 & m^2 \le |z|^2 \lt  1 \\ 1 & \text{otherwise} \end{cases}, \qquad z \leftarrow k\,z \quad \text{(sphere fold)}
```

```math
z \leftarrow s\,z + p, \qquad dr \leftarrow |s|\,k\,dr + 1
```

with $`s`$ the `fold_scale`, $`m`$ the `min_radius`, and the sphere fold's fixed radius 1. The box fold reflects any component beyond $`\pm 1`$ back inside; the sphere fold inverts points between radius $`m`$ and 1 and scales those inside $`m`$ by $`1/m^2`$. Distance estimate: $`d = |z| / |dr|`$. Orbit trap: the smallest $`|z|`$ just after the box folds. The shape is large (about 8 units across for negative scales, 12 at scale 2), so the Add example sets `scale` to 0.25.

**Origin.** Tom Lowe (Tglad) posted it on Fractal Forums on 1 February 2010 in the thread [Amazing fractal](https://www.fractalforums.com/3d-fractal-generation/amazing-fractal/), as a member of the family of fractals discussed in the earlier thread on continuous conformal Mandelbrots; his first renders used scales 3 and 2, a fixed radius of 1 and a minimum radius of 0.5. See Lowe's [What is a Mandelbox?](https://sites.google.com/site/mandelbox/what-is-a-mandelbox), [Mandelbox](https://en.wikipedia.org/wiki/Mandelbox), and Mikael Hvidtfeldt Christensen's [Distance Estimated 3D Fractals (VI): The Mandelbox](https://blog.hvidtfeldts.net/index.php/2011/11/distance-estimated-3d-fractals-vi-the-mandelbox/) for the distance estimate.

**Parameters.** `fold_scale` (2; the Add example uses -1.5): the scale after each fold, which sets the overall shape. Negative values (about -3 to -1.2) give crumpled cubes, positive ones (2 to 3) boxy cities; avoid -1 to 1. `iterations` (12): fold steps, 1 to 40; more give finer detail at a linear cost. Iterations are not added close up. `min_radius` (0.5): the inner radius of the sphere fold, 0.01 to 1; smaller makes sharper, deeper bulbs.

---

[← Mandelbulb](mandelbulb.md) · [All fractals](../FRACTALS.md#the-classics) · [Quaternion Julia →](julia.md)
