# Connections between fractalkit and qwla

[qwla](https://github.com/brentharts/qwla) is a quantum-network model on aperiodic tilings (Spectre, and the Hat Trick ball for the sphere). This repository builds horned spheres and their relatives. They already share one object, the Hat Trick ball, and a few ideas. This note lists each connection, says what has been checked, and says what has not. Labels follow qwla: **Exact**, **Counted**, **Known**, **Definition**, **Conjecture**, **Not done**.

I read qwla's `README.md`, `HAT_TIME.md` and the roadmap stages that mention this repository (9c, 4i); I did not run its code.

## The dictionary

| qwla | fractalkit | status |
|---|---|---|
| 12 cone points = 12 ring bits = 12 spin-1/2 punctures on the Hat Trick ball | the 12 pentagon centres, where `HatTrickHorned` hangs children | **Exact**: same points, a regular icosahedron (`HAT_TRICK_HORN_SITES.md`) |
| rotation group of order 60 permutes the bits, flips none (`ring_bits.py`) | group I of order 60 places the children | same group; **Not done**: checking the two permutation actions agree site by site |
| Heisenberg model on the 30 icosahedron edges | 30 edge pairs among the sites | **Exact** (same graph) |
| Hopf fibres over Bloch points; clasp when the states are orthogonal (Stage 4i, "not implemented as code") | `HopfFibers(base=...)`; fibres over the ball's own axes (`examples/mixing_seamtrees.py`) | **Known** formula; evaluated on the 12 sites: 6 antipodal pairs have overlap 0 |
| tiers nest: 9 in 71 in 559 in 4401 tiles (Stage 4i) | a seam tree hangs scaled copies on a parent | different recursion, see below |
| Temperley-Lieb / Kauffman skein, wild embedding as quantisation (arXiv 1211.3012) | the horned sphere, clasp as linked panel loops | an analogy only, see below |
| two kinds of handedness: the hat (mirror) and the level's sign (time reversal) | chirality of the seam network; sphere eversion | **Conjecture** for eversion, see below |

## 1. The shared object

**Exact.** The 12 sites are a regular icosahedron with 30 edges, 30 second-neighbour pairs and 6 antipodal pairs. `HAT_TRICK_HORN_SITES.md` has the table and the Moebius invariants of the Hopf fibres over them: overlaps 0.8507, 0.5257 and 0.

## 2. Two recursions

qwla's tiers are *nested regions of one tiling*: each patch contains the smaller one, and the tile counts follow N_k = 8 N_(k-1) - N_(k-2) (1, 9, 71, 559, 4401), whose growth rate is the larger root of x^2 - 8x + 1, namely 4 + sqrt 15 = 7.873 (**Known**, arithmetic). A seam tree is different: copies shrunk by `shrink` are hung on the axes, and the number of balls per generation is a geometric sequence (Trionda 1, 4, 12, 36; Hat Trick 1, 12, 132).

- Both are self-similar, but only the tiers sit inside each other; the tree's copies are attached outside the parent.
- A measurement that compares them is the *growth of counts against scale*. For the tree it is the similarity dimension log(children)/log(1/shrink) (**Counted** here: Trionda 1.20, Hat Trick 1.99). For the tiers, qwla's roadmap (R4) gives the inflation `lambda^2 = 4 + sqrt 15`, which is the count ratio 7.873, so the linear factor is `lambda` and the similarity dimension is log(`lambda^2`)/log(`lambda`) = 2 exactly: the tiers fill a region of the plane, whatever the tiling. The trees' fractional dimensions are what a plane-filling nesting is not.
- **Not done.** A nested (inside, not attached) variant of the seam tree.

## 3. Chirality

**Counted.** The standard Hat Trick tiling's seam network is 0.96 degrees from its mirror image (this repository); qwla finds the vertex count and all small-neighbourhood counts mirror symmetric, with a mirror-odd part only from third neighbours on (`chi4`, order 4 in the ring bits). Both say the same in words: the tiles are chiral but the network built from them is nearly achiral.

**Counted, two tests of whether the numbers agree** (`tools/chi4_comparison.py`, `tools/chi4_signed.py`; details in `HAT_TRICK_HORN_SITES.md`, section 3). The 0.96 degrees is specific to the symmetric tiling: for random ring patterns the seam network's mirror distance is 1.2 to 2.7 degrees (mean 2.26), and one flipped ring already gives 2.58. The unsigned distance shows a weak hint of a relation to |`chi4`| (Spearman 0.18, p = 0.047, 120 patterns). The signed-style test on 300 patterns compares each pattern's network with the mirror of the reflected pattern's network, and finds nothing: correlation with |`chi4`| 0.04 (p = 0.52), the same mean for chiral and achiral patterns. Consistency checks pass (`chi4` flips sign under reflection, and is non-zero only on chiral patterns). The conclusion is that this global measure does not see `chi4`; a local measure around each pentagon is **not done**.

The repository also has a control that bears on qwla's question. Scaling the Trionda's seam amplitude to 0 makes the ball achiral, and children still clasp at about the same rate (7 of 39 random placements against 3 of 29 at the default amplitude). So in the horned-sphere setting the clasp does not need a chiral ball; chirality enters only through the placement. **Counted**; I draw no conclusion about the quantum model.

## 4. Clasps and the skein algebra

qwla quotes the claim of Asselmeyer-Maluga and Krol that a wild embedding is the quantisation of a tame one, through the Kauffman bracket skein algebra (Temperley-Lieb for a disc with marked points). This repository counts *Gauss linking numbers* between panel loops of different balls. The two are related but not the same:

- A linking number is an integer invariant of two closed curves in space. The skein algebra works with framed links in a thickened surface and crossing-by-crossing relations, with a parameter; it contains the linking numbers only after a specialisation. **Known** in outline; I have not derived the specialisation.
- Counting linked loop pairs is what decides whether a seam tree is a "clasp". The skein picture would instead give an *operator* for each crossing. **Not done**: any operator built from the tree's crossings.
- qwla's own caveat stands: the 12 cone points are tame, and a Cantor set in a surface is always tame. The seam tree is a finite-depth stand-in for the horned sphere; it adds no wild point either.

## 5. Eversion and the two handedness choices

qwla's argument: a function of the tiling is odd under reflection and even under complex conjugation (time reversal), while the face flux of the level-k model is odd under both; so the hat's hand cannot fix the sign of the flux, and the sign of the level is a separate choice. In `fractalkit/eversion.py` the sphere is turned inside out through immersions: the final sphere is the antipodal image of the first with the paint on the other side, and the signed volume changes sign. See `EVERSION.md`.

**Conjecture, not tested.** Eversion exchanges the two sides of a surface, and in the skein picture exchanging over and under is `A -> 1/A`, which qwla identifies with complex conjugation of the level-k operators. If the two exchanges were the same operation, an eversion of the Hat Trick ball would be the geometric counterpart of flipping the sign of the level. I have no argument that they are the same: reversing a surface's orientation and swapping the crossings of a link diagram are different operations, and nothing here connects them. It is listed because it is the one place the two repositories pose the same kind of question (what flips a handedness), not because anything supports it.

## What would be worth doing

1. The binary Hat Trick tree is built (`HAT_TRICK_HORN_SITES.md`, sections 4 and 5): on antipodal pentagons siblings never link; on adjacent pentagons they do, and with thin tubes the clasp holds to depth 2 with clear tubes.
2. A local chirality measure to test `chi4` (section 3): the global signed test found nothing.
3. A site-by-site check that the two copies of the rotation group act the same way on the 12 sites.
4. Seam networks of the ring-flipped tilings as new seam trees: the 4096 tilings give 4096 balls with the same 12 sites.
