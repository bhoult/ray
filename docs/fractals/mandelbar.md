[← Juliabulb](juliabulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Burning Ship →](burning_ship.md)

# Mandelbar (`mandelbar`)

*Bulbs*

![Mandelbar](../images/fractals/mandelbar.jpg)

The Tricorn grown into 3D: the slice facing you is the three-cornered Mandelbar set, the level slice through it is the Mandelbrot set, and between them the surface sweeps round in smooth, swirled folds, with a beaded needle running out to the left.

**Formula.** Take $`x`$ as the real part and $`(y, z)`$ as one imaginary direction, conjugate $`y`$, and raise to the $`n`$th power as a complex number: with $`\rho = \sqrt{y^2+z^2}`$ and $`w = (x + i\rho)^n`$,

```math
\mathbf z \leftarrow \Bigl(\mathrm{Re} w,\ -\mathrm{Im} w\,\frac{y}{\rho},\ \mathrm{Im} w\,\frac{z}{\rho}\Bigr) + c,
```

starting from $`\mathbf z = c = p`$. For $`n = 2`$ this is $`(x^2-y^2-z^2,\ -2xy,\ 2xz) + c`$: the $`xy`$ plane iterates $`\bar z^2 + c`$ (the Tricorn) and the $`xz`$ plane $`z^2 + c`$ (the Mandelbrot set). Since $`|w| = r^n`$, the distance estimate is $`d = \tfrac12\,r\ln r/r'`$ with $`r' \leftarrow n\,r^{n-1}r' + 1`$, bailing out at $`r = 256`$. For $`n = 2`$ the set is shifted 0.5 along $`x`$ to center it; the higher powers are centered already.

**Origin.** The Mandelbar set $`\bar z^2 + c`$ was first studied by W. D. Crowe, R. Hasson, P. J. Rippon and P. E. D. Strain-Clark ("On the structure of the Mandelbar set", *Nonlinearity* 2, 1989) and is better known as the Tricorn; its higher powers are the multicorns. This 3D blend of the Mandelbrot and Mandelbar sets is credited by Paul Nylander to Eric Baird; Nylander rendered it as the "3D Mandelbrot / Mandelbar Set" in July 2009 with $`(x^2-y^2-z^2,\ 2xy,\ -2xz)`$, the same formula with $`y`$ and $`z`$ exchanged ([Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/)). Ported from Mandelbulber 2's Mandelbar (`mandelbar.cl`), whose axis arrangement it keeps; the higher powers are this version's extension.

**Parameters.** `power` (2): whole, 2 to 8. 2 is the Tricorn; higher powers give the multicorns, whose upright slice has power + 1 corners and whose level slice is the Multibrot set of that power. `iterations` (12, up to 32): finer bulbs along the needle and the arms, at a few multiplications per power per point each.

---

[← Juliabulb](juliabulb.md) · [All fractals](../FRACTALS.md#bulbs) · [Burning Ship →](burning_ship.md)
