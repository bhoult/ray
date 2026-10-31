[← Collatz](collatz.md) · [All fractals](../FRACTALS.md#algebras) · [Newton →](newton.md)

# Collatz Mod (`collatz_mod`)

*Algebras*

![Collatz Mod](../images/fractals/collatz_mod.jpg)

The 3D Collatz map with its turn, axis, offset and scale opened up. At its default angle the shell twists into a three-armed pinwheel with fringed blades; angles either side of a half turn twist it opposite ways, and adding the point gives a swirled, Mandelbrot-type shape.

**Formula.** Start from $`z = p`$ (scaled 6.2 times and moved to $`(3.92, 3.92, 3.92)`$) and repeat:

```math
z \leftarrow s\left(b + 4z - (n + 2z) \odot R_\theta(z)\right) + m \odot (p_z, p_y, p_x)
```

```math
R_\theta(z) = z\cos\theta + (n \times z)\sin\theta + n\,(n \cdot z)(1 - \cos\theta)
```

with $`\theta = \pi\cdot`$`angle`, $`n`$ = `axis` (not normalized), $`b`$ = `offset`, $`s`$ = `step_scale` and $`m`$ = `c_mult`. With $`\theta = \pi`$, $`n = b = (1, 1, 1)`$, $`s = \tfrac14`$ and $`m = 0`$ it is the plain Collatz map. Bailout $`|z| \gt  100`$; distance estimate as for Collatz, with $`J \leftarrow f'(z)\,J + \mathrm{diag}(m)\,P`$ ($`P`$ the x-z swap).

**Origin.** Mandelbulber 2's Collatz Mod (2017), ported here. Its default angle differs from Mandelbulber's (0.8 of a half turn rather than a whole one) so that it is not a copy of `collatz`; set `angle` to 1 for Mandelbulber's default.

**Parameters.** `angle` (0.8): the turn in half turns; 1 is the plain map, and either side of it the shell twists into arms one way or the other (0.8 and 1.2 are mirror images; 0.7 to 1.3). `axis` ([1, 1, 1]): the turn's axis, whose length also stretches the map, and the constant in the $`(n + 2z)`$ factor; tilting it skews the arms, and far from (1, 1, 1) the shape can outgrow its bounds. `offset` ([1, 1, 1]): the constant added before scaling; changes the arms' lengths and where the cusps fall. `step_scale` (0.25): the factor on each step; smaller shrinks the set, larger grows it (0.2 to 0.3). `c_mult` ([0, 0, 0]): how much of the point is added each step, taken z, y, x as Mandelbulber adds it; towards 0.5 the shell swirls into a Mandelbrot-type shape, whose distance estimate is rougher, so expect some fuzz. `iterations` (16): more makes the arms and fringes finer.

---

[← Collatz](collatz.md) · [All fractals](../FRACTALS.md#algebras) · [Newton →](newton.md)
