[← Hobold Riemann Bulb](riemann_hobold.md) · [All fractals](../FRACTALS.md#bulbs)

# Hopfbrot (`hopfbrot`)

*Bulbs*

![Hopfbrot](../images/fractals/hopfbrot.jpg)

The Mandelbulb's power map in four dimensions, written in the Hopf coordinates of the 3-sphere and cut by a 3D slice. The plane facing you holds the Mandelbrot set of the same power, the cubic one at the default power 3. Behind that plane the body swells into a ribbed barrel fringed with bulbs, whorled where the Hopf circles wind.

**Formula.** The point is $`c = (x + \delta,\ y,\ z\cos\tau - \sigma\sin\tau,\ z\sin\tau + \sigma\cos\tau)`$ in four dimensions, where $`\tau`$ is the slice's tilt, $`\sigma`$ its offset, and $`\delta`$ centers the power 2 set. Start from $`w = c`$ and repeat

```math
w \leftarrow |w|^n\,U(n\theta_1,\ n\theta_2,\ n\varphi) + c,
```

with the Hopf parametrization $`U(\theta_1, \theta_2, \varphi) = (\cos\varphi\cos\theta_1,\ \cos\varphi\sin\theta_1,\ \sin\varphi\cos\theta_2,\ \sin\varphi\sin\theta_2)`$. Here $`\theta_1 = \mathrm{atan2}(w_2, w_1)`$, $`\theta_2 = \mathrm{atan2}(w_4, w_3)`$ and $`\varphi = \mathrm{atan2}(|(w_3, w_4)|, |(w_1, w_2)|)`$. The first two coordinates are the complex plane where the map is $`w^n + c`$. Distance estimate: $`d = \tfrac12|w|\ln|w|/\partial`$ with $`\partial \leftarrow n|w|^{n-1}\partial + 1`$. For odd $`n`$ the map stretches no direction more than the radius, so the estimate is sound.

**Origin.** Paul Nylander coined the name, as Oliver Knill records in [Mandelbulb, Mandelbrot, Mandelring and Hopfbrot](https://arxiv.org/abs/2305.17848) (arXiv, 2023). Knill defines the degree-$`d`$ Hopfbrot this way and renders its $`z = 0`$ slice. See also [MathWorld](https://mathworld.wolfram.com/Hopfbrot.html), and [a 2025 post on it](https://uehapetsijque.substack.com/p/the-hopfbrot) that renders slices volumetrically. This implementation, its slicing and its distance estimate are ray's own.

**Parameters.** `power` (3): the exponent. Odd powers keep the map continuous, since the Hopf circle where $`|(w_1, w_2)| = 0`$ maps to a single point, so their surfaces are clean. 2 is Knill's choice, with the Mandelbrot set itself in its middle plane, but there the map tears and leaves seams. Higher powers give more, smaller lobes. `tilt` (90): which 3D slice of the four dimensions, as where the depth axis points in the plane of the last two coordinates. 90 is Knill's $`z = 0`$ cut, 0 the $`w = 0`$ cut, and values between blend them. `slice` (0): how far the slice sits from the center along the hidden fourth axis; away from 0 the body thins, shrinks and breaks apart. `iterations` (10): more give finer detail, at three atan2, three sines, three cosines and a pow per step.

---

[← Hobold Riemann Bulb](riemann_hobold.md) · [All fractals](../FRACTALS.md#bulbs)
