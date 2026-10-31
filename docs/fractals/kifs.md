[← Apollonian](apollonian.md) · [All fractals](../FRACTALS.md#the-classics) · [Pseudo-Kleinian →](kleinian.md)

# Kaleidoscopic IFS (`kifs`)

*The classics*

![Kaleidoscopic IFS](../images/fractals/kifs.jpg)

Knighty's kaleidoscopic IFS in its Menger form: space is folded into one mirror wedge of the cube, turned, and scaled away from an offset, again and again. With no turn it is exactly the Menger sponge; the Add example's small turns (12° and 8°) twist it into crusty towers and alien machinery.

**Formula.** Start from $`q = p`$ and repeat `iterations` times:

1. Fold into the cube's fundamental wedge: $`q \leftarrow (|q_x|, |q_y|, |q_z|)`$, then sort the components so that $`q_x \ge q_y \ge q_z`$.
2. Turn: $`q \leftarrow R\,q`$, with $`R = R_z(\gamma)\,R_y(\beta)\,R_x(\alpha)`$ built from `angles` $`= (\alpha, \beta, \gamma)`$.
3. Scale about the offset $`o`$: $`q \leftarrow s\,q - (s - 1)\,o`$, and then, if $`q_z \lt  -\tfrac12 (s - 1)\,o_z`$, $`q_z \leftarrow q_z + (s - 1)\,o_z`$.

The last test keeps a second copy along $`z`$: with $`s = 3`$, $`o = (1, 1, 1)`$ and no turn, the copies centered at $`(\tfrac23, \tfrac23, \tfrac23)`$ and $`(\tfrac23, \tfrac23, 0)`$ become, by symmetry, the Menger sponge's 8 corner and 12 edge cubes. After $`n`$ steps, measure the cube $`[-1, 1]^3`$ and scale back:

```math
d = \mathrm{sdBox}(q, 1)\; s^{-n}
```

Orbit trap: the smallest $`|q| / s^k`$ along the orbit (the nearest it comes to the center in the shape's own units), times 1.5.

**Origin.** Knighty introduced kaleidoscopic IFS on Fractal Forums on 1 May 2010, in the thread [Kaleidoscopic (escape time) IFS](https://www.fractalforums.com/sierpinski-gasket/kaleidoscopic-(escape-time-ifs)/): fold space by the symmetry planes of a polyhedron, rotate before or after the folds, and stretch about a center. ray's kernel is his `Menger3` from that first post, whose $`z`$ test above is written there as $`z \leftarrow s z`$, then $`z \leftarrow z - (s-1)\,o_z`$ if $`z \gt  \tfrac12 (s - 1)\,o_z`$ (the same thing). ray keeps one rotation, in Knighty's second slot (after the folds), and measures a box where Knighty used $`(|q| - 2)\,s^{-n}`$. See also Mikael Hvidtfeldt Christensen's [Folding Space II: Kaleidoscopic Fractals](https://blog.hvidtfeldts.net/index.php/2010/06/folding-space-ii-kaleidoscopic-fractals/) (2010).

**Parameters.** `fold_scale` (3): how much each step scales, 1.2 to 6; 3 is the Menger sponge, and 2 to 4 is typical. `offset` ((1, 1, 1)): the point each step scales away from, which sets where the copies sit. `angles` ((0, 0, 0); the Add example uses (12, 8, 0)): degrees turned about x, then y, then z at every step; 0 is the Menger sponge, and a few degrees twist it into something new. `iterations` (6): steps, 1 to 12, at a linear cost. Close up, ray adds steps as the pixels can resolve them, up to 24.

---

[← Apollonian](apollonian.md) · [All fractals](../FRACTALS.md#the-classics) · [Pseudo-Kleinian →](kleinian.md)
