"""A *Trionda* horned sphere: the horned-sphere recursion run on the Trionda ball.

The building block is the Trionda seam network of ``ballgen`` (tetrahedral
ball, four chiral boomerang panels, rotation group T of order 12): six
S-shaped seams on the unit sphere joining the four tetrahedron vertices (the
3-fold axes).  See ``seamtree.py`` for how copies are hung on the vertices.
"""
from __future__ import annotations

import numpy as np

from .seamtree import SeamTree, group_from_sites, rot, densify_arc

TETRA = np.array([[1, 1, 1], [1, -1, -1], [-1, 1, -1], [-1, -1, 1]], float) / np.sqrt(3.0)


def seam_arcs(amp: float = 0.21, harmonic: float = 0.035, n: int = 28):
    """The six Trionda seams as polylines on the unit sphere, keyed by vertex pair.

    ``amp``/``harmonic``: offset from the great arc (fraction of its length), odd about the
    midpoint, so T's 2-fold axes map each seam to itself (amp = 0: the achiral tetrahedron)."""
    t = np.linspace(0, 1, n)
    arcs = {}
    for i in range(4):
        for j in range(i + 1, 4):
            a, b = TETRA[i], TETRA[j]
            om = np.arccos(a @ b)
            pole = np.cross(a, b); pole /= np.linalg.norm(pole)
            p = (np.sin((1 - t) * om)[:, None] * a + np.sin(t * om)[:, None] * b) / np.sin(om)
            d = om * (amp * np.sin(2 * np.pi * t) + harmonic * np.sin(4 * np.pi * t))
            arcs[(i, j)] = np.cos(d)[:, None] * p + np.sin(d)[:, None] * pole
    return arcs


def seam_network(amp=0.21, harmonic=0.035, n=28):
    return list(seam_arcs(amp, harmonic, n).values())


def panel_loops(amp=0.21, harmonic=0.035, n=28):
    """The four boomerang panels as closed loops (panel f = the face opposite vertex f)."""
    arcs = seam_arcs(amp, harmonic, n)

    def arc(i, j):
        return arcs[(i, j)] if i < j else arcs[(j, i)][::-1]
    loops = []
    for f in range(4):
        v = [k for k in range(4) if k != f]
        loops.append(np.vstack([arc(v[0], v[1])[:-1], arc(v[1], v[2])[:-1], arc(v[2], v[0])[:-1]]))
    return loops


class TriondaHorned(SeamTree):
    n_classes = 4

    def __init__(self, shrink: float = 0.4, sink: float = 0.4, turn: float = 10.0,
                 tube: float = 0.07, amp: float = 0.21, harmonic: float = 0.035,
                 body: float = 0.0, blend: float = 0.5, iters: int = 4, n: int = 28,
                 lateral: float = 0.0, lean: float = 0.0):
        self.amp, self.harmonic = amp, harmonic
        self.setup(seam_network(amp, harmonic, n), panel_loops(amp, harmonic, n), range(4), TETRA,
                   group_from_sites(TETRA, 3, 12), 3, shrink, sink, turn, tube, body, blend, iters,
                   lateral=lateral, lean=lean)

    @classmethod
    def clasp(cls, **kw):
        """Preset found by ``tools/scan_clasp.py`` + a local search: a trio of children around every vertex, the three
        clasped pairwise (all 12 sibling pairs have linked panel loops), tubes clear of each other and of the parent
        (clearance 0.029 at tube 0.05), parent and child loops linked too."""
        d = dict(shrink=0.628, sink=0.003, lateral=0.792, lean=325.85, turn=7.61, tube=0.05)
        d.update(kw)
        return cls(**d)
