[← Bristorbrot](bristorbrot.md) · [All fractals](../FRACTALS.md#algebras) · [Mandelcup →](mandelcup.md)

# Buffalo (`buffalo`)

*Algebras*

![Buffalo](../images/fractals/buffalo.jpg)

A shaggy 3D relative of the 2D Buffalo and Burning Ship fractals: the power 2 Mandelbulb's squaring with some of its components made positive. The default holds the 2D Buffalo in its xy plane, facing the camera, with a radial whorl where the bulb's axis comes through; from other sides it grows into tufts and fronds.

**Formula.** Start from $`z = c = p`$ (shifted so the shape sits centered) and repeat, with $`\rho^2 = x^2 + y^2`$,

```math
z \leftarrow \Big((x^2 - y^2)\big(1 - \tfrac{z^2}{\rho^2}\big),\; 2xy\big(1 - \tfrac{z^2}{\rho^2}\big),\; \mp 2 z \rho\Big) + c
```

taking the absolute value of the chosen components before the squaring (`pre_abs`) and after it (`abs`). Escape at $`|z| = 100`$.
Distance estimate: $`d = \tfrac12 |z| \ln|z| / \partial`$ with $`\partial \leftarrow 2|z|\,\partial + 1`$, blended across the escape as for `aexion`. The squaring is exact in length ($`|f(z)| = |z|^2`$) but turns a point near its axis by an arbitrary amount, so the estimate jumps a little across the surfaces where orbits pass the axis; the march's step (0.6) allows for it.

**Origin.** youhn posted "Buffalo fractals" in Fractal Forums' Mandelbulber gallery on February 10, 2015 ([thread](http://www.fractalforums.com/gallery-b177/buffalo-fractals/)). He took Mandelbulber's power 2 Mandelbulb iteration (White and Nylander's triplex square) and added absolute values to some of its outputs, hoping for a 3D Burning Ship. It looked like the Buffalo fractal of Kalles Fraktaler instead. Ported from Mandelbulber 2's Buffalo.

**Parameters.** `abs` ([1, 1, 0]): which components of the squared point are made positive (1) or left as they are (0), x, y and z, with z toward the default camera. [1, 1, 0] puts the 2D Buffalo in the xy plane; [0, 0, 0] is the plain power 2 Mandelbulb; each other choice is another member of the family. `pre_abs` ([0, 0, 0]): the same before the squaring, for more members of the family. `flip_z` (0): flips the sign of the squared point's z (no change when z is made positive anyway). `iterations` (12): more iterations give finer fronds.

---

[← Bristorbrot](bristorbrot.md) · [All fractals](../FRACTALS.md#algebras) · [Mandelcup →](mandelcup.md)
