[← Schottky group](schottky.md) · [All fractals](../FRACTALS.md#spheres-and-tori)

# Pseudo-Kleinian Mod 3 (`kleinian_mod`)

*Spheres and tori*

![Pseudo-Kleinian Mod 3](../images/fractals/kleinian_mod.jpg)

Stories of domed floors held up by columns of beads, an endless arcade. It has the pseudo-Kleinian's box folds and inversions, but its distance is measured to a floor plate and to struts along the axes instead of to rings. Where ray's `kleinian` is a tangle of wire rings, this one is architecture.

**Formula.** Start from $`z = p`$ and $`\partial = 1`$ and repeat: push out by the box offset, $`z \leftarrow z - o\,\mathrm{sgn}(z)`$; box fold, $`z \leftarrow 2\,\mathrm{clamp}(z, -c, c) - z`$; invert, $`k = \max(s/|z|^2, 1)`$, $`z \leftarrow k z + u`$, $`\partial \leftarrow k\,\partial`$. Distance estimate:

```math
d = \frac{1}{2\partial} \min\left(\sqrt{\min(z_x^2 + z_y^2,\; z_y^2 + z_z^2,\; z_z^2 + z_x^2)} - r_s,\;\; |z_y| - r_p\right),
```

the distance to struts along the three axes and to a plate across $`y`$.

**Origin.** Knighty's pseudo-Kleinian (Fractal Forums, 2011, in the [Fragmentarium thread](http://www.fractalforums.com/3d-fractal-generation/fragmentarium-an-ide-for-exploring-3d-fractals-and-other-systems-on-the-gpu/msg32270/#msg32270); Fragmentarium's `PseudoKleinian.frag` credits it as Theli-at's "scale 1 JuliaBox + something"). Mod 3 is mclarekin's variant in Mandelbulber 2 (v2.21, March 2020); this is a port of its `pseudo_kleinian_mod3.cl`. Of Mandelbulber's variants (Mod 1 to Mod 7 and Trig), Mod 3 looks the least like ray's `kleinian` as it stands. Most of the others keep the ring estimate and change the look only through optional extras; Mod 3 changes what is measured. There are two changes from Mandelbulber. The plate lies across y instead of z (Mandelbulber's z is up, ray's y), so it stays a floor. The fold box and inversion default to Knighty's values, as ray's `kleinian` does, rather than Mandelbulber's 0.7 and 0.5.

**Parameters.** `fold_box` ([0.92436, 0.90756, 0.92436]): half sizes of the box fold along x, y and z (0.7 to 1.1). Smaller pinches the columns to cusps; larger widens the halls. `inversion` (1): the inversion sphere's radius squared (0.5 to 1.5): larger swells the beads and domes. `box_offset` ([0, 0, 0]): pushes each coordinate away from the center before the fold (0 to 0.1). The columns melt into ribbed trunks; much more fills the space solid. `offset` ([0, 0, 0]): added after each inversion (-0.1 to 0.1): warps and melts the arcades. `strut` (0): the radius of the struts along the axes (0 to 0.6). They hide inside the columns until about 0.4, then swell into drums and discs. `plate` (0): the floor plate's half thickness (0 to 0.2): thicker plates facet the domes like umbrellas. `iterations` (8): fold steps, up to 20; more gives finer beads, at linear cost.

---

[← Schottky group](schottky.md) · [All fractals](../FRACTALS.md#spheres-and-tori)
