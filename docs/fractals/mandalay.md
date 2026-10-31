[← Kalibox](kalibox.md) · [All fractals](../FRACTALS.md#box-folds) · [Kaliset →](kaliset.md)

# Mandalay (`mandalay`)

*Box folds*

![Mandalay](../images/fractals/mandalay.jpg)

A Mandelbox whose box fold is DarkBeam's Mandalay fold, which carves every face into stepped frames instead of mirroring it. The set grows into a cathedral: ornamented panels and buttresses outside, and inside, chambers with vaults, arches and framed windows to the sky.

**Formula.** Start from $`z = p`$ and repeat:

```math
z \leftarrow s\,\mathrm{sphereFold}(\mathrm{mandalay}(z)) + w\,p + J
```

where the Mandalay fold works on $`a = |z_i|`$ for each axis, with $`o`$ the larger of the other two coordinates, $`f`$ the fold size and $`g`$ the offset:

```math
\mathrm{mandalay}(a) = \min\big(\max(|a - f| - f,\ o - 4f),\ \max(a - 2f - g,\ o),\ a\big)
```

then gives each coordinate its sign back. The sphere fold scales by $`1/r_{\min}^2`$ inside the inner sphere and by $`1/r^2`$ up to the unit sphere. Orbits stop past radius 100.
Distance estimate: $`d = |z| / \partial z`$ with $`\partial z \leftarrow |s|\,k\,\partial z + 1`$, $`k`$ the sphere fold's factor.

**Origin.** The fold is DarkBeam's, posted on Fractal Forums in the thread ["'New' fractal type: Mandalay"](http://www.fractalforums.com/amazing-box-amazing-surf-and-variations/'new'-fractal-type-mandalay/msg81348/#msg81348), in the form Knighty adapted it to. Mandelbulber 2 has it as Mandalay Box V1 and V2 and as Mandalay KIFS. Ported from Mandelbulber 2's [mandalay_box_v1.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/mandalay_box_v1.cl), with its three swaps written out as maxima; the Julia constant and the weight of the point are Mandelbulber's Julia mode.

**Parameters.** `fold_scale` (1.8): the scale after each fold; 1.5 to 2.2 gives the cathedral, higher values break it into separate towers, and negative ones a larger, crumpled palace (avoid -1 to 1). `min_radius` (0.5): radius of the sphere fold's inner sphere; smaller gives deeper vaults and more arches, and near 0.75 the facade turns into plain stacked blocks. `fold` (0.5, 0.5, 0.5): the Mandalay fold's size per axis, the width of the frames carved into each face; unequal values make a tower or a long hall. `offset` (0, 0, 0): shifts where each axis's fold steps back; small values (±0.5) add balconies and recesses. `julia` (0, 0, 0): a constant added at every step; with `point` at 0 it is a Julia set (Mandelbulber's example uses (-0.9, 0.7, 0) with `fold_scale` 1.6). `point` (1): how much of the point itself is added; 1 is the Mandelbrot form, the cathedral, 0 the Julia form. `iterations` (12): more gives finer ornament, at linear cost.

---

[← Kalibox](kalibox.md) · [All fractals](../FRACTALS.md#box-folds) · [Kaliset →](kaliset.md)
