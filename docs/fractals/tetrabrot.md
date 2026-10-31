[← Quaternion Mandelbrot](quaternion.md) · [All fractals](../FRACTALS.md#algebras) · [Hypercomplex Mandelbrot →](hypercomplex.md)

# Tetrabrot (`tetrabrot`)

*Algebras*

![Tetrabrot](../images/fractals/tetrabrot.jpg)

Dominic Rochon's bicomplex Mandelbrot set, sliced through $`1`$, $`i_1`$ and $`i_2`$. A bicomplex number is two complex numbers squared independently, so the shape is two Mandelbrot sets drawn out along the diagonals and intersected: a faceted, cubist Mandelbrot set with diamond cross-sections and box-like bulbs.

**Formula.** The point $`c = x + y\,i_1 + z\,i_2 + w\,j`$ (with $`x = p_x - 0.75`$ for power 2, $`p_x`$ otherwise, $`y = p_y`$, $`z = p_z`$ and $`j = i_1 i_2`$) has the idempotent components

```math
c_1 = (x + w) + (y - z)\,i, \qquad c_2 = (x - w) + (y + z)\,i,
```

and $`c^n`$ is $`c_1^n`$ and $`c_2^n`$, so each is iterated as a complex number, $`u_k \leftarrow u_k^n + c_k`$ from $`u_k = c_k`$; the point is in the set when both orbits stay bounded. Distance estimate: $`d = \sqrt{(d_1^2 + d_2^2)/2}`$, the distance to the pair of sets in the bicomplex norm, from the complex estimates $`d_k = |u_k| \ln|u_k| / (2|u_k'|)`$ with $`|u_k'| \leftarrow n|u_k|^{n-1}|u_k'| + 1`$ ($`d_k = 0`$ while an orbit is still bounded).

**Origin.** Dominic Rochon, "A generalized Mandelbrot set for bicomplex numbers" (*Fractals*, 2000). The distance estimate is Martineau and Rochon's ("On a bicomplex distance estimation for the Tetrabrot", *International Journal of Bifurcation and Chaos*, 2005), in the general form of Brouillette, Parisé and Rochon, ["Tricomplex distance estimation for filled-in Julia sets and Multibrot sets"](https://arxiv.org/abs/1811.09697) (*International Journal of Bifurcation and Chaos*, 2019), which also draws the Tetrabrots of higher powers.

**Parameters.** `power` (2, from 2 to 8): the exponent $`n`$. 2 is Rochon's Tetrabrot; higher powers give the Tetrabrots of the Multibrot sets, blocky bodies with $`n - 1`$ lobes about the center. `w` (0): the slice's fourth ($`j`$) coordinate. It slides the two Mandelbrot sets the shape is cut from apart along the real axis, so the shape shrinks and its facets break up into crossed bulbs (try 0.2 to 0.4). `iterations` (16, up to 40): escape-time iterations; more give sharper edges and finer bulbs.

---

[← Quaternion Mandelbrot](quaternion.md) · [All fractals](../FRACTALS.md#algebras) · [Hypercomplex Mandelbrot →](hypercomplex.md)
