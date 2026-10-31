[← Quadratic Mandelbulb](power2_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Kosalos →](kosalos.md)

# Benesi (`benesi`)

*Bulbs*

![Benesi](../images/fractals/benesi.jpg)

Matt Benesi's power-2 bulb, stood on end: a draped, fluted spire with a needle on top and a skirt of ridges below. It is the Mandelbrot set spun about its real axis, but the sign flip on one side and the squared cross-plane pull it into long folds rather than the smooth "whipped cream" of a plain power-2 bulb.

**Formula.** In Mandelbulber's axes (its $`x`$ is the spire's axis, pointing down here), start from $`z = p`$ and repeat:

```math
x \leftarrow \pm\left(x^2 - \rho^2\right) + o_x + c_x, \qquad (y, z) \leftarrow -\frac{2|x|}{\rho}\left(y^2 - z^2,\; 2yz\right) + (o_y, o_z) + (c_y, c_z)
```

with $`\rho = \sqrt{y^2 + z^2}`$, the minus sign taken where $`c_x \ge 0`$ and $`x \ge \rho`$, the offset $`o`$, and $`c`$ the point itself (Mandelbrot type) or the constant `c` (Julia type), blended by `julia`. The Mandelbrot-type set is shifted to sit centered. Distance estimate: $`d = \tfrac12 |z| \ln|z| / \partial z`$ with $`\partial z \leftarrow 2|z|\,\partial z + 1`$; the orbit trap is the nearest the orbit comes to the axis.

**Origin.** Matt Benesi ("M Benesi" on Fractal Forums) devised a long run of 3D $`z^2`$ formulas from 2010 on, starting with [3d Fractal Type that works for z^2](https://www.fractalforums.com/3d-fractal-generation/3d-fractal-type-that-works-for-z2/) (February 2010), and later the magnitude transforms of his "pine tree" ([Fractal Forums, 2015](https://www.fractalforums.com/new-theories-and-research/do-m3d-formula-have-to-be-distance-estimation-formulas/)). This one is Mandelbulber 2's `benesi.cl` ("Benesi - Pwr2"), which credits him; the Julia constant is the one in Mandelbulber's example file `benesi 001.fract`. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `julia` (0): 0 is the Mandelbrot-type set, 1 the Julia set of `c`, and values in between morph one into the other. `c` (0.85, -0.34, 0.41): the Julia constant, used as `julia` rises; small changes reshape it completely. `offset` (0, 0, 0): added after each square; $`x`$ stretches or pinches the folds along the axis and $`y`$, $`z`$ skew them (-0.5 to 0.5). `iterations` (20): more give finer detail cheaply, since there is no trigonometry.

---

[← Quadratic Mandelbulb](power2_bulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Kosalos →](kosalos.md)
