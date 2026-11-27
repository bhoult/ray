# Sphere eversion in fractalkit

`fractalkit/eversion.py` implements the explicit sphere eversion of Bednorz and Bednorz ([arXiv 1711.10466](https://arxiv.org/abs/1711.10466)) as a triangle-mesh pipeline, with paint carried on the sphere. The intermediate surfaces are immersions with double curves, so they have no signed distance field and do not go through the ray marcher.

## The construction

A twisted cylinder, for n = 2, parametrised by (h, phi):

    x = t cos(phi) + p sin((n-1) phi) - h sin(phi)
    y = t sin(phi) + p cos((n-1) phi) + h cos(phi)
    z = h sin(n phi) - (t/n) cos(n phi) - q t h

The sphere is mapped on by h = omega sin(theta) / cos(theta)^n. The cylinder's two ends are closed at infinity by a damping map (7) and an inversion (8); the last stage (12), (15) turns the result into a round sphere. `schedule(s)` follows the paper's Table 1 with Q = 2/3: t runs from -1.5 to +1.5, and q, xi, alpha and lambda are changed linearly in the other stages. The last stage's exponents were checked against the paper's PDF.

## What was measured

- **Open cylinder** (`tools/eversion_double_curve.py`): regular for every t tried (smallest |r_h x r_phi| is 0.895). The double curve has 3 pieces for about 0.65 < |t| < 1, 1 piece for |t| < 0.62, and 2 pieces beyond |t| = 1. The change at |t| = 1 matches the paper's D0 and D2; the change at 0.62 to 0.66 is about 10% from the paper's T events at 0.56. The count depends on the cut-off, mesh and joining radius, so the second match is not established.
- **Closed sphere** (`tools/eversion_sphere_census.py`): the first and last surfaces are embedded spheres of radius sqrt(2/3), with no intersecting triangle pairs. The signed volume goes from +2.28 through 0 at t = 0 to -2.28; 2.28 is the volume of that sphere. The middle has up to 2,490 intersecting pairs (101 x 200 mesh), most at t = 0.
- **Paint** (`examples/eversion_trionda.py`): the four Trionda panels and six seams, saturated on the +n side and dark on the -n side. The first frame shows the panels from outside, the last frame the same ball with the inside paint outward.

## What this is not

- The map has no tetrahedral symmetry, so the Trionda's 3-fold axes are not axes of the eversion.
- The renderer is a painter's sort, so the interpenetrating middle frames have artifacts. The poles are cut at 0.002. About 15 to 25% of the way through, 6 triangle pairs intersect; I have not traced them to the paper's point at infinity or to the mesh.
- The code has no tests.

## Open questions

1. **Linking through the eversion.** The panel loops are closed curves on the moving surface. Their linking numbers can change only when the surface passes through itself, which happens on the double curves. Track the Gauss linking numbers of the Trionda's four panel loops along the movie. Not done.
2. **Boy's surface.** The same family at n = 3 gives Boy's surface, with 3-fold symmetry, and the Trionda has 3-fold axes. Whether a Trionda can be painted on a Boy surface compatibly is not checked.
3. **An everting tree.** Evert each generation of a seam tree in turn with a delay. Not done.
4. **Relation to qwla.** `QWLA_CONNECTIONS.md`, section 5, states one conjecture (eversion against the sign of the level) and says why there is no argument for it.
