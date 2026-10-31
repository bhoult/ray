[← Kaliset](kaliset.md) · [All fractals](../FRACTALS.md#box-folds)

# Bulb Box (`bulbbox`)

*Box folds*

![Bulb Box](../images/fractals/bulbbox.jpg)

A Mandelbox and a Mandelbulb in one step: the box and sphere folds and a scale, then the power 2 bulb's squaring. The result is a craggy floating block of rippled ledges, caves and bulb-like knots, with a pattern of flowers on its top.

**Formula.** Start from $`z = p`$ and repeat: fold each coordinate at $`F`$, $`x \leftarrow 2F\mathrm{sign}(x) - x`$ where $`|x| \gt  F`$. Then sphere fold with $`k = 1/m^2`$ if $`|z|^2 \lt  m^2`$, $`1/|z|^2`$ if $`|z|^2 \lt  1`$, else 1, and scale: $`w = 2 s k\, z`$. Then square as the power 2 Mandelbulb does, with $`\rho = \sqrt{w_x^2 + w_z^2}`$:

```math
z \leftarrow \left(\frac{2w_y(w_x^2 - w_z^2)}{\rho},\ Z\,(w_y^2 - \rho^2),\ \frac{4w_x w_y w_z}{\rho}\right) + p.
```

Distance estimate: $`d = \tfrac12|z|\ln(4s^2|z|)/\partial`$ with $`\partial \leftarrow 2sk \cdot 2|Q|/|w| \cdot \partial + 1`$, where $`Q`$ is the squared point. The $`4s^2`$ makes the far-field potential square exactly, so the estimate agrees whichever step the orbit escapes at.

**Origin.** After Mandelbulber's Box Fold Bulb Pow 2 (`box_fold_bulb_pow2.cl`, FoldingIntPow2 in Mandelbulber 1), with two changes. The scale between the folds and the squaring is a free parameter, as in its `box_fold_bulb_pow2_v2.cl`. With the original's scale of 1, nearly every orbit escapes and the fractal is dust. And the squaring is the bulb's trigonometric form (polar angle and azimuth doubled) instead of White's triplex (latitude doubled). The triplex sends the whole vertical axis to the origin, which leaves needles along it and a distance estimate too unreliable to march.

**Parameters.** `fold` (1): the box fold's limit $`F`$, which sets the block's size; larger values make a bigger block with more ledges. `min_radius` (0.5): the sphere fold's inner radius $`m`$, which reshapes the fine detail in the hollows more than the overall form. `fold_scale` (0.6): the scale $`s`$ between the folds and the squaring. 0.5 makes a solid craggy block, 0.6 carves caves into it, and from about 0.7 it thins toward dust. `z_factor` (1): stretches the squaring's vertical axis ($`Z`$). Below 1 the walls flatten into ledges, above it they thin into sheets and threads; the original's default is 5. `iterations` (10): more give finer detail.

---

[← Kaliset](kaliset.md) · [All fractals](../FRACTALS.md#box-folds)
