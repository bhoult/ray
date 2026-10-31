[← Distance IFS](difs.md) · [All fractals](../FRACTALS.md#more) · [eiffie's bulb →](eiffie.md)

# msltoe Sym4 (`msltoe_sym`)

*More*

![msltoe Sym4](../images/fractals/msltoe_sym.jpg)

A bulb whose point is sorted and sign-matched before every squaring, so it grows with the symmetry of a cube. Its Julia sets are ornate, carved knots of bulbs on bulbs; the default is the one at $`c = (-1, 0, 0)`$.

**Formula.** Start from $`z = p`$ and repeat: swap coordinates so that $`|z_x| \ge |z_y| \ge |z_z|`$ (x with z, x with y, then y with z, each when the second is larger in magnitude), give $`z_y`$ and $`z_z`$ the sign of $`z_x`$, then

```math
z \leftarrow \left(z_x^2 - z_y^2 - z_z^2,\ 2 z_x z_y,\ 2 z_x z_z\right) + c
```

with $`c = p`$ (Mandelbrot form) or the Julia constant, and the $`(y, z)`$ part turned by the `twist` about the x axis. This square is the quaternion square restricted to three dimensions, so $`|z|^2`$ is squared exactly, and the sorting fold is a symmetry of the cube, which is what keeps the set from being a plain solid of revolution. Distance estimate: $`d = \tfrac12\, r \ln r / \partial r`$ with $`\partial r \leftarrow 2 r\, \partial r + 1`$, bailing out at $`r = 1000`$.

**Origin.** msltoe started the Fractal Forums thread [Choosing the squaring formula by location](https://www.fractalforums.com/theory/choosing-the-squaring-formula-by-location/) on 29 November 2009, proposing to pick a different squaring for each region of space, the way an IFS applies symmetry operators. Through early 2010 he posted a series of symmetrized squarings in it, with four-, eight- and sixteen-fold variants. The Sym2, Sym3 and Sym4 versions became formulas for Mandelbulb 3D (MsltoeSym2/3/4); youhn posted their code to the thread on 27 January 2015. This is Mandelbulber 2's [Msltoe Sym4 Mod](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/msltoe_sym4_mod.cl) at its defaults, with its rotation reduced to a twist about x.

**Parameters.** `julia` (1): 1 draws the Julia set of `c`; 0 the Mandelbrot form, where each point is its own constant (a lopsided, whipped shape). `c` ([-1, 0, 0]): the Julia constant (-1.5 to 1.5 per axis); on the x axis it keeps the symmetry, and -0.9 to -1.3 gives the richest knots; off the axis the knot warps, and far from the center it falls apart into dust. `twist` (0): degrees the square turns about the x axis at each step; 10 to 30 opens the knot into lace. `iterations` (10): escape-time iterations (up to 30); more adds finer bulbs on the bulbs.

---

[← Distance IFS](difs.md) · [All fractals](../FRACTALS.md#more) · [eiffie's bulb →](eiffie.md)
