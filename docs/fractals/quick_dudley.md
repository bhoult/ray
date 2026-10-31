[← Mandelcup](mandelcup.md) · [All fractals](../FRACTALS.md#algebras) · [Collatz →](collatz.md)

# Quick Dudley (`quick_dudley`)

*Algebras*

![Quick Dudley](../images/fractals/quick_dudley.jpg)

A lopsided, wind-swept lump with a crest of spikes, frilled edges and a long needle: the Mandelbrot set of squaring in a three-dimensional algebra of its own. It is not a power map in spherical coordinates like the bulbs, so it stretches unevenly and its surface comes out streaked and furry rather than lobed.

**Formula.** Start from $`z = p`$ (moved by $`(-0.67, -0.2, -0.2)`$ to center the set) and repeat:

```math
z \leftarrow \begin{pmatrix} a_x x^2 - b_x yz \\ a_y z^2 + b_y xy \\ a_z y^2 + b_z xz \end{pmatrix} + c
```

with $`a`$ = `square`, $`b`$ = `cross` and $`c = p`$ (or the Julia constant). With $`a = 1`$, $`b = 2`$ this squares $`x + ym + zn`$ in the algebra whose imaginary units obey $`m^2 = n`$, $`n^2 = m`$, $`mn = -1`$. Bailout $`|z| \gt  10`$. The map stretches differently in different directions, so the whole Jacobian is carried along the orbit, $`J \leftarrow Q'(z)\,J + I`$, and the distance estimate is $`d = \tfrac12 r \ln r / \lVert J \rVert_F`$; the estimates of the last two iterates are blended by how far past the bailout the orbit escaped, so that neighboring escape bands meet without a step.

**Origin.** Fractal Forums user quick-dudley proposed the algebra in the thread [Another shot at the holy grail](http://www.fractalforums.com/3d-fractal-generation/another-shot-at-the-holy-grail/) (November 2012). In 2015 Kerry Mitchell (lkmitch) wrote the square out component by component and DarkBeam built it into Mandelbulb 3D; Mandelbulber 2 took it over as Quick Dudley, and its Quick Dudley Mod added the weights used here. Ported from Mandelbulber 2.

**Parameters.** `square` ([1, 1, 1]): weights on the squared terms of the new x, y and z; lowering one flattens and spreads the shape along that axis (0.5 to 1.5). `cross` ([2, 2, 2]): weights on the cross terms; larger pulls out longer spikes and frills, smaller rounds the shape off (1.5 to 2.5). `julia` (0): 1 adds `c` instead of the point, giving one of the map's Julia sets, swirled, flatter and often broken into islands. `c` ([-0.6, 0.3, 0.2]): the Julia constant. `iterations` (9): more makes the edges finer and hairier without changing the overall shape.

---

[← Mandelcup](mandelcup.md) · [All fractals](../FRACTALS.md#algebras) · [Collatz →](collatz.md)
