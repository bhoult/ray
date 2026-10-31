[← Amazing Surf](amazing_surf.md) · [All fractals](../FRACTALS.md#box-folds) · [Mandalay →](mandalay.md)

# Kalibox (`kalibox`)

*Box folds*

![Kalibox](../images/fractals/kalibox.jpg)

Kali's Julia-set Mandelbox, where the box fold becomes an absolute value and a shift: every step mirrors space into one octant and moves it before the sphere fold and the scale. Its outside is usually a closed box with flat faces; these defaults open two sides of it into a gothic pillar of vaulted halls with ornamented walls, chambers stacked inside it.

**Formula.** Start from $`z = p`$ and repeat:

```math
z \leftarrow s\,\mathrm{sphereFold}(|R_y z| + T) + J + w\,p
```

with $`|\cdot|`$ taken per component, $`T`$ the shift (`trans`), $`J`$ the Julia constant, $`w`$ the weight of the point (`point`), $`R_y`$ the turn about y (`turn`), and the sphere fold scaling by $`1/r_{\min}^2`$ inside the inner sphere, by $`1/r^2`$ between it and the unit sphere, and not at all outside. Orbits stop past radius 100.
Distance estimate (Rrrola's, as in Kali's Fragmentarium code): $`d = (|z| - |s - 1|) / \partial z - |s|^{1-n}`$ with $`\partial z \leftarrow |s|\,k\,\partial z + 1`$, where $`k`$ is the sphere fold's factor and $`n`$ the iterations.

**Origin.** By Kali (Fractal Forums). His Kalibox.frag ships with Syntopia's Fragmentarium among "Kali's Creations" since [version 0.9.12 "Prague"](https://blog.hvidtfeldt.net/index.php/2012/08/fragmentarium-version-0-9-12-prague-released/) (2012); the same fold, written as $`T - |z|`$ with the point added instead of a constant, is Mandelbulb 3D's ABoxModKali ([Fractal Forums](http://www.fractalforums.com/new-theories-and-research/aboxmodkali-the-2d-version/)), Mandelbulber 2's [abox_mod_kali.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/abox_mod_kali.cl). Ported from Kalibox.frag; `point` blends in the Mandelbulb 3D form. The defaults are new, chosen for their halls; Kali's own preset (`fold_scale` 2.04, `min_radius` 0.59, `trans` (0.04, -1.86, 0.04), `julia` (-0.67, -1.30, -0.46)) is a thin tower about 10 units tall.

**Parameters.** `fold_scale` (-1.54): the scale after each fold; negative values (-1.5 to -2) give vaulted halls, positive ones (1.5 to 2.2) boxy cities (avoid -1 to 1). `min_radius` (0.56): radius of the sphere fold's inner sphere; smaller makes deeper niches and more ornament. `trans` (-0.81, -0.85, -0.92): the shift after the abs fold, per axis; it sets where the halls open and how large they are, and small changes move whole walls. `julia` (-0.55, -0.3, -0.8): the constant added at every step, per axis; it decides which faces of the box stay closed and which open into halls. `point` (0): how much of the point itself is added; 0 is Kali's Julia form, 1 the ABoxModKali Mandelbrot form (which outgrows the bound with these defaults). `turn` (0): degrees turned about y before each fold; a few degrees twist the halls, larger turns grow a hairy, rocky skin. `iterations` (14): more gives finer ornament, at linear cost.

---

[← Amazing Surf](amazing_surf.md) · [All fractals](../FRACTALS.md#box-folds) · [Mandalay →](mandalay.md)
