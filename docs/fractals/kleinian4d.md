[← Mandelbox 4D](mandelbox4d.md) · [All fractals](../FRACTALS.md#4d-slices)

# Kleinian 4D (`kleinian4d`)

*4D slices*

![Kleinian 4D](../images/fractals/kleinian4d.jpg)

A slice through a four-dimensional pseudo-Kleinian (pictured at `w` = 0.2): box folds and a 4D sphere inversion, drawn around a vertical pillar and a floor plane. Each slice is a hall of floors, columns, domes and arches. Like the 3D one it fills space, so it is cut to a box here.

**Formula.** Start from $`z = (x, y, z, w)`$, $`D = 1`$, and repeat:

```math
z \leftarrow k\,\mathrm{boxFold}(z), \qquad k = \max\bigl(a / |z|^2,\ 1\bigr), \qquad D \leftarrow k\,D,
```

with the box fold $`f(t) = |t + l| - |t - l| - t`$ on each axis. Distance estimate: the distance to the pillar (the y axis, in 4D) or the floor ($`y = 0`$), whichever is nearer, scaled back and halved as Mandelbulber does:

```math
d = \tfrac12\Bigl(\min\bigl(\sqrt{x^2 + z^2 + w^2} - \rho,\ |y|\bigr) - t\Bigr) / D.
```

Mandelbulber's z axis is up; here it is y, so the floors lie flat.

**Origin.** TGlad's 4D pseudo-Kleinian, from the fractalforums.org thread ["New sphere tree"](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557/msg22100#msg22100), with extras by sabine62. It builds on Knighty's pseudo-Kleinian (Fractal Forums). Ported from Mandelbulber 2's Pseudo Kleinian 4D (`pseudo_kleinian4d.cl`; definition dated 2020). The defaults use Knighty's classic fold sizes and a larger inversion rather than Mandelbulber's 1 and 1, which give sparse slices that lose everything but the floor away from w = 0. Mandelbulber's 0.05 derivative tweak and optional extras are left out.

**Parameters.** `w` (0): the slice's 4th coordinate (−2 to 2). At 0 it is the 3D form, a closed block of walls with round windows. At 0.2 goblet-topped columns stand on a floor, at 0.4 they become arches, at 0.6 floating crowns, and near 0.9 the floor is bare but for beads. The slices repeat every 2 × the w fold (1.8 by default): −w and w match, and so do 1.8 − w and w. `fold` ([0.92436, 0.90756, 0.92436, 0.9]): the box fold's limit on x, y (up), z and w, half the repeating cell on each axis. Just under 1 keeps the spheres from lining up with the cells, for richer halls; 1 gives tidy, sparse ones (0.25 to 2). `inversion` (1.5): the inversion sphere's radius squared. Larger values fill the halls with walls, domes and round windows and keep more of it away from w = 0; smaller ones leave bare floors and beads (0.25 to 2). `radius` (0.1): the columns' radius around the vertical axis, in the folded space; 0 leaves only the floors and domes (0 to 0.5). `thickness` (0.02): thickens the floors and columns alike (0 to 0.2). `iterations` (10, up to 30): more gives finer floors and columns.

---

[← Mandelbox 4D](mandelbox4d.md) · [All fractals](../FRACTALS.md#4d-slices)
