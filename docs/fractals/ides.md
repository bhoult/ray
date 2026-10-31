[← Makin](makin.md) · [All fractals](../FRACTALS.md#more) · [Distance IFS →](difs.md)

# Ides (`ides`)

*More*

![Ides](../images/fractals/ides.jpg)

A mixed quadratic and cubic map that grows a mirror-symmetric, moth-like body of hollow wings, ribs and hair-fine spikes. Seen edge-on it shows a long stem; from behind, a smooth hollow.

**Formula.** Start from $`z = p`$ and repeat: shrink $`x`$ and $`z`$ by a tenth, $`x' = s(x)`$ and $`z' = s(z)`$ with $`s(u) = 0.9u`$, then

```math
z \leftarrow \big(m_1 x'^2 - s_1(y^2 + z'^2),\ m_2\,x' y z',\ m_3 z'^2 - s_2(x'^2 + y^2)\big) + p
```

with $`(m_1, m_2, m_3) = (1, 2, 1)`$ and $`s_1 = s_2 = \tfrac12`$ for Ide's formula. Bailout $`|z| \ge 10`$, as in Mandelbulber. Mandelbulber shrinks only inside $`|u| \lt  2.5`$, a jump that no distance estimate can follow. Here the shrink eases into slope 1 between 2.5 and 3; the set itself lies within 2.5. The map mixes degrees and is far from conformal, and its potential's gradient varies too sharply to march by. The estimate therefore uses the Jacobian's Frobenius norm, which bounds its largest stretch: $`d = \tfrac12 r \ln r / \lVert J \rVert_F`$, blended across the bailout as for the pine tree, with a march step of 0.3. The map is symmetric under swapping $`x`$ and $`z`$. It is turned 135 degrees about $`y`$ so that this mirror plane faces the default view, and centered. The orbit trap is the closest the orbit comes to the planes $`x = 0`$ and $`z = 0`$.

**Origin.** Trafassel, the author of the Gestaltlupe renderer, started the Fractal Forums thread [Formula 21 (Julia Set Interpretation)](https://web.archive.org/web/20110316072858/http://www.fractalforums.com:80/videos/formula-21-(julia-set-interpretation)/) on February 16, 2010. In June 2010 he called it "Formula 21 (Ide's Fractal)" and noted that it has some nice properties but is no candidate for a true 3D Mandelbrot set. Who Ide is, the thread does not say. Mandelbulb 3D's IdesFormula ("a formula made by Trafassel") uses the same multipliers (1, 2, 1) and (0.5, 0.5), as does Mandelbulber's `ides`. The 0.9 shrink is not in Trafassel's 2010 loop; it appears in Mandelbulber's version, whose `ides2` leaves it out. Ported from Mandelbulber 2's `ides`.

**Parameters.** `mult` (1, 2, 1): the multipliers of $`x^2`$, $`xyz`$ and $`z^2`$, each from -3 to 3. 1, 2, 1 is Ide's; the middle one sets how spiky the ribs are. `sub` (0.5, 0.5): how much of the other two squares is subtracted from $`x`$ and from $`z`$, each from -2 to 2. Lower values grow the body and its spikes; higher values shrink it to a compact, blade-edged core. `iterations` (12, at most 30): more iterations give finer spikes.

---

[← Makin](makin.md) · [All fractals](../FRACTALS.md#more) · [Distance IFS →](difs.md)
