[← Pine Tree](pine_tree.md) · [All fractals](../FRACTALS.md#more) · [Ides →](ides.md)

# Makin (`makin`)

*More*

![Makin](../images/fractals/makin.jpg)

A 3D squaring with a twist in its cross terms. It comes out as a swept, three-cornered sail under a spire, its edges frayed into spikes.

**Formula.** Start from $`z = p`$ and repeat:

```math
z \leftarrow (x^2 + 2yz,\ -y^2 - 2xz,\ -z^2 + 2xy) + (1 - j)\,p + j\,c
```

where $`j`$ morphs from the Mandelbrot type ($`j = 0`$) to the Julia set of $`c`$ ($`j = 1`$). Bailout $`|z| \ge 8`$. Like the pine tree, the map is far from conformal. The kernel carries the orbit's Jacobian and estimates $`d = \tfrac12 r \ln r / g`$ from the potential's gradient $`g = |J^{\mathsf T} z| / r`$, blended across the bailout, with a march step of 0.4. The set is turned a quarter turn about $`y`$, so the default view looks along the sail, and centered. The orbit trap is the closest the orbit comes to the formula's own $`y`$ axis.

**Origin.** David Makin, an Ultra Fractal formula author and Fractal Forums moderator. The map is "WIP3D5" from his Ultra Fractal formula file MMFwip3D.ufm, which was circulating by mid-2010. Luca GN (DarkBeam) ported it to Mandelbulb 3D in 2011 as [Makin3D-2](https://raw.githubusercontent.com/thargor6/mb3d/master/M3Formulas/Makin3D-2.m3f). When Makin first posted the -2 variant is unknown. Mandelbulber cites the thread [Another shot at the holy grail](https://web.archive.org/web/20150908093646/http://www.fractalforums.com/3d-fractal-generation/another-shot-at-the-holy-grail/), started by quick-dudley in November 2012. There, in January 2015, DarkBeam noted that the squaring derived in the thread was almost identical to Makin3D-2. Makin himself did not post in it. Ported from Mandelbulber 2's `makin3d2`.

**Parameters.** `julia` (0): from the Mandelbrot type (0, the point added at each step) to the Julia set of `c` (1); values in between morph one into the other. `c` (-0.5, 0.5, 0.5): the Julia constant, each component from -1.5 to 1.5, used to the extent that `julia` is above 0. Near the origin it gives a solid, cushion-like lump; further out the lump becomes swirled and holed, then breaks into dust. `iterations` (14, at most 30): more iterations give finer spikes.

---

[← Pine Tree](pine_tree.md) · [All fractals](../FRACTALS.md#more) · [Ides →](ides.md)
