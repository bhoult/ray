[← Cantor Dust](cantor_dust.md) · [All fractals](../FRACTALS.md#geometric) · [Polyhedral IFS →](polyhedral.md)

# Jerusalem Cube (`jerusalem`)

*Geometric*

![Jerusalem Cube](../images/fractals/jerusalem.jpg)

A cube with a Greek cross drilled through each face. What remains is eight corner cubes and twelve smaller edge cubes, and each of those is drilled the same way. Unlike the 3×3×3 fractals, its two sizes of cube are in an irrational ratio, which gives it a stepped, architectural look.

**Formula.** The corner cubes are $`k = \sqrt2 - 1`$ times the parent's size and the edge cubes $`k^2`$. This fits because $`2k + k^2 = 1`$: two corners and one edge cube exactly span an edge. Two scales can't share one periodic fold, so it is an IFS. Start from $`z = p`$, scale $`s = 1`$, and repeat:

```math
z \leftarrow \mathrm{sort}_\downarrow(\lvert z\rvert),\qquad
\begin{cases} z \leftarrow (z - (1-k)\mathbf{1})/k, & s \leftarrow s\,k \quad\text{(corner nearer)}\\ z \leftarrow (z - (1-k^2, 1-k^2, 0))/k^2, & s \leftarrow s\,k^2 \quad\text{(edge nearer)} \end{cases}
```

After the fold and sort, every other piece is a mirror image of one of these two and no nearer. Each step therefore goes into the nearer box and keeps the other's box distance (times $`s`$) as a bound: $`d = \min(\text{bounds},\ s\,\mathrm{box}(z))`$. The estimate is the least of boxes that hold the fractal, so it never overshoots. Where two pieces touch, the box of the one not followed can show through holes in the other, but only deep inside the tunnels. Dimension $`\approx 2.529`$, the root of $`8k^d + 12k^{2d} = 1`$.

**Origin.** Eric Baird described it in 2011 on his blog Alt.Fractals ("[The Jerusalem Cube](http://alt-fractals.blogspot.com/2011/08/jerusalem-cube.html)"). It was also published in *Tangente* 150 (2013). The name comes from the Jerusalem cross, which each face resembles ([Wikipedia](https://en.wikipedia.org/wiki/Jerusalem_cube)).

**Parameters.** `iterations` (5): levels of drilling as seen from afar (1 to 14). Each level drills crosses into cubes about 2.4 times smaller. Close up, more levels are added on their own.

---

[← Cantor Dust](cantor_dust.md) · [All fractals](../FRACTALS.md#geometric) · [Polyhedral IFS →](polyhedral.md)
