# Fractals

Every kind of fractal ray draws: what it is, its formula and distance estimate, where it came from, and its parameters. Each is a shape type: `{"type": "mandalay", "scale": 0.6}` in a scene, or Add > Fractals in the editor (the families are submenus there). `saves/fractal-catalog.json` shows them all, a family to a row; the screenshots below are from `ray SCENE --out` renders.

All of them take the common fields (`position`, `rotation`, `scale`, `material`...). A fractal colored by the `orbit` material pattern shows its orbit trap: where its iteration came closest to its center, or which level of it the surface belongs to.

Most of the catalog's formulas come from the 3D fractal community of Fractal Forums (2009 onward), many by way of [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) (GPL-3, as ray is), whose authors collected and coded them. Each entry credits its formula's authors. Where ray's version differs (a different distance estimate, defaults or orientation), the entry says so.

## Contents

- [The classics](#the-classics): `menger`, `mandelbulb`, `mandelbox`, `julia`, `sierpinski`, `apollonian`, `kifs`, `kleinian`
- [Box folds](#box-folds): `amazing_surf`, `kalibox`, `mandalay`, `kaliset`, `bulbbox`
- [Bulbs](#bulbs): `juliabulb`, `mandelbar`, `burning_ship`, `power2_bulb`, `benesi`, `kosalos`, `lambda_bulb`, `sincos_bulb`, `riemann_sphere`, `riemann_hobold`, `hopfbrot`
- [Algebras](#algebras): `quaternion`, `tetrabrot`, `hypercomplex`, `aexion`, `bristorbrot`, `buffalo`, `mandelcup`, `quick_dudley`, `collatz`, `collatz_mod`, `newton`
- [Spheres and tori](#spheres-and-tori): `msltoe_donut`, `mandeltorus`, `spheretree`, `sphere_cluster`, `jos_kleinian`, `schottky`, `kleinian_mod`
- [Geometric](#geometric): `mosely`, `vicsek`, `cantor_dust`, `jerusalem`, `polyhedral`, `koch`, `knot`
- [4D slices](#4d-slices): `menger4d`, `sierpinski4d`, `mandelbox4d`, `kleinian4d`
- [More](#more): `xenodreambuie`, `pine_tree`, `makin`, `ides`, `difs`, `msltoe_sym`, `eiffie`, `mandelnest`
- [Originals](#originals): `seifert_gasket`, `indra_cells`, `zorich_pagodas`

## The classics

The eight fractals ray has always had, built into its renderer.

### Menger Sponge (`menger`)

![Menger Sponge](images/fractals/menger.jpg)

A cube with a square tunnel bored through the middle of each face, then the same done to each of the 20 smaller cubes that remain, level after level. ray carves the holes directly rather than by escape time, so its distance estimate is tight, and it adds levels by itself as the camera closes in.

**Formula.** Start from the exact distance to the cube $[-1, 1]^3$, $d = \mathrm{sdBox}(p, 1)$. For each level $i = 0, 1, \dots, n-1$, fold each coordinate into that level's cell:

$$a_c = \Bigl|\,1 - 3\,\bigl|\mathrm{mod}(3^i c,\ 2) - 1\bigr|\,\Bigr|, \qquad c \in \{p_x, p_y, p_z\}$$

then measure the level's cross of three square tunnels and carve it out:

$$h_i = \frac{\mathrm{mid}(a_x, a_y, a_z) - 1}{3^{\,i+1}}, \qquad d \leftarrow \max(d, h_i)$$

where $\mathrm{mid}$ is the middle of the three values, computed as $\min\bigl(\max(a_x, a_y), \max(a_y, a_z), \max(a_z, a_x)\bigr)$: a point is in the cross when two of its coordinates are inside the hole. $h_i$ is positive inside the tunnels, so the final $d$ is the distance estimate itself (no derivative needed). Orbit trap: the level whose hole sets $d$, divided by `iterations` (0 on the outer faces).

**Origin.** Karl Menger described the sponge in 1926, in his work on topological dimension (*General Spaces and Cartesian Spaces*, communications to the Amsterdam Academy of Sciences): every curve, in the sense of any compact one-dimensional space, fits inside it. Each face is a Sierpiński carpet. See [Menger sponge](https://en.wikipedia.org/wiki/Menger_sponge). The distance function is the mod-and-cross construction from Inigo Quilez's article [Menger Fractal](https://iquilezles.org/articles/menger/).

**Parameters.** `iterations` (4): levels of holes as seen from afar, 0 to 12 (0 is the plain cube); each level adds holes a third the size of the last, at the cost of one more fold. Close up, ray adds levels as the pixels can resolve them: up to 12, or up to 20 when it evaluates the sponge relative to an anchor point near the camera (that anchor is computed in double precision, so the finest holes stay sharp in single precision).

### Mandelbulb (`mandelbulb`)

![Mandelbulb](images/fractals/mandelbulb.jpg)

The best-known 3D relative of the Mandelbrot set: a round bulb covered in buds, ridges and cauliflower-like growths, made by raising a point to a power in spherical coordinates. ray stands it upright, with the y axis as its pole.

**Formula.** Start from $z = p$ and $dr = 1$, and while $r = |z| < 2$, for at most `iterations` steps, repeat:

$$\theta = \operatorname{atan2}\!\bigl(\sqrt{z_x^2 + z_z^2},\ z_y\bigr), \qquad \phi = \operatorname{atan2}(z_z,\ z_x)$$

$$dr \leftarrow n\,r^{\,n-1}\,dr + 1, \qquad z \leftarrow r^n\bigl(\sin n\theta\,\cos n\phi,\ \cos n\theta,\ \sin n\theta\,\sin n\phi\bigr) + p$$

with $n$ the `power`. This is White and Nylander's formula with $\theta$ measured from the $+y$ axis instead of $+z$; starting from $z = p$ is the same as starting from 0 and taking one more step. Distance estimate: $d = \tfrac12\, r \ln r / dr$, from the last radius and the running derivative. Orbit trap: the smallest $|z|$ along the orbit, halved.

**Origin.** Daniel White (twinbee) started the Fractal Forums thread [True 3D mandelbrot type fractal](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/) in September 2007, and that November proposed doubling a point's angles in spherical coordinates, as complex squaring does in the plane. In 2009 Paul Nylander raised the power instead of squaring, and power 8 gave the Mandelbulb. White tells the story in [The Unravelling of the Real 3D Mandelbrot Fractal](https://www.skytopia.com/project/fractal/mandelbulb.html) (8 November 2009); see also [Mandelbulb](https://en.wikipedia.org/wiki/Mandelbulb). The distance estimate is the standard running-derivative estimate for escape-time fractals, as in Inigo Quilez's [Mandelbulb](https://iquilezles.org/articles/mandelbulb/) article and Mikael Hvidtfeldt Christensen's [Distance Estimated 3D Fractals (V)](https://blog.hvidtfeldts.net/index.php/2011/09/distance-estimated-3d-fractals-v-the-mandelbulb-different-de-approximations/).

**Parameters.** `power` (8): the exponent, 2 to 16. 8 is the classic bulb; higher powers give more, finer lobes, lower ones fewer and broader. `iterations` (8): escape-time steps, 1 to 32. More steps give finer detail and a crisper surface; each step costs a pow, two atan2, a sin and a cos per point. Iterations are not added close up.

### Mandelbox (`mandelbox`)

![Mandelbox](images/fractals/mandelbox.jpg)

Box folds and sphere folds repeated with a scale. At the Add example's fold scale of -1.5 it is a cube whose faces are carved with symmetric relief and whose inside is a maze; positive scales from 2 to 3 give boxy cities.

**Formula.** Start from $z = p$ and $dr = 1$, and repeat `iterations` times (there is no bailout):

$$z \leftarrow 2\,\mathrm{clamp}(z, -1, 1) - z \quad \text{(box fold, on each component)}$$

$$k = \begin{cases} 1/m^2 & |z|^2 < m^2 \\ 1/|z|^2 & m^2 \le |z|^2 < 1 \\ 1 & \text{otherwise} \end{cases}, \qquad z \leftarrow k\,z \quad \text{(sphere fold)}$$

$$z \leftarrow s\,z + p, \qquad dr \leftarrow |s|\,k\,dr + 1$$

with $s$ the `fold_scale`, $m$ the `min_radius`, and the sphere fold's fixed radius 1. The box fold reflects any component beyond $\pm 1$ back inside; the sphere fold inverts points between radius $m$ and 1 and scales those inside $m$ by $1/m^2$. Distance estimate: $d = |z| / |dr|$. Orbit trap: the smallest $|z|$ just after the box folds. The shape is large (about 8 units across for negative scales, 12 at scale 2), so the Add example sets `scale` to 0.25.

**Origin.** Tom Lowe (Tglad) posted it on Fractal Forums on 1 February 2010 in the thread [Amazing fractal](https://www.fractalforums.com/3d-fractal-generation/amazing-fractal/), as a member of the family of fractals discussed in the earlier thread on continuous conformal Mandelbrots; his first renders used scales 3 and 2, a fixed radius of 1 and a minimum radius of 0.5. See Lowe's [What is a Mandelbox?](https://sites.google.com/site/mandelbox/what-is-a-mandelbox), [Mandelbox](https://en.wikipedia.org/wiki/Mandelbox), and Mikael Hvidtfeldt Christensen's [Distance Estimated 3D Fractals (VI): The Mandelbox](https://blog.hvidtfeldts.net/index.php/2011/11/distance-estimated-3d-fractals-vi-the-mandelbox/) for the distance estimate.

**Parameters.** `fold_scale` (2; the Add example uses -1.5): the scale after each fold, which sets the overall shape. Negative values (about -3 to -1.2) give crumpled cubes, positive ones (2 to 3) boxy cities; avoid -1 to 1. `iterations` (12): fold steps, 1 to 40; more give finer detail at a linear cost. Iterations are not added close up. `min_radius` (0.5): the inner radius of the sphere fold, 0.01 to 1; smaller makes sharper, deeper bulbs.

### Quaternion Julia (`julia`)

![Quaternion Julia](images/fractals/julia.jpg)

A Julia set of $z \mapsto z^2 + c$ over the quaternions, a 4D object, sliced in 3D where the fourth coordinate is 0. It is smooth and organic, with swirling shells and whorls, and a small change to $c$ reshapes it completely.

**Formula.** Start from the quaternion $z = (p_x, p_y, p_z, 0)$, whose first component is the real part, and $|z'|^2 = 1$. While $|z|^2 < 16$, for at most `iterations` steps, repeat:

$$|z'|^2 \leftarrow 4\,|z|^2\,|z'|^2, \qquad z \leftarrow z^2 + c$$

where for $z = (a, \mathbf{v})$ with real part $a$ and vector part $\mathbf{v}$, $z^2 = (a^2 - |\mathbf{v}|^2,\ 2a\,\mathbf{v})$. Only the derivative's length is tracked, with the scalar rule $|z'| \leftarrow 2\,|z|\,|z'|$, as for complex numbers. Distance estimate:

$$d = \tfrac14 \sqrt{|z|^2 / |z'|^2}\ \ln |z|^2 = \frac{|z| \ln |z|}{2\,|z'|}$$

Orbit trap: the smallest $|z|$ along the orbit, the starting point included.

**Origin.** Alan Norton first generated and displayed quaternion Julia sets as 3D slices in "Generation and display of geometric fractals in 3-D" ([SIGGRAPH 1982](https://history.siggraph.org/?p=106343)). John Hart, Daniel Sandin and Louis Kauffman ray-traced them with a distance estimate in "Ray tracing deterministic 3-D fractals" ([SIGGRAPH 1989](https://history.siggraph.org/?p=119380)). ray's form, which tracks only $|z'|^2$, is Inigo Quilez's: see his [3D Julia sets](https://iquilezles.org/articles/juliasets3d/) article and the Shadertoy [Quaternion Julia](https://www.shadertoy.com/view/MsfGRr) (2013, a port of his 2007 demo Kindernoiser). ray bails out at $|z|^2 = 16$ where iq uses 4.

**Parameters.** `c` ((-0.2, 0.6, 0.2, 0.2)): the constant quaternion (x, y, z, w), x being the real part. Small changes reshape the set completely, and constants too far from the origin break it into scattered pieces. `iterations` (10): escape-time steps, 1 to 32; more give finer detail. Iterations are not added close up.

### Sierpinski Tetrahedron (`sierpinski`)

![Sierpinski Tetrahedron](images/fractals/sierpinski.jpg)

The 3D analog of Sierpiński's triangle: a tetrahedron made of four half-size copies of itself, each made of four more, and so on, so that it is mostly air. ray builds it by folding space and adds levels by itself close up.

**Formula.** Start from $q = p$ and repeat `iterations` times: reflect in three of the tetrahedron's mirror planes,

$$\text{if } q_x + q_y < 0:\ (q_x, q_y) \leftarrow (-q_y, -q_x); \quad \text{if } q_x + q_z < 0:\ (q_x, q_z) \leftarrow (-q_z, -q_x); \quad \text{if } q_y + q_z < 0:\ (q_y, q_z) \leftarrow (-q_z, -q_y)$$

which brings points toward the copy at the vertex $(1, 1, 1)$, then scale by 2 about that vertex: $q \leftarrow 2q - (1, 1, 1)$. After $n$ steps, measure the base tetrahedron with vertices $(1,1,1)$, $(-1,-1,1)$, $(-1,1,-1)$, $(1,-1,-1)$ by its four face planes, and scale back:

$$d = \frac{\max\bigl(|q_x + q_y| - q_z,\ |q_x - q_y| + q_z\bigr) - 1}{\sqrt3}\; 2^{-n}$$

Orbit trap: the share of the reflections that were taken, relative to `iterations`.

**Origin.** Wacław Sierpiński described the triangle in 1915 ([Sierpiński triangle](https://en.wikipedia.org/wiki/Sierpi%C5%84ski_triangle)); the tetrahedron, sometimes called the tetrix, is its standard 3D analog. The folding construction, with exactly these three folds and a scale of 2 about $(1, 1, 1)$, is Knighty's: it is the first algorithm in his post [Kaleidoscopic (escape time) IFS](https://www.fractalforums.com/sierpinski-gasket/kaleidoscopic-(escape-time-ifs)/) on Fractal Forums (1 May 2010), and Mikael Hvidtfeldt Christensen explains it in [Distance Estimated 3D Fractals (III): Folding Space](https://blog.hvidtfeldts.net/index.php/2011/08/distance-estimated-3d-fractals-iii-folding-space/) (2011). Knighty measured the result as $(|q| - 2)\,2^{-n}$, a ball; ray measures the base tetrahedron instead, so the finest copies are sharp tetrahedra.

**Parameters.** `iterations` (8): levels of subdivision, 1 to 20; each halves the size of the smallest tetrahedra. Close up, ray adds levels as the pixels can resolve them, up to 24.

### Apollonian (`apollonian`)

![Apollonian](images/fractals/apollonian.jpg)

Inigo Quilez's Apollonian fractal: a lattice of spheres in which every gap holds smaller spheres, and every gap between those smaller ones still, like foam or coral. It fills all of space, so the Add example cuts a 2 m cube out of it; try the orbit texture on it.

**Formula.** Start from $q = p$ and $S = 1$, and repeat `iterations` times:

$$q \leftarrow 2\,\mathrm{fract}\!\left(\tfrac12 q + \tfrac12\right) - 1 \quad \text{(wrap into the cell } [-1, 1)^3\text{)}$$

$$k = f / |q|^2, \qquad q \leftarrow k\,q, \qquad S \leftarrow k\,S$$

with $f$ the `fold`: the second line inverts $q$ in the sphere of radius $\sqrt f$ about the cell's center. Distance estimate: $d = \tfrac14\,|q_y| / S$, the distance to the plane $q_y = 0$ through the result, scaled back; the inversions bend that plane into the spheres. Orbit trap: the smallest $|q|^2$ after the wraps, times 6.

**Origin.** The name comes from Apollonius of Perga's problem of circles tangent to three others, whose repeated solution packs circles inside circles ([Apollonian gasket](https://en.wikipedia.org/wiki/Apollonian_gasket)); spheres pack the same way in 3D ([Apollonian sphere packing](https://en.wikipedia.org/wiki/Apollonian_sphere_packing)). This particular formula, repeated cell wrapping and sphere inversion measured against a plane, is from Inigo Quilez's Shadertoy [Apollonian](https://www.shadertoy.com/view/4ds3zn) (2013), which he calls the "classic standard apollonian/kleinian fractal" and says he can't recall where he learned it. ray drops iq's lower limit on the inversion factor ($\max(f/|q|^2, 0.1)$) and exposes iq's $s$ as `fold`.

**Parameters.** `fold` (1.1): the inversion's strength, 0.5 to 2. Above 1 the spheres swell and merge into a sponge; below 1 they thin into a lace. `iterations` (8): levels of packing, 1 to 20; more add smaller spheres between the big ones. Iterations are not added close up.

### Kaleidoscopic IFS (`kifs`)

![Kaleidoscopic IFS](images/fractals/kifs.jpg)

Knighty's kaleidoscopic IFS in its Menger form: space is folded into one mirror wedge of the cube, turned, and scaled away from an offset, again and again. With no turn it is exactly the Menger sponge; the Add example's small turns (12° and 8°) twist it into crusty towers and alien machinery.

**Formula.** Start from $q = p$ and repeat `iterations` times:

1. Fold into the cube's fundamental wedge: $q \leftarrow (|q_x|, |q_y|, |q_z|)$, then sort the components so that $q_x \ge q_y \ge q_z$.
2. Turn: $q \leftarrow R\,q$, with $R = R_z(\gamma)\,R_y(\beta)\,R_x(\alpha)$ built from `angles` $= (\alpha, \beta, \gamma)$.
3. Scale about the offset $o$: $q \leftarrow s\,q - (s - 1)\,o$, and then, if $q_z < -\tfrac12 (s - 1)\,o_z$, $q_z \leftarrow q_z + (s - 1)\,o_z$.

The last test keeps a second copy along $z$: with $s = 3$, $o = (1, 1, 1)$ and no turn, the copies centered at $(\tfrac23, \tfrac23, \tfrac23)$ and $(\tfrac23, \tfrac23, 0)$ become, by symmetry, the Menger sponge's 8 corner and 12 edge cubes. After $n$ steps, measure the cube $[-1, 1]^3$ and scale back:

$$d = \mathrm{sdBox}(q, 1)\; s^{-n}$$

Orbit trap: the smallest $|q| / s^k$ along the orbit (the nearest it comes to the center in the shape's own units), times 1.5.

**Origin.** Knighty introduced kaleidoscopic IFS on Fractal Forums on 1 May 2010, in the thread [Kaleidoscopic (escape time) IFS](https://www.fractalforums.com/sierpinski-gasket/kaleidoscopic-(escape-time-ifs)/): fold space by the symmetry planes of a polyhedron, rotate before or after the folds, and stretch about a center. ray's kernel is his `Menger3` from that first post, whose $z$ test above is written there as $z \leftarrow s z$, then $z \leftarrow z - (s-1)\,o_z$ if $z > \tfrac12 (s - 1)\,o_z$ (the same thing). ray keeps one rotation, in Knighty's second slot (after the folds), and measures a box where Knighty used $(|q| - 2)\,s^{-n}$. See also Mikael Hvidtfeldt Christensen's [Folding Space II: Kaleidoscopic Fractals](https://blog.hvidtfeldts.net/index.php/2010/06/folding-space-ii-kaleidoscopic-fractals/) (2010).

**Parameters.** `fold_scale` (3): how much each step scales, 1.2 to 6; 3 is the Menger sponge, and 2 to 4 is typical. `offset` ((1, 1, 1)): the point each step scales away from, which sets where the copies sit. `angles` ((0, 0, 0); the Add example uses (12, 8, 0)): degrees turned about x, then y, then z at every step; 0 is the Menger sponge, and a few degrees twist it into something new. `iterations` (6): steps, 1 to 12, at a linear cost. Close up, ray adds steps as the pixels can resolve them, up to 24.

### Pseudo-Kleinian (`kleinian`)

![Pseudo-Kleinian](images/fractals/kleinian.jpg)

Knighty's pseudo-Kleinian: box folds and sphere inversions, like a Mandelbox of scale 1, whose limit set is a lace of rings, arches and tunnels, a cathedral of coral that resembles the limit sets of Kleinian groups. It fills all of space, so the Add example cuts a 2 m cube out of it; it is at its best seen from inside (as here).

**Formula.** Start from $q = p$ and $T = 1$, and repeat `iterations` times:

$$q \leftarrow 2\,\mathrm{clamp}(q, -b, b) - q, \qquad k = \max\!\bigl(r / |q|^2,\ 1\bigr), \qquad q \leftarrow k\,q, \qquad T \leftarrow k\,T$$

with $b$ the `fold_box` (a box fold with a different half size on each axis) and $r$ the `inversion`: points inside the sphere of radius $\sqrt r$ are inverted in it, and the rest are left alone. Then measure a ring-shaped set and scale back: with $\rho = \sqrt{q_x^2 + q_y^2}$,

$$d = \max\!\Bigl(\rho - 0.92784,\ \frac{|\rho\, q_z|}{|q|}\Bigr) \Big/ T$$

Orbit trap: the smallest $|q|$ just after the box folds.

**Origin.** Theli-at found it while experimenting with the Amazing Box (the Mandelbox) and its folding parameters in Mandelbulb3D, and posted [Kleinian drops](https://www.deviantart.com/theli-at/art/Kleinian-drops-192676501) on DeviantArt on 8 January 2011, noting its likeness to Kleinian groups ([Kleinian group](https://en.wikipedia.org/wiki/Kleinian_group)). Knighty reduced it to a standalone formula, a scale-1 Julia Mandelbox plus a final shape, and shared it as a Fragmentarium script in May 2011 ([Fractal Forums](https://www.fractalforums.com/3d-fractal-generation/fragmentarium-an-ide-for-exploring-3d-fractals-and-other-systems-on-the-gpu/msg32270/#msg32270)). It ships with Fragmentarium as [`PseudoKleinian.frag`](https://github.com/Syntopia/Fragmentarium/blob/master/Fragmentarium-Source/Examples/Knighty%20Collection/PseudoKleinian.frag), whose "P3" preset is ray's default fold box with inversion 1. The final ring distance, with its 0.92784, is from eiffie's Shadertoy [Pseudo Knightyan](https://www.shadertoy.com/view/lls3Wf); it gives lacier rings than the thin "thingy" (thickness 0.01) in Knighty's script.

**Parameters.** `fold_box` ((0.92436, 0.90756, 0.92436)): half sizes of the box fold along x, y and z; small changes reshape the tunnels. `inversion` (1): the inversion sphere's radius squared, 0.1 to 4; larger opens the structure up. `iterations` (8): steps, 1 to 20; more give finer detail. Iterations are not added close up.

## Box folds

Mandelbox relatives: space folded back into a box and inverted in a sphere, again and again. The best of them for walking through: halls, vaults and corridors.

### Amazing Surf (`amazing_surf`)

![Amazing Surf](images/fractals/amazing_surf.jpg)

A Mandelbox that box-folds only x and y, so z is never folded back: unturned, the set is a flat fractal sheet with the 2D Mandelbox's pattern of holes. A quarter turn about y at every step folds that sheet into a leaning tower of thin plates pierced with round windows, with narrow slots and corridors between the plates.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow R\,m\,\mathrm{boxFold}_{xy}(z) + (p_y,\ p_x,\ p_z), \qquad m = \frac{s}{\mathrm{clamp}(r^2,\ r_{\min}^2,\ 1)}$$

with the box fold $f(x) = |x+F| - |x-F| - x$ on $x$ and $y$ only ($z$ passes through), $r^2$ the folded point's squared length ($x^2 + y^2$ only with `cylinder`), $R$ the turn by `angles`, and the point added with $x$ and $y$ swapped, as Mandelbulb 3D does. Orbits stop past radius 100.
Distance estimate: $d = |z| / \partial z - t$ with $\partial z \leftarrow |m|\,\partial z + 1$, where $t$ is `thickness`: the bare set is a surface, and the thickness turns it into plates.

**Origin.** Proposed by Kali on Fractal Forums around 2011, when he was exploring 2D Mandelboxes and a rotated "Mandelbox surface" (the forum's board for these is named [Amazing Box, Amazing Surf and variations](https://www.fractalforums.com/amazing-box-amazing-surf-and-variations/)). DarkBeam added features for it as a Mandelbulb 3D formula ([custom formulas thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/)), and Mandelbulber 2 adopted it from there, with variants by mclarekin (Graeme McLaren). Ported from Mandelbulber 2's [amazing_surf.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/amazing_surf.cl), with its x/y swap of the added point. The thickness is this port's addition.

**Parameters.** `fold` (1, 1): where the box fold turns back on x and y; larger widens the plates and the tower, and unequal values stretch it. `fold_scale` (2): the scale after each fold; 1.6 to 2.5 is typical, lower spreads the plates apart and grows the tower, negative values crumple it (avoid -1 to 1). `min_radius` (0.84): radius of the sphere fold's inner sphere; smaller cuts deeper round windows (Mandelbulber's minimum radius of 0.5 is 0.84 here, since it compares that value's square root with $r^2$). `angles` (0, 90, 0): degrees turned about x, y and z at every step, which is what lifts the sheet into 3D; quarter turns stack plates, 0 leaves the flat sheet, and odd angles give rocky lumps. `cylinder` (0): 1 measures the sphere fold's radius on x and y only (Mandelbulber's cylinder fold), turning round windows into slots. `thickness` (0.02): thickens the sheets, in the shape's units; 0 leaves the bare surface, which renders as fuzz, and 0.01 to 0.05 gives solid plates. `iterations` (10): more gives finer plates and windows; past about 14 the plates break into dust unless the thickness grows.

### Kalibox (`kalibox`)

![Kalibox](images/fractals/kalibox.jpg)

Kali's Julia-set Mandelbox, where the box fold becomes an absolute value and a shift: every step mirrors space into one octant and moves it before the sphere fold and the scale. Its outside is usually a closed box with flat faces; these defaults open two sides of it into a gothic pillar of vaulted halls with ornamented walls, chambers stacked inside it.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow s\,\mathrm{sphereFold}(|R_y z| + T) + J + w\,p$$

with $|\cdot|$ taken per component, $T$ the shift (`trans`), $J$ the Julia constant, $w$ the weight of the point (`point`), $R_y$ the turn about y (`turn`), and the sphere fold scaling by $1/r_{\min}^2$ inside the inner sphere, by $1/r^2$ between it and the unit sphere, and not at all outside. Orbits stop past radius 100.
Distance estimate (Rrrola's, as in Kali's Fragmentarium code): $d = (|z| - |s - 1|) / \partial z - |s|^{1-n}$ with $\partial z \leftarrow |s|\,k\,\partial z + 1$, where $k$ is the sphere fold's factor and $n$ the iterations.

**Origin.** By Kali (Fractal Forums). His Kalibox.frag ships with Syntopia's Fragmentarium among "Kali's Creations" since [version 0.9.12 "Prague"](https://blog.hvidtfeldt.net/index.php/2012/08/fragmentarium-version-0-9-12-prague-released/) (2012); the same fold, written as $T - |z|$ with the point added instead of a constant, is Mandelbulb 3D's ABoxModKali ([Fractal Forums](http://www.fractalforums.com/new-theories-and-research/aboxmodkali-the-2d-version/)), Mandelbulber 2's [abox_mod_kali.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/abox_mod_kali.cl). Ported from Kalibox.frag; `point` blends in the Mandelbulb 3D form. The defaults are new, chosen for their halls; Kali's own preset (`fold_scale` 2.04, `min_radius` 0.59, `trans` (0.04, -1.86, 0.04), `julia` (-0.67, -1.30, -0.46)) is a thin tower about 10 units tall.

**Parameters.** `fold_scale` (-1.54): the scale after each fold; negative values (-1.5 to -2) give vaulted halls, positive ones (1.5 to 2.2) boxy cities (avoid -1 to 1). `min_radius` (0.56): radius of the sphere fold's inner sphere; smaller makes deeper niches and more ornament. `trans` (-0.81, -0.85, -0.92): the shift after the abs fold, per axis; it sets where the halls open and how large they are, and small changes move whole walls. `julia` (-0.55, -0.3, -0.8): the constant added at every step, per axis; it decides which faces of the box stay closed and which open into halls. `point` (0): how much of the point itself is added; 0 is Kali's Julia form, 1 the ABoxModKali Mandelbrot form (which outgrows the bound with these defaults). `turn` (0): degrees turned about y before each fold; a few degrees twist the halls, larger turns grow a hairy, rocky skin. `iterations` (14): more gives finer ornament, at linear cost.

### Mandalay (`mandalay`)

![Mandalay](images/fractals/mandalay.jpg)

A Mandelbox whose box fold is DarkBeam's Mandalay fold, which carves every face into stepped frames instead of mirroring it. The set grows into a cathedral: ornamented panels and buttresses outside, and inside, chambers with vaults, arches and framed windows to the sky.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow s\,\mathrm{sphereFold}(\mathrm{mandalay}(z)) + w\,p + J$$

where the Mandalay fold works on $a = |z_i|$ for each axis, with $o$ the larger of the other two coordinates, $f$ the fold size and $g$ the offset:

$$\mathrm{mandalay}(a) = \min\big(\max(|a - f| - f,\ o - 4f),\ \max(a - 2f - g,\ o),\ a\big)$$

then gives each coordinate its sign back. The sphere fold scales by $1/r_{\min}^2$ inside the inner sphere and by $1/r^2$ up to the unit sphere. Orbits stop past radius 100.
Distance estimate: $d = |z| / \partial z$ with $\partial z \leftarrow |s|\,k\,\partial z + 1$, $k$ the sphere fold's factor.

**Origin.** The fold is DarkBeam's, posted on Fractal Forums in the thread ["'New' fractal type: Mandalay"](http://www.fractalforums.com/amazing-box-amazing-surf-and-variations/'new'-fractal-type-mandalay/msg81348/#msg81348), in the form Knighty adapted it to. Mandelbulber 2 has it as Mandalay Box V1 and V2 and as Mandalay KIFS. Ported from Mandelbulber 2's [mandalay_box_v1.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/mandalay_box_v1.cl), with its three swaps written out as maxima; the Julia constant and the weight of the point are Mandelbulber's Julia mode.

**Parameters.** `fold_scale` (1.8): the scale after each fold; 1.5 to 2.2 gives the cathedral, higher values break it into separate towers, and negative ones a larger, crumpled palace (avoid -1 to 1). `min_radius` (0.5): radius of the sphere fold's inner sphere; smaller gives deeper vaults and more arches, and near 0.75 the facade turns into plain stacked blocks. `fold` (0.5, 0.5, 0.5): the Mandalay fold's size per axis, the width of the frames carved into each face; unequal values make a tower or a long hall. `offset` (0, 0, 0): shifts where each axis's fold steps back; small values (±0.5) add balconies and recesses. `julia` (0, 0, 0): a constant added at every step; with `point` at 0 it is a Julia set (Mandelbulber's example uses (-0.9, 0.7, 0) with `fold_scale` 1.6). `point` (1): how much of the point itself is added; 1 is the Mandelbrot form, the cathedral, 0 the Julia form. `iterations` (12): more gives finer ornament, at linear cost.

### Kaliset (`kaliset`)

![Kaliset](images/fractals/kaliset.jpg)

Kali's Kaliset in 3D: mirror space into one octant, invert it in the unit sphere, add a constant, and repeat. Everything far away lands near the constant, so the set fills space; what shapes it are the holes, the points whose orbit falls close to the center, and cutting them away leaves a lace of domed chambers, round windows and bony struts. Cut to a box here; fly inside it.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow R\left(\frac{k\,|z|}{|z|^2} + c + w\,p\right)$$

with $|z|$ per component in the numerator, $k$ the inversion's radius² (`inversion`), $c$ the constant (`julia`), $w$ the weight of the point, and $R$ the turn by `angles`. A point is cut away when its orbit comes within $h$ (`hole`) of the center at any step.
Distance estimate: the depth into the deepest such hole, $d = \max_i\,(h - |z_i|) / \partial z_i$ with $\partial z \leftarrow (k / |z|^2)\,\partial z + w$; it is negative in the solid.

**Origin.** Kali posted the formula, in 2D, on Fractal Forums on May 1, 2011 as a ["Very simple formula for fractal patterns"](https://www.fractalforums.com/new-theories-and-research/very-simple-formula-for-fractal-patterns/): $x \leftarrow |x|,\ y \leftarrow |y|$, then divide by $x^2 + y^2$ and add $c$. He later named it the Kaliset and took it into 3D, among other uses as coloring and bump maps on other fractals ([Fractal Forums](https://www.fractalforums.com/3d-fractal-generation/kaliset-3d-fractal-used-as-coloring-and-bump-mapping-for-de-systems)). In Mandelbulb 3D it became Kalisets1 (with KaliDucks), in Mandelbulber 2 [kalisets1.cl](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/kalisets1.cl), whose iteration this is. Mandelbulber draws it by escape past a bailout; this port cuts away the bailout's source instead, a ball about the center, which gives the estimate above and clean chamber walls.

**Parameters.** `inversion` (1): radius² of the sphere it inverts in; above 1 swells the chambers, below 1 thins the struts into lace. `julia` (-0.5, -0.5, -0.5): the constant added at every step (the Kaliset's $c$); -0.3 to -1 per axis works, and unequal values break the cubic symmetry. `hole` (0.5): radius of the hole about the center; larger opens bigger chambers, smaller leaves a solid block pitted with bubbles. `point` (0): how much of the point itself is added at every step; 0 is the Julia form. `angles` (0, 0, 0): degrees turned about x, y and z at every step; a few degrees twist the chambers, 0 keeps the cubic symmetry. `iterations` (8): each cuts away the next, smaller generation of holes; past 10 the walls are pitted with dust.

### Bulb Box (`bulbbox`)

![Bulb Box](images/fractals/bulbbox.jpg)

A Mandelbox and a Mandelbulb in one step: the box and sphere folds and a scale, then the power 2 bulb's squaring. The result is a craggy floating block of rippled ledges, caves and bulb-like knots, with a pattern of flowers on its top.

**Formula.** Start from $z = p$ and repeat: fold each coordinate at $F$, $x \leftarrow 2F\operatorname{sign}(x) - x$ where $|x| > F$. Then sphere fold with $k = 1/m^2$ if $|z|^2 < m^2$, $1/|z|^2$ if $|z|^2 < 1$, else 1, and scale: $w = 2 s k\, z$. Then square as the power 2 Mandelbulb does, with $\rho = \sqrt{w_x^2 + w_z^2}$:

$$z \leftarrow \left(\frac{2w_y(w_x^2 - w_z^2)}{\rho},\ Z\,(w_y^2 - \rho^2),\ \frac{4w_x w_y w_z}{\rho}\right) + p.$$

Distance estimate: $d = \tfrac12|z|\ln(4s^2|z|)/\partial$ with $\partial \leftarrow 2sk \cdot 2|Q|/|w| \cdot \partial + 1$, where $Q$ is the squared point. The $4s^2$ makes the far-field potential square exactly, so the estimate agrees whichever step the orbit escapes at.

**Origin.** After Mandelbulber's Box Fold Bulb Pow 2 (`box_fold_bulb_pow2.cl`, FoldingIntPow2 in Mandelbulber 1), with two changes. The scale between the folds and the squaring is a free parameter, as in its `box_fold_bulb_pow2_v2.cl`. With the original's scale of 1, nearly every orbit escapes and the fractal is dust. And the squaring is the bulb's trigonometric form (polar angle and azimuth doubled) instead of White's triplex (latitude doubled). The triplex sends the whole vertical axis to the origin, which leaves needles along it and a distance estimate too unreliable to march.

**Parameters.** `fold` (1): the box fold's limit $F$, which sets the block's size; larger values make a bigger block with more ledges. `min_radius` (0.5): the sphere fold's inner radius $m$, which reshapes the fine detail in the hollows more than the overall form. `fold_scale` (0.6): the scale $s$ between the folds and the squaring. 0.5 makes a solid craggy block, 0.6 carves caves into it, and from about 0.7 it thins toward dust. `z_factor` (1): stretches the squaring's vertical axis ($Z$). Below 1 the walls flatten into ledges, above it they thin into sheets and threads; the original's default is 5. `iterations` (10): more give finer detail.

## Bulbs

The Mandelbulb's family: power maps in spherical coordinates, with other constants, signs, projections and twists.

### Juliabulb (`juliabulb`)

![Juliabulb](images/fractals/juliabulb.jpg)

The Mandelbulb's Julia sets: the bulb's power-8 map with a fixed constant added at every step in place of the point. Every constant sculpts something different; the default stacks knobbed, ringed tiers studded with small rosettes, like an ornate urn.

**Formula.** Start from $\mathbf z = p$ and repeat, in spherical coordinates about the vertical axis ($r = |\mathbf z|$, $\theta = \operatorname{atan2}(\sqrt{x^2+z^2},\, y)$, $\phi = \operatorname{atan2}(z, x)$):

$$\mathbf z \leftarrow r^n\,(\sin n\theta\,\cos n\phi,\ \cos n\theta,\ \sin n\theta\,\sin n\phi) + c$$

until $r \ge 2$. Distance estimate: $d = \tfrac12\, r \ln r / r'$, with $r' \leftarrow n\, r^{n-1}\, r'$ from $r' = 1$ (no $+1$: the constant does not depend on the point).

**Origin.** Julia sets arrived with the Mandelbulb itself. Daniel White published his "triplex" squaring formula on Fractal Forums in November 2007, and Paul Nylander rendered its Julia set, a "3D Juliabulb", in July 2009, along with a 4th-order one, a month before his 8th-power Mandelbulb. Krzysztof Marczak (who wrote Mandelbulber), David Makin, Tom Beddard and Iñigo Quílez followed with Juliabulbs of other powers and with animations ([Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/), [White's Mandelbulb history](https://www.skytopia.com/project/fractal/mandelbulb.html)). This kernel is ray's built-in Mandelbulb with $c$ held fixed.

**Parameters.** `power` (8): the exponent, 2 to 16; 8 is the classic bulb's, and higher powers give more, finer lobes. `c` ([0.2, -0.64, 0.2]): the constant added at every step. Small changes reshape the set completely: near the origin it is a lumpy ball, with a large vertical part (like the default) it stacks into ringed tiers, and much past 0.8 from the origin it breaks into islands, where the estimate can overshoot in the gaps (lower the node's `step` if holes appear). `iterations` (10, up to 32): finer detail, at a pow, two atan2, a sine and a cosine per point each.

### Mandelbar (`mandelbar`)

![Mandelbar](images/fractals/mandelbar.jpg)

The Tricorn grown into 3D: the slice facing you is the three-cornered Mandelbar set, the level slice through it is the Mandelbrot set, and between them the surface sweeps round in smooth, swirled folds, with a beaded needle running out to the left.

**Formula.** Take $x$ as the real part and $(y, z)$ as one imaginary direction, conjugate $y$, and raise to the $n$th power as a complex number: with $\rho = \sqrt{y^2+z^2}$ and $w = (x + i\rho)^n$,

$$\mathbf z \leftarrow \Bigl(\operatorname{Re} w,\ -\operatorname{Im} w\,\frac{y}{\rho},\ \operatorname{Im} w\,\frac{z}{\rho}\Bigr) + c,$$

starting from $\mathbf z = c = p$. For $n = 2$ this is $(x^2-y^2-z^2,\ -2xy,\ 2xz) + c$: the $xy$ plane iterates $\bar z^2 + c$ (the Tricorn) and the $xz$ plane $z^2 + c$ (the Mandelbrot set). Since $|w| = r^n$, the distance estimate is $d = \tfrac12\,r\ln r/r'$ with $r' \leftarrow n\,r^{n-1}r' + 1$, bailing out at $r = 256$. For $n = 2$ the set is shifted 0.5 along $x$ to center it; the higher powers are centered already.

**Origin.** The Mandelbar set $\bar z^2 + c$ was first studied by W. D. Crowe, R. Hasson, P. J. Rippon and P. E. D. Strain-Clark ("On the structure of the Mandelbar set", *Nonlinearity* 2, 1989) and is better known as the Tricorn; its higher powers are the multicorns. This 3D blend of the Mandelbrot and Mandelbar sets is credited by Paul Nylander to Eric Baird; Nylander rendered it as the "3D Mandelbrot / Mandelbar Set" in July 2009 with $(x^2-y^2-z^2,\ 2xy,\ -2xz)$, the same formula with $y$ and $z$ exchanged ([Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/)). Ported from Mandelbulber 2's Mandelbar (`mandelbar.cl`), whose axis arrangement it keeps; the higher powers are this version's extension.

**Parameters.** `power` (2): whole, 2 to 8. 2 is the Tricorn; higher powers give the multicorns, whose upright slice has power + 1 corners and whose level slice is the Multibrot set of that power. `iterations` (12, up to 32): finer bulbs along the needle and the arms, at a few multiplications per power per point each.

### Burning Ship (`burning_ship`)

![Burning Ship](images/fractals/burning_ship.jpg)

The Burning Ship in 3D: seen from the front, its silhouette is the familiar ship, masts and flames on top and the keel's needle trailing to the left, while in depth the hull rounds out into a swirled, glassy bowl.

**Formula.** Take the absolute value of every component, then apply the Mandelbulb's power map in its latitude form, about a pole running front to back ($r = |\mathbf z|$, latitude $\theta = \operatorname{atan2}(z, \sqrt{x^2+y^2})$, longitude $\phi = \operatorname{atan2}(y, x)$, with $x, y, z$ the absolute values of the components):

$$\mathbf z \leftarrow r^n\,(\cos n\theta\,\cos n\phi,\ \cos n\theta\,\sin n\phi,\ \sin n\theta) + c$$

from $\mathbf z = c$, where $c$ is the point with $y$ flipped (so the masts point up, as the 2D ship is always shown) and, for $n = 2$, shifted to center the hull. In the plane $z = 0$ this is exactly $(|x| + i|y|)^2 + c$. With `polar` set, the map works in polar angles about the $x$ axis instead ($\theta = \operatorname{atan2}(\sqrt{y^2+z^2}, x)$, $\phi = \operatorname{atan2}(z, y)$, $\mathbf z \leftarrow r^n(\cos n\theta,\ \sin n\theta\cos n\phi,\ \sin n\theta\sin n\phi) + c$), which leaves the same 2D ship in the plane $z = 0$. Distance estimate: $d = \tfrac12\,r\ln r/r'$ with $r' \leftarrow n\,r^{n-1}r' + 1$ (absolute values are reflections, which keep lengths). The latitude map jumps across its pole, so the estimate overshoots near it, and the march takes 0.6 of each step.

**Origin.** The Burning Ship, $(|\operatorname{Re} z| + i\,|\operatorname{Im} z|)^2 + c$, was introduced by Michael Michelitsch and Otto E. Rössler ("The 'Burning Ship' and its quasi-Julia sets", *Computers & Graphics* 16(4), 1992). Its 3D form, the standard Mandelbulb with absolute values taken on all three axes before the power, was posted with a Fragmentarium script by laser blaster on Fractal Forums in October 2014 (["3D Burning Ship formula and .frag file"](https://www.fractalforums.com/fragmentarium/3d-burning-ship-formula-and-frag-file/)); Mandelbulber 2 offers the same as an option of its Mandelbulb Abs formula (`mandelbulb_abs.cl`). The polar variant is this port's addition: its map is continuous at the pole, so its estimate is close to exact and its hull comes out crisper and more crystalline.

**Parameters.** `power` (2): the exponent, 2 to 8. 2 is the Burning Ship; 3 and 4 fold it into a rounder hull with more points, and higher powers into a knobbly ball. `polar` (0): 0 is the original latitude form; 1 the polar form, crisper and with a truer estimate. `iterations` (12, up to 32): finer spires and smaller ships along the keel, at a pow, two atan2, a sine and a cosine per point each.

### Quadratic Mandelbulb (`power2_bulb`)

![Quadratic Mandelbulb](images/fractals/power2_bulb.jpg)

The first Mandelbulb, before the power went up to 8: the slice facing you is exactly the Mandelbrot set, and in depth it swells into a smeared, "whipped cream" body with a whorl at its heart. Shown here from behind, so the needle points right.

**Formula.** Daniel White's triplex square, in its non-trigonometric form: with $\rho^2 = x^2 + y^2$,

$$\mathbf z \leftarrow \Bigl((x^2-y^2)\bigl(1 - \tfrac{z^2}{\rho^2}\bigr),\ 2xy\bigl(1 - \tfrac{z^2}{\rho^2}\bigr),\ -2z\rho\Bigr) + c,$$

which is $r^2(\cos 2\phi\cos 2\theta,\ \sin 2\phi\cos 2\theta,\ -\sin 2\theta)$ for longitude $\phi$ and latitude $\theta$, from $\mathbf z = c = p$ (shifted 0.62 along $x$ to center it). The $z = 0$ plane is the Mandelbrot set. Because $|\mathbf z^2| = |\mathbf z|^2$, the distance estimate is the analytic $d = \tfrac12\,r\ln r/r'$ with $r' \leftarrow 2r\,r' + 1$. On the pole axis ($\rho = 0$), where the map has no limit, the kernel takes the one along $y = 0$. With `julia` set, $\mathbf z$ starts at $p$, $c$ is the fixed constant, and $r' \leftarrow 2r\,r'$.

**Origin.** Daniel White posted his "triplex" squaring formula, two rotations in spherical coordinates, to Fractal Forums in November 2007 (["True 3D mandelbrot type fractal"](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/)), and Thomas Ludwig was the first to render the power-2 object in 3D that same month. Paul Nylander rendered it as the "Quadratic 3D Mandelbulb Set" in July 2009, judging it the best-looking quadratic 3D Mandelbrot set so far, and a month later raised the power to 8, which gave the Mandelbulb its finer detail. The minus sign on the $z$ term is White's original; Nylander's later positive-$z$ variant (October 2009) is more consistent algebraically but gives a flat-looking quadratic set. Nylander also rendered this formula's Julia sets in July 2009, the first Juliabulbs. Sources: [White's Mandelbulb history](https://www.skytopia.com/project/fractal/mandelbulb.html), [Nylander's hypercomplex fractals](http://www.bugman123.com/Hypercomplex/), and Mandelbulber 2's `mandelbulb_power2.cl`.

**Parameters.** `julia` (0): 0 renders the bulb; 1 its Julia set for the constant `c`. `c` ([-0.12, 0.74, 0]): the Julia constant, used when `julia` is 1. With a $z$ of 0 the slice facing you is that 2D Julia set (the default is the Douady rabbit's), puffed into 3D around it. `iterations` (12, up to 32): finer bulbs and smears, at a few multiplications and a square root per point each.

### Benesi (`benesi`)

![Benesi](images/fractals/benesi.jpg)

Matt Benesi's power-2 bulb, stood on end: a draped, fluted spire with a needle on top and a skirt of ridges below. It is the Mandelbrot set spun about its real axis, but the sign flip on one side and the squared cross-plane pull it into long folds rather than the smooth "whipped cream" of a plain power-2 bulb.

**Formula.** In Mandelbulber's axes (its $x$ is the spire's axis, pointing down here), start from $z = p$ and repeat:

$$x \leftarrow \pm\left(x^2 - \rho^2\right) + o_x + c_x, \qquad (y, z) \leftarrow -\frac{2|x|}{\rho}\left(y^2 - z^2,\; 2yz\right) + (o_y, o_z) + (c_y, c_z)$$

with $\rho = \sqrt{y^2 + z^2}$, the minus sign taken where $c_x \ge 0$ and $x \ge \rho$, the offset $o$, and $c$ the point itself (Mandelbrot type) or the constant `c` (Julia type), blended by `julia`. The Mandelbrot-type set is shifted to sit centered. Distance estimate: $d = \tfrac12 |z| \ln|z| / \partial z$ with $\partial z \leftarrow 2|z|\,\partial z + 1$; the orbit trap is the nearest the orbit comes to the axis.

**Origin.** Matt Benesi ("M Benesi" on Fractal Forums) devised a long run of 3D $z^2$ formulas from 2010 on, starting with [3d Fractal Type that works for z^2](https://www.fractalforums.com/3d-fractal-generation/3d-fractal-type-that-works-for-z2/) (February 2010), and later the magnitude transforms of his "pine tree" ([Fractal Forums, 2015](https://www.fractalforums.com/new-theories-and-research/do-m3d-formula-have-to-be-distance-estimation-formulas/)). This one is Mandelbulber 2's `benesi.cl` ("Benesi - Pwr2"), which credits him; the Julia constant is the one in Mandelbulber's example file `benesi 001.fract`. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `julia` (0): 0 is the Mandelbrot-type set, 1 the Julia set of `c`, and values in between morph one into the other. `c` (0.85, -0.34, 0.41): the Julia constant, used as `julia` rises; small changes reshape it completely. `offset` (0, 0, 0): added after each square; $x$ stretches or pinches the folds along the axis and $y$, $z$ skew them (-0.5 to 0.5). `iterations` (20): more give finer detail cheaply, since there is no trigonometry.

### Kosalos (`kosalos`)

![Kosalos](images/fractals/kosalos.jpg)

A power-9 bulb whose polar angle is squashed the more, the less the orbit moved on its last step. The first step moves least, so it is squashed most, and the bulb stacks into tiers like a pagoda under a star-shaped crown, each tier studded with scaly buds.

**Formula.** With $z$ up, start from $z = z_{\text{prev}} = p$ and repeat:

$$t = 1 - \frac{k}{\left|z_{\text{prev}} - z + 0.001\,(1,1,1)\right| + o}, \qquad \theta = \operatorname{atan2}\!\left(t\sqrt{x^2+y^2},\, z\right), \qquad \varphi = \operatorname{atan2}(y, x)$$

$$z_{\text{prev}} \leftarrow z, \qquad z \leftarrow z + r^n\left(\sin n\theta\cos n\varphi,\; \sin n\theta\sin n\varphi,\; \cos n\theta\right) + (0, 0, \ell)$$

with $k$ and $o$ the `tweak` and `tweak_offset` in hundredths, $\ell$ the `lift`, and (with `add_point`) $z$ in the sum replaced by $p$. Distance estimate: $d = \tfrac12 r\ln r/\partial z$ with $\partial z \leftarrow (1 + n r^{n-1} s)\,\partial z + 1$, where $s = \max\!\left(1,\; |t|\,r^2/(z^2 + t^2(x^2+y^2))\right)$ is how much the tweak stretches the polar angle. The orbit trap is the nearest the orbit comes to the equator ($|z|/r$).

**Origin.** The formula is Kosalos's, from his post in the fractalforums.org thread [Julia sets and altering the iterate afterwards](https://fractalforums.org/fractal-mathematics-and-new-theories/28/julia-sets-and-altering-the-iterate-afterwards/2871/msg16342), on changing each iterate after the power is taken; Graeme McLaren added it to Mandelbulber 2 as `mandelbulb_kosalos.cl` in August 2019. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2), with the derivative made to allow for the tweak's stretch.

**Parameters.** `power` (9): the exponent and the number of ribs per tier (3 to 16). `tweak` (0.2): how hard a short step squashes the polar angle; 0 is a plain bulb, 0.1 to 0.3 stacks it into tiers, and beyond about ±1 it flattens into a ribbed pumpkin. `tweak_offset` (0): added to the step's length before it divides the tweak; 1 to 5 softens the first step's squash, back toward a plain bulb. `lift` (0): added to the vertical component each step, shifting the lobes along the axis (-1 to 1). `add_point` (0): 0 adds the power to the previous orbit point (Mandelbulber's default), 1 to the starting point as the Mandelbulb does; the difference is subtle. `iterations` (10): more give finer detail.

### Lambdabulb (`lambda_bulb`)

![Lambdabulb](images/fractals/lambda_bulb.jpg)

The logistic map's Julia set in 3D: a twisted, cratered lump with three-fold symmetry about its axis, its rims seamed with chains of tiny buds. Unlike the bulbs around it, it is a Julia set, and it multiplies by its constant instead of adding one.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow c \otimes \left(z - z^{\circ n}\right)$$

where $z^{\circ n} = r^n(\cos n\theta\cos n\varphi,\; \cos n\theta\sin n\varphi,\; \sin n\theta)$ with the latitude $\theta = \arcsin(z/r)$, and $\otimes$ is Nylander's triplex product $a \otimes b = \left(k(a_x b_x - a_y b_y),\; k(a_x b_y + a_y b_x),\; \rho_b a_z + \rho_a b_z\right)$ with $\rho = \sqrt{x^2+y^2}$ and $k = 1 - a_z b_z/(\rho_a\rho_b)$. Distance estimate: $d = \tfrac12 r\ln r/\partial z$ with $\partial z \leftarrow |c|\,(n r^{n-1} + 1)\,\partial z$; the orbit trap is the nearest the orbit comes to the axis. Its example is tipped 50° so its crater faces the camera.

**Origin.** Ron Barnett ("fractalrebel") posted the [LambdaBulb](https://www.fractalforums.com/mandelbulb-renderings/lambdabulb/) on Fractal Forums on November 23, 2009: a 4th-power lambdabulb with seed $1.0625 + 0.2375i$. Paul Nylander rendered it two days later and published the triplex power and product it uses on his [Hypercomplex Fractals](http://bugman123.com/Hypercomplex/) page, crediting Barnett. Graeme McLaren added it to Mandelbulber 2 as `mandelbulb_lambda.cl` in October 2019. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `power` (4): the exponent $n$; 4 is Barnett's, higher powers give more lobes (2 to 12). `c` (1.0625, 0.2375, 0): the constant multiplied in each step, as a triplex number ($x$ and $y$ across its axis, $z$ along it). Its length sets how fat the set is (much under 1 swells it past its bounding box), its angle the twist, and $z$ tips it out of the plane. `iterations` (12): more give finer detail.

### Sine/cosine bulb (`sincos_bulb`)

![Sine/cosine bulb](images/fractals/sincos_bulb.jpg)

A Mandelbulb whose polar angle is measured with acos, asin, or a blend of the two, then put back as a latitude, so sine and cosine trade places. The default blend halves the polar angle's power against the longitude's, which turns the bulb into stacked onion domes draped in long folds, with star-centered buds around them.

**Formula.** With $z$ up, start from $z = p$ and repeat:

$$\theta = n\left((1-m)\arccos\tfrac{z}{r} + m\arcsin\tfrac{z}{r} + \beta\right), \qquad \varphi = n\left(\operatorname{atan2}(y, x) + \alpha\right)$$

$$z \leftarrow r^n\left(\cos\theta\cos\varphi,\; \cos\theta\sin\varphi,\; \sin\theta\right) + o + p, \qquad z_z \leftarrow s\,z_z$$

with the `mix` $m$, the `angles` $(\alpha, \beta)$, the `offset` $o$ and the `z_scale` $s$. Since $\arcsin = \pi/2 - \arccos$, the polar angle's multiplier is $n(1 - 2m)$: 0.25 halves it. Distance estimate: $d = \tfrac12 r\ln r/\partial z$ with $\partial z \leftarrow (n r^{n-1}\partial z + 1)\max(1, s)$; the orbit trap is the nearest the orbit comes to the pole axis ($\sqrt{x^2+y^2}/r$).

**Origin.** Graeme McLaren added `mandelbulb_sin_cos.cl` to Mandelbulber 2 in December 2021, as a variation on the Mandelbulb of Daniel White and Paul Nylander ([Fractal Forums, 2009](https://www.fractalforums.com/3d-fractal-generation/true-3d-mandlebrot-type-fractal/)). With `mix` 0 (pure acos) it differs from the ordinary bulb in detail but looks much like it, so the default is a quarter of the way to asin, which Mandelbulber's blend control also allows. Ported from [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2).

**Parameters.** `power` (9): the exponent (3 to 16); higher powers give more, finer lobes. `mix` (0.25): blends the polar angle from acos (0) to asin (1, the ordinary bulb); 0.15 to 0.35 is the useful range, and at 0.5 the bulb collapses. `z_scale` (1): scales the vertical component after each step; below 1 squashes the domes, above 1 stretches them (0.75 to 1.5). `offset` (0, 0, 0): added after each step, which shifts the orbit and makes the bulb lopsided (-0.5 to 0.5). `angles` (0, 0): degrees added to the longitude and the polar angle before the power, which twist and tilt the lobes. `iterations` (10): more give finer detail.

### Riemann Sphere (`riemann_sphere`)

![Riemann Sphere](images/fractals/riemann_sphere.jpg)

A bell-shaped body covered in a curved grid of studs, with clusters of pyramids around its rim and on its face. Each step folds the point's direction on the Riemann sphere into a grid of cells, and the radius is raised to a power that grows toward the sphere's pole, so the shape is lopsided top to bottom.

**Formula.** Start from $z = p$ and repeat: put the direction on a sphere of radius $\rho$, $u = \rho\,z/|z|$, project it stereographically from the pole $+y$,

$$s = \frac{u_x}{1 - u_y}, \qquad t = \frac{u_z}{1 - u_y}, \qquad k = \min(1 + s^2 + t^2,\ 36),$$

fold the plane with $s' = |\sin(\pi s + a)|$, $t' = |\sin(\pi t + b)|$, and project back with the new radius:

$$z \leftarrow \left(|z|^{2k} - \tfrac14\right)\frac{(2s',\ s'^2 + t'^2 - 1,\ 2t')}{1 + s'^2 + t'^2} + c + m\,p.$$

The exponent $k$ is the projection's own stretch, 1 at the opposite pole and capped at 36 near this one. Distance estimate: $d = \tfrac12 |z| \ln|z| / \partial$ with $\partial \leftarrow 2k\,(|z|^{2k} + \pi/8)\,\partial/|z| + m$. The $\pi/8$ term covers how the angles still stretch where $|z|^{2k}$ is tiny and the $-\tfrac14$ holds the radius up. The escape radius is 1000; the last step's radius is worked out in log space so that it can't overflow.

**Origin.** Posted by msltoe on Fractal Forums on June 27, 2011, opening the thread [Riemann fractals](http://www.fractalforums.com/the-3d-mandelbulb/riemann-fractals/msg33500/). His code added the constant $(0, 0, 1)$ along the pole, which is this formula's default $c$. Ported from Mandelbulber 2's `riemann_sphere_msltoe.cl` (DarkBeam generalized it there as the M3D variant).

**Parameters.** `radius` (1): the sphere the direction is put on before the projection; 1 is the true Riemann sphere, and values down to 0.5 pull the grid of clusters away from the pole and stretch the body into a pointed heart. `offset` ([0, 0]): phases of the two sine folds in radians ($a$, $b$), which slide the grid of clusters across the surface. `c` ([0, 1, 0]): the constant added at every step; along the pole it gives the clustered bell, at zero a plain ball, and sideways values lean and melt it. `mandel` (0): how much of the point itself is added at every step, from msltoe's Julia-like form (0) to a lopsided Mandelbrot-like one (1). `iterations` (8): more give finer pyramids on the pyramids.

### Hobold Riemann Bulb (`riemann_hobold`)

![Hobold Riemann Bulb](images/fractals/riemann_hobold.jpg)

A round, quilted ball caged in a curved lattice of ribs, with a star of ribbed lobes around one pole. It is a Mandelbulb that turns its angles with no trigonometry at all: points on the Riemann sphere are moved by squaring points on two circles. The Add example is turned to show both the cage and the star.

**Formula.** Start from $z = p$ and repeat: put the direction on a sphere of radius $\rho$, $u = \rho\,z/|z|$, and with $v = 1 - u_y$ project it from the pole $(0, 1, 0)$ onto the unit circle in the $xy$ plane:

$$K = \frac{(2u_x v,\ u_x^2 - v^2)}{u_x^2 + v^2}.$$

$K$ sits at the point's central angle. Double that angle $n$ times by complex squaring, $K \leftarrow (-2K_x K_y,\ K_x^2 - K_y^2)$, and take the normal $n_1 = (K_y - 1, -K_x)$ of the plane through the pole at the new angle. Do the same with $u_z$ in place of $u_x$ for a second plane, normal $n_2$. The two planes meet the unit sphere at the new direction $u'$, which has a closed form in $n_1$ and $n_2$ with no division by zero. Then

$$z \leftarrow |z|^{q/2}\,u' + p + c.$$

Optional offsets tweak the normals, and an angle turns them into each other. Distance estimate: $d = \tfrac12|z|\ln|z|/\partial$ with $\partial \leftarrow \tfrac q2 |z|^{q/2-1}\partial + 1$, and an escape radius as large as the power allows.

**Origin.** hobold introduced the "Riemandelettuce", a Mandelbulb in Riemann sphere coordinates, on [Fractal Forums](https://www.fractalforums.com/the-3d-mandelbulb/riemandelettuce) in February 2010. He later reworked it without trigonometry in the fractalforums.org thread "Riemandelettuce without trigonometry". Ported from Mandelbulber 2's `riemann_sphere_hobold_pow8.cl` and `riemann_sphere_hobold_multi.cl`.

**Parameters.** `radius` (0.8): the sphere the direction is put on before the projection; 1 is the exact construction, and Mandelbulber's 0.8 bends it into the ribbed lattice. `offset` ([0, 0]): tweaks to the two planes' normals that skew the lattice. `angle` (0): turns the two normals into each other, in degrees, and twists the lattice. `c` ([0, 0, 0]): a constant added at every step besides the point, which shifts it off its symmetric form. `doublings` (3): how many times the angles are doubled; 3 multiplies them by 8, 2 (by 4) gives a simpler shape with fewer lobes, and 4 (by 16) a finer one. `power` (16): the radius is raised to half this each step, so 16 keeps pace with the eightfold angles and gives the round, quilted ball. Mandelbulber's pow8 uses 8, a spikier ball of hair and long lobes whose distance estimate is rougher (lower the node's `step` if it frays). `iterations` (8): more give finer ribs.

### Hopfbrot (`hopfbrot`)

![Hopfbrot](images/fractals/hopfbrot.jpg)

The Mandelbulb's power map in four dimensions, written in the Hopf coordinates of the 3-sphere and cut by a 3D slice. The plane facing you holds the Mandelbrot set of the same power, the cubic one at the default power 3. Behind that plane the body swells into a ribbed barrel fringed with bulbs, whorled where the Hopf circles wind.

**Formula.** The point is $c = (x + \delta,\ y,\ z\cos\tau - \sigma\sin\tau,\ z\sin\tau + \sigma\cos\tau)$ in four dimensions, where $\tau$ is the slice's tilt, $\sigma$ its offset, and $\delta$ centers the power 2 set. Start from $w = c$ and repeat

$$w \leftarrow |w|^n\,U(n\theta_1,\ n\theta_2,\ n\varphi) + c,$$

with the Hopf parametrization $U(\theta_1, \theta_2, \varphi) = (\cos\varphi\cos\theta_1,\ \cos\varphi\sin\theta_1,\ \sin\varphi\cos\theta_2,\ \sin\varphi\sin\theta_2)$. Here $\theta_1 = \operatorname{atan2}(w_2, w_1)$, $\theta_2 = \operatorname{atan2}(w_4, w_3)$ and $\varphi = \operatorname{atan2}(|(w_3, w_4)|, |(w_1, w_2)|)$. The first two coordinates are the complex plane where the map is $w^n + c$. Distance estimate: $d = \tfrac12|w|\ln|w|/\partial$ with $\partial \leftarrow n|w|^{n-1}\partial + 1$. For odd $n$ the map stretches no direction more than the radius, so the estimate is sound.

**Origin.** Paul Nylander coined the name, as Oliver Knill records in [Mandelbulb, Mandelbrot, Mandelring and Hopfbrot](https://arxiv.org/abs/2305.17848) (arXiv, 2023). Knill defines the degree-$d$ Hopfbrot this way and renders its $z = 0$ slice. See also [MathWorld](https://mathworld.wolfram.com/Hopfbrot.html), and [a 2025 post on it](https://uehapetsijque.substack.com/p/the-hopfbrot) that renders slices volumetrically. This implementation, its slicing and its distance estimate are ray's own.

**Parameters.** `power` (3): the exponent. Odd powers keep the map continuous, since the Hopf circle where $|(w_1, w_2)| = 0$ maps to a single point, so their surfaces are clean. 2 is Knill's choice, with the Mandelbrot set itself in its middle plane, but there the map tears and leaves seams. Higher powers give more, smaller lobes. `tilt` (90): which 3D slice of the four dimensions, as where the depth axis points in the plane of the last two coordinates. 90 is Knill's $z = 0$ cut, 0 the $w = 0$ cut, and values between blend them. `slice` (0): how far the slice sits from the center along the hidden fourth axis; away from 0 the body thins, shrinks and breaks apart. `iterations` (10): more give finer detail, at three atan2, three sines, three cosines and a pow per step.

## Algebras

The Mandelbrot iteration in other number systems (quaternions, bicomplex numbers, hypercomplex numbers) and other polynomial and analytic maps in 3D.

### Quaternion Mandelbrot (`quaternion`)

![Quaternion Mandelbrot](images/fractals/quaternion.jpg)

The Mandelbrot set iterated with quaternions and sliced in 3D. Squaring a quaternion never takes it out of the plane spanned by the real axis and its own imaginary part, so the set is the Mandelbrot set spun about its real axis: a lathe-turned ornament of discs and rings with the antenna for a spire. Moving the slice along the fourth axis cuts it down to a dish.

**Formula.** With the point $p$, let $c = (p_x - 0.75) + p_y\,i + p_z\,j + w\,k$, start from $q = c$ and repeat:

$$q \leftarrow q^2 + c, \qquad (a + \mathbf{v})^2 = a^2 - |\mathbf{v}|^2 + 2a\,\mathbf{v}$$

where $a$ is the real part and $\mathbf{v}$ the imaginary part. Distance estimate (Inigo Quilez's, as for ray's quaternion Julia set, with the point as $c$): $d = |q| \ln|q| / (2|q'|)$ with $|q'| \leftarrow 2|q|\,|q'| + 1$, and $d = 0$ for an orbit still bounded after the last iteration.

**Origin.** Quaternion Julia sets were first drawn by Alan Norton (1982), and Hart, Sandin and Kauffman (1989) gave them a distance estimate for ray tracing. The quaternion Mandelbrot set was studied by Gomatam, Doyle, Steves and McFarlane ("Generalization of the Mandelbrot set: quaternionic quadratic maps", *Chaos, Solitons & Fractals*, 1995) and by Bedding and Briggs ("Iteration of quaternion maps", *International Journal of Bifurcation and Chaos*, 1995), who both found it to be a surface of revolution of the Mandelbrot set. Mandelbulber 2's "Quaternion" formula is its $w = 0$ slice.

**Parameters.** `w` (0): the slice's fourth ($k$) coordinate. At 0 it is the spun Mandelbrot set; away from 0 only the parts of the set farther than $|w|$ from its real axis are left, pulled in to the axis, so the antenna and the left-hand bulbs go first and the rest thins to a dish and rings (nothing is left past about 1.1). `iterations` (14, up to 40): escape-time iterations; more give sharper bulbs and finer rings.

### Tetrabrot (`tetrabrot`)

![Tetrabrot](images/fractals/tetrabrot.jpg)

Dominic Rochon's bicomplex Mandelbrot set, sliced through $1$, $i_1$ and $i_2$. A bicomplex number is two complex numbers squared independently, so the shape is two Mandelbrot sets drawn out along the diagonals and intersected: a faceted, cubist Mandelbrot set with diamond cross-sections and box-like bulbs.

**Formula.** The point $c = x + y\,i_1 + z\,i_2 + w\,j$ (with $x = p_x - 0.75$ for power 2, $p_x$ otherwise, $y = p_y$, $z = p_z$ and $j = i_1 i_2$) has the idempotent components

$$c_1 = (x + w) + (y - z)\,i, \qquad c_2 = (x - w) + (y + z)\,i,$$

and $c^n$ is $c_1^n$ and $c_2^n$, so each is iterated as a complex number, $u_k \leftarrow u_k^n + c_k$ from $u_k = c_k$; the point is in the set when both orbits stay bounded. Distance estimate: $d = \sqrt{(d_1^2 + d_2^2)/2}$, the distance to the pair of sets in the bicomplex norm, from the complex estimates $d_k = |u_k| \ln|u_k| / (2|u_k'|)$ with $|u_k'| \leftarrow n|u_k|^{n-1}|u_k'| + 1$ ($d_k = 0$ while an orbit is still bounded).

**Origin.** Dominic Rochon, "A generalized Mandelbrot set for bicomplex numbers" (*Fractals*, 2000). The distance estimate is Martineau and Rochon's ("On a bicomplex distance estimation for the Tetrabrot", *International Journal of Bifurcation and Chaos*, 2005), in the general form of Brouillette, Parisé and Rochon, ["Tricomplex distance estimation for filled-in Julia sets and Multibrot sets"](https://arxiv.org/abs/1811.09697) (*International Journal of Bifurcation and Chaos*, 2019), which also draws the Tetrabrots of higher powers.

**Parameters.** `power` (2, from 2 to 8): the exponent $n$. 2 is Rochon's Tetrabrot; higher powers give the Tetrabrots of the Multibrot sets, blocky bodies with $n - 1$ lobes about the center. `w` (0): the slice's fourth ($j$) coordinate. It slides the two Mandelbrot sets the shape is cut from apart along the real axis, so the shape shrinks and its facets break up into crossed bulbs (try 0.2 to 0.4). `iterations` (16, up to 40): escape-time iterations; more give sharper edges and finer bulbs.

### Hypercomplex Mandelbrot (`hypercomplex`)

![Hypercomplex Mandelbrot](images/fractals/hypercomplex.jpg)

Clyde Davenport's commutative quaternions, iterated as $z^2 + c$ and sliced through $1$, $i$ and $k$. They are the bicomplex numbers again, so the set is two Mandelbrot sets crossed, but this slice draws them out along the real axis instead of across it: a stepped arrowhead of stacked slabs with pyramid-like bulbs. Turning the slice toward $j$ morphs it into the Tetrabrot.

**Formula.** Davenport's units have $i^2 = j^2 = -1$, $k^2 = +1$, $ij = k$, $ik = -j$ and $jk = -i$, so

$$(a + b\,i + c\,j + d\,k)^2 = (a^2 - b^2 - c^2 + d^2) + 2(ab - cd)\,i + 2(ac - bd)\,j + 2(ad + bc)\,k.$$

With the slice angle $\theta$, the point is $c = x + y\,i + u\,j + v\,k$ with $x = p_x - 0.75$, $y = p_y$, $u = p_z \sin\theta + w\cos\theta$ and $v = p_z\cos\theta - w\sin\theta$. Its idempotent components are $c_1 = (x + v) + (y - u)\,i$ and $c_2 = (x - v) + (y + u)\,i$, each iterated as a complex number $u_k \leftarrow u_k^2 + c_k$, with the Tetrabrot's distance estimate $d = \sqrt{(d_1^2 + d_2^2)/2}$.

**Origin.** Clyde M. Davenport, *A Hypercomplex Calculus with Applications to Relativity* (self-published, ISBN 0-9623837-0-8), the basis of Fractint's hypercomplex fractals (Tim Wegner's [`hcmplx.c`](https://github.com/jhol/fractint/blob/master/common/hcmplx.c) has the same square). The 1-$i$-$k$ slice is the bicomplex Mandelbrot set's "Arrowheadbrot" in Brouillette, Parisé and Rochon's [catalog of principal slices](https://arxiv.org/abs/1811.09697) (2019). Mandelbulber 2's ["Hypercomplex"](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/hypercomplex.cl) formula squares with $k^2 = -1$ and $ij = -k$ instead: a non-associative algebra in which squaring scales $|z|^2$ by anything from 0.35 to 1.8 depending on direction, so its escape potential is too rough for a usable distance estimate (the estimate jumps from one escape iteration to the next). Davenport's algebra is used here.

**Parameters.** `angle` (0, from 0 to 90): the slice's third axis, turned from $k$ (0: the arrowhead) toward $j$ (90: the Tetrabrot), in degrees. In between, the crossed Mandelbrot sets lean and the bulbs become slanted wedges. `w` (0): the slice's fourth coordinate. It slides the two Mandelbrot sets apart, so the shape shrinks and splits into crossed bulbs. `iterations` (16, up to 40): escape-time iterations; more give sharper edges and finer bulbs.

### Aexion (`aexion`)

![Aexion](images/fractals/aexion.jpg)

Aexion's quadray Mandelbrot set: the point is written in four tetrahedral coordinates and squared in four dimensions. At the default shift the set falls apart into six blade-like lobes along the axes, four around the middle and two at the poles, with Mandelbrot-style bulbs along their edges.

**Formula.** Write the point in folded quadray coordinates, each shifted by $s$,

$$c = \big(|x+y+z|,\; |z-x-y|,\; |y-x-z|,\; |x-y-z|\big) + s,$$

start from $z = c$ and repeat

$$z \leftarrow (A + B,\; B - A,\; C + D,\; D - C) + c$$

with $A = z_1^2 - z_2^2$, $B = 2 z_3 z_4$, $C = z_3^2 - z_4^2$ and $D = 2 z_1 z_2$. In complex pairs $u = z_1 + i z_2$, $v = z_3 + i z_4$ this is two complex squares that trade imaginary parts: $u \leftarrow (1-i)(\mathrm{Re}\,u^2 + i\,\mathrm{Im}\,v^2) + c_u$ and $v \leftarrow (1-i)(\mathrm{Re}\,v^2 + i\,\mathrm{Im}\,u^2) + c_v$. Escape at $|z| = 10^4$.
Distance estimate: $d = \tfrac12 |z| \ln|z| / \partial$ with $\partial \leftarrow 2\,\frac{|f(z)|}{|z|}\,\partial + 2$ from $\partial = 2$, where $f$ is the squaring alone and $|f(z)| = \sqrt{2(|u|^4 + |v|^4)}$ lies between $|z|^2$ and $\sqrt2\,|z|^2$. The estimate is blended with the previous iteration's as $|z|$ runs from the bailout to its square, so it doesn't jump where an orbit escapes one step later.

**Origin.** Aexion (the author of the Incendia renderer) posted his "Quadray Sets" on Fractal Forums on April 26, 2011: the OctoMandelbrot (no shift), an eight-dimensional mirrored variant, and the "Mandelbrot Six", with every quadray coordinate shifted by −1.35 ([thread](http://www.fractalforums.com/the-3d-mandelbulb/quadray-sets/msg31458/#msg31458)). Syntopia posted a GLSL distance estimate there the same day. Ported from Mandelbulber 2's Aexion formula, which starts the orbit at $c$ rather than 0 (one step ahead; the same set) and defaults to a shift of −1.3.

**Parameters.** `shift` (−1.3): added to each of the four quadray coordinates, −1.6 to 0. Around −1.3 you get the Mandelbrot Six; lower values part the lobes further, and toward 0 they merge into one small octahedral star (the OctoMandelbrot, about a fifth the size). `iterations` (20): few iterations fatten the lobes and join them in the middle; past about 20 little changes.

### Bristorbrot (`bristorbrot`)

![Bristorbrot](images/fractals/bristorbrot.jpg)

Doug Bristor's 3D Mandelbrot set: the Mandelbrot set's outline along x, swept around into a swirled, lopsided solid with a long antenna. Squaring in his algebra mixes the two imaginary axes, which gives the surface its twisted bands.

**Formula.** Start from $z = c = p$ (shifted so the solid sits centered) and repeat

$$z \leftarrow \big(x^2 - y^2 - z^2,\; y\,(2x - t z),\; z\,(2x + t y)\big) + c$$

with $t$ the twist ($t = 1$ is Bristor's algebra). Escape at $|z| = 100$.
Distance estimate: $d = \tfrac12 |z| \ln|z| / \partial$ with $\partial \leftarrow 2\,\frac{|f(z)|}{|z|}\,\partial + 1$, $f$ the squaring alone (its stretch varies with direction, up to about $3|z|$, which the march's step allows for), blended across the escape as for `aexion`.

**Origin.** Doug Bristor's "rotational" complex numbers, two imaginary units whose products depend on their order (according to his site, [bristorbrot.org](https://bristorbrot.org/about/), he worked them out in the mid-1990s). He posted "Bristorbrot 3D" on Fractal Forums on November 28, 2009, with his original 3D iteration and renders by Jos Leys ([thread](http://www.fractalforums.com/theory/bristorbrot-3d/)). Ported from Mandelbulber 2's Bristorbrot. The twist parameter is this port's addition.

**Parameters.** `twist` (1): how much the two imaginary axes mix, 0 to 2. At 1 you get Bristor's algebra; 0 spins the Mandelbrot set about its axis into a smooth solid of revolution; above 1 it shears further. `iterations` (12): more iterations give finer bands and antenna detail.

### Buffalo (`buffalo`)

![Buffalo](images/fractals/buffalo.jpg)

A shaggy 3D relative of the 2D Buffalo and Burning Ship fractals: the power 2 Mandelbulb's squaring with some of its components made positive. The default holds the 2D Buffalo in its xy plane, facing the camera, with a radial whorl where the bulb's axis comes through; from other sides it grows into tufts and fronds.

**Formula.** Start from $z = c = p$ (shifted so the shape sits centered) and repeat, with $\rho^2 = x^2 + y^2$,

$$z \leftarrow \Big((x^2 - y^2)\big(1 - \tfrac{z^2}{\rho^2}\big),\; 2xy\big(1 - \tfrac{z^2}{\rho^2}\big),\; \mp 2 z \rho\Big) + c$$

taking the absolute value of the chosen components before the squaring (`pre_abs`) and after it (`abs`). Escape at $|z| = 100$.
Distance estimate: $d = \tfrac12 |z| \ln|z| / \partial$ with $\partial \leftarrow 2|z|\,\partial + 1$, blended across the escape as for `aexion`. The squaring is exact in length ($|f(z)| = |z|^2$) but turns a point near its axis by an arbitrary amount, so the estimate jumps a little across the surfaces where orbits pass the axis; the march's step (0.6) allows for it.

**Origin.** youhn posted "Buffalo fractals" in Fractal Forums' Mandelbulber gallery on February 10, 2015 ([thread](http://www.fractalforums.com/gallery-b177/buffalo-fractals/)). He took Mandelbulber's power 2 Mandelbulb iteration (White and Nylander's triplex square) and added absolute values to some of its outputs, hoping for a 3D Burning Ship. It looked like the Buffalo fractal of Kalles Fraktaler instead. Ported from Mandelbulber 2's Buffalo.

**Parameters.** `abs` ([1, 1, 0]): which components of the squared point are made positive (1) or left as they are (0), x, y and z, with z toward the default camera. [1, 1, 0] puts the 2D Buffalo in the xy plane; [0, 0, 0] is the plain power 2 Mandelbulb; each other choice is another member of the family. `pre_abs` ([0, 0, 0]): the same before the squaring, for more members of the family. `flip_z` (0): flips the sign of the squared point's z (no change when z is made positive anyway). `iterations` (12): more iterations give finer fronds.

### Mandelcup (`mandelcup`)

![Mandelcup](images/fractals/mandelcup.jpg)

A goblet: a smooth bowl on a stem, with layered ornament inside the bowl and at the foot. Each point's direction is projected onto a plane, squared there as a complex number and wrapped back onto the sphere, and its distance is squared.

**Formula.** Start from $z = c = p$. With $r = |z|$, shape $s$ and stretch $k$, let

$$w = k\,(1 + s^2)\left(\frac{x + i y}{z + s r}\right)^2$$

(a gnomonic projection of the direction, squared) and repeat

$$z \leftarrow r^2\,\frac{\big(2\,\mathrm{Re}\,w,\; 2\,\mathrm{Im}\,w,\; |w|^2 - 1\big)}{|w|^2 + 1} + c,$$

an inverse stereographic projection back onto the sphere, scaled by $r^2$. (Written over a common denominator so the equator, where the projection runs off to infinity, stays finite.) The formula's own z axis points down, so the cup stands upright. Escape at $|z| = 100$.
Distance estimate: $d = \tfrac12 r \ln r / \partial$ with $\partial \leftarrow 2r\,\partial + 1$, blended across the escape as for `aexion`. The projection stretches up to twice the radial rate, which the march's step (0.6) allows for.

**Origin.** pupukuusikko posted "A new 3d mandelbrot variant - Mandelcup" on Fractal Forums on November 12, 2015 ([thread](http://www.fractalforums.com/the-3d-mandelbulb/a-new-3d-mandelbrot-variant-mandelcup/)). He found it by varying Tglad's formulas from the "2D conformal formula suggestion" thread, and noted its grail-like shape. His later version's constants c1 and c2 are `shape` and `stretch` here. Ported from Mandelbulber 2's Mandelcup, which normalizes the direction and scales the result by $r^2$.

**Parameters.** `stretch` (2): multiplies the projected square, 0.25 to 4. At 2, the original's value, you get the goblet; 1 (Mandelbulber's default) gives a straighter vase; higher values give a rounder, shallower bowl. `shape` (0): added to the direction's height before the projection, −1 to 1. Positive values close the cup into a rounded pod; negative values open a wide, shallow bowl on a thin stem. `iterations` (12): more iterations give finer ornament in the bowl.

### Quick Dudley (`quick_dudley`)

![Quick Dudley](images/fractals/quick_dudley.jpg)

A lopsided, wind-swept lump with a crest of spikes, frilled edges and a long needle: the Mandelbrot set of squaring in a three-dimensional algebra of its own. It is not a power map in spherical coordinates like the bulbs, so it stretches unevenly and its surface comes out streaked and furry rather than lobed.

**Formula.** Start from $z = p$ (moved by $(-0.67, -0.2, -0.2)$ to center the set) and repeat:

$$z \leftarrow \begin{pmatrix} a_x x^2 - b_x yz \\ a_y z^2 + b_y xy \\ a_z y^2 + b_z xz \end{pmatrix} + c$$

with $a$ = `square`, $b$ = `cross` and $c = p$ (or the Julia constant). With $a = 1$, $b = 2$ this squares $x + ym + zn$ in the algebra whose imaginary units obey $m^2 = n$, $n^2 = m$, $mn = -1$. Bailout $|z| > 10$. The map stretches differently in different directions, so the whole Jacobian is carried along the orbit, $J \leftarrow Q'(z)\,J + I$, and the distance estimate is $d = \tfrac12 r \ln r / \lVert J \rVert_F$; the estimates of the last two iterates are blended by how far past the bailout the orbit escaped, so that neighboring escape bands meet without a step.

**Origin.** Fractal Forums user quick-dudley proposed the algebra in the thread [Another shot at the holy grail](http://www.fractalforums.com/3d-fractal-generation/another-shot-at-the-holy-grail/) (November 2012). In 2015 Kerry Mitchell (lkmitch) wrote the square out component by component and DarkBeam built it into Mandelbulb 3D; Mandelbulber 2 took it over as Quick Dudley, and its Quick Dudley Mod added the weights used here. Ported from Mandelbulber 2.

**Parameters.** `square` ([1, 1, 1]): weights on the squared terms of the new x, y and z; lowering one flattens and spreads the shape along that axis (0.5 to 1.5). `cross` ([2, 2, 2]): weights on the cross terms; larger pulls out longer spikes and frills, smaller rounds the shape off (1.5 to 2.5). `julia` (0): 1 adds `c` instead of the point, giving one of the map's Julia sets, swirled, flatter and often broken into islands. `c` ([-0.6, 0.3, 0.2]): the Julia constant. `iterations` (9): more makes the edges finer and hairier without changing the overall shape.

### Collatz (`collatz`)

![Collatz](images/fractals/collatz.jpg)

A curved, three-cornered shell with layered ridges and fine fringed edges, symmetric under any swap of the axes; from behind it is a lantern of nested rims. It is Mandelbulber 2's 3D lift of the Collatz fractal, the escape-time picture of the Collatz rule carried over to complex numbers.

**Formula.** Start from $z = p$ (scaled 4.4 times and moved to $(2.69, 2.69, 2.69)$ to center the set) and repeat:

$$z \leftarrow \tfrac14\left(1 + 4z - (1 + 2z) \odot R(z)\right), \qquad R(z) = 2\,(n \cdot z)\,n - z, \quad n = (1, 1, 1)$$

where $\odot$ multiplies component by component. In 2D the map is $\tfrac14(1 + 4z - (1 + 2z)\cos \pi z)$, which gives $n/2$ for even integers and $(3n+1)/2$ for odd ones; Mandelbulber replaces $\cos \pi z$ by the half turn of $z$ about $(1, 1, 1)$, written with the axis unnormalized, so $R$ also stretches $z$ five times along the diagonal. Bailout $|z| > 100$. Distance estimate as for Quick Dudley: $d = \tfrac12 r \ln r / \lVert J \rVert_F$ with $J \leftarrow f'(z)\,J$, blended across escape bands.

**Origin.** The complex Collatz map is an old extension of the conjecture (see [Wikipedia](https://en.wikipedia.org/wiki/Collatz_conjecture#Iterating_on_real_or_complex_numbers)); Claude Heiland-Allen rendered its fractal with a distance estimate in [The Collatz fractal](https://mathr.co.uk/blog/2016-04-10_collatz_fractal.html) (mathr.co.uk, April 2016). Mandelbulber 2's Collatz (2017) is the 3D lift ported here.

**Parameters.** `iterations` (16): more makes the ridges and fringes finer; past about 10 the shape barely changes.

### Collatz Mod (`collatz_mod`)

![Collatz Mod](images/fractals/collatz_mod.jpg)

The 3D Collatz map with its turn, axis, offset and scale opened up. At its default angle the shell twists into a three-armed pinwheel with fringed blades; angles either side of a half turn twist it opposite ways, and adding the point gives a swirled, Mandelbrot-type shape.

**Formula.** Start from $z = p$ (scaled 6.2 times and moved to $(3.92, 3.92, 3.92)$) and repeat:

$$z \leftarrow s\left(b + 4z - (n + 2z) \odot R_\theta(z)\right) + m \odot (p_z, p_y, p_x)$$

$$R_\theta(z) = z\cos\theta + (n \times z)\sin\theta + n\,(n \cdot z)(1 - \cos\theta)$$

with $\theta = \pi\cdot$`angle`, $n$ = `axis` (not normalized), $b$ = `offset`, $s$ = `step_scale` and $m$ = `c_mult`. With $\theta = \pi$, $n = b = (1, 1, 1)$, $s = \tfrac14$ and $m = 0$ it is the plain Collatz map. Bailout $|z| > 100$; distance estimate as for Collatz, with $J \leftarrow f'(z)\,J + \mathrm{diag}(m)\,P$ ($P$ the x-z swap).

**Origin.** Mandelbulber 2's Collatz Mod (2017), ported here. Its default angle differs from Mandelbulber's (0.8 of a half turn rather than a whole one) so that it is not a copy of `collatz`; set `angle` to 1 for Mandelbulber's default.

**Parameters.** `angle` (0.8): the turn in half turns; 1 is the plain map, and either side of it the shell twists into arms one way or the other (0.8 and 1.2 are mirror images; 0.7 to 1.3). `axis` ([1, 1, 1]): the turn's axis, whose length also stretches the map, and the constant in the $(n + 2z)$ factor; tilting it skews the arms, and far from (1, 1, 1) the shape can outgrow its bounds. `offset` ([1, 1, 1]): the constant added before scaling; changes the arms' lengths and where the cusps fall. `step_scale` (0.25): the factor on each step; smaller shrinks the set, larger grows it (0.2 to 0.3). `c_mult` ([0, 0, 0]): how much of the point is added each step, taken z, y, x as Mandelbulber adds it; towards 0.5 the shell swirls into a Mandelbrot-type shape, whose distance estimate is rougher, so expect some fuzz. `iterations` (16): more makes the arms and fringes finer.

### Newton (`newton`)

![Newton](images/fractals/newton.jpg)

A Newton fractal in 3D: a ribbed barrel whose ends are sets of concentric rings. Newton's method for $w^3 = 1$ in the triplex (Mandelbulb) algebra sends most points to the root 1; the solid is everything that goes to the other two roots instead.

**Formula.** Start from $z = p$ (moved by $-0.655$ along x) and repeat:

$$w = \frac{\bar z}{|z|^2} + 1, \qquad w \leftarrow \tfrac23 w + \frac{1}{3w^2}, \qquad z \leftarrow \frac{\overline{w - 1}}{|w - 1|^2} + o$$

where $\bar{\ }$ negates y and z, $w^2$ is the triplex square $\left((x^2 - y^2)(1 - \tfrac{z^2}{x^2 + y^2}),\ 2xy(1 - \tfrac{z^2}{x^2 + y^2}),\ 2z\sqrt{x^2 + y^2}\right)$, $1/w^2 = \overline{w^2}/|w|^4$, and $o$ = `offset`. The inversions turn Newton's convergence to the root 1 into an escape of $z$ to infinity, so the basin of 1 is the outside. Near that root $z$ is squared like a quaternion, $|z| \to |z|^2$, so the usual logarithmic estimate applies: bailout $|z| > 30$, $d = \tfrac12 r \ln r / \lVert J \rVert_F$, with $J$ carried by forward-mode dual numbers and blended across escape bands as above.

**Origin.** The formula and its inversion trick are by gannjondal, from the Fractal Forums thread [Revisiting the 3D Newton](https://fractalforums.org/fractal-mathematics-and-new-theories/28/revisiting-the-3d-newton/1026); mclarekin added it to Mandelbulber 2 as Newton Pow3 (2022). Ported from Mandelbulber 2.

**Parameters.** `offset` ([0, 0, 0]): added after every step, like a Julia constant; along x it splits the barrel into two ribbed halves joined by filaments, sideways it twists and roughens it (-0.6 to 0.6). `iterations` (16): Newton steps; the rings are sharp by about 12, and more adds finer filaments where the basins meet.

## Spheres and tori

Sphere packings, sphere inversions and Kleinian groups, and fractals built on the torus.

### Msltoe Donut (`msltoe_donut`)

![Msltoe Donut](images/fractals/msltoe_donut.jpg)

A torus with nine smaller tori looped around its tube like chain links, each of those looped with nine smaller ones, and so on, so the ring sprouts bushy clusters of links. Unlike the escape-time bulbs, it is built from exact tori, so every generation is crisp.

**Formula.** In a frame with the ring about the third axis, start from $(x, y, z) = p$ and repeat for generation $n = 0, 1, \dots$:

$$d_n = \frac{\sqrt{(\sqrt{x^2+y^2} - 1)^2 + z^2} - r}{f^{\,n}}, \qquad \theta_k = \frac{2\pi}{N}\,\mathrm{round}\!\left(\frac{N\,\mathrm{atan2}(y, x)}{2\pi}\right)$$

$$(x', y') = R_{-\theta_k}(x, y), \qquad z \leftarrow f\,(x' - 1,\ z,\ y')$$

where $d_n$ is the distance to that generation's torus (ring radius 1, tube radius $r$), $\theta_k$ is the nearest of the $N$ sectors, and the map moves that sector's point on the ring to the center and turns the torus's cross-section plane into the next torus's ring plane. Distance estimate: $d = \min_n d_n$. A point stops early once no finer generation can come closer, because they all lie within $\max\!\left(\frac{1}{f-1}, \frac{1+r}{f}\right)$ of the ring line, scaled by $f^{-n}$.

**Origin.** Posted by msltoe on Fractal Forums in January 2016 ("Low-hanging dessert: an escape-time donut fractal", [thread](http://www.fractalforums.com/new-theories-and-research/low-hanging-dessert-an-escape-time-donut-fractal/msg90171/#msg90171)) and added to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by Krzysztof Marczak the same month. Mandelbulber runs it as an escape-time formula: a point inside the torus is divided by the torus function so that it escapes, and the distance comes from a numerical gradient. This port draws the same union of tori with an exact distance.

**Parameters.** `thickness` (0.1): the tube radius of every torus as a share of its ring radius; useful from 0.01 (wire rings) to 0.3 (fat doughnuts that crowd their links). `factor` (3): how many times smaller each generation is, 2 to 5; low values tangle the links into dense clusters, high ones leave them sparse. `number` (9): how many tori loop around each torus, 3 to 12. `iterations` (5): generations drawn; the renderer adds more close up (up to 12), assuming a factor of about 3.

### Mandeltorus (`mandeltorus`)

![Mandeltorus](images/fractals/mandeltorus.jpg)

A Mandelbulb wrapped around a ring. The bulb's power map takes its radius from the distance to a circle instead of a point, so lobes run around a doughnut and ridges wrap its tube, with a fluted crater in the hole.

**Formula.** Start from $z = p$. With $\rho = \sqrt{z_x^2 + z_z^2}$, $\varphi = \mathrm{atan2}(z_z, z_x)$, $\theta = \mathrm{atan2}(\rho, z_y)$ and $q = \sqrt{(\rho - R)^2 + z_y^2}$, the distance to the ring of radius $R$, repeat:

$$z \leftarrow \left(q^{a}\sin(b\theta)\cos(a\varphi),\ q^{b}\cos(b\theta),\ q^{a}\sin(b\theta)\sin(a\varphi)\right) + p$$

with $a$ = `power` and $b$ = `power2`, until $|z| \ge 10$. In mode 2, $\theta$ becomes $\tau = \mathrm{atan2}(|\rho - R|, z_y)$, measured from the ring, and the map is $\left((R + \cos b\tau)\,q^a\cos a\varphi,\ q^b\sin b\tau,\ (R + \cos b\tau)\,q^a\sin a\varphi\right) + p$. Distance estimate: $d = \tfrac12\, q \ln q / \partial$, where $q$ is the escaped point's distance to the ring and $\partial \leftarrow \max(a, b)\,|F|/q\;\partial + 1$, with $F$ the map's output before $p$ is added. An orbit that never escapes is inside.

**Origin.** By Aexion on Fractal Forums, 6 August 2014 ("MandelDonuts", [thread](http://www.fractalforums.com/the-3d-mandelbulb/mandeldonuts/)), as a toroidal Mandelbulb. Ported to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by mclarekin in 2019. Mandelbulber's analytic estimate grows its derivative by $(a - 1)\,q^a$ and measures $|z|$, which makes the estimate jump wherever the escape count changes and draws onion-skin shells. This port follows the map's actual growth and the ring distance instead. The default powers of 18 are those of Mandelbulber's example image; its own defaults of 8 give a lumpy blob.

**Parameters.** `power` (18): the power around the ring, or how many lobes run around the doughnut; 4 to 24. `power2` (18): the power across the tube, or how many ridges wrap its cross-section; low values make a lumpy blob. `ring` (1.5): the radius of the circle the map is measured from; 1 to 2, and larger opens the hole. `mode` (1): 1 measures the angle across the tube from the center, giving a fluted doughnut; 2 measures it from the ring, giving stacked, wavy rings. `iterations` (10): escape-time iterations, up to 20; more give finer lobes.

### Sphere Tree (`spheretree`)

![Sphere Tree](images/fractals/spheretree.jpg)

A dome of spheres within spheres. Every ball is lined inside with a hexagonal layer of smaller balls, and each of those with smaller ones again, so the surface grows like a cauliflower of bubbles.

**Formula.** With $z$ up and the base sphere of diameter 1 resting on the origin, start from $z = p$ and repeat:

- Stop inside the solid inner sphere of diameter $\sqrt3/(1+\sqrt3)$ resting on the origin. After the first step, also stop where $z_z > 0.4$ and the point is outside the sphere of radius 0.6 about $(0, 0, 0.6)$; that region is empty.
- If $z_z < 0.4$ and the point is outside the base sphere, invert: $z \leftarrow z/|z|^2$.
- Otherwise, invert, which turns the base sphere into the plane $w_z = 1$, then flip and stretch below it and twist: $z \leftarrow R_{30^\circ}\!\left(\sqrt3\,w_x,\ \sqrt3\,w_y,\ 1 + \sqrt3\,(1 - w_z)\right)$ with $w = z/|z|^2$.
- Fold $(z_x, z_y)$ into the central hexagon of a unit triangular lattice, then into its kite, so a lattice of unit spheres resting on $z = 0$ stands for the base sphere's children.

Distance estimate: $d = \left(|z - (0, 0, \tfrac12)| - \tfrac12\right)/(2.22\,\partial)$, where $\partial$ multiplies the inversions' $1/|z|^2$ and the stretches' $\sqrt3$, and 2.22 is Mandelbulber's safety factor. At every step $d$ is also held below the inner sphere's distance and below $\max(\text{base sphere distance}, -\text{distance into the stop region})$, so it does not jump where a point stops one generation earlier.

**Origin.** By TGlad (Tom Lowe) on Fractal Forums in 2020 ("New sphere tree", [thread](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557/msg22100#msg22100)). Ported to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by mclarekin, with extras by sabine62, in June 2020. In Mandelbulber, the stops set the estimate to the current generation's sphere, which jumps between generations. The extra bounds here remove the jumps.

**Parameters.** `twist` (30): how far each generation's hexagonal layer turns against its parent's, 0 to 60 degrees. It moves where the small balls sit, a subtle change. `lattice` (2): the density of each hexagonal layer, 1.5 to 2.5. At 2 the balls touch; lower values leave fewer, bigger balls and higher ones crowd in more, smaller ones. `iterations` (4): generations of balls, each about five times finer; the renderer adds more close up, up to 16.

### Sphere Cluster (`sphere_cluster`)

![Sphere Cluster](images/fractals/sphere_cluster.jpg)

A hollow ball of sphere clusters with icosahedral symmetry, spiked like a snowflake. Clusters crowd toward an inner sphere in ever smaller copies, and every ball in a cluster is a smaller cluster in turn.

**Formula.** Twenty face spheres of radius $r$, centered at distance $l = \sqrt{1 + r^2}$ along the icosahedron's face directions so that each meets the unit sphere at right angles, leave a hole of radius $h = l - r$ about the center. For each generation, starting from $z = p$:

$$z \leftarrow h\,\frac{z}{|z|^2}$$

Then, up to three times, fold $z$ into the icosahedron's fundamental cell with its mirror planes and, if it lies inside the face sphere there, invert in that sphere:

$$z \leftarrow c + r^2\,\frac{z - c}{|z - c|^2}$$

A point that was never inverted is in the hole itself, which is empty, so it is inverted once toward the nearest copy. Then $z \leftarrow z/(h k)$ with $k$ = `pack` for the next generation. Distance estimate, with $\Delta = |z| - hk$ after the last generation:

$$d = \frac{\Delta}{\partial + S\,\Delta}$$

where $\partial$ multiplies every inversion's and scaling's factor and $S = \sum_j \partial_j/|z_j|$ over the inversions about the center. A sphere inversion can shrink a distance by no more than that term, so the estimate does not outgrow the true distance far from the center.

**Origin.** By TGlad on Fractal Forums in 2023 ("Fractal clusters", [thread](https://fractalforums.org/fractal-mathematics-and-new-theories/28/fractal-clusters/5107)). Ported to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) by mclarekin in June 2023. Mandelbulber recurses only from inside the hole and measures a ball from outside it, which makes the estimate jump at the hole's edge. It also gives everything beyond radius 1.5 a fixed distance. This port always recurses and bounds the inversions instead, so the estimate is steady.

**Parameters.** `pack` (0.8): how much of its gap each sub-cluster fills, 0.5 to 0.95. Low values leave airy, separate clusters; high ones swell them until they touch. `dodecahedral` (0): 0 hangs the sub-clusters over the 20 faces of an icosahedron, and 1 hangs them over the 12 faces of a dodecahedron, for fewer, bigger spikes. `alternate` (0): 1 switches between the two arrangements at every generation. `iterations` (4): generations, each turning the balls of the last into clusters; the renderer adds more close up, up to 8.

### Jos Leys's Kleinian (`jos_kleinian`)

![Jos Leys's Kleinian](images/fractals/jos_kleinian.jpg)

The limit set of a true Kleinian group: floors of great domes with necklaces of ever smaller spheres strung between them into columns, layer upon layer. It is what the pseudo-Kleinian imitates, done with the group itself.

**Formula.** Start from $z = p$ and $\partial = 1$ (or from $p$ inverted in the sphere `inversion_centre`, `inversion_radius`, with $\partial$ that inversion's scale) and repeat: wrap $z$ into the cell $[-1, 1) \times [0, a) \times [-1, 1)$ (with `box` at 1); above the separation line, $z_y \ge a\left(\tfrac12 + \tfrac15 \sin\left(\pi\,\mathrm{sgn}(b)\,(z_x + \tfrac b2)\right)\right)$, turn it half round, $z \leftarrow (-b, a, 0) - z$; then apply the generator

$$z \leftarrow \left(\frac{z_x}{|z|^2} - b,\; a - \frac{z_y}{|z|^2},\; -\frac{z_z}{|z|^2}\right), \qquad \partial \leftarrow \frac{\partial}{|z|^2},$$

an inversion in the unit sphere and a half turn. In the plane $w = z_x + i z_y$ it is Maskit's generator $w \mapsto t + 1/w$ with $t = -b + ai$, and the wrap in $x$ is the other generator, $w \mapsto w + 2$. The loop stops when $|z| > 10$ or the orbit closes a 2-cycle. Distance estimate (Knighty's): $d = \min(h, c_y) / \max(\partial, c_\partial)$, where $h = \min(z_y, a - z_y)$ is the height above the floor or below the ceiling (just $z_y$ with `spheres` at 0), taking the smaller of its values at the last two steps.

**Origin.** Jos Leys's escape-time algorithm for the limit sets of Kleinian groups in Maskit's slice, from his Fractal Forums thread [An escape time algorithm for Kleinian group limit sets](http://www.fractalforums.com/3d-fractal-generation/an-escape-tim-algorithm-for-kleinian-group-limit-sets/) (2016-17) and his paper [A fast algorithm for limit sets of Kleinian groups with the Maskit parametrisation](http://www.josleys.com/articles/Kleinian%20escape-time_3.pdf) (January 2017), with the distance estimate by Knighty; more of his pictures are at [josleys.com](https://www.josleys.com/galleries.php). Ported from Mandelbulber 2's `jos_kleinian.cl` (added as JosLeys-Kleinian in v2.12, November 2017), with two changes. Like Jos Leys's original, it stops once the orbit closes a 2-cycle. It also takes the smaller estimate of the cycle's two phases, so neighboring points that stop in different phases agree. The defaults differ too: $a = 1.9$ rather than 2 (open necklaces rather than cusps), and a step clamp of 0.2 rather than 0.05. At 0.05 the estimate is 5 to 10 times too small near the cusps, and that shows as fuzz. With $b \ne 0$, Mandelbulber's unsheared wrap makes the estimate jump in places; lower `clamp` if that shows.

**Parameters.** `klein` ([1.9, 0]): Jos Leys's KleinR and KleinI. The first, $a$, is the height of a layer (1.8 to 2.1, at least 0.5): below 2 the necklaces open into columns of beads, at 2 they pinch to points. The second, $b$ (-0.2 to 0.2), curls them into seahorse tails, though the estimate gets rougher. `box` ([1, 1, 1]): half sizes of the wrap cell along x, y and z. At 1 it is the group's own period; other values cut and repeat it. y also spaces the layers: 5 or more leaves a single layer between a solid floor and ceiling. `spheres` (1): 1 draws the solid spheres on both faces of each layer, the classic look; 0 draws only the floor's images, a finer and dustier lattice. `clamp` ([0.2, 1]): the most one step of the estimate may be, and the least the maps' scale may count as. Smaller is safer and slower; neither changes the shape. `inversion_centre` ([0, 1, 0]) and `inversion_radius` (0): an inversion applied before anything else, off at radius 0. About a point inside a layer it bends the stacked layers into a tower of nested bubbles. With a single layer (`box` y of 5 or more), `spheres` at 0 and the center above the layer, it folds the whole infinite layer into one bounded cluster. `iterations` (20): steps of the maps, up to 64; more resolves smaller spheres in the necklaces, at linear cost.

### Schottky group (`schottky`)

![Schottky group](images/fractals/schottky.jpg)

Indra's Pearls in 3D: kissing spheres at the corners of a regular solid, each holding the others' images under inversion in it, and so on down, until necklaces of pearls within pearls gather on the group's limit set. It is bounded, unlike the two Kleinian sets beside it.

**Formula.** Spheres $S_j$ of radius $r$ sit on the corners $c_j$ of a regular solid, a unit from the center; with `centre` above 0, one more of radius $r_0$ sits at the center. Inversion in one is $\sigma_j(z) = c_j + r_j^2\,(z - c_j)/|z - c_j|^2$. What is drawn is every sphere $\sigma_{j_1} \sigma_{j_2} \cdots \sigma_{j_n}(S_{j_{n+1}})$ with no two neighboring indices equal and $n$ = `iterations`: the pearls $n$ levels down. Start from $z = p$ and repeat $n$ times: invert $z$ in the sphere nearest it other than the last one used, $z \leftarrow \sigma_j(z)$. The rest of that branch then lies in the spheres other than $S_j$, and the other branches are no nearer than the second-nearest sphere. Distances are carried back to $p$ as strict lower bounds. For an inversion of radius $R$ about $c$, with $q = |z - c|$ before it, a distance $d$ after it is at least

$$\frac{d}{R^2/q^2 + d/q} \quad\text{and at least}\quad \frac{d\,q}{M},$$

where $M = |c| + 1 + r$ bounds how far the spheres lie from $c$. The second bound is the one that holds up near the center. Both have the form $1/d_0 \le A/d + B$, so they accumulate step by step as $B \leftarrow B + A\beta$, $A \leftarrow A\alpha$, with $(\alpha, \beta) = (R^2/q^2, 1/q)$, or $(M/q, 0)$ when $q < 2R^2/M$. Distance estimate: the smallest of $d/(A + Bd)$, taken for the nearest remaining sphere at the end and for the second-nearest sphere at every step.

**Origin.** After the kissing Schottky groups of *Indra's Pearls: The Vision of Felix Klein* (David Mumford, Caroline Series and David Wright, Cambridge, 2002), whose limit sets are drawn as chains of pearls; Schottky groups go back to Friedrich Schottky (1877). Here the group is generated by inversions in spheres on a regular solid's corners; its even words form a Schottky group, a kissing one when the spheres touch. The layouts, the greedy descent and the distance bound are this project's own, not a port.

**Parameters.** `layout` (3): the solid. 0 is a tetrahedron (4 spheres), 1 an octahedron (6), 2 a cube (8), 3 an icosahedron (12). The octahedron and cube make cages of necklace rings; the icosahedron makes a ball of pentagonal pearl webs. `radius` (1): the spheres' radius as a fraction of the radius at which neighbors kiss (0.3 to 1). At 1 the pearls string into necklaces; below it they part, and the set breaks into separate snowflake clusters, a Cantor dust. `centre` (0): a sphere at the center, as a fraction of the size at which it kisses all the others (0 to 1); 0 leaves it out, and its images thicken the web between the necklaces. `iterations` (4): levels of inversion, up to 24. Each puts the spheres' images inside every pearl; past 5 the pearls are finer than the pixels from afar. Each level costs a pass over the spheres.

### Pseudo-Kleinian Mod 3 (`kleinian_mod`)

![Pseudo-Kleinian Mod 3](images/fractals/kleinian_mod.jpg)

Stories of domed floors held up by columns of beads, an endless arcade. It has the pseudo-Kleinian's box folds and inversions, but its distance is measured to a floor plate and to struts along the axes instead of to rings. Where ray's `kleinian` is a tangle of wire rings, this one is architecture.

**Formula.** Start from $z = p$ and $\partial = 1$ and repeat: push out by the box offset, $z \leftarrow z - o\,\mathrm{sgn}(z)$; box fold, $z \leftarrow 2\,\mathrm{clamp}(z, -c, c) - z$; invert, $k = \max(s/|z|^2, 1)$, $z \leftarrow k z + u$, $\partial \leftarrow k\,\partial$. Distance estimate:

$$d = \frac{1}{2\partial} \min\left(\sqrt{\min(z_x^2 + z_y^2,\; z_y^2 + z_z^2,\; z_z^2 + z_x^2)} - r_s,\;\; |z_y| - r_p\right),$$

the distance to struts along the three axes and to a plate across $y$.

**Origin.** Knighty's pseudo-Kleinian (Fractal Forums, 2011, in the [Fragmentarium thread](http://www.fractalforums.com/3d-fractal-generation/fragmentarium-an-ide-for-exploring-3d-fractals-and-other-systems-on-the-gpu/msg32270/#msg32270); Fragmentarium's `PseudoKleinian.frag` credits it as Theli-at's "scale 1 JuliaBox + something"). Mod 3 is mclarekin's variant in Mandelbulber 2 (v2.21, March 2020); this is a port of its `pseudo_kleinian_mod3.cl`. Of Mandelbulber's variants (Mod 1 to Mod 7 and Trig), Mod 3 looks the least like ray's `kleinian` as it stands. Most of the others keep the ring estimate and change the look only through optional extras; Mod 3 changes what is measured. There are two changes from Mandelbulber. The plate lies across y instead of z (Mandelbulber's z is up, ray's y), so it stays a floor. The fold box and inversion default to Knighty's values, as ray's `kleinian` does, rather than Mandelbulber's 0.7 and 0.5.

**Parameters.** `fold_box` ([0.92436, 0.90756, 0.92436]): half sizes of the box fold along x, y and z (0.7 to 1.1). Smaller pinches the columns to cusps; larger widens the halls. `inversion` (1): the inversion sphere's radius squared (0.5 to 1.5): larger swells the beads and domes. `box_offset` ([0, 0, 0]): pushes each coordinate away from the center before the fold (0 to 0.1). The columns melt into ribbed trunks; much more fills the space solid. `offset` ([0, 0, 0]): added after each inversion (-0.1 to 0.1): warps and melts the arcades. `strut` (0): the radius of the struts along the axes (0 to 0.6). They hide inside the columns until about 0.4, then swell into drums and discs. `plate` (0): the floor plate's half thickness (0 to 0.2): thicker plates facet the domes like umbrellas. `iterations` (8): fold steps, up to 20; more gives finer beads, at linear cost.

## Geometric

Exact self-similar solids: cubes kept and cut by rule, polyhedral kaleidoscopes, the Koch surface and a knotted rope of ropes.

### Mosely Snowflake (`mosely`)

![Mosely Snowflake](images/fractals/mosely.jpg)

A cube whose corners are notched off at every scale, leaving a crinkled block that looks like a six-pointed snowflake when you view it along a long diagonal. The light variant is hollow and can be seen straight through along its diagonals (pictured: `light` 1, looking down one of them).

**Formula.** Split the cube $[-1,1]^3$ into $3\times3\times3$ cells and drop the 8 corner cells, keeping 19 of 27 (the light variant also drops the center cell, keeping 18), then do the same to every cell that is left. Level $i$ uses cells $2/3^i$ across. One periodic fold covers all of them at once: $u = 1 - \lvert 2\,\mathrm{fract}(c\,3^i/2) - 1\rvert$ on each axis is 0 at a cell's center and 1 at its faces. In those coordinates the corners are the cells with $\min(u) > 1/3$, and their signed distance is

$$h_i = \bigl\lVert \max(\tfrac13 - u,\, 0) \bigr\rVert - \max(\min(u) - \tfrac13,\, 0)$$

(for the center, a box of half size 1/3). The solid is the cube minus every level's cut, $d = \max\bigl(\mathrm{box}(p),\ \max_i (-h_i / 3^i)\bigr)$, as in ray's Menger sponge. The estimate is exact at each level and conservative overall. Dimension $\log 19/\log 3 \approx 2.68$ (light: $\log 18/\log 3 \approx 2.63$).

**Origin.** Named after Jeannine Mosely, the software engineer and origami artist known for her business-card Menger sponge. In 2012 she and Margaret Wertheim of the Institute For Figuring led the folding of a giant Mosely Snowflake Sponge from about 49,000 business cards, which was shown at USC's Doheny Library ([IFF](https://theiff.org/current/?p=1864), [Wikipedia](https://en.wikipedia.org/wiki/Mosely_snowflake)). It is the Menger sponge's opposite: the sponge removes the center and face cells, the snowflake removes the corners.

**Parameters.** `light` (0): 0 keeps 19 of 27 cells (the heavier snowflake), 1 also removes each center cell (the lighter one), which makes it hollow. `iterations` (4): levels of cutting as seen from afar (1 to 12). Close up, more levels are added on their own as the pixels allow, so it keeps finding detail.

### Vicsek Fractal (`vicsek`)

![Vicsek Fractal](images/fractals/vicsek.jpg)

A 3D cross whose six arms are themselves crosses, and so on down: a branching, loop-free tree of beams. Seen face on, it is the flat Vicsek cross. From any other angle, its arms reach out in all six directions.

**Formula.** Of the cube's $3\times3\times3$ cells keep only the center and the six face cells (7 of 27), and repeat in each. Using the same periodic fold as the Mosely snowflake, the cut cells are those where at least two of $u$ pass $1/3$. With $a \ge b$ the two largest of $u$, its distance is

$$h_i = \bigl\lVert \max(\tfrac13 - (a, b),\, 0) \bigr\rVert - \max(b - \tfrac13,\, 0)$$

and $d = \max\bigl(\mathrm{box}(p),\ \max_i (-h_i / 3^i)\bigr)$. Dimension $\log 7/\log 3 \approx 1.77$, so it thins toward lines as the levels go up.

**Origin.** Tamás Vicsek introduced the flat version in 1983 as a deterministic model of diffusion-limited aggregation ("Fractal models for diffusion controlled aggregation", *J. Phys. A* 16, L647). This is its natural 3D form, also called the box fractal ([Wikipedia](https://en.wikipedia.org/wiki/Vicsek_fractal)).

**Parameters.** `iterations` (4): levels of crosses as seen from afar (1 to 12). Each level builds the arms out of crosses a third the size. Close up, more levels are added on their own.

### Cantor Dust (`cantor_dust`)

![Cantor Dust](images/fractals/cantor_dust.jpg)

A cloud of cubes in clusters of eight, the clusters in clusters of eight, with none of them touching. It is the three-dimensional Cantor set: a set with no volume and no connected piece larger than a point.

**Formula.** Keep only the 8 corner cells of the $3\times3\times3$ split, and repeat. What a level keeps is exactly what the Mosely snowflake cuts. So with the same fold, the solid at level $i$ has the exact signed distance

$$g_i = \bigl\lVert \max(\tfrac13 - u,\, 0) \bigr\rVert - \max(\min(u) - \tfrac13,\, 0)$$

and $d = \max\bigl(\mathrm{box}(p),\ \max_i\, g_i / 3^i\bigr)$. That makes it the product $C\times C\times C$ of the middle-thirds Cantor set $C$, with dimension $\log 8/\log 3 \approx 1.89$.

**Origin.** Georg Cantor described the middle-thirds set in 1883 (it had appeared earlier in work by Henry Smith). Benoit Mandelbrot popularized the name "Cantor dust" for it and its higher-dimensional versions in *The Fractal Geometry of Nature* (1982).

**Parameters.** `iterations` (4): levels of splitting as seen from afar (1 to 12). Each splits every cube into eight a third the size. Close up, more levels are added on their own.

### Jerusalem Cube (`jerusalem`)

![Jerusalem Cube](images/fractals/jerusalem.jpg)

A cube with a Greek cross drilled through each face. What remains is eight corner cubes and twelve smaller edge cubes, and each of those is drilled the same way. Unlike the 3×3×3 fractals, its two sizes of cube are in an irrational ratio, which gives it a stepped, architectural look.

**Formula.** The corner cubes are $k = \sqrt2 - 1$ times the parent's size and the edge cubes $k^2$. This fits because $2k + k^2 = 1$: two corners and one edge cube exactly span an edge. Two scales can't share one periodic fold, so it is an IFS. Start from $z = p$, scale $s = 1$, and repeat:

$$z \leftarrow \mathrm{sort}_\downarrow(\lvert z\rvert),\qquad
\begin{cases} z \leftarrow (z - (1-k)\mathbf{1})/k, & s \leftarrow s\,k \quad\text{(corner nearer)}\\ z \leftarrow (z - (1-k^2, 1-k^2, 0))/k^2, & s \leftarrow s\,k^2 \quad\text{(edge nearer)} \end{cases}$$

After the fold and sort, every other piece is a mirror image of one of these two and no nearer. Each step therefore goes into the nearer box and keeps the other's box distance (times $s$) as a bound: $d = \min(\text{bounds},\ s\,\mathrm{box}(z))$. The estimate is the least of boxes that hold the fractal, so it never overshoots. Where two pieces touch, the box of the one not followed can show through holes in the other, but only deep inside the tunnels. Dimension $\approx 2.529$, the root of $8k^d + 12k^{2d} = 1$.

**Origin.** Eric Baird described it in 2011 on his blog Alt.Fractals ("[The Jerusalem Cube](http://alt-fractals.blogspot.com/2011/08/jerusalem-cube.html)"). It was also published in *Tangente* 150 (2013). The name comes from the Jerusalem cross, which each face resembles ([Wikipedia](https://en.wikipedia.org/wiki/Jerusalem_cube)).

**Parameters.** `iterations` (5): levels of drilling as seen from afar (1 to 14). Each level drills crosses into cubes about 2.4 times smaller. Close up, more levels are added on their own.

### Polyhedral IFS (`polyhedral`)

![Polyhedral IFS](images/fractals/polyhedral.jpg)

Knighty's kaleidoscopic IFS for all three polyhedral mirror groups: space is folded into one cell of the tetrahedral, octahedral or icosahedral group, then scaled away from a point of that cell. Moving the point around the cell's corners and setting the scale gives the icosahedral, dodecahedral and octahedral flakes, the Sierpinski tetrahedron and everything between them. The built-in `kifs` folds by the octahedral group alone, sorting the axes; this one takes any of the three groups and any point of the cell, and ends each copy as that point's polyhedron.

**Formula.** With $n$ the symmetry (3, 4 or 5) and $c = \cos(\pi/n)$, the third mirror has normal $\mathbf{m} = (-\tfrac12, -c, \sqrt{3/4 - c^2})$; the other two are the planes $x = 0$ and $y = 0$. The fold $F$ repeats $n$ times

$$(x, y) \leftarrow (|x|, |y|), \qquad \mathbf{z} \leftarrow \mathbf{z} - 2\min(0, \mathbf{z}\cdot\mathbf{m})\,\mathbf{m}$$

which lands every point in the cell whose corners are the unit vectors $\mathbf{a}$ (a 2-fold axis, the midpoint of an edge), $\mathbf{b} \propto (\sqrt{3/4 - c^2}, 0, \tfrac12)$ (the $n$-fold axis, a vertex of the tetrahedron, octahedron or icosahedron) and $\mathbf{c} \propto (0, \sqrt{3/4 - c^2}, c)$ (a 3-fold axis, the center of a face). The vertex is $\mathbf{v} = \mathrm{normalize}(u\,\mathbf{a} + v\,\mathbf{b} + w\,\mathbf{c})$ for the weights $(u, v, w)$. Start from $\mathbf{z} = \mathbf{p}$ and repeat $N$ times

$$\mathbf{z} \leftarrow s\,R_{\mathbf{v}}(F(\mathbf{z})) - (s - 1)\,\mathbf{v}$$

with $R_{\mathbf{v}}$ the twist about the axis through $\mathbf{v}$. Each copy ends as the Wythoff polyhedron of vertex $r\mathbf{v}$, whose faces are square to the cell's corners, blended with a ball by the roundness $k$: with $\mathbf{q} = F(\mathbf{z})$,

$$d = s^{-N}\Big[(1 - k)\max_{\mathbf{e} \in \{\mathbf{a}, \mathbf{b}, \mathbf{c}\}} \big(\mathbf{q}\cdot\mathbf{e} - r\,\mathbf{v}\cdot\mathbf{e}\big) + k\,(|\mathbf{z}| - r)\Big].$$

The folds and the twist are isometries, so only the scale has to be divided out. The orbit trap counts the reflections the first few folds made, which tells the copies apart.

**Origin.** Knighty introduced kaleidoscopic IFS on Fractal Forums in May 2010 ([Kaleidoscopic (escape time IFS)](https://www.fractalforums.com/sierpinski-gasket/kaleidoscopic-(escape-time-ifs)/)), with Sierpinski, Menger, octahedral, icosahedral and dodecahedral examples. Syntopia (Mikael Hvidtfeldt Christensen) wrote them up in [Folding Space II: Kaleidoscopic Fractals](https://blog.hvidtfeldts.net/index.php/2010/06/folding-space-ii-kaleidoscopic-fractals/) (2010) and shipped them as [Fragmentarium](https://github.com/Syntopia/Fragmentarium) examples (Icosahedron, Dodecahedron, Octahedron). The fold for any of the three groups and the vertex picked by weights on the cell's corners (a Wythoff construction) come from Knighty's later Fragmentarium work: his iterated icosahedral polyhedra (2011) and fold-and-cut regular polyhedra (2012, from the Fractal Forums thread "Solids, many many solids").

**Parameters.** `symmetry` (5): the mirror group, 3 tetrahedral, 4 octahedral, 5 icosahedral. `vertex` ([0, 0, 1]): the weights of the cell's corners (edge midpoint, vertex, face center) that pick the point copies are scaled away from; mixtures give truncated forms. The recipes: icosahedral flake `symmetry` 5, `vertex` [0, 1, 0], `fold_scale` 2.618; dodecahedral flake 5, [0, 0, 1], 3.618 (the default); octahedral flake 4, [0, 1, 0], 2; Sierpinski tetrahedron 3, [0, 1, 0], 2. `fold_scale` (3.618): how much smaller each generation is, 1.2 to 4.5; at the flakes' values the copies just touch, less fuses them into a solid, more scatters them into dust. `size` (1): the size of each copy's final shape against the vertex at 1, 0.2 to 1.3; smaller beads the copies apart, larger swells them together. `roundness` (0): 0 ends each copy as the polyhedron of that vertex (an icosahedron, a dodecahedron, a truncated form), 1 as a ball. `twist` (0): degrees each generation turns about its vertex's axis, -180 to 180; small turns wind the copies into whorls. `iterations` (6): generations of copies; more are added close up.

### Koch Delta (`koch`)

![Koch Delta](images/fractals/koch.jpg)

Eric Baird's Koch curve in three dimensions, the "delta" fractal: a triangular body whose copies hinge up out of its plane at the Koch angle, again and again, so that its edges and profile are Koch curves and its body is a crumpled crystal. The angle runs from a flat Koch tree at 60° to a solid at 90°.

**Formula.** With the Koch angle $\theta$, let $\beta = \theta/2$, $t = \tfrac14\sqrt{3\tan^2\beta - 1}$, the hinge normal $\mathbf{h} = (1, 0, -2t)/\sqrt{1 + 4t^2}$ and the scale $s = 4\cos^2\beta = 2(1 + \cos\theta)$. Start from $\mathbf{z} = \mathbf{p}$ and repeat: mirror $y \leftarrow |y|$, $z \leftarrow |z|$; fold the plane into the triangle's 60° wedge,

$$(x, y) \leftarrow (x, y) - 2\min\!\big(0, \tfrac{\sqrt3}{2}x - \tfrac12 y\big)\big(\tfrac{\sqrt3}{2}, -\tfrac12\big), \qquad y \leftarrow |y|;$$

fold the raised half of the Koch bump onto the flat one about their hinge through $\mathbf{o} = (\tfrac12, 0, t)$,

$$\mathbf{z} \leftarrow \mathbf{z} - 2\min\!\big(0, (\mathbf{z} - \mathbf{o})\cdot\mathbf{h}\big)\,\mathbf{h};$$

and scale about the corner: $\mathbf{z} \leftarrow s\,(\mathbf{z} - (1, 0, 0)) + (1, 0, 0)$. The unit ball holds the fractal and every image of itself, so it ends as a ball: $d = (|\mathbf{z}| - 1)\,s^{-N}$. The orbit trap records which copies were hinged up, in the first generations more than the later ones.

**Origin.** Eric Baird, [The Koch curve in three dimensions](https://www.researchgate.net/publication/262600735_The_Koch_curve_in_three_dimensions) (2014); Wikipedia's [Koch snowflake](https://en.wikipedia.org/wiki/Koch_snowflake) article lists it as the "Koch curve in 3D". Knighty coded it as a kaleidoscopic IFS in Fragmentarium, and the Mandelbulber team ported that to [Mandelbulber 2](https://github.com/buddhi1980/mandelbulber2) as Koch V3 ("Baird Delta"), which this is ported from. ray ends it in the unit ball rather than Mandelbulber's radius 3, which keeps it within its bound at every iteration count.

**Parameters.** `angle` (70): the Koch angle in degrees, 60 to 90. At 60 the copies lie flat, a planar Koch tree; more raises them out of the plane into a crumpled delta; toward 90 they pack into a solid. Each generation is $2(1 + \cos\theta)$ times smaller: 3 at 60°, 2.68 at 70°, 2 at 90°. `iterations` (8): generations of copies; more are added close up.

### Knotted Rope (`knot`)

![Knotted Rope](images/fractals/knot.jpg)

A torus knot whose tube is a rope: strands twisted about its core, each of them a rope of finer strands twisted the other way, and so on down, the way rope is laid. It is a fractal that keeps its shape: close up, every strand turns out to be a rope too.

**Formula.** With $\theta = \operatorname{atan2}(y, x)$ and $\rho = \sqrt{x^2 + y^2}$, the point's place in the meridian plane, $(\rho - 1, z)$, is turned by $(q/p)\,\theta$ and folded to the nearest of the $p$ strands at distance $r_g$ from the core circle: that is DarkBeam's torus knot of $p$ laps and $q$ turns. The strand leans across the plane by $\tan\alpha = r_g\,(q/p)/\rho$, so its round section is the plane's ellipse: the section is taken square to it, $(u, v) = (u, v\cos\alpha)/r_t$ for the tube radius $r_t$, and $\theta$ is moved along the strand to its nearest point, $\varphi$. Each rope level then repeats the knot's step about the strand's own axis:

$$\arg(u, v) \leftarrow \arg(u, v) + w_l\,\varphi, \quad \text{folded to the nearest of } n \text{ strands at } 1 - f, \quad (u, v) \leftarrow \big((u - (1 - f)),\ v\cos\beta_l\big)/f$$

with $f$ the strands' radius (a little over touching), $\beta_l$ their lean, and $w_l = \pm\operatorname{round}(w f^{1-l})$ the turns per lap, alternating in sign. Distance: at each level $l$, $(|(u, v)| - 1)\,r_t f^l$, no further than the nearest sibling strand at that level or above; the result is the largest of these. Far from a strand, it is divided by $\sqrt{1 + g^2}$, with $g$ bounding how much the twists stretch space. The orbit trap is the strand's index at each level, so every strand has its own color.

**Origin.** The knot is DarkBeam's "Not fractal but funny: trefoil knot routine" on Fractal Forums, refined by Knighty in 2012 (the "knot thingy" in [Fragmentarium](https://github.com/Syntopia/Fragmentarium)'s examples); Mandelbulber 2 has both as Knot V1 and V2. As those names say, it was not a fractal: the rope of ropes, with each level laid against the one above it, and the section taken square to the leaning strands are ray's own.

**Parameters.** `laps` (2) and `turns` (3): the torus knot $T(p, q)$, 1 to 8 and 0 to 8; 2 and 3 is the trefoil, 2 and 5 the cinquefoil, 3 and 4 a knot of 8 crossings; when they share a factor the knot falls into that many linked loops. `radius` (0.45): how far the knot strays from its core circle of radius 1, 0.1 to 0.6. `tube` (0.18): the rope's radius, 0.02 to 0.3; keep it under half the gap between passes. `strands` (3): strands in each rope, at every level, 2 to 6. `twist` (6): turns the strands make about their rope's core each lap of the knot, -24 to 24; each finer level turns proportionally faster, the other way; negative is the other hand, 0 lays them straight. `iterations` (4): levels, 1 a plain tube and each more a rope of finer strands; more are added close up.

## 4D slices

Four-dimensional fractals cut by a 3D slice. Each takes `w`, where the slice is, so a scene can slide it to animate the shape.

### Menger 4D (`menger4d`)

![Menger 4D](images/fractals/menger4d.jpg)

A slice through a four-dimensional Menger sponge (pictured at `w` = 0.15). Its holes run in all four directions, so a 3D slice cuts them at angles. Each slice is a cube, and sliding `w` changes its pattern of tunnels and pores.

**Formula.** Start from $z = (x, y, z, w)$ and repeat $n$ times: fold into the positive orthant, $z_i \leftarrow |z_i|$; sort so that $z_1 \ge z_2 \ge z_3 \ge z_4$; then

$$z_{1,2,4} \leftarrow s\,z_{1,2,4} - (s-1)\,c_{1,2,4}, \qquad z_3 \leftarrow s\,\bigl(h - |z_3 - h|\bigr),\quad h = \tfrac{(s-1)\,c_3}{2s}.$$

The two largest axes and the smallest are pushed toward the corners; the third is folded about its middle. Distance estimate: the distance from $z$ to the 4D box of half size $c_1$, divided by $s^n$ (past $c_1$ the largest axis only grows, so the box holds the whole sponge). The offsets here are scaled by $s - 1$, as in Knighty's original, so the sponge keeps its size as $s$ changes; Mandelbulber's defaults (1, 1, 1, 0.5) without that factor give the same shape at half the size. Iterations add finer holes close up on their own.

**Origin.** Syntopia's and DarkBeam's Menger4 formula for Mandelbulb 3D, released in DarkBeam's [custom formulas thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/) on Fractal Forums, built on Knighty's kaleidoscopic IFS folding (Fractal Forums). Ported from Mandelbulber 2's Menger 4D (`menger4d.cl`). Mandelbulber's optional six-plane rotation is left out.

**Parameters.** `w` (0): the slice's 4th coordinate, from −1 to 1; −w gives the same slice as w. At 0 the slice is a ragged cube with cross-shaped holes. From 0.1 to 0.2 each face has one square tunnel and fine pores, at 0.33 a grid of holes, and at 0.45 the block is nearly solid. At 0.6 each face has a ring of eight square holes, and near ±1 the block turns rugged and crumbling. Past ±1 the slice is empty. `fold_scale` (3): how much smaller each level's copies are. 3 is the sponge; from 1.5 to 3 the copies overlap into solid lattices, from 3 to 4 they part into dust. `offset` ([1, 1, 1, 0.5]): where the copies sit along each sorted axis, largest first. The first is also the sponge's half size; lowering any one pulls its copies in and fills holes. The 4th, at 0.5, puts the smallest axis's copies halfway, which is what makes the 4D holes cross the slices (0.05 to 1.5). `iterations` (5, up to 10): levels of holes, each three times finer.

### Sierpinski 4D (`sierpinski4d`)

![Sierpinski 4D](images/fractals/sierpinski4d.jpg)

A slice through a four-dimensional Sierpinski solid: eight half-size copies on the corners of a 16-cell, each made the same way. At `w` = 0 (pictured) the slice is a cuboctahedron walled with Sierpinski triangles; at ±1 it is the Sierpinski tetrahedron.

**Formula.** Start from $z = (x, y, z, w)$ and repeat $n$ times: for each pair of axes in turn, $(x,y), (x,z), (y,z), (x,w), (y,w), (z,w)$, reflect if their sum is negative,

$$z_i + z_j < 0:\quad (z_i, z_j) \leftarrow (-z_j, -z_i),$$

then scale about the corner $(1, 1, 1, 1)$: $z \leftarrow s\,z - (s-1)$. The folds bring the point near the one corner of the 16-cell (the corners of the hypercube with an even number of −1s) it is closest to. Distance estimate: the distance to that 16-cell, scaled back,

$$d = \max\Bigl(\max_i |z_i| - 1,\ \tfrac12 \max_i \bigl|\textstyle\sum_j z_j - 2 z_i\bigr| - 1\Bigr) / s^n.$$

Its eight cells $|z_i| = 1$ are tetrahedra, which is why the slices at ±1 are the 3D Sierpinski tetrahedron and its mirror image. Iterations add finer copies close up on their own.

**Origin.** Syntopia's and DarkBeam's Sierpinski 4D for Mandelbulb 3D (DarkBeam's [custom formulas thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/) on Fractal Forums), the 4D form of the folding Sierpinski tetrahedron in Syntopia's [Distance Estimated 3D Fractals III: Folding Space](http://blog.hvidtfeldts.net/index.php/2011/08/distance-estimated-3d-fractals-iii-folding-space/) (2011). Ported from Mandelbulber 2's Sierpinski 4D (`sierpinski4d.cl`). Mandelbulber's per-axis offset and six-plane rotation are left out, so the base shape is always the 16-cell.

**Parameters.** `w` (0): the slice's 4th coordinate, from −1 to 1. At 0 the slice is a cuboctahedron walled with Sierpinski triangles. At 0.5 it is four half-size cuboctahedra on the corners of a tetrahedron, at 0.75 sixteen quarter-size ones; in general $w = 1 - 2^{-k}$ gives $4^k$ of them. Between those values the slice is spiky lace. At 1 it is exactly the Sierpinski tetrahedron and at −1 its mirror image; past ±1 the slice is empty. `fold_scale` (2): how much smaller each level's copies are. At 2 they touch; from 1.5 to 2 they overlap into a solid, from 2 to 3 they part into dust. `iterations` (10, up to 20): levels of copies, each half the size.

### Mandelbox 4D (`mandelbox4d`)

![Mandelbox 4D](images/fractals/mandelbox4d.jpg)

A slice through the four-dimensional Mandelbox (pictured at `w` = 1.2). At `w` = 0 it is exactly the ordinary Mandelbox. Other slices keep its cube, about 4 across at negative `fold_scale` and up to 12 at 2, but swap its round pits for square studs and back.

**Formula.** Start from $z = c = (x, y, z, w)$, $\mathrm{d}r = 1$, and repeat:

$$z \leftarrow s\,k\,\mathrm{boxFold}(z) + c, \qquad \mathrm{d}r \leftarrow |s|\,k\,\mathrm{d}r + 1,$$

with the box fold $f(t) = |t + l| - |t - l| - t$ on each of the four axes (limit $l$), and the sphere fold's factor $k = 1/r_{\min}^2$ if $|z|^2 < r_{\min}^2$, $1/|z|^2$ if $|z|^2 < 1$, else 1, using the 4D radius. Distance estimate: $d = |z| / |\mathrm{d}r|$, stopping once $|z| > 10^5$. Mandelbulber stops at 100, which leaves the estimate jumping between escape bands far from the surface.

**Origin.** Tom Lowe's (Tglad's) Mandelbox ([Fractal Forums, 2010](https://www.fractalforums.com/3d-fractal-generation/amazing-fractal/)), carried to four dimensions as Mandelbulber 2's ABox 4D (`abox4d.cl`) by the Mandelbulber team. Its scale-varying, parabolic and rotation extras are left out.

**Parameters.** `w` (0): the slice's 4th coordinate, useful from −2 to 2 at negative `fold_scale` and −6 to 6 at 2. At 0 it is the ordinary Mandelbox. At the default −2 the faces carry round pits near 0 and turn to square studs by 0.75. At ±1 the slice is a plain smooth cube: the 4th coordinate alone keeps the radius at 1 or more, so the sphere fold never acts. Pits and bubbles return from 1.2, then the slice breaks into specks and empties just past ±2. `fold_scale` (−2): the scale each step. From −1.5 to −2.5 it gives crumpled cubes, from 2 to 3 boxy cities; near ±1 it fattens into a blob (−3 to 3). `min_radius` (0.5): the sphere fold's inner radius. Smaller values give finer, sharper detail; 1 turns the sphere fold off (0.05 to 1). `fold` ([1, 1, 1, 1]): the box fold's limit on x, y, z and w. Lower values fold more tightly; the w limit sets how the 4th axis folds in (0.25 to 2). `iterations` (12, up to 40): more gives finer, sharper detail.

### Kleinian 4D (`kleinian4d`)

![Kleinian 4D](images/fractals/kleinian4d.jpg)

A slice through a four-dimensional pseudo-Kleinian (pictured at `w` = 0.2): box folds and a 4D sphere inversion, drawn around a vertical pillar and a floor plane. Each slice is a hall of floors, columns, domes and arches. Like the 3D one it fills space, so it is cut to a box here.

**Formula.** Start from $z = (x, y, z, w)$, $D = 1$, and repeat:

$$z \leftarrow k\,\mathrm{boxFold}(z), \qquad k = \max\bigl(a / |z|^2,\ 1\bigr), \qquad D \leftarrow k\,D,$$

with the box fold $f(t) = |t + l| - |t - l| - t$ on each axis. Distance estimate: the distance to the pillar (the y axis, in 4D) or the floor ($y = 0$), whichever is nearer, scaled back and halved as Mandelbulber does:

$$d = \tfrac12\Bigl(\min\bigl(\sqrt{x^2 + z^2 + w^2} - \rho,\ |y|\bigr) - t\Bigr) / D.$$

Mandelbulber's z axis is up; here it is y, so the floors lie flat.

**Origin.** TGlad's 4D pseudo-Kleinian, from the fractalforums.org thread ["New sphere tree"](https://fractalforums.org/fractal-mathematics-and-new-theories/28/new-sphere-tree/3557/msg22100#msg22100), with extras by sabine62. It builds on Knighty's pseudo-Kleinian (Fractal Forums). Ported from Mandelbulber 2's Pseudo Kleinian 4D (`pseudo_kleinian4d.cl`; definition dated 2020). The defaults use Knighty's classic fold sizes and a larger inversion rather than Mandelbulber's 1 and 1, which give sparse slices that lose everything but the floor away from w = 0. Mandelbulber's 0.05 derivative tweak and optional extras are left out.

**Parameters.** `w` (0): the slice's 4th coordinate (−2 to 2). At 0 it is the 3D form, a closed block of walls with round windows. At 0.2 goblet-topped columns stand on a floor, at 0.4 they become arches, at 0.6 floating crowns, and near 0.9 the floor is bare but for beads. The slices repeat every 2 × the w fold (1.8 by default): −w and w match, and so do 1.8 − w and w. `fold` ([0.92436, 0.90756, 0.92436, 0.9]): the box fold's limit on x, y (up), z and w, half the repeating cell on each axis. Just under 1 keeps the spheres from lining up with the cells, for richer halls; 1 gives tidy, sparse ones (0.25 to 2). `inversion` (1.5): the inversion sphere's radius squared. Larger values fill the halls with walls, domes and round windows and keep more of it away from w = 0; smaller ones leave bare floors and beads (0.25 to 2). `radius` (0.1): the columns' radius around the vertical axis, in the folded space; 0 leaves only the floors and domes (0 to 0.5). `thickness` (0.02): thickens the floors and columns alike (0 to 0.2). `iterations` (10, up to 30): more gives finer floors and columns.

## More

Named formulas from the 3D fractal community that fit no family.

### Xenodreambuie (`xenodreambuie`)

![Xenodreambuie](images/fractals/xenodreambuie.jpg)

A Mandelbulb built on true spherical coordinates whose polar angle is folded back into one hemisphere before it is multiplied. The result is mirrored top to bottom and covered in stacked, petal-like domes rather than the classic bulb's swirls.

**Formula.** Start from $z = p$ and repeat, with $r = |z|$ and $y$ as the polar axis:

$$\theta = n\,(\mathrm{atan2}(z_z, z_x) + \beta), \qquad \varphi = n\,\mathrm{fold}(\arccos(z_y / r) + \alpha)$$

$$z \leftarrow r^n (\sin\varphi\cos\theta,\ \cos\varphi,\ \sin\varphi\sin\theta) + p$$

with $\mathrm{fold}(\varphi) = \mathrm{sign}(\varphi)\,\pi - \varphi$ when $|\varphi| > \pi/2$. With $\alpha = 0$ the fold amounts to replacing $z_y$ by $|z_y|$. Bailout $r \ge 4$. Distance estimate: $d = \tfrac12 r \ln r / \partial r$ with $\partial r \leftarrow n\,r^{n-1}\partial r + 1$, as for the Mandelbulb. The orbit trap is the closest the orbit comes to the equator, the fold's mirror plane.

**Origin.** Garth Thornton, the author of XenoDream, who posts on Fractal Forums as xenodreambuie. In October 2009, in the "True 3D mandelbrot type fractal" thread that produced the Mandelbulb, he suggested replacing White and Nylander's latitude form ($z = r\sin\varphi$) with standard spherical coordinates ($\varphi = \arccos(z/r)$, $z = r\cos\varphi$), which suit inverse iteration. On October 17, 2009 he posted the forward version with the fold ("ph = acos(zj/magn); if ph>pi/2 then ph = pi-ph"), [msg8281](https://web.archive.org/web/20091124172646/http://www.fractalforums.com:80/3d-fractal-generation/true-3d-mandlebrot-type-fractal/msg8281/), and later called it the "flipped root cosine triplex". Mandelbulber added it in version 0.80 as "xenodreambuie's fractal formula", citing his image post [msg8246](https://web.archive.org/web/20091126041703/http://www.fractalforums.com:80/3d-fractal-generation/true-3d-mandlebrot-type-fractal/msg8246/). Ported from Mandelbulber 2's `xenodreambuie`.

**Parameters.** `power` (9): the exponent $n$, from 2 to 16. Mandelbulber's default is 9; higher powers give more and finer domes around the equator and over the poles. `polar_offset` (0): degrees $\alpha$ added to the polar angle before the fold, from -180 to 180. Small offsets break the top-bottom mirror and turn the domes into crowns and shells. `azimuth_offset` (0): degrees $\beta$ added to the angle around the axis before it is multiplied, from -180 to 180. It turns the lobes about the axis at every step, shifting and shearing the pattern. `iterations` (10, at most 32): more iterations give finer detail.

### Pine Tree (`pine_tree`)

![Pine Tree](images/fractals/pine_tree.jpg)

A quadratic Mandelbrot set spun about its needle: the angle around the needle doubles at every step, and the two cross axes swap. Stood on its needle, it becomes a tiered, drooping conifer topped by a spire of tiny minibrots.

**Formula.** In Mandelbulber's frame, where $x$ runs along the needle (here it points up), with $\rho = \sqrt{y^2 + z^2}$, start from $z = p$ and repeat:

$$x \leftarrow x^2 - \rho^2 + m_x p_x, \qquad y \leftarrow \tfrac{2x}{\rho}\,2yz + m_z p_z, \qquad z \leftarrow \tfrac{2x}{\rho}\,(y^2 - z^2) + m_y p_y$$

then turn $(y, z)$ by the twist angle. In the meridian plane $(x, \rho)$ this is complex squaring; around the axis it doubles the angle and then swaps $y$ and $z$, with the point's $y$ and $z$ swapped to match. Bailout $|z| \ge 8$. The map stretches up to twice as much around the needle as along it, so a scalar derivative is far off. The kernel carries the orbit's Jacobian $J \leftarrow Df\,J + C$ and estimates $d = \tfrac12 r \ln r / g$ from the escape potential's gradient $g = |J^{\mathsf T} z| / r$. It blends that estimate with the previous step's according to how far the escape overshot the bailout (in $\ln\ln r$), so the estimate does not jump where the escape count changes. Where an orbit grazes the needle the gradient swings sharply, so $g$ is kept to at least a tenth of $\lVert J \rVert_F$, and the march step is 0.25. The orbit trap is the closest the orbit comes to the needle.

**Origin.** The map is Paul Nylander's (bugman) [3D Christmas Tree Mandelbrot Set](http://bugman123.com/Hypercomplex/index.html#ChristmasTree) from December 2009: $r^n(\cos\theta, \sin\theta\cos\varphi, \sin\theta\sin\varphi)$ with $\theta = n\arccos(x/r)$ and $\varphi = n\,\mathrm{atan2}(z, y)$. M. Benesi (M Benesi on Fractal Forums) was using it by June 2010, calling it the "Christmas tree fractal (Name from Bugman's website)" in the thread [Question about Formula](https://web.archive.org/web/20120229053814/http://www.fractalforums.com:80/mandelbulb-3d/question-about-formula/). On October 9, 2015 he posted the trig-free power-2 form, with y and z switched "to add symmetry", in the thread [Do M3D formula have to be Distance Estimation formulas?](https://web.archive.org/web/20250728083538/http://www.fractalforums.com/new-theories-and-research/do-m3d-formula-have-to-be-distance-estimation-formulas/), later retitled Benesi "pinetree" (fold+bulb); by then he called it the pine tree. Mandelbulber's `benesi_pine_tree` ("benesiFastPwr2PineTree") ports that form, and Mandelbulb 3D has it as BPine_only. Ported from Mandelbulber 2. Mandelbulber adds only the point's x by default (1, 0, 0), which gives a smooth, banded pagoda; the default here adds a quarter of the cross coordinates as well.

**Parameters.** `c_scale` (1, 0.25, 0.25): how much of the point's height and of its two cross coordinates is added at each step, each from -2 to 2. Mandelbulber's 1, 0, 0 is a smooth, banded pagoda. Raising the last two drapes and tears the tiers, and 1, 1, 1 (the full Mandelbrot type) is a ragged storm. `twist` (0): degrees the cross section turns about the needle at every step, from -180 to 180; it spirals the tiers. `iterations` (14, at most 30): more iterations give finer tiers and a longer spire.

### Makin (`makin`)

![Makin](images/fractals/makin.jpg)

A 3D squaring with a twist in its cross terms. It comes out as a swept, three-cornered sail under a spire, its edges frayed into spikes.

**Formula.** Start from $z = p$ and repeat:

$$z \leftarrow (x^2 + 2yz,\ -y^2 - 2xz,\ -z^2 + 2xy) + (1 - j)\,p + j\,c$$

where $j$ morphs from the Mandelbrot type ($j = 0$) to the Julia set of $c$ ($j = 1$). Bailout $|z| \ge 8$. Like the pine tree, the map is far from conformal. The kernel carries the orbit's Jacobian and estimates $d = \tfrac12 r \ln r / g$ from the potential's gradient $g = |J^{\mathsf T} z| / r$, blended across the bailout, with a march step of 0.4. The set is turned a quarter turn about $y$, so the default view looks along the sail, and centered. The orbit trap is the closest the orbit comes to the formula's own $y$ axis.

**Origin.** David Makin, an Ultra Fractal formula author and Fractal Forums moderator. The map is "WIP3D5" from his Ultra Fractal formula file MMFwip3D.ufm, which was circulating by mid-2010. Luca GN (DarkBeam) ported it to Mandelbulb 3D in 2011 as [Makin3D-2](https://raw.githubusercontent.com/thargor6/mb3d/master/M3Formulas/Makin3D-2.m3f). When Makin first posted the -2 variant is unknown. Mandelbulber cites the thread [Another shot at the holy grail](https://web.archive.org/web/20150908093646/http://www.fractalforums.com/3d-fractal-generation/another-shot-at-the-holy-grail/), started by quick-dudley in November 2012. There, in January 2015, DarkBeam noted that the squaring derived in the thread was almost identical to Makin3D-2. Makin himself did not post in it. Ported from Mandelbulber 2's `makin3d2`.

**Parameters.** `julia` (0): from the Mandelbrot type (0, the point added at each step) to the Julia set of `c` (1); values in between morph one into the other. `c` (-0.5, 0.5, 0.5): the Julia constant, each component from -1.5 to 1.5, used to the extent that `julia` is above 0. Near the origin it gives a solid, cushion-like lump; further out the lump becomes swirled and holed, then breaks into dust. `iterations` (14, at most 30): more iterations give finer spikes.

### Ides (`ides`)

![Ides](images/fractals/ides.jpg)

A mixed quadratic and cubic map that grows a mirror-symmetric, moth-like body of hollow wings, ribs and hair-fine spikes. Seen edge-on it shows a long stem; from behind, a smooth hollow.

**Formula.** Start from $z = p$ and repeat: shrink $x$ and $z$ by a tenth, $x' = s(x)$ and $z' = s(z)$ with $s(u) = 0.9u$, then

$$z \leftarrow \big(m_1 x'^2 - s_1(y^2 + z'^2),\ m_2\,x' y z',\ m_3 z'^2 - s_2(x'^2 + y^2)\big) + p$$

with $(m_1, m_2, m_3) = (1, 2, 1)$ and $s_1 = s_2 = \tfrac12$ for Ide's formula. Bailout $|z| \ge 10$, as in Mandelbulber. Mandelbulber shrinks only inside $|u| < 2.5$, a jump that no distance estimate can follow. Here the shrink eases into slope 1 between 2.5 and 3; the set itself lies within 2.5. The map mixes degrees and is far from conformal, and its potential's gradient varies too sharply to march by. The estimate therefore uses the Jacobian's Frobenius norm, which bounds its largest stretch: $d = \tfrac12 r \ln r / \lVert J \rVert_F$, blended across the bailout as for the pine tree, with a march step of 0.3. The map is symmetric under swapping $x$ and $z$. It is turned 135 degrees about $y$ so that this mirror plane faces the default view, and centered. The orbit trap is the closest the orbit comes to the planes $x = 0$ and $z = 0$.

**Origin.** Trafassel, the author of the Gestaltlupe renderer, started the Fractal Forums thread [Formula 21 (Julia Set Interpretation)](https://web.archive.org/web/20110316072858/http://www.fractalforums.com:80/videos/formula-21-(julia-set-interpretation)/) on February 16, 2010. In June 2010 he called it "Formula 21 (Ide's Fractal)" and noted that it has some nice properties but is no candidate for a true 3D Mandelbrot set. Who Ide is, the thread does not say. Mandelbulb 3D's IdesFormula ("a formula made by Trafassel") uses the same multipliers (1, 2, 1) and (0.5, 0.5), as does Mandelbulber's `ides`. The 0.9 shrink is not in Trafassel's 2010 loop; it appears in Mandelbulber's version, whose `ides2` leaves it out. Ported from Mandelbulber 2's `ides`.

**Parameters.** `mult` (1, 2, 1): the multipliers of $x^2$, $xyz$ and $z^2$, each from -3 to 3. 1, 2, 1 is Ide's; the middle one sets how spiky the ribs are. `sub` (0.5, 0.5): how much of the other two squares is subtracted from $x$ and from $z$, each from -2 to 2. Lower values grow the body and its spikes; higher values shrink it to a compact, blade-edged core. `iterations` (12, at most 30): more iterations give finer spikes.

### Distance IFS (`difs`)

![Distance IFS](images/fractals/difs.jpg)

A Menger sponge copied smaller and folded outward ring after ring, so one big sponge stands on a twisting skirt of ever smaller ones that thins to a carpet. Every copy is a whole, crisp sponge, because the distance is measured to real copies rather than estimated from an escaping orbit.

**Formula.** A DIFS ("distance IFS") turns the usual fractal recipe inside out. An ordinary iterated function system folds and scales space and then judges by how fast a point runs away. A DIFS instead places an ordinary shape (here a Menger sponge) in every frame the folds pass through, measures the exact distance to it there, and divides by the scale built up so far. The minimum over all the frames is the distance to the union of all the copies. Because folds, turns and uniform scalings never stretch distances unevenly, the result is exact to the copies; only the sponge's own distance is knighty's folded estimate, which stays on the safe side in its tunnels. Start from $z = p$ (with $y$ vertical), $d = \infty$, and for each ring $i = 0, 1, \dots$:

$$d \leftarrow \min\left(d,\ s^{-i}\, D_\text{sponge}(z)\right)$$

$$z_x \leftarrow s\,|z_x| - (s-1)\,o_x, \quad z_z \leftarrow s\,|z_z| - (s-1)\,o_z, \quad z_y \leftarrow s\,z_y - (s-1)\,o_y$$

then turn $z$ by $\theta$ about the vertical. The absolute values fold space into one quadrant and the scaling about $o$ pushes the next ring out to the corners, so ring $i$ holds $4^i$ copies $s^i$ times smaller, sinking toward height $o_y$. With `fold_up` the vertical is folded too ($s\,|z_y|$), and the copies spread to all eight corners. $D_\text{sponge}$ is knighty's Menger sponge of half size $h$: scale $z$ by $1/h$, then repeat `sponge_levels` times: take $|z|$, sort so $z_y$ is the least, fold $z_y \leftarrow \tfrac23 - z_y$ when $z_y < \tfrac13$, and set $z \leftarrow 3z - 2$; the sponge's distance is the distance to the box $[-1, 1]^3$ times $h / 3^\text{levels}$.

**Origin.** knighty posted the idea as `mdifs` ("mdifs by knighty (jan 2012)") in [Fragmentarium's Knighty Collection](https://github.com/Syntopia/Fragmentarium/blob/master/Fragmentarium-Source/Examples/Knighty%20Collection/mdifs.frag): a fold-and-scale iteration with a Menger sponge measured at every step. Darkbeam built a whole family of dIFS formulas for Mandelbulb 3D around the same idea, with boxes, cylinders, tori and more shapes in place of the sponge; they circulated in Fractal Forums' [custom formulas and transforms thread](http://www.fractalforums.com/mandelbulb-3d/custom-formulas-and-transforms-release-t17106/). Mandelbulber 2 gathered them as its DIFS formulas; this one follows its [DIFS Menger](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/difs_menger.cl), with knighty's exact $s^{-i}$ scaling instead of Mandelbulber's derivative-plus-one, and a turn per ring.

**Parameters.** `fold_scale` (2.3): how much smaller each ring is than the last (1.5 to 3); about 2 packs the copies edge to edge into a carpet, higher leaves gaps that show the branching, lower crowds them into a dense mound. `offset` ([2, -1.2, 2]): x and z set how far out each ring is folded (0 to 2; negative folds the rings back into the big sponge); y is the height the rings sink to (negative: below the big sponge; positive: they climb above it; with `fold_up`, how far above and below). `size` (0.55): the sponge's half size (0.1 to 1); small leaves the copies apart, about 0.5 lets them touch, larger merges them into a stepped mass. `sponge_levels` (3): the sponge's own Menger levels (0 to 6); 0 copies plain cubes. `turn` (12): degrees each ring turns about the vertical; 0 lines the copies up in a square, 10 to 30 twists the skirt into spiral arms. `fold_up` (0): 1 folds the vertical too, for a cluster of sponges in all eight directions instead of a mound. `iterations` (8): rings of copies (up to 12); close up, more come in by themselves. The orbit texture colors each copy by its ring.

### msltoe Sym4 (`msltoe_sym`)

![msltoe Sym4](images/fractals/msltoe_sym.jpg)

A bulb whose point is sorted and sign-matched before every squaring, so it grows with the symmetry of a cube. Its Julia sets are ornate, carved knots of bulbs on bulbs; the default is the one at $c = (-1, 0, 0)$.

**Formula.** Start from $z = p$ and repeat: swap coordinates so that $|z_x| \ge |z_y| \ge |z_z|$ (x with z, x with y, then y with z, each when the second is larger in magnitude), give $z_y$ and $z_z$ the sign of $z_x$, then

$$z \leftarrow \left(z_x^2 - z_y^2 - z_z^2,\ 2 z_x z_y,\ 2 z_x z_z\right) + c$$

with $c = p$ (Mandelbrot form) or the Julia constant, and the $(y, z)$ part turned by the `twist` about the x axis. This square is the quaternion square restricted to three dimensions, so $|z|^2$ is squared exactly, and the sorting fold is a symmetry of the cube, which is what keeps the set from being a plain solid of revolution. Distance estimate: $d = \tfrac12\, r \ln r / \partial r$ with $\partial r \leftarrow 2 r\, \partial r + 1$, bailing out at $r = 1000$.

**Origin.** msltoe started the Fractal Forums thread [Choosing the squaring formula by location](https://www.fractalforums.com/theory/choosing-the-squaring-formula-by-location/) on 29 November 2009, proposing to pick a different squaring for each region of space, the way an IFS applies symmetry operators. Through early 2010 he posted a series of symmetrized squarings in it, with four-, eight- and sixteen-fold variants. The Sym2, Sym3 and Sym4 versions became formulas for Mandelbulb 3D (MsltoeSym2/3/4); youhn posted their code to the thread on 27 January 2015. This is Mandelbulber 2's [Msltoe Sym4 Mod](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/msltoe_sym4_mod.cl) at its defaults, with its rotation reduced to a twist about x.

**Parameters.** `julia` (1): 1 draws the Julia set of `c`; 0 the Mandelbrot form, where each point is its own constant (a lopsided, whipped shape). `c` ([-1, 0, 0]): the Julia constant (-1.5 to 1.5 per axis); on the x axis it keeps the symmetry, and -0.9 to -1.3 gives the richest knots; off the axis the knot warps, and far from the center it falls apart into dust. `twist` (0): degrees the square turns about the x axis at each step; 10 to 30 opens the knot into lace. `iterations` (10): escape-time iterations (up to 30); more adds finer bulbs on the bulbs.

### eiffie's bulb (`eiffie`)

![eiffie's bulb](images/fractals/eiffie.jpg)

msltoe's squaring with the point first folded into a one-sixteenth wedge around the vertical axis, so the bulb grows eight-fold: in its Mandelbrot form, a fluted stone vase standing on a long spike. Its Julia sets with the constant on the axis are knobbed spindles, eight-pointed in cross-section.

**Formula.** In a frame whose x axis is the vertical, start from $z = p$ and repeat: fold the angle around the axis, $\psi = \operatorname{atan2}(z_z, z_y)$, into $[0, \pi/8]$,

$$\psi \leftarrow \left|\left(\psi + \tfrac{9\pi}{8}\right) \bmod \tfrac{\pi}{4} - \tfrac{\pi}{8}\right|, \qquad (z_y, z_z) \leftarrow \rho\,(\cos\psi, \sin\psi),\ \rho = \sqrt{z_y^2 + z_z^2}$$

then apply msltoe's squaring, with $m = 1 - z_z^2 / |z|^2$ (the squared cosine of the latitude):

$$z \leftarrow \left((z_x^2 - z_y^2)\,m,\ 2 z_x z_y\, m\, k_y,\ 2 z_z \sqrt{z_x^2 + z_y^2}\right) + c$$

with $c = p$ or the Julia constant and $k_y$ the `y_scale`. The fold is a reflection or rotation, so it costs the estimate nothing; the squaring, though, does not square $|z|$ exactly (between about 0.73 and 1.03 times $|z|^2$ inside the wedge). The log estimate $d = k\, r \ln r / \partial r$, $\partial r \leftarrow 2 r\, \partial r + 1$, therefore bails out far away ($r = 1000$, so it changes smoothly between escape counts), and uses $k = 0.75$ instead of the textbook 0.5: measured along rays, the textbook estimate stays below 0.75 of the true distance, and the shading's occlusion reads the estimate, so a too-small one darkens the surface. The march steps 0.9 of it.

**Origin.** msltoe first folded the angle into eight sectors in the thread [Choosing the squaring formula by location](https://www.fractalforums.com/theory/choosing-the-squaring-formula-by-location/) (19 March 2010), with a loop subtracting $\pi/4$ until the angle fell below $\pi/8$, applied before this squaring. eiffie posted the closed-form fold, the `fmod` line above, in the same thread on 27 January 2015 (reply #69), noting it was meant for Julia sets. Ported from Mandelbulber 2's [Msltoe - Julia Bulb Eiffie](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/eiffie_msltoe.cl). In the Mandelbrot form the added $c = p$ is not folded, so the eight-fold symmetry is only approximate; in a Julia set with $c$ on the axis it is exact.

**Parameters.** `y_scale` (1): stretches the squaring's sideways term (0.8 to 1.2); 1 is eiffie's, lower swells the vase and frays its rim into fur, higher shrinks it fast (at 1.2 little is left). `julia` (0): 0 is the Mandelbrot form (each point its own constant); 1 draws the Julia set of `c`. `c` ([0, -1, 0]): the Julia constant (-1.5 to 1.5 per axis); on the vertical axis the Julia set keeps the eight-fold symmetry ((0, -1, 0) is a knobbed spindle, eight-pointed in cross-section), and (0, 0, -1) flattens it into a rosette ringed by satellites. `iterations` (10): escape-time iterations (up to 30).

### Mandelnest (`mandelnest`)

![Mandelnest](images/fractals/mandelnest.jpg)

A power-9 bulb built axis by axis: each coordinate's angle to its own axis is multiplied, instead of the Mandelbulb's two spherical angles, so the result has a cube's symmetry rather than a pole. It is a ball of nested, pinched lobes, with star-shaped seams where they meet and a porthole on each axis.

**Formula.** Start from $z = p$ and repeat, with $r = |z|$ and power $n$:

$$v_i = \cos\!\left(n \arccos\frac{z_i}{r} + \varphi\right) \quad (i = x, y, z), \qquad z \leftarrow r^{\,n-1}\,\frac{v}{|v|} + c$$

with $c = p$ (Mandelbrot form) or the Julia constant, and $\varphi$ the `shift`. $\cos(n \arccos t)$ is the Chebyshev polynomial $T_n(t)$ for whole $n$; in a plane through two axes with $n = 9$ the step is $z^9 / |z| + c$, the power-9 Mandelbrot's angles with the radius raised only to the 8th. Distance estimate: $d = \tfrac12\, r \ln r / \partial r$ with $\partial r \leftarrow n\, r^{\,n-2}\, \partial r + 1$, bailing out at $r = 10$. Mandelbulber's derivative also divides by $|v|$, but that makes the estimate jump wherever $|v|$ dips. Without it, the estimate measured along rays stays below the true distance except in rare spots (at most 1.4 times it), so the march steps 0.8 of it. The orbit texture follows the smallest $|v|$ along the orbit, which is low on the seams between lobes.

**Origin.** Jeannot shared the Mandelnest on Fractal Forums ([Mandelbrot 3D: Mandelnest](https://fractalforums.org/share-a-fractal/22/mandelbrot-3d-mandelnest/4028/)) and as a [Shadertoy](https://www.shadertoy.com/view/4lKfzy). Ported from Mandelbulber 2's [Mandelnest](https://github.com/buddhi1980/mandelbulber2/blob/master/mandelbulber2/formula/opencl/mandelnest.cl).

**Parameters.** `power` (9): the power each axis's angle is multiplied by (5 to 16); higher packs in more, smaller lobes. Odd whole powers keep the cube's mirror symmetry; others break it. `shift` (0): a phase added to every multiplied angle, in half turns (-1 to 1); 0.1 to 0.3 swells some lobes and sinks others into a blockier, more cubic form. `julia` (0): 0 is the Mandelbrot form (each point its own constant); 1 draws the Julia set of `c`. `c` ([-0.8, 0, 0]): the Julia constant (-1.5 to 1.5 per axis); (-0.8, 0, 0) is a banded barrel, and nearer the center it is a lumpy ball. `iterations` (8): escape-time iterations (up to 30); more adds finer lobes along the seams.

## Originals

Formulas invented for ray: new constructions, not new parameters for old ones.

### Seifert Gasket (`seifert_gasket`)

![Seifert Gasket](images/fractals/seifert_gasket.jpg)

A ring of knotted strands of every size, packed like the circles of an Apollonian gasket. Every strand is a torus knot (a trefoil by default), and any two strands are linked. Right: the ring cut open, showing the strands' cross-sections, which are the gasket's circles.

**Formula.** The 3-sphere $S^3 \subset \mathbb{C}^2$ is filled by the orbits of the circle action $(z_1, z_2) \mapsto (e^{ipt} z_1, e^{iqt} z_2)$: a Seifert fibration whose fibers are $(p, q)$ torus knots (for $p = q = 1$, the Hopf fibration). The function

$$u = z_2^{\,p} / z_1^{\,q}$$

is constant on each fiber, so it is a coordinate on the orbit space. A point $x$ of space goes to $S^3$ by inverse stereographic projection, $(z_1, z_2) = (2x, |x|^2 - 1)/(1 + |x|^2)$, and then to $u$. In the $u$-plane sits an Apollonian-type gasket inside the unit circle: $n$ circles of radius $r = \sin(\pi/n)/(1 + \sin(\pi/n))$ in a ring around a central circle of radius $1 - 2r$. Every gap between them is a triangle of mutually tangent circles, so the packing is completed by repeated inversion. Start from $u$ and repeat: turn $u$ into the wedge $0 \le \arg u \le \pi/n$ (with a mirror), stop if it lies inside the big circle on the axis or the central circle, and otherwise invert it in the dual circle of the gap it is in (the circle through the gap's three tangency points, orthogonal to its sides). Each strand is the preimage of a disk, narrowed in that disk's own frame by `gap` of its radius plus `spacing`. The disk $|u| \le 1$ lifts to a solid torus that stereographic projection makes round: the envelope, with major radius $1/a$ and minor radius $b/a$, where $b^p = a^q$ and $a^2 + b^2 = 1$.

Distance estimate. The circle action is by isometries, so the distance in $S^3$ from a point to a union of fibers is the distance in the orbit space. Since $u$ is holomorphic on $\mathbb{C}^2$, its stretch is exactly $|\nabla u| = |u|\sqrt{q^2/|z_1|^2 + p^2/|z_2|^2}$. Stereographic projection multiplies lengths by $2/(1 + |x|^2)$, and each inversion in a dual circle of radius $R$ about $m$ by $R^2/|u - m|^2$. With $K$ the product of these factors, the estimate is

$$d = \min\big(|u - c| - (r(1-g) - wK),\ r - |u - c| + wK\big)/K$$

in the frame where $u$ landed in the circle of center $c$ and radius $r$. The first term is the narrowed strand. The second bounds every other strand: they all lie beyond the circle's rim, at least `spacing` ($w$) further in. Outside the envelope, the estimate is the torus's exact distance plus $w$. The inversions are carried to first order; with a step of 0.75 the estimate passes the steepness test at the defaults and at every corner of the parameter ranges we tried. The shape is scaled so that the ring's outer radius is 2 units whatever the knot.

**Origin.** Designed for this catalog (2026); to our knowledge new. We searched the web, Fractal Forums and Shadertoy ("Hopf fibration Apollonian gasket fractal linked tori", "Seifert fibration torus knots fractal shadertoy", "Hopf fibration fractal pullback Julia set", "fractalforums hopf fibration fractal distance estimator", "shadertoy hopf apollonian gasket raymarch"), Mandelbulber 2's formula list (its knotted formulas are `knot_v1`, `knot_v2` and `msltoe_donut`) and Fragmentarium's examples (`Knot.frag`, the polychora scripts, the stereographic Julia sets), and found no Hopf- or Seifert-fibered fractal and no fractal pulled back along the Hopf map. Nearest relatives: pictures of the Hopf fibration as nested linked tori (Niles Johnson's animations; [3D-XplorMath's Hopf-fibered linked tori](https://virtualmathmuseum.org/Surface/hopf_fibered/hopf_fibered.html)); Dror Bar-Natan's [Planet Hopf](https://www.math.utoronto.ca/drorbn/Gallery/KnottedObjects/PlanetHopf/index2.html), which pulls a map of the Earth back along the Hopf map; DarkBeam's and knighty's torus-knot distance estimators (Fractal Forums, 2012; Mandelbulber 2's `knot_v1` and `knot_v2`, Fragmentarium's `Knot.frag`), which draw a single knotted bundle of tubes; and the Apollonian gasket itself.

**Parameters.** `turns` (2): how many times each strand winds around the ring's axis, 1 to 5. `twists` (3): how many times it winds around the ring's tube, 1 to 8. Together they pick the knot: 2 and 3 make trefoils, 2 and 5 cinquefoils, and 1 and 1 plain rings that link pairwise like the Hopf fibration. Coprime pairs give one knot per strand; others give several parallel ones. `strands` (3): the big strands around the core, 3 to 8. `gap` (0.35): the space around each strand as a share of its thickness, 0 to 0.7; 0 packs the strands into one solid ring. `spacing` (0.025): the least space between strands, in the shape's units, 0.002 to 0.1; strands thinner than about twice this are left out, so it also sets how fine the packing goes. `iterations` (20): inversions followed into the gaps; more finds smaller strands.

### Indra's Cells (`indra_cells`)

![Indra's Cells](images/fractals/indra_cells.jpg)

The rooms of the 120-cell, the four-dimensional polytope of 120 dodecahedra, seen in our space by stereographic projection. The rooms have pentagonal windows and grow larger toward the edge. Each room holds a pearl, a small inside-out copy of the whole, and each room in the pearl holds a pearl of its own. Left: the rooms within a ball, as Add makes it. Right: inside a room, with its pearl and the rooms and pearls beyond the windows.

**Formula.** A point $x$ goes to $S^3 \subset \mathbb{R}^4$ by inverse stereographic projection, $q = (1 - |x|^2, 2x)/(1 + |x|^2)$, which sends the origin to the center $e = (1, 0, 0, 0)$ of one room. Then repeat, up to `iterations` times:

1. Fold $q$ into the room around $e$ by the 120-cell's symmetry group $H_4$ (order 14400). Fold the vector part into the dodecahedron's chamber three times: take absolute values, then reflect in the plane with normal $(-\varphi/2, -1/2, 1/(2\varphi))$. Then reflect in the face $\nu$ between this room and its neighbor $36°$ away if $q$ is past it, and go back to the dodecahedral fold. These five mirrors generate $H_4$; six rounds always reach the chamber.
2. Measure the walls: on $S^3$, the wall is the face's slab, $\arcsin(-q \cdot \nu) - t$, intersected with a strip along the face's edge, $\arcsin(q_y) - b$. This leaves a pentagonal window in every face.
3. If $q$ is in the pearl, the cap of angular radius $\rho$ around $e$, invert it. The inversion is in the sphere of $\mathbb{R}^4$ centered at $\sec\rho\, e$ with radius $\tan\rho$; that sphere is orthogonal to $S^3$, so the map is a conformal involution of $S^3$ that swaps the pearl with the rest. Inside the pearl, then, is the whole outside turned inside out.

The group generated by $H_4$ and this inversion is a Kleinian group acting on $S^3$, the boundary of hyperbolic 4-space. Its limit set is a Cantor dust inside the pearls, and the walls are its orbit of a polytope's faces.

Distance estimate. The $H_4$ fold is made of isometries of $\mathbb{R}^4$, and the inversion multiplies lengths by $\tan^2\rho/|q - \sec\rho\, e|^2$. With $D$ the product so far, the estimate on $S^3$ is the least of $w/D$ over the levels. When the point is outside a pearl, the bound $(\angle(q, e) - \rho_{in})/D$ is added for everything the pearl holds, all of which lies within $\rho_{in}$ of its center, where $\tan(\rho_{in}/2) = \tan^2(\rho/2)/\tan((\pi/10 - t)/2)$ is the inverted radius of the room's walls. At the last level, that ball stands in for what the pearl holds. As with every Kleinian estimate, this is first order in the inversions, hence the step of 0.7. To get from $S^3$ back to space we do not use the local factor. Instead, stereographic projection satisfies the exact identity $|x - x'| = c\sqrt{1 + |x|^2}\sqrt{1 + |x'|^2}/2$, where $c$ is the chord in $\mathbb{R}^4$. Together with $\sqrt{1 + |x'|^2} \ge \sqrt{1 + |x|^2} - |x - x'|$, it gives the lower bound

$$|x - x'| \ge \frac{c\,(1 + |x|^2)}{2 + c\sqrt{1 + |x|^2}}, \qquad c = 2\sin(d/2),$$

whose gradient is at most 1.

**Origin.** Designed for this catalog (2026); to our knowledge new. We searched the web ("120-cell stereographic projection fractal kleinian inversion raymarching"), Mandelbulber 2's formula list, and Fragmentarium's examples (knighty's `polychora-special.frag` and `polychora03`/`06`, Syntopia's `StereographicQuaternionJulia.frag` and `Stereographic4DJulia.frag`), and found no 4D polytope group combined with an inversion, and no Kleinian fractal built on a polytope. The name follows *Indra's Pearls* (Mumford, Series and Wright, 2002), the book on Kleinian groups, after the net of Indra, in which every pearl reflects all the others. Nearest relatives: stereographic projections of the 120-cell as sculpture and pictures ([Schleimer and Segerman, *Puzzling the 120-cell*](https://arxiv.org/abs/1310.3549); Jenn3D); knighty's polychora distance estimators (Fractal Forums, 2012; Fragmentarium's `polychora-special.frag`), which fold space by 4D polytope groups and project stereographically, but are not fractal; Syntopia's stereographic quaternion Julia sets in Fragmentarium, which lift space onto $S^3$ by inverse stereographic projection as this does, and slice a 4D fractal there; knighty's pseudo-Kleinian, a fold followed by a sphere inversion, of which this is a spherical, polytope-based cousin; and Jos Leys's renderings of 3D Kleinian groups.

**Parameters.** `pearl` (0.5): the pearl's radius as a share of a room's inner radius, 0.1 to 0.9. Small pearls hold a tiny world; large ones press it against the walls. `thickness` (0.04): the walls' thickness as a share of the inner radius, 0.005 to 0.2. `frame` (0.16): the width of the frame around each window, 0 to 1; 0 leaves only the edges (a wireframe), and 1 closes the windows into solid walls. `centre` (0): what sits at the origin: 0 a room (icosahedral symmetry), 1 a window, 2 an edge, 3 a corner where four rooms meet (tetrahedral symmetry). `iterations` (5): pearls within pearls; each level is several times smaller than the last. The rooms fill all of space (the room opposite the center contains infinity), so Add cuts them to a ball of radius 1. For the inside view, scale the shape up (scale 5 makes the center room about 1.6 m across) and put the camera in a room.

### Zorich Pagodas (`zorich_pagodas`)

![Zorich Pagodas](images/fractals/zorich_pagodas.jpg)

A floor of domed pagodas, each crowned with tiers of spiky pinnacles and ringed by smaller pinnacles that are ringed in turn, in a lattice that runs on forever. They are the points that escape upward under a three-dimensional analogue of the exponential function.

**Formula.** Zorich's map is the 3D counterpart of $e^z$. On the square beam $|x_1|, |x_2| \le 1$,

$$Z(x) = e^{x_3}\,\frac{v}{|v|}, \qquad v = (x_1,\ x_2,\ 1 - \max(|x_1|, |x_2|)),$$

so the beam's cross-section goes radially onto the upper half of the unit sphere, and height becomes the logarithm of distance. Reflecting across the beam's sides extends $Z$ to all of space, period 4 in $x_1$ and $x_2$, with each reflection mirroring the image in the plane $x_3 = 0$. Space is mapped to $(x_1, x_2, x_3) = (x, z, \text{depth} - y)$. A point belongs to the shape if some iterate of $x \mapsto \lambda Z(x)$, the point itself included, reaches $x_3 \ge \text{depth}$. The floor $y \le 0$ has escaped already; the pagodas are the points that escape at the first, second, third... step.

Distance estimate. The radial projection's derivative is at most $|Dv|/|v| \le \sqrt{2}/|v|$, so $|DZ| \le e^{x_3}\max(1, \sqrt{2}/|v|)$, and $L_n = \prod_{k<n} \lambda |DZ(x_k)|$ bounds how fast the $n$th iterate moves. The estimate is

$$d = \min\Big(\text{depth} - y,\ \min_n \frac{\min(\text{depth} - x_{3,n},\ 2)}{L_n}\Big).$$

This first-order estimate assumes $x_3$ climbs toward the escape level at a steady rate, but it climbs exponentially, so the estimate is off in both directions. Uncapped, it overshoots far below the escape level; capped too tightly (we tried 0.5), it creeps, and rays stall into grain around the pinnacles. The gap is capped at 2 map units and the step held at 0.25. The steepness test bounds the overshoot, at the defaults and at every corner of the parameter ranges we tried, and the renders show the creep is gone.

**Origin.** Designed for this catalog (2026) as a distance-estimated fractal; the map and its dynamics are mathematics' own. V. A. Zorich introduced the map in 1967 ("A theorem of M. A. Lavrent'ev on quasiconformal space maps"), as a quasiregular analogue of the exponential. Its iteration has been studied by W. Bergweiler ([*Karpińska's paradox in dimension three*](https://arxiv.org/abs/0902.2686), 2010), Bergweiler and D. Nicks ([*Foundations for an iteration theory of entire quasiregular maps*](https://arxiv.org/abs/1210.3972), 2014) and A. Tsantaris ([*Julia sets of Zorich maps*](https://www.cambridge.org/core/journals/ergodic-theory-and-dynamical-systems/article/julia-sets-of-zorich-maps/7E9F7B562CE5FA4A01D2420BDDB9BB8D), 2022). For small $\lambda$, its Julia sets are "Cantor bouquets" of hairs reaching up to infinity, of which these pagodas are the thickened, finitely iterated form. To our knowledge, it has not been rendered before. We searched the web, Fractal Forums and Shadertoy ("Zorich map fractal raymarch render shadertoy fractalforums", "Zorich map Julia set Cantor bouquet hairs three dimensions", "3D exponential Julia set quasiregular Cantor bouquet ray traced"), Mandelbulber 2's formula list and Fragmentarium's examples, and found only the papers. Nearest relatives: the 2D exponential Julia sets of $\lambda e^z$ (Devaney and Krych's Cantor bouquets), and, among 3D fractals, the Mandelbulb, which builds a 3D analogue of $z^n$ much as this builds one of $e^z$.

**Parameters.** `lambda` (0.42): the map's factor, 0.2 to 0.5. Small values grow low domes with a few spikes, values around 0.4 grow tall tiered pagodas, and toward 0.5 the pagodas branch into a tangled lace. (Beyond 0.5, and for `depth` beyond 4, the finest tiers turn to grain, so the ranges stop there.) `depth` (3): the escape level's height above the floor, in the map's units, 1.5 to 4; low values leave domes with thin spines, and high ones grow dense towers. `iterations` (6): steps of the map; each adds a tier of smaller pinnacles. The lattice is infinite, so Add cuts a 2.4 m square of it, holding five pagodas, with a box. The pagodas stand on a square lattice turned 45°, $2\sqrt{2}$ units apart.
