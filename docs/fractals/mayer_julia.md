[← Quasicrystal Rooms](quasicrystal_rooms.md) · [All fractals](../FRACTALS.md#originals) · [Screw Sponge →](screw_sponge.md)

# Mayer Julia (`mayer_julia`)

*Originals*

![Mayer Julia](../images/fractals/mayer_julia.jpg)

A spire of icy fronds: fans of long blades with serrated, feathered edges, smaller fronds sprouting from the larger ones, and seen end on, a four-armed star. It is a Julia set of $`z^2 + c`$ made three-dimensional the way analysts do it: not with an algebra of triples, but with a quasiregular map, one of bounded distortion: it turns small balls into ellipsoids at most a fixed number of times as long as they are wide, where a holomorphic map keeps small disks round. The long blades come from that distortion, compounded step after step. Left: the Add example, its axis standing up. Right: fronds fanning out of one of the star points.

**Formula.** Zorich's map, as in Zorich Pagodas, takes the beam over the square $`|x_1|, |x_2| \le 1`$ onto the upper half-space,

```math
Z(x) = e^{x_3}\,\frac{v}{|v|}, \qquad v = \big(x_1,\ x_2,\ 1 - \max(|x_1|, |x_2|)\big),
```

and is extended to all of space by reflecting across the beam's sides (period 4 in $`x_1`$ and $`x_2`$; each reflection mirrors the image in $`y_3 = 0`$). Points of the plane with the same image are related by the group $`G`$ of translations by $`4\mathbb{Z}^2`$ and half-turns about the vertical lines through $`(\text{odd}, \text{odd})`$. Write $`(x_1, x_2)`$ as $`w = x_1 + i x_2`$ and pick a Gaussian integer $`m = a + bi`$ (the `multiplier`). The linear map

```math
A:\ w \mapsto m\,\big(w - (1 + i)\big) + (1 + i), \qquad x_3 \mapsto |m|\,x_3
```

carries $`G`$ into itself ($`m\,\mathbb{Z}[i] \subset \mathbb{Z}[i]`$), so the Schröder equation $`P(Z(x)) = Z(A x)`$ defines a map $`P`$ of space: Mayer's power map. Since $`A`$ is a similarity, $`P`$ is uniformly quasiregular: all its iterates have the same bounded distortion, the defining property of a holomorphic map's higher-dimensional analogue. Explicitly, $`P(y) = |y|^{|m|}\,H(y/|y|)`$, where $`H`$ is a degree-$`|m|^2`$ map of the sphere (a "pillowcase" Lattès map: the two hemispheres are the two faces of a pillow, and $`H`$ stretches them by $`m`$, turning by $`\arg m`$ and scaling by $`|m|`$, and folds the result back over the pillow). To evaluate it, invert $`Z`$, which for the pyramid is cheap,

```math
x_3 = \ln|y|, \qquad (x_1, x_2) = \frac{(y_1, y_2)}{|y_3| + \max(|y_1|, |y_2|)}
```

(below the equator the point lies in the next beam: $`x_1 \to 2 - x_1`$), apply $`A`$, and fold back through $`Z`$. The shape is the filled Julia set of $`y \mapsto P(y) + c`$: the points whose orbits stay bounded. The frame $`(y_1, y_2, y_3) = \big((x + z)/\sqrt2,\ (x - z)/\sqrt2,\ y\big)`$ puts the ray through the corner $`(1, 1)`$, which $`P`$ fixes, along the shape's x axis. On that axis $`P`$ is $`x \mapsto |x|^{\sqrt2}`$ (for $`m = 1 + i`$), so with $`c`$ on it the map restricted to the axis is the real family $`|x|^{\sqrt2} + c`$, whose Julia sets are intervals and Cantor sets; the crystals grow around those. For $`m = 1 + i`$ every Julia set is unchanged by the half-turn about Zorich's axis (the shape's y), since $`P`$ is, just as a quadratic Julia set is unchanged by $`z \mapsto -z`$.

**Distance estimate.** With $`r_n = |y_n|`$ at the escape, the estimate is the potential over its gradient,

```math
d = \frac{r_n \ln r_n}{2\,|\nabla r_n|},
```

where $`\nabla r_n`$ is carried by forward-mode dual numbers through the exact map, folds and maxima included, and the estimate is blended with the previous iterate's across the bailout (100). It uses the gradient of $`r_n`$, which is exactly the potential's, rather than the norm of the whole Jacobian, as the Mandelbulb's running derivative does: the map stretches unevenly, its Jacobian's norm jumps across the creases of $`Z`$, and with it the estimate failed the steepness test even at the defaults. Bounded distortion is what makes the potential estimate meaningful: $`|P(y)| = |y|^{|m|}`$ exactly, and $`P`$'s iterates are $`K`$-quasiregular for one $`K`$, so near infinity the potential behaves like a conformal one up to fixed constants. But $`P + c`$ is not uniformly quasiregular, so no Koebe-type bound holds, and the step (0.6) is set by the steepness test, at the defaults and at 21 corners of the parameters. Two corrections keep it there. Near the critical point $`0`$, where $`P`$'s stretch fades as $`|y|^{|m| - 1}`$ and the potential is far from linear, the stretch is floored at its value at $`|y| = 0.6`$. And when the Julia set is a dust (the critical value $`c`$ escapes), the estimate is divided by $`1 + 0.4\,(|m|^2 - 1)\,G(c)`$, where $`G(c)`$ is $`c`$'s escape rate, computed once per shape: between the grains the potential has saddle points (the preimages of $`0`$), near which it overshoots. Lanes whose gradient passes $`10^{15}`$ count as inside. The orbit trap is the nearest the orbit comes to the x axis in its first three steps, which colors each frond as a whole.

**Origin.** Invented for ray by Claude (Anthropic's Claude Opus 5.5), October 2026; to our knowledge new as a picture, though the map is mathematics' own. V. Mayer constructed uniformly quasiregular maps from Zorich's map by solving Schröder equations: Lattès-type maps ([*Uniformly quasiregular mappings of Lattès type*](https://www.ams.org/journals/ecgd/1997-01-07/S1088-4173-97-00013-1), Conform. Geom. Dyn. 1, 1997) and power maps of degree $`d^2`$, whose Julia set is the unit sphere (*Quasiregular analogues of critically finite rational functions with parabolic orbifold*, J. Anal. Math. 75, 1998). A. Fletcher built the degree-two one, with $`A`$ a rotation by $`\pi/4`$ scaled by $`\sqrt2`$, which is $`m = 1 + i`$ here ([*Attractor sets and Julia sets in low dimensions*](https://arxiv.org/abs/1810.02834), Conform. Geom. Dyn. 23, 2019), and with D. Macclure proved that $`A G A^{-1} \subset G`$ is enough for the Schröder equation to have a unique uniformly quasiregular solution ([*Strongly automorphic mappings and Julia sets of uniformly quasiregular mappings*](https://arxiv.org/abs/1703.02455), J. Anal. Math. 141, 2020). The power map alone has a sphere for its Julia set; adding a constant, as $`z^2 + c`$ adds one to $`z^2`$, is what makes these shapes, and we found no picture of that family. We searched the web, Fractal Forums and Shadertoy ("quasiregular power map Mayer Julia set z^d + c three dimensions picture render", "\"quasiregular\" mandelbulb fractal render", "fractalforums quasiregular map Julia set 3D", "fractalforums.org quasiconformal OR quasiregular 3D Mandelbrot true analogue bounded distortion", "shadertoy Zorich map fractal", "\"uniformly quasiregular\" power map Julia set \"z^2+c\" three dimensional analogue rendered image", "Lattès map pillowcase three-dimensional fractal ray marching"), Mandelbulber 2's formula list (463 files, none named for Zorich, Lattès or quasiregular maps) and Fragmentarium's examples (none named for one). The papers we read have no pictures of it. Nearest relatives: the Mandelbulb and Juliabulb, White and Nylander's $`z^n`$ in spherical coordinates, which is not quasiregular (it collapses whole circles of latitude to the poles); the quaternion Julia sets, which are $`z^2 + c`$ in an algebra and so are 2D Julia sets spun about an axis; V. Kučera's [*New Generalizations of Quadratic Julia Sets to 3D*](https://archive.bridgesmathart.org/2019/bridges2019-441.html) (Bridges 2019), which goes through Kleinian groups instead; the Fractal Forums [tetrahedral Mandelbulb](https://www.fractalforums.com/the-3d-mandelbulb/a-new-tetrahedral-mandelbulb) thread, which sought a squaring "as conformal as possible"; and Zorich Pagodas, built on the same Zorich map but iterating the exponential itself.

**Parameters.** `multiplier` ([1, 1]): the Gaussian integer $`a + bi`$, each part -3 to 3. The degree is $`a^2 + b^2`$ and the radial power $`\sqrt{a^2 + b^2}`$: [1, 1] is degree two, the analogue of $`z^2 + c`$; [2, 0] gives degree 4, [2, 1] degree 5, [2, 2] 8 and [3, 0] 9, rounder and furrier. $`-m`$ is the same map as $`m`$, $`\bar m`$ its mirror image, and anything of degree under 2 is taken as [1, 1]. `c` ([-0.75, 0, 0]): the constant, each component -1.5 to 1.5. On the x axis, from -0.4 to -1.5 it grows the spiky crystals, slimmer and spikier the further out (the axis's own interval stretches to about -5); sideways components of 0.1 to 0.3 bend them into curled dragons; most values off the axis give a furry ball or, past the edge of connectedness, a dust of grains. `iterations` (14, up to 40): more give finer blades. The shape fits in a cube of half-size 3.7 whatever the parameters; the default crystal reaches 2 units along its axis and 1.1 across, so the Add example (scale 0.55) is about 2.2 m tall, turned so that its x axis stands up.

---

[← Quasicrystal Rooms](quasicrystal_rooms.md) · [All fractals](../FRACTALS.md#originals) · [Screw Sponge →](screw_sponge.md)
