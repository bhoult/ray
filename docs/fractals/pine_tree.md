[← Xenodreambuie](xenodreambuie.md) · [All fractals](../FRACTALS.md#more) · [Makin →](makin.md)

# Pine Tree (`pine_tree`)

*More*

![Pine Tree](../images/fractals/pine_tree.jpg)

A quadratic Mandelbrot set spun about its needle: the angle around the needle doubles at every step, and the two cross axes swap. Stood on its needle, it becomes a tiered, drooping conifer topped by a spire of tiny minibrots.

**Formula.** In Mandelbulber's frame, where $`x`$ runs along the needle (here it points up), with $`\rho = \sqrt{y^2 + z^2}`$, start from $`z = p`$ and repeat:

```math
x \leftarrow x^2 - \rho^2 + m_x p_x, \qquad y \leftarrow \tfrac{2x}{\rho}\,2yz + m_z p_z, \qquad z \leftarrow \tfrac{2x}{\rho}\,(y^2 - z^2) + m_y p_y
```

then turn $`(y, z)`$ by the twist angle. In the meridian plane $`(x, \rho)`$ this is complex squaring; around the axis it doubles the angle and then swaps $`y`$ and $`z`$, with the point's $`y`$ and $`z`$ swapped to match. Bailout $`|z| \ge 8`$. The map stretches up to twice as much around the needle as along it, so a scalar derivative is far off. The kernel carries the orbit's Jacobian $`J \leftarrow Df\,J + C`$ and estimates $`d = \tfrac12 r \ln r / g`$ from the escape potential's gradient $`g = |J^{\mathsf T} z| / r`$. It blends that estimate with the previous step's according to how far the escape overshot the bailout (in $`\ln\ln r`$), so the estimate does not jump where the escape count changes. Where an orbit grazes the needle the gradient swings sharply, so $`g`$ is kept to at least a tenth of $`\lVert J \rVert_F`$, and the march step is 0.25. The orbit trap is the closest the orbit comes to the needle.

**Origin.** The map is Paul Nylander's (bugman) [3D Christmas Tree Mandelbrot Set](http://bugman123.com/Hypercomplex/index.html#ChristmasTree) from December 2009: $`r^n(\cos\theta, \sin\theta\cos\varphi, \sin\theta\sin\varphi)`$ with $`\theta = n\arccos(x/r)`$ and $`\varphi = n\,\mathrm{atan2}(z, y)`$. M. Benesi (M Benesi on Fractal Forums) was using it by June 2010, calling it the "Christmas tree fractal (Name from Bugman's website)" in the thread [Question about Formula](https://web.archive.org/web/20120229053814/http://www.fractalforums.com:80/mandelbulb-3d/question-about-formula/). On October 9, 2015 he posted the trig-free power-2 form, with y and z switched "to add symmetry", in the thread [Do M3D formula have to be Distance Estimation formulas?](https://web.archive.org/web/20250728083538/http://www.fractalforums.com/new-theories-and-research/do-m3d-formula-have-to-be-distance-estimation-formulas/), later retitled Benesi "pinetree" (fold+bulb); by then he called it the pine tree. Mandelbulber's `benesi_pine_tree` ("benesiFastPwr2PineTree") ports that form, and Mandelbulb 3D has it as BPine_only. Ported from Mandelbulber 2. Mandelbulber adds only the point's x by default (1, 0, 0), which gives a smooth, banded pagoda; the default here adds a quarter of the cross coordinates as well.

**Parameters.** `c_scale` (1, 0.25, 0.25): how much of the point's height and of its two cross coordinates is added at each step, each from -2 to 2. Mandelbulber's 1, 0, 0 is a smooth, banded pagoda. Raising the last two drapes and tears the tiers, and 1, 1, 1 (the full Mandelbrot type) is a ragged storm. `twist` (0): degrees the cross section turns about the needle at every step, from -180 to 180; it spirals the tiers. `iterations` (14, at most 30): more iterations give finer tiers and a longer spire.

---

[← Xenodreambuie](xenodreambuie.md) · [All fractals](../FRACTALS.md#more) · [Makin →](makin.md)
