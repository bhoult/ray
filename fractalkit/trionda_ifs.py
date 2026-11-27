"""The Trionda as an iterated function system: a twisted, Trionda-symmetric Sierpinski tetrahedron.

The Sierpinski tetrahedron is the attractor of four contractions ``f_k(z) = v_k + s (z - v_k)`` fixed at the
tetrahedron's vertices (s = 1/2).  Here each map also turns by ``twist`` about its vertex axis,

    f_k(z) = v_k + s * R(v_k, twist) (z - v_k).

The four maps are conjugate under the rotation group T, so the attractor is T-symmetric; for ``twist`` != 0 it is
chiral, like the Trionda's four boomerang panels.  ``twist = 0`` gives the plain Sierpinski tetrahedron (checked
by the similarity dimension log 4 / log(1/s) = 2 at s = 1/2).

The seed is a seam network on the unit sphere through the four vertices (so every vertex, the fixed point of its
map, lies on the seed): ``"seams"`` is the Trionda's chiral S-curves (``amp``), ``"edges"`` the straight edges of the
tetrahedron.  The field is the union of the seed's images at every level up to ``iters``, with tubes that shrink
by ``s`` per level, and is searched level by level with bounds like ``SeamTree``.
"""
from __future__ import annotations

import numpy as np
from scipy.spatial import cKDTree

from .core import Field
from .clasptree import _smin
from .seamtree import rot, densify_arc
from .trionda_horned import TETRA, seam_network


class TriondaIFS(Field):
    bound = 1.3

    def __init__(self, shrink: float = 0.5, twist: float = 30.0, tube: float = 0.04, seed: str = "seams",
                 amp: float = 0.21, harmonic: float = 0.035, blend: float = 0.0, iters: int = 5):
        self.s, self.tube, self.iters, self.blend = float(shrink), float(tube), int(iters), float(blend)
        self.twist = float(twist)
        if seed == "seams":
            curves = seam_network(amp, harmonic)
            pts = np.vstack([densify_arc(c, 0.004) for c in curves])
        else:
            pts = np.vstack([np.linspace(TETRA[i], TETRA[j], 400) for i in range(4) for j in range(i + 1, 4)])
        self.curves = curves if seed == "seams" else None
        self.kd = cKDTree(pts)
        th = np.radians(twist)
        # inverse maps: q = v_k + R^T (p - v_k) / s
        self.inv = [(rot(v, -th), v) for v in TETRA]
        self.rb = 1.0 + self.tube
        self.bound = 1.0 + self.tube + 0.05

    def seed(self, q):
        return self.kd.query(q)[0] - self.tube

    def _de(self, P):
        N = len(P); s = self.s
        idx = np.arange(N)
        q = np.asarray(P, float).copy()
        sc = np.ones(N)
        best = np.full(N, 1e9)
        lvl = np.zeros(N)
        for gen in range(self.iters + 1):
            if len(idx) == 0:
                break
            d = self.seed(q) * sc
            gmin = np.full(N, np.inf); np.minimum.at(gmin, idx, d)
            better = gmin < best
            lvl = np.where(better, gen, lvl)
            best = np.minimum(best, gmin)
            if gen == self.iters:
                break
            sc2 = sc * s
            nidx, nq, nsc = [], [], []
            for R, v in self.inv:
                c = v + (q - v) @ R / s            # R^T (q - v) as row vectors: (q - v) @ R
                bound = (np.linalg.norm(c, axis=1) - self.rb) * sc2
                m = bound < best[idx]
                nidx.append(idx[m]); nq.append(c[m]); nsc.append(sc2[m])
            idx, q, sc = (np.concatenate(v) for v in (nidx, nq, nsc))
        return best, np.clip(lvl / max(self.iters, 1), 0, 1)
