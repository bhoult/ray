"""Alexander's horned sphere (ray's ``horned_sphere``) as a ``ClaspTree``.

A unit ring in the xy plane is cut open across +x by the half angle g
(sin g = clasp * s).  On each cut end T+- = (cos g, +-sin g, 0) hangs a child
ring ``s`` times smaller, placed by a similarity so the two children clasp
like chain links (planes square to each other, turned by ``turn``); each
child is cut open and forks in turn.  See ``clasptree.py`` for the distance
and the tree-as-curves methods; this file only places the children, and keeps
the Rust kernel's tighter bounding ball for pruning.
"""
from __future__ import annotations

import numpy as np

from .clasptree import ClaspTree, Ring, _smin   # noqa: F401  (re-exported)


class HornedSphere(ClaspTree):
    bound = 2.0

    def __init__(self, shrink: float = 0.45, clasp: float = 1.35, turn: float = 45.0,
                 tube: float = 0.28, taper: float = 1.0, blend: float = 0.5,
                 ball: float = 0.7, iters: int = 8):
        s = float(shrink)
        t = np.radians(turn)
        self.cp, self.sp = np.cos(t), np.sin(t)
        sg = min(clasp * s, 0.95)
        g = np.arcsin(sg)
        cg = np.sqrt(1.0 - sg ** 2)
        self.off = sg - s                       # children's centres are (cg, +-off, 0)
        cp, sp = self.cp, self.sp
        Aa = np.array([[0, -1, 0], [sp, 0, cp], [-cp, 0, sp]], float)    # child-local = A (q - t) / s
        Ab = np.array([[0, 1, 0], [cp, 0, -sp], [-sp, 0, -cp]], float)
        maps = [(Aa.T, np.array([cg, self.off, 0.0])), (Ab.T, np.array([cg, -self.off, 0.0]))]
        self._setup(s, g, maps, tube, taper, blend, ball, iters)
        c = np.hypot(cg, self.off)
        self.rb = max(1.0 + self.tube, c / (1.0 - s)) + 1e-3     # same value as the generic reach

    def subtree(self, q: np.ndarray) -> np.ndarray:
        """The Rust kernel's bound: the torus of the thickest tube, and the ball
        about the middle of the ring's cut holding all its descendants."""
        x, y, z = q[:, 0], q[:, 1], q[:, 2]
        torus = np.hypot(np.hypot(x, y) - 1.0, z) - self.tube
        front = np.sqrt((x - self.cg) ** 2 + y * y + z * z) - (self.off + self.s * self.rb)
        return np.minimum(torus, front)

    def children(self, q: np.ndarray):
        """Child frames ``a``, ``b`` (own units) of ring-frame points ``q``."""
        s, cp, sp = self.s, self.cp, self.sp
        x, y, z = q[:, 0], q[:, 1], q[:, 2]
        dx, dz = x - self.cg, z
        a = np.stack([-(y - self.off), sp * dx + cp * dz, sp * dz - cp * dx], -1) / s
        b = np.stack([(y + self.off), cp * dx - sp * dz, -sp * dx - cp * dz], -1) / s
        return [a, b]
