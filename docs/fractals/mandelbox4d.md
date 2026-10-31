[← Sierpinski 4D](sierpinski4d.md) · [All fractals](../FRACTALS.md#4d-slices) · [Kleinian 4D →](kleinian4d.md)

# Mandelbox 4D (`mandelbox4d`)

*4D slices*

![Mandelbox 4D](../images/fractals/mandelbox4d.jpg)

A slice through the four-dimensional Mandelbox (pictured at `w` = 1.2). At `w` = 0 it is exactly the ordinary Mandelbox. Other slices keep its cube, about 4 across at negative `fold_scale` and up to 12 at 2, but swap its round pits for square studs and back.

**Formula.** Start from $`z = c = (x, y, z, w)`$, $`\mathrm{d}r = 1`$, and repeat:

```math
z \leftarrow s\,k\,\mathrm{boxFold}(z) + c, \qquad \mathrm{d}r \leftarrow |s|\,k\,\mathrm{d}r + 1,
```

with the box fold $`f(t) = |t + l| - |t - l| - t`$ on each of the four axes (limit $`l`$), and the sphere fold's factor $`k = 1/r_{\min}^2`$ if $`|z|^2 \lt  r_{\min}^2`$, $`1/|z|^2`$ if $`|z|^2 \lt  1`$, else 1, using the 4D radius. Distance estimate: $`d = |z| / |\mathrm{d}r|`$, stopping once $`|z| \gt  10^5`$. Mandelbulber stops at 100, which leaves the estimate jumping between escape bands far from the surface.

**Origin.** Tom Lowe's (Tglad's) Mandelbox ([Fractal Forums, 2010](https://www.fractalforums.com/3d-fractal-generation/amazing-fractal/)), carried to four dimensions as Mandelbulber 2's ABox 4D (`abox4d.cl`) by the Mandelbulber team. Its scale-varying, parabolic and rotation extras are left out.

**Parameters.** `w` (0): the slice's 4th coordinate, useful from −2 to 2 at negative `fold_scale` and −6 to 6 at 2. At 0 it is the ordinary Mandelbox. At the default −2 the faces carry round pits near 0 and turn to square studs by 0.75. At ±1 the slice is a plain smooth cube: the 4th coordinate alone keeps the radius at 1 or more, so the sphere fold never acts. Pits and bubbles return from 1.2, then the slice breaks into specks and empties just past ±2. `fold_scale` (−2): the scale each step. From −1.5 to −2.5 it gives crumpled cubes, from 2 to 3 boxy cities; near ±1 it fattens into a blob (−3 to 3). `min_radius` (0.5): the sphere fold's inner radius. Smaller values give finer, sharper detail; 1 turns the sphere fold off (0.05 to 1). `fold` ([1, 1, 1, 1]): the box fold's limit on x, y, z and w. Lower values fold more tightly; the w limit sets how the 4th axis folds in (0.25 to 2). `iterations` (12, up to 40): more gives finer, sharper detail.

---

[← Sierpinski 4D](sierpinski4d.md) · [All fractals](../FRACTALS.md#4d-slices) · [Kleinian 4D →](kleinian4d.md)
