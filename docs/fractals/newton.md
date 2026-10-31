[← Collatz Mod](collatz_mod.md) · [All fractals](../FRACTALS.md#algebras)

# Newton (`newton`)

*Algebras*

![Newton](../images/fractals/newton.jpg)

A Newton fractal in 3D: a ribbed barrel whose ends are sets of concentric rings. Newton's method for $`w^3 = 1`$ in the triplex (Mandelbulb) algebra sends most points to the root 1; the solid is everything that goes to the other two roots instead.

**Formula.** Start from $`z = p`$ (moved by $`-0.655`$ along x) and repeat:

```math
w = \frac{\bar z}{|z|^2} + 1, \qquad w \leftarrow \tfrac23 w + \frac{1}{3w^2}, \qquad z \leftarrow \frac{\overline{w - 1}}{|w - 1|^2} + o
```

where $`\bar{\ }`$ negates y and z, $`w^2`$ is the triplex square $`\left((x^2 - y^2)(1 - \tfrac{z^2}{x^2 + y^2}),\ 2xy(1 - \tfrac{z^2}{x^2 + y^2}),\ 2z\sqrt{x^2 + y^2}\right)`$, $`1/w^2 = \overline{w^2}/|w|^4`$, and $`o`$ = `offset`. The inversions turn Newton's convergence to the root 1 into an escape of $`z`$ to infinity, so the basin of 1 is the outside. Near that root $`z`$ is squared like a quaternion, $`|z| \to |z|^2`$, so the usual logarithmic estimate applies: bailout $`|z| \gt  30`$, $`d = \tfrac12 r \ln r / \lVert J \rVert_F`$, with $`J`$ carried by forward-mode dual numbers and blended across escape bands as above.

**Origin.** The formula and its inversion trick are by gannjondal, from the Fractal Forums thread [Revisiting the 3D Newton](https://fractalforums.org/fractal-mathematics-and-new-theories/28/revisiting-the-3d-newton/1026); mclarekin added it to Mandelbulber 2 as Newton Pow3 (2022). Ported from Mandelbulber 2.

**Parameters.** `offset` ([0, 0, 0]): added after every step, like a Julia constant; along x it splits the barrel into two ribbed halves joined by filaments, sideways it twists and roughens it (-0.6 to 0.6). `iterations` (16): Newton steps; the rings are sharp by about 12, and more adds finer filaments where the basins meet.

---

[← Collatz Mod](collatz_mod.md) · [All fractals](../FRACTALS.md#algebras)
