[All fractals](../FRACTALS.md#more) · [Pine Tree →](pine_tree.md)

# Xenodreambuie (`xenodreambuie`)

*More*

![Xenodreambuie](../images/fractals/xenodreambuie.jpg)

A Mandelbulb built on true spherical coordinates whose polar angle is folded back into one hemisphere before it is multiplied. The result is mirrored top to bottom and covered in stacked, petal-like domes rather than the classic bulb's swirls.

**Formula.** Start from $`z = p`$ and repeat, with $`r = |z|`$ and $`y`$ as the polar axis:

```math
\theta = n\,(\mathrm{atan2}(z_z, z_x) + \beta), \qquad \varphi = n\,\mathrm{fold}(\arccos(z_y / r) + \alpha)
```

```math
z \leftarrow r^n (\sin\varphi\cos\theta,\ \cos\varphi,\ \sin\varphi\sin\theta) + p
```

with $`\mathrm{fold}(\varphi) = \mathrm{sign}(\varphi)\,\pi - \varphi`$ when $`|\varphi| \gt  \pi/2`$. With $`\alpha = 0`$ the fold amounts to replacing $`z_y`$ by $`|z_y|`$. Bailout $`r \ge 4`$. Distance estimate: $`d = \tfrac12 r \ln r / \partial r`$ with $`\partial r \leftarrow n\,r^{n-1}\partial r + 1`$, as for the Mandelbulb. The orbit trap is the closest the orbit comes to the equator, the fold's mirror plane.

**Origin.** Garth Thornton, the author of XenoDream, who posts on Fractal Forums as xenodreambuie. In October 2009, in the "True 3D mandelbrot type fractal" thread that produced the Mandelbulb, he suggested replacing White and Nylander's latitude form ($`z = r\sin\varphi`$) with standard spherical coordinates ($`\varphi = \arccos(z/r)`$, $`z = r\cos\varphi`$), which suit inverse iteration. On October 17, 2009 he posted the forward version with the fold ("ph = acos(zj/magn); if ph>pi/2 then ph = pi-ph"), [msg8281](https://web.archive.org/web/20091124172646/http://www.fractalforums.com:80/3d-fractal-generation/true-3d-mandlebrot-type-fractal/msg8281/), and later called it the "flipped root cosine triplex". Mandelbulber added it in version 0.80 as "xenodreambuie's fractal formula", citing his image post [msg8246](https://web.archive.org/web/20091126041703/http://www.fractalforums.com:80/3d-fractal-generation/true-3d-mandlebrot-type-fractal/msg8246/). Ported from Mandelbulber 2's `xenodreambuie`.

**Parameters.** `power` (9): the exponent $`n`$, from 2 to 16. Mandelbulber's default is 9; higher powers give more and finer domes around the equator and over the poles. `polar_offset` (0): degrees $`\alpha`$ added to the polar angle before the fold, from -180 to 180. Small offsets break the top-bottom mirror and turn the domes into crowns and shells. `azimuth_offset` (0): degrees $`\beta`$ added to the angle around the axis before it is multiplied, from -180 to 180. It turns the lobes about the axis at every step, shifting and shearing the pattern. `iterations` (10, at most 32): more iterations give finer detail.

---

[All fractals](../FRACTALS.md#more) · [Pine Tree →](pine_tree.md)
