[← Burning Ship](burning_ship.md) · [All fractals](../FRACTALS.md#bulbs) · [Benesi →](benesi.md)

# Quadratic Mandelbulb (`power2_bulb`)

*Bulbs*

![Quadratic Mandelbulb](../images/fractals/power2_bulb.jpg)

The first Mandelbulb, before the power went up to 8: the slice facing you is exactly the Mandelbrot set, and in depth it swells into a smeared, "whipped cream" body with a whorl at its heart. Shown here from behind, so the needle points right.

**Formula.** Daniel White's triplex square, in its non-trigonometric form: with $`\rho^2 = x^2 + y^2`$,

```math
\mathbf z \leftarrow \Bigl((x^2-y^2)\bigl(1 - \tfrac{z^2}{\rho^2}\bigr),\ 2xy\bigl(1 - \tfrac{z^2}{\rho^2}\bigr),\ -2z\rho\Bigr) + c,
```

which is $`r^2(\cos 2\phi\cos 2\theta,\ \sin 2\phi\cos 2\theta,\ -\sin 2\theta)`$ for longitude $`\phi`$ and latitude $`\theta`$, from $`\mathbf z = c = p`$ (shifted 0.62 along $`x`$ to center it). The $`z = 0`$ plane is the Mandelbrot set. Because $`|\mathbf z^2| = |\mathbf z|^2`$, the distance estimate is the analytic $`d = \tfrac12\,r\ln r/r'`$ with $`r' \leftarrow 2r\,r' + 1`$. On the pole axis ($`\rho = 0`$), where the map has no limit, the kernel takes the one along $`y = 0`$. With `julia` set, $`\mathbf z`$ starts at $`p`$, $`c`$ is the fixed constant, and $`r' \leftarrow 2r\,r'`$.

**Origin.** Daniel White posted his "triplex" squaring formula, two rotations in spherical coordinates, to Fractal Forums in November 2007 (["True 3D mandelbrot type fractal"](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/)), and Thomas Ludwig was the first to render the power-2 object in 3D that same month. Paul Nylander rendered it as the "Quadratic 3D Mandelbulb Set" in July 2009, judging it the best-looking quadratic 3D Mandelbrot set so far, and a month later raised the power to 8, which gave the Mandelbulb its finer detail. The minus sign on the $`z`$ term is White's original; Nylander's later positive-$`z`$ variant (October 2009) is more consistent algebraically but gives a flat-looking quadratic set. Nylander also rendered this formula's Julia sets in July 2009, the first Juliabulbs. Sources: [White's Mandelbulb history](https://www.skytopia.com/project/fractal/mandelbulb.html), [Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/), and Mandelbulber 2's `mandelbulb_power2.cl`.

**Parameters.** `julia` (0): 0 renders the bulb; 1 its Julia set for the constant `c`. `c` ([-0.12, 0.74, 0]): the Julia constant, used when `julia` is 1. With a $`z`$ of 0 the slice facing you is that 2D Julia set (the default is the Douady rabbit's), puffed into 3D around it. `iterations` (12, up to 32): finer bulbs and smears, at a few multiplications and a square root per point each.

---

[← Burning Ship](burning_ship.md) · [All fractals](../FRACTALS.md#bulbs) · [Benesi →](benesi.md)
