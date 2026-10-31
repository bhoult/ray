[All fractals](../FRACTALS.md#the-classics) · [Mandelbulb →](mandelbulb.md)

# Menger Sponge (`menger`)

*The classics*

![Menger Sponge](../images/fractals/menger.jpg)

A cube with a square tunnel bored through the middle of each face, then the same done to each of the 20 smaller cubes that remain, level after level. ray carves the holes directly rather than by escape time, so its distance estimate is tight, and it adds levels by itself as the camera closes in.

**Formula.** Start from the exact distance to the cube $`[-1, 1]^3`$, $`d = \mathrm{sdBox}(p, 1)`$. For each level $`i = 0, 1, \dots, n-1`$, fold each coordinate into that level's cell:

```math
a_c = \Bigl|\,1 - 3\,\bigl|\mathrm{mod}(3^i c,\ 2) - 1\bigr|\,\Bigr|, \qquad c \in \{p_x, p_y, p_z\}
```

then measure the level's cross of three square tunnels and carve it out:

```math
h_i = \frac{\mathrm{mid}(a_x, a_y, a_z) - 1}{3^{\,i+1}}, \qquad d \leftarrow \max(d, h_i)
```

where $`\mathrm{mid}`$ is the middle of the three values, computed as $`\min\bigl(\max(a_x, a_y), \max(a_y, a_z), \max(a_z, a_x)\bigr)`$: a point is in the cross when two of its coordinates are inside the hole. $`h_i`$ is positive inside the tunnels, so the final $`d`$ is the distance estimate itself (no derivative needed). Orbit trap: the level whose hole sets $`d`$, divided by `iterations` (0 on the outer faces).

**Origin.** Karl Menger described the sponge in 1926, in his work on topological dimension (*General Spaces and Cartesian Spaces*, communications to the Amsterdam Academy of Sciences): every curve, in the sense of any compact one-dimensional space, fits inside it. Each face is a Sierpiński carpet. See [Menger sponge](https://en.wikipedia.org/wiki/Menger_sponge). The distance function is the mod-and-cross construction from Inigo Quilez's article [Menger Fractal](https://iquilezles.org/articles/menger/).

**Parameters.** `iterations` (4): levels of holes as seen from afar, 0 to 12 (0 is the plain cube); each level adds holes a third the size of the last, at the cost of one more fold. Close up, ray adds levels as the pixels can resolve them: up to 12, or up to 20 when it evaluates the sponge relative to an anchor point near the camera (that anchor is computed in double precision, so the finest holes stay sharp in single precision).

---

[All fractals](../FRACTALS.md#the-classics) · [Mandelbulb →](mandelbulb.md)
