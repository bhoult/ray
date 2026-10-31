[← Ides](ides.md) · [All fractals](../FRACTALS.md#more) · [msltoe Sym4 →](msltoe_sym.md)

# Distance IFS (`difs`)

*More*

![Distance IFS](../images/fractals/difs.jpg)

A Menger sponge copied smaller and folded outward ring after ring, so one big sponge stands on a twisting skirt of ever smaller ones that thins to a carpet. Every copy is a whole, crisp sponge, because the distance is measured to real copies rather than estimated from an escaping orbit.

**Formula.** A DIFS ("distance IFS") turns the usual fractal recipe inside out. An ordinary iterated function system folds and scales space and then judges by how fast a point runs away. A DIFS instead places an ordinary shape (here a Menger sponge) in every frame the folds pass through, measures the exact distance to it there, and divides by the scale built up so far. The minimum over all the frames is the distance to the union of all the copies. Because folds, turns and uniform scalings never stretch distances unevenly, the result is exact to the copies; only the sponge's own distance is knighty's folded estimate, which stays on the safe side in its tunnels. Start from $`z = p`$ (with $`y`$ vertical), $`d = \infty`$, and for each ring $`i = 0, 1, \dots`$:

```math
d \leftarrow \min\left(d,\ s^{-i}\, D_\text{sponge}(z)\right)
```

```math
z_x \leftarrow s\,|z_x| - (s-1)\,o_x, \quad z_z \leftarrow s\,|z_z| - (s-1)\,o_z, \quad z_y \leftarrow s\,z_y - (s-1)\,o_y
```

then turn $`z`$ by $`\theta`$ about the vertical. The absolute values fold space into one quadrant and the scaling about $`o`$ pushes the next ring out to the corners, so ring $`i`$ holds $`4^i`$ copies $`s^i`$ times smaller, sinking toward height $`o_y`$. With `fold_up` the vertical is folded too ($`s\,|z_y|`$), and the copies spread to all eight corners. $`D_\text{sponge}`$ is knighty's Menger sponge of half size $`h`$: scale $`z`$ by $`1/h`$, then repeat `sponge_levels` times: take $`|z|`$, sort so $`z_y`$ is the least, fold $`z_y \leftarrow \tfrac23 - z_y`$ when $`z_y \lt  \tfrac13`$, and set $`z \leftarrow 3z - 2`$; the sponge's distance is the distance to the box $`[-1, 1]^3`$ times $`h / 3^\text{levels}`$.

**Origin.** knighty posted the idea as `mdifs` ("mdifs by knighty (jan 2012)") in [Fragmentarium's Knighty Collection](https://github.com/Syntopia/Fragmentarium/blob/master/Fragmentarium-Source/Examples/Knighty%20Collection/mdifs.frag): a fold-and-scale iteration with a Menger sponge measured at every step. Darkbeam built a whole family of dIFS formulas for Mandelbulb 3D around the same idea, with boxes, cylinders, tori and more shapes in place of the sponge; they circulated in Fractal Forums' [custom formulas and transforms thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/). Mandelbulber 2 gathered them as its DIFS formulas; this one follows its [DIFS Menger](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/difs_menger.cl), with knighty's exact $`s^{-i}`$ scaling instead of Mandelbulber's derivative-plus-one, and a turn per ring.

**Parameters.** `fold_scale` (2.3): how much smaller each ring is than the last (1.5 to 3); about 2 packs the copies edge to edge into a carpet, higher leaves gaps that show the branching, lower crowds them into a dense mound. `offset` ([2, -1.2, 2]): x and z set how far out each ring is folded (0 to 2; negative folds the rings back into the big sponge); y is the height the rings sink to (negative: below the big sponge; positive: they climb above it; with `fold_up`, how far above and below). `size` (0.55): the sponge's half size (0.1 to 1); small leaves the copies apart, about 0.5 lets them touch, larger merges them into a stepped mass. `sponge_levels` (3): the sponge's own Menger levels (0 to 6); 0 copies plain cubes. `turn` (12): degrees each ring turns about the vertical; 0 lines the copies up in a square, 10 to 30 twists the skirt into spiral arms. `fold_up` (0): 1 folds the vertical too, for a cluster of sponges in all eight directions instead of a mound. `iterations` (8): rings of copies (up to 12); close up, more come in by themselves. The orbit texture colors each copy by its ring.

---

[← Ides](ides.md) · [All fractals](../FRACTALS.md#more) · [msltoe Sym4 →](msltoe_sym.md)
