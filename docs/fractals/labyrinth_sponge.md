[← Zorich Pagodas](zorich_pagodas.md) · [All fractals](../FRACTALS.md#originals) · [Nil Staircase →](nil_stair.md)

# Labyrinth Sponge (`labyrinth_sponge`)

*Originals*

![Labyrinth Sponge](../images/fractals/labyrinth_sponge.jpg)

The two interwoven labyrinths of a gyroid, whose walls are not solid. Each wall is a lattice of small rooms (Schwarz's P surface), each room with doors onto both sides of the wall. The rooms' own walls are pierced by gyroid corridors five times finer again, and so on down. Every tunnel opens into the larger ones, so all of it can be walked: from the corridors into the rooms in their walls, and from the rooms into the tunnels in theirs. Left: a cube of it, as Add makes it. Right: down one of the gyroid's corridors, lit through the windows in its walls.

**Formula.** A triply periodic minimal surface is approximated by the zero set of a short trigonometric sum, its nodal approximation:

```math
G = \sin x\cos y + \sin y\cos z + \sin z\cos x, \qquad D = \sin x\cos(y - z) + \cos x\sin(y + z),
```

```math
P = \cos x + \cos y + \cos z, \qquad N = 3(\cos x + \cos y + \cos z) + 4\cos x\cos y\cos z
```

for the gyroid, Schwarz's D and P surfaces, and Neovius's surface. P and N have one large room per period, against the many narrower channels of G and D, so they are taken at twice the frequency ($`m = 2`$, else $`m = 1`$). The sheet $`|f| \le w`$ is a wall between the surface's two labyrinths, and they pinch shut when $`w`$ reaches $`f`$'s saddle value: $`\sqrt 2`$ for G, 1 for D and P, 3/4 for N. Level $`i`$ is a sheet at scale $`r^i`$, where $`r`$ is the whole-number ratio:

```math
S_i = \{\, p : |f_i(m_i r^i p)| \le w_i \,\}.
```

The surfaces alternate between `surface` (even levels) and `inner` (odd levels). The thresholds are shares of the saddle value: $`w_0`$ is `wall`, and every later $`w_i`$ is `tunnels`. The sponge is the intersection $`F = \bigcap_i S_i`$. Level $`i + 1`$ takes both of its labyrinths out of what is left of the walls, so its tunnels run through the walls of level $`i`$ and open into its corridors on both sides. Because $`r`$ is whole, the levels' lattices are commensurate, so the sponge is periodic, and below its first level it is exactly self-similar: $`F = S_0 \cap T`$, where $`T = S_1 \cap S_2 \cap r^{-2}T`$ ($`T = S_1 \cap r^{-1}T`$ with one surface throughout). Each level only adds finer holes, so ray adds levels close up. The point is carried in radians reduced level by level, $`q_{i+1} = r q_i \bmod 2\pi`$ (exact for a whole $`r`$), so sine and cosine stay accurate however deep it goes.

**Distance estimate.** Each sheet's estimate comes from bounds on its function. Every $`f`$ here is a sum of waves $`\sum_k a_k\cos(k \cdot q + \phi_k)`$. Along any unit direction $`u`$, its second derivative is therefore at most $`\sum_k |a_k|(k \cdot u)^2`$ and its slope at most $`\sum_k |a_k||k \cdot u|`$. For these cubic sums the first is the same in every direction, $`H`$ = 2, 2, 1 and 7 for G, D, P and N. By the Cauchy–Schwarz inequality the second is at most $`L`$ = $`\sqrt 6`$, 2, $`\sqrt 3`$ and $`3\sqrt 3 + 4`$. Let $`h = \big||f| - w\big|`$, and let $`g = |\nabla f|`$, computed exactly alongside $`f`$. Taylor's theorem gives $`|f(q + v)| \ge |f(q)| - g|v| - H|v|^2/2`$ outside the sheet, and the mirror bound inside. So $`|f|`$ cannot reach $`w`$ within

```math
\rho = \max\Big(\frac{2h}{g + \sqrt{g^2 + 2Hh}},\ \frac{h}{L}\Big),
```

the root of that quadratic or the plain Lipschitz bound, whichever is larger. This is a rigorous lower bound on the distance to the sheet, from either side. Near the sheet it is the first-order distance $`h/g`$, so rays are not slowed where the surface is. (Ray's `gyroid` primitive uses the global bound $`h/\sqrt 6`$ alone, about two-thirds of this near the surface.) Each level's estimate is divided by $`m_i r^i`$, made negative inside, and joined to the others by a smooth maximum, $`\max(a, b) + \max(k - |a - b|, 0)^2/4k`$, where $`k`$ is `round` over $`m_i r^i`$. The smooth maximum defines the rounded shape itself, not an approximation of the sharp one. Its gradient is a weighted average of the two gradients, so it is never steeper than the steeper of them. When the estimate exceeds the most any finer level could add (its $`\rho`$ at $`|f| = f_{\max}`$ and $`g = 0`$, plus its rounding), the remaining levels are skipped. The step is 1. At the defaults and at the corners of the parameter ranges, the 99th percentile of the slope is under 0.95 and the steepest seen is 1.0. Of about 580,000 probes within the estimate of a point outside, none was inside the shape.

**Origin.** Invented for ray by Claude (Anthropic's Claude Opus 5.5), October 2026; to our knowledge new as a fractal. Hierarchical TPMS lattices are an active topic in materials science, but with two levels, sized for 3D printing, and the word "hierarchical" usually means something else. We searched the web ("hierarchical gyroid lattice multiscale TPMS", "fractal TPMS self-similar minimal surface recursive levels", "fractal gyroid shadertoy raymarch nested gyroid", "fractalforums gyroid fractal distance estimator", "\"fractal gyroid\" OR \"gyroid fractal\" render", "\"self-similar\" hierarchical gyroid lattice wall made of smaller gyroid additive manufacturing", "fractal-like hierarchical triply periodic minimal surface \"three levels\" OR \"multi-level\" Boolean intersection sheet", "shadertoy gyroid octaves fbm layered gyroid"). We also checked Mandelbulber 2's formula list, which has no gyroid or minimal surface, only sine and cosine transforms. We listed the example trees of Syntopia's Fragmentarium and 3Dickulus's FragM, with no gyroid, Schwarz or minimal-surface files, and GitHub's code search for "gyroid" in all three repositories found nothing. Nearest relatives:

- G. Di Frisco, R. Yousefi Nooraie and M. Guagliano, *Structural design and characterization of hybrid hierarchical lattice structures based on sheet-network triply periodic minimal surface topology* (Materials & Design, 2024). They place a smaller TPMS lattice in the hollow of a sheet Schwarz P cell: two levels, the inner one in the void rather than the wall.
- *Experimental evaluation of biomimicry-inspired metallic hybrid lattice structures produced by laser powder bed fusion* (2025): coral-inspired "gyroid-in-gyroid", "gyroid-in-diamond" and "gyroid-in-Schwarz" metal lattices, also two levels.
- Lawrence Livermore's patent US 11,389,765, *Hierarchical triply periodic minimal surface structures as heat exchangers and reactors* (P. Roy, D. Nguyen and J. K. Stolaroff, 2022). There, hierarchical means cell sizes graded across the part.
- Shadertoy's gyroid noise: gyroids summed over octaves (Leon Denise's fbm, nimitz's cyclic noise, XorDev's dot noise). It is an additive texture, with no walls or labyrinths at the finer scales.
- The Menger sponge, which is the same recursion done with cubes. P inside P at ratio 3 is close to a smoothed Menger sponge, which is one reason the default alternates surfaces.

What is new here: the levels go on without end; each level's labyrinths lie in the walls of the last, so every tunnel opens into the larger ones; the surfaces alternate; a whole ratio makes the sponge exactly self-similar; and the distance estimate is a rigorous second-order bound.

**Parameters.** `surface` (0): the minimal surface of the largest level and of every second level after it: 0 the gyroid, 1 Schwarz's D (diamond), 2 Schwarz's P (primitive), 3 Neovius's. `inner` (2): the surface of the levels in between, so that the walls of one are made of the other's labyrinths; set it to `surface` for a single surface throughout. `wall` (0.35): the largest level's wall thickness, as a share of the most it can have before its corridors close, 0.15 to 0.95; low values leave wide corridors and thin walls one room deep. `tunnels` (0.85): the finer levels' walls, as for `wall`, 0.4 to 0.97; high values drill narrow tunnels and keep the walls solid, and low ones hollow them into lace. `ratio` (5): how many times finer each level is than the last, 2 to 6. Close up, ray adds levels as if it were 5, so with smaller ratios the finest level stops short of the pixels, and with larger ones it goes a little past them. `round` (0.2): how much the rim of each tunnel is rounded where it meets the larger wall, as a share of its level's scale, 0 to 1; 0 cuts it sharp. `iterations` (2): the finer levels as seen from afar, up to 8 close up. The lattice fills space, so Add cuts a cube of it about 2 m across, 1.75 periods of the gyroid, which repeats every $`2\pi`$ units. For walking, scale it to 1, where the gyroid's corridors are about 2 m wide, and put the camera at $`(\pi/2, 0, z)`$ looking along $`z`$: a straight sight line runs down a helical corridor there. The rooms in the walls are centered at multiples of $`2\pi/10`$; one is at the origin, and at scale 10 it is a few meters across.

---

[← Zorich Pagodas](zorich_pagodas.md) · [All fractals](../FRACTALS.md#originals) · [Nil Staircase →](nil_stair.md)
