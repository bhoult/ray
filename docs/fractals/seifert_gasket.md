[All fractals](../FRACTALS.md#originals) · [Indra's Cells →](indra_cells.md)

# Seifert Gasket (`seifert_gasket`)

*Originals*

![Seifert Gasket](../images/fractals/seifert_gasket.jpg)

A ring of knotted strands of every size, packed like the circles of an Apollonian gasket. Every strand is a torus knot (a trefoil by default), and any two strands are linked. Right: the ring cut open, showing the strands' cross-sections, which are the gasket's circles.

**Formula.** The 3-sphere $`S^3 \subset \mathbb{C}^2`$ is filled by the orbits of the circle action $`(z_1, z_2) \mapsto (e^{ipt} z_1, e^{iqt} z_2)`$: a Seifert fibration whose fibers are $`(p, q)`$ torus knots (for $`p = q = 1`$, the Hopf fibration). The function

```math
u = z_2^{\,p} / z_1^{\,q}
```

is constant on each fiber, so it is a coordinate on the orbit space. A point $`x`$ of space goes to $`S^3`$ by inverse stereographic projection, $`(z_1, z_2) = (2x, |x|^2 - 1)/(1 + |x|^2)`$, and then to $`u`$. In the $`u`$-plane sits an Apollonian-type gasket inside the unit circle: $`n`$ circles of radius $`r = \sin(\pi/n)/(1 + \sin(\pi/n))`$ in a ring around a central circle of radius $`1 - 2r`$. Every gap between them is a triangle of mutually tangent circles, so the packing is completed by repeated inversion. Start from $`u`$ and repeat: turn $`u`$ into the wedge $`0 \le \arg u \le \pi/n`$ (with a mirror), stop if it lies inside the big circle on the axis or the central circle, and otherwise invert it in the dual circle of the gap it is in (the circle through the gap's three tangency points, orthogonal to its sides). Each strand is the preimage of a disk, narrowed in that disk's own frame by `gap` of its radius plus `spacing`. The disk $`|u| \le 1`$ lifts to a solid torus that stereographic projection makes round: the envelope, with major radius $`1/a`$ and minor radius $`b/a`$, where $`b^p = a^q`$ and $`a^2 + b^2 = 1`$.

Distance estimate. The circle action is by isometries, so the distance in $`S^3`$ from a point to a union of fibers is the distance in the orbit space. Since $`u`$ is holomorphic on $`\mathbb{C}^2`$, its stretch is exactly $`|\nabla u| = |u|\sqrt{q^2/|z_1|^2 + p^2/|z_2|^2}`$. Stereographic projection multiplies lengths by $`2/(1 + |x|^2)`$, and each inversion in a dual circle of radius $`R`$ about $`m`$ by $`R^2/|u - m|^2`$. With $`K`$ the product of these factors, the estimate is

```math
d = \min\big(|u - c| - (r(1-g) - wK),\ r - |u - c| + wK\big)/K
```

in the frame where $`u`$ landed in the circle of center $`c`$ and radius $`r`$. The first term is the narrowed strand. The second bounds every other strand: they all lie beyond the circle's rim, at least `spacing` ($`w`$) further in. Outside the envelope, the estimate is the torus's exact distance plus $`w`$. The inversions are carried to first order; with a step of 0.75 the estimate passes the steepness test at the defaults and at every corner of the parameter ranges we tried. The shape is scaled so that the ring's outer radius is 2 units whatever the knot.

**Origin.** Invented for ray by Claude (Anthropic's Claude Opus 5.5), October 2026; to our knowledge new. We searched the web, Fractal Forums and Shadertoy ("Hopf fibration Apollonian gasket fractal linked tori", "Seifert fibration torus knots fractal shadertoy", "Hopf fibration fractal pullback Julia set", "fractalforums hopf fibration fractal distance estimator", "shadertoy hopf apollonian gasket raymarch"), Mandelbulber 2's formula list (its knotted formulas are `knot_v1`, `knot_v2` and `msltoe_donut`) and Fragmentarium's examples (`Knot.frag`, the polychora scripts, the stereographic Julia sets), and found no Hopf- or Seifert-fibered fractal and no fractal pulled back along the Hopf map. Nearest relatives: pictures of the Hopf fibration as nested linked tori (Niles Johnson's animations; [3D-XplorMath's Hopf-fibered linked tori](https://virtualmathmuseum.org/Surface/hopf_fibered/hopf_fibered.html)); Dror Bar-Natan's [Planet Hopf](https://www.math.utoronto.ca/drorbn/Gallery/KnottedObjects/PlanetHopf/index2.html), which pulls a map of the Earth back along the Hopf map; DarkBeam's and knighty's torus-knot distance estimators (Fractal Forums, 2012; Mandelbulber 2's `knot_v1` and `knot_v2`, Fragmentarium's `Knot.frag`), which draw a single knotted bundle of tubes; and the Apollonian gasket itself.

**Parameters.** `turns` (2): how many times each strand winds around the ring's axis, 1 to 5. `twists` (3): how many times it winds around the ring's tube, 1 to 8. Together they pick the knot: 2 and 3 make trefoils, 2 and 5 cinquefoils, and 1 and 1 plain rings that link pairwise like the Hopf fibration. Coprime pairs give one knot per strand; others give several parallel ones. `strands` (3): the big strands around the core, 3 to 8. `gap` (0.35): the space around each strand as a share of its thickness, 0 to 0.7; 0 packs the strands into one solid ring. `spacing` (0.025): the least space between strands, in the shape's units, 0.002 to 0.1; strands thinner than about twice this are left out, so it also sets how fine the packing goes. `iterations` (20): inversions followed into the gaps; more finds smaller strands.

---

[All fractals](../FRACTALS.md#originals) · [Indra's Cells →](indra_cells.md)
