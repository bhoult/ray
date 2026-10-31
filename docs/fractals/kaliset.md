[← Mandalay](mandalay.md) · [All fractals](../FRACTALS.md#box-folds) · [Bulb Box →](bulbbox.md)

# Kaliset (`kaliset`)

*Box folds*

![Kaliset](../images/fractals/kaliset.jpg)

Kali's Kaliset in 3D: mirror space into one octant, invert it in the unit sphere, add a constant, and repeat. Everything far away lands near the constant, so the set fills space; what shapes it are the holes, the points whose orbit falls close to the center, and cutting them away leaves a lace of domed chambers, round windows and bony struts. Cut to a box here; fly inside it.

**Formula.** Start from $`z = p`$ and repeat:

```math
z \leftarrow R\left(\frac{k\,|z|}{|z|^2} + c + w\,p\right)
```

with $`|z|`$ per component in the numerator, $`k`$ the inversion's radius² (`inversion`), $`c`$ the constant (`julia`), $`w`$ the weight of the point, and $`R`$ the turn by `angles`. A point is cut away when its orbit comes within $`h`$ (`hole`) of the center at any step.
Distance estimate: the depth into the deepest such hole, $`d = \max_i\,(h - |z_i|) / \partial z_i`$ with $`\partial z \leftarrow (k / |z|^2)\,\partial z + w`$; it is negative in the solid.

**Origin.** Kali posted the formula, in 2D, on Fractal Forums on May 1, 2011 as a ["Very simple formula for fractal patterns"](https://www.fractalforums.com/new-theories-and-research/very-simple-formula-for-fractal-patterns/): $`x \leftarrow |x|,\ y \leftarrow |y|`$, then divide by $`x^2 + y^2`$ and add $`c`$. He later named it the Kaliset and took it into 3D, among other uses as coloring and bump maps on other fractals ([Fractal Forums](https://www.fractalforums.com/3d-fractal-generation/kaliset-3d-fractal-used-as-coloring-and-bump-mapping-for-de-systems)). In Mandelbulb 3D it became Kalisets1 (with KaliDucks), in Mandelbulber 2 [kalisets1.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/kalisets1.cl), whose iteration this is. Mandelbulber draws it by escape past a bailout; this port cuts away the bailout's source instead, a ball about the center, which gives the estimate above and clean chamber walls.

**Parameters.** `inversion` (1): radius² of the sphere it inverts in; above 1 swells the chambers, below 1 thins the struts into lace. `julia` (-0.5, -0.5, -0.5): the constant added at every step (the Kaliset's $`c`$); -0.3 to -1 per axis works, and unequal values break the cubic symmetry. `hole` (0.5): radius of the hole about the center; larger opens bigger chambers, smaller leaves a solid block pitted with bubbles. `point` (0): how much of the point itself is added at every step; 0 is the Julia form. `angles` (0, 0, 0): degrees turned about x, y and z at every step; a few degrees twist the chambers, 0 keeps the cubic symmetry. `iterations` (8): each cuts away the next, smaller generation of holes; past 10 the walls are pitted with dust.

---

[← Mandalay](mandalay.md) · [All fractals](../FRACTALS.md#box-folds) · [Bulb Box →](bulbbox.md)
