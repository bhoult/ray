[← Benesi](benesi.md) · [All fractals](../FRACTALS.md#bulbs) · [Lambdabulb →](lambda_bulb.md)

# Kosalos (`kosalos`)

*Bulbs*

![Kosalos](../images/fractals/kosalos.jpg)

A power-9 bulb whose polar angle is squashed the more, the less the orbit moved on its last step. The first step moves least, so it is squashed most, and the bulb stacks into tiers like a pagoda under a star-shaped crown, each tier studded with scaly buds.

**Formula.** With $`z`$ up, start from $`z = z_{\text{prev}} = p`$ and repeat:

```math
t = 1 - \frac{k}{\left|z_{\text{prev}} - z + 0.001\,(1,1,1)\right| + o}, \qquad \theta = \mathrm{atan2}\!\left(t\sqrt{x^2+y^2},\, z\right), \qquad \varphi = \mathrm{atan2}(y, x)
```

```math
z_{\text{prev}} \leftarrow z, \qquad z \leftarrow z + r^n\left(\sin n\theta\cos n\varphi,\; \sin n\theta\sin n\varphi,\; \cos n\theta\right) + (0, 0, \ell)
```

with $`k`$ and $`o`$ the `tweak` and `tweak_offset` in hundredths, $`\ell`$ the `lift`, and (with `add_point`) $`z`$ in the sum replaced by $`p`$. Distance estimate: $`d = \tfrac12 r\ln r/\partial z`$ with $`\partial z \leftarrow (1 + n r^{n-1} s)\,\partial z + 1`$, where $`s = \max\!\left(1,\; |t|\,r^2/(z^2 + t^2(x^2+y^2))\right)`$ is how much the tweak stretches the polar angle. The orbit trap is the nearest the orbit comes to the equator ($`|z|/r`$).

**Origin.** The formula is Kosalos's, from his post in the fractalforums.org thread [Julia sets and altering the iterate afterwards](https://fractalforums.org/fractal-mathematics-and-new-theories/28/julia-sets-and-altering-the-iterate-afterwards/2871/msg16342), on changing each iterate after the power is taken; Graeme McLaren added it to Mandelbulber 2 as `mandelbulb_kosalos.cl` in August 2019. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2), with the derivative made to allow for the tweak's stretch.

**Parameters.** `power` (9): the exponent and the number of ribs per tier (3 to 16). `tweak` (0.2): how hard a short step squashes the polar angle; 0 is a plain bulb, 0.1 to 0.3 stacks it into tiers, and beyond about ±1 it flattens into a ribbed pumpkin. `tweak_offset` (0): added to the step's length before it divides the tweak; 1 to 5 softens the first step's squash, back toward a plain bulb. `lift` (0): added to the vertical component each step, shifting the lobes along the axis (-1 to 1). `add_point` (0): 0 adds the power to the previous orbit point (Mandelbulber's default), 1 to the starting point as the Mandelbulb does; the difference is subtle. `iterations` (10): more give finer detail.

---

[← Benesi](benesi.md) · [All fractals](../FRACTALS.md#bulbs) · [Lambdabulb →](lambda_bulb.md)
