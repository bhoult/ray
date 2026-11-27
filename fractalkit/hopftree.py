"""A ``ClaspTree`` whose clasped pairs are Hopf pairs.

In the horned sphere the two children of a ring are placed by hand (``clasp``
and ``turn``).  Here they are two *fibres of the Hopf fibration*: the fibres
over two antipodal base points of S^2 on the Clifford torus, stereographically
projected to R^3.  Those are two congruent circles (radius sqrt 2, normalised
to 1) on a torus, linked once, their planes square to each other, and
swapped by a half turn about the torus's axis.  The pair is carried onto the
parent by the similarity that

* sends the half-turn axis to the parent ring's x axis,
* sends the two chosen *back points* (one on each fibre) to the parent's cut
  tips ``T+-``, which fixes the children's size: ``sin g = shrink * rho``,
  with ``rho`` the back point's distance from the axis (in child radii).

So the recursion carries the fibration's linking: every sibling pair is, up to
similarity, a pair of Hopf fibres.  Free parameters: ``psi`` (where on the
fibre the back point is), ``hang`` (+-1, which side of the tips the pair
hangs), ``hand`` (+-1, the chirality of the tree).  ``clasp`` and ``turn`` are
not free any more: the fibration decides them.
"""
from __future__ import annotations

import numpy as np

from .clasptree import ClaspTree
from .geometry import circle_through, hopf_fiber_points, stereographic

_RZ_PI = np.diag([-1.0, -1.0, 1.0])       # half turn about the z axis (swaps the pair)


def hopf_pair():
    """The two Clifford-torus Hopf fibres over antipodal base points, as unit
    circles about the origin: ``[(centre, normal), (centre, normal)]``.  The
    second is the first turned by a half turn about z (checked in the tests)."""
    out = []
    for phi in (0.0, np.pi):
        t = np.array([0.0, 2 * np.pi / 3, 4 * np.pi / 3])
        c, r, n = circle_through(*stereographic(hopf_fiber_points(np.pi / 4, phi, t)))
        out.append((c / r, n, r))
    (ca, na, ra), (cb, nb, rb) = out
    assert abs(ra - rb) < 1e-9
    return ca, na, cb, nb, ra


class HopfTree(ClaspTree):
    """Tree of Hopf-paired rings (see module doc)."""

    def __init__(self, shrink: float = 0.4, psi: float = 0.0, hang: int = 1, hand: int = 1,
                 tube: float = 0.2, taper: float = 1.0, blend: float = 0.5, ball: float = 0.5,
                 iters: int = 8):
        self.psi, self.hang, self.hand = float(psi), 1 if hang >= 0 else -1, 1 if hand >= 0 else -1
        s = float(shrink)
        ca, na, cb, nb, self.fibre_radius = hopf_pair()
        # back point: psi = 0 is the point of the fibre nearest the axis
        w = np.array([0.0, 0.0, ca[2]]) - ca
        e1 = w - (w @ na) * na
        e1 /= np.linalg.norm(e1)
        e2 = np.cross(na, e1)
        pa = ca + np.cos(self.psi) * e1 + np.sin(self.psi) * e2
        rho = float(np.hypot(pa[0], pa[1]))
        if s * rho > 0.95:
            raise ValueError(f"shrink * rho = {s * rho:.3f} > 0.95: the back point is {rho:.3f} from the axis; "
                             "lower shrink or move psi toward the axis")
        self.rho, self.pair_back = rho, pa
        sg = s * rho
        g = np.arcsin(sg)
        cg = np.sqrt(1 - sg * sg)
        # frames of the two unit rings in the pair's coordinates: x toward the front, z the normal
        fa = ca - pa
        fa /= np.linalg.norm(fa)
        ez = self.hand * na
        Fa = np.stack([fa, np.cross(ez, fa), ez], 1)
        Fb = _RZ_PI @ Fa
        cb = _RZ_PI @ ca
        # similarity: u (radial toward pa) -> y, z axis -> hang * x, tips -> (cg, +-sg, 0)
        u = np.array([pa[0], pa[1], 0.0]) / rho
        wv = np.cross([0.0, 0.0, 1.0], u)
        eps = self.hang
        Q = (np.outer([0, 1.0, 0], u) + eps * np.outer([0, 0, 1.0], wv) + eps * np.outer([1.0, 0, 0], [0, 0, 1.0]))
        o = np.array([0.0, 0.0, pa[2]])
        m = np.array([cg, 0.0, 0.0])
        maps = [(Q @ Fa, m + s * Q @ (ca - o)), (Q @ Fb, m + s * Q @ (cb - o))]
        self._setup(s, g, maps, tube, taper, blend, ball, iters)
        # the pair's pose in the pair frame, for the tests: (centre, F) of each circle
        self.pair = [(ca, Fa), (cb, Fb)]
        self.to_parent = (Q, o, m)

    def back_points(self):
        """Where each child's back sits, in the parent ring's frame: ``T+``, ``T-``."""
        return [t + self.s * R @ np.array([-1.0, 0, 0]) for R, t in self.maps]
