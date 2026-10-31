[← msltoe Sym4](msltoe_sym.md) · [All fractals](../FRACTALS.md#more) · [Mandelnest →](mandelnest.md)

# eiffie's bulb (`eiffie`)

*More*

![eiffie's bulb](../images/fractals/eiffie.jpg)

msltoe's squaring with the point first folded into a one-sixteenth wedge around the vertical axis, so the bulb grows eight-fold: in its Mandelbrot form, a fluted stone vase standing on a long spike. Its Julia sets with the constant on the axis are knobbed spindles, eight-pointed in cross-section.

**Formula.** In a frame whose x axis is the vertical, start from $`z = p`$ and repeat: fold the angle around the axis, $`\psi = \mathrm{atan2}(z_z, z_y)`$, into $`[0, \pi/8]`$,

```math
\psi \leftarrow \left|\left(\psi + \tfrac{9\pi}{8}\right) \bmod \tfrac{\pi}{4} - \tfrac{\pi}{8}\right|, \qquad (z_y, z_z) \leftarrow \rho\,(\cos\psi, \sin\psi),\ \rho = \sqrt{z_y^2 + z_z^2}
```

then apply msltoe's squaring, with $`m = 1 - z_z^2 / |z|^2`$ (the squared cosine of the latitude):

```math
z \leftarrow \left((z_x^2 - z_y^2)\,m,\ 2 z_x z_y\, m\, k_y,\ 2 z_z \sqrt{z_x^2 + z_y^2}\right) + c
```

with $`c = p`$ or the Julia constant and $`k_y`$ the `y_scale`. The fold is a reflection or rotation, so it costs the estimate nothing; the squaring, though, does not square $`|z|`$ exactly (between about 0.73 and 1.03 times $`|z|^2`$ inside the wedge). The log estimate $`d = k\, r \ln r / \partial r`$, $`\partial r \leftarrow 2 r\, \partial r + 1`$, therefore bails out far away ($`r = 1000`$, so it changes smoothly between escape counts), and uses $`k = 0.75`$ instead of the textbook 0.5: measured along rays, the textbook estimate stays below 0.75 of the true distance, and the shading's occlusion reads the estimate, so a too-small one darkens the surface. The march steps 0.9 of it.

**Origin.** msltoe first folded the angle into eight sectors in the thread [Choosing the squaring formula by location](https://www.fractalforums.com/theory/choosing-the-squaring-formula-by-location/) (19 March 2010), with a loop subtracting $`\pi/4`$ until the angle fell below $`\pi/8`$, applied before this squaring. eiffie posted the closed-form fold, the `fmod` line above, in the same thread on 27 January 2015 (reply #69), noting it was meant for Julia sets. Ported from Mandelbulber 2's [Msltoe - Julia Bulb Eiffie](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/eiffie_msltoe.cl). In the Mandelbrot form the added $`c = p`$ is not folded, so the eight-fold symmetry is only approximate; in a Julia set with $`c`$ on the axis it is exact.

**Parameters.** `y_scale` (1): stretches the squaring's sideways term (0.8 to 1.2); 1 is eiffie's, lower swells the vase and frays its rim into fur, higher shrinks it fast (at 1.2 little is left). `julia` (0): 0 is the Mandelbrot form (each point its own constant); 1 draws the Julia set of `c`. `c` ([0, -1, 0]): the Julia constant (-1.5 to 1.5 per axis); on the vertical axis the Julia set keeps the eight-fold symmetry ((0, -1, 0) is a knobbed spindle, eight-pointed in cross-section), and (0, 0, -1) flattens it into a rosette ringed by satellites. `iterations` (10): escape-time iterations (up to 30).

---

[← msltoe Sym4](msltoe_sym.md) · [All fractals](../FRACTALS.md#more) · [Mandelnest →](mandelnest.md)
