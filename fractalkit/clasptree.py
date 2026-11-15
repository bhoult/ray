"""Recursive trees of clasped rings, as distance fields and as curves.

A *ring* is the unit circle in the xy plane, cut open across +x by the half
angle ``g`` (tips ``T+- = (cos g, +-sin g, 0)``), with a tube that narrows
toward the cut ends.  A ``ClaspTree`` hangs two rings, ``shrink`` times
smaller, from the tips, each cut open and forking in turn.  What distinguishes
one tree from another is only *where the two children go*: a pair of
similarity maps, child-local -> parent, ``q = t_k + shrink * R_k @ l``.

* ``HornedSphere`` (horned.py): Alexander's horned sphere, the children
  placed by ``clasp`` and ``turn``.
* ``HopfTree`` (hopftree.py): the children are a *Hopf pair*, two fibres of
  the Hopf fibration, so every clasp carries the fibration's linking.

The distance, as in the Rust kernel, is the min over rings of ``smin(parent,
ring)``; it does not depend on visiting order, so the tree is searched level
by level for all sample points at once, pruning subtrees that cannot matter.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .core import Field
from .geometry import FiberNetwork


def _smin(a, b, k):
    k = np.asarray(k, float)
    kk = np.maximum(k, 1e-30)
    h = np.maximum(kk - np.abs(a - b), 0.0) / kk
    return np.where(k <= 0, np.minimum(a, b), np.minimum(a, b) - h * h * kk * 0.25)


@dataclass
class Ring:
    """One ring of a tree, as a curve in space."""
    gen: int
    parent: int            # index in the list, -1 for the root
    sibling: int           # index of the ring it clasps, -1 for the root
    scale: float           # radius of the ring (root = 1)
    curve: np.ndarray      # (n,3) polyline: closed circle, or the open arc if closed=False
    angle: np.ndarray      # ring-local angle of each point (radians from the front, +x)


class ClaspTree(Field):
    """Base class: subclasses call ``_setup`` with the child maps."""

    bound = 2.0

    def _setup(self, shrink, g, maps, tube, taper, blend, ball, iters):
        s = self.s = float(shrink)
        self.tube, self.blend, self.ball, self.iters = float(tube), float(blend), float(ball), int(iters)
        self.g = float(g)
        self.sg, self.cg = np.sin(self.g), np.cos(self.g)
        self.maps = [(np.asarray(R, float), np.asarray(t, float)) for R, t in maps]   # q = t + s R l
        self.tip = self.tube * s ** taper
        self.rate = taper * np.log(s) / (np.pi - self.g)
        # radius about a ring's centre holding it and all its descendants
        reach = max(float(np.linalg.norm(t)) for _, t in self.maps)
        self.rb = max(1.0 + self.tube, reach / (1.0 - s)) + 1e-3

    # -- pieces -----------------------------------------------------------
    def ring(self, q: np.ndarray) -> np.ndarray:
        """Distance (own units) to the cut, tapered unit ring."""
        x, y, z = q[:, 0], np.abs(q[:, 1]), q[:, 2]
        th = np.arctan2(y, x)
        body = np.hypot(np.hypot(x, y) - 1.0, z) - self.tube * np.exp(self.rate * (np.pi - th))
        cap = np.sqrt((x - self.cg) ** 2 + (y - self.sg) ** 2 + z * z) - self.tip
        return np.where(th >= self.g, body, cap)

    def subtree(self, q: np.ndarray) -> np.ndarray:
        """Lower bound on the distance to a ring and all its descendants:
        the torus of its thickest tube, and the ball about its centre."""
        x, y, z = q[:, 0], q[:, 1], q[:, 2]
        torus = np.hypot(np.hypot(x, y) - 1.0, z) - self.tube
        return np.minimum(torus, np.sqrt(x * x + y * y + z * z) - self.rb)

    def children(self, q: np.ndarray):
        """Points ``q`` of a ring's frame expressed in each child's frame."""
        return [((q - t) @ R) / self.s for R, t in self.maps]

    # -- distance -----------------------------------------------------------
    def _de(self, P):
        N = len(P)
        s, tube, blend = self.s, self.tube, self.blend
        idx = np.arange(N)
        q = np.stack([P[:, 1], P[:, 0], -P[:, 2]], -1)          # the shape's frame
        sc = np.ones(N)
        dp = np.full(N, 1e9)
        bd = np.full(N, -1e9)
        best = np.full(N, 1e9)
        plain = np.full(N, 1e9)
        level = np.zeros(N)
        for gen in range(self.iters):
            if len(idx) == 0:
                break
            kf = blend * tube * sc
            keep = bd < best[idx] + kf                          # bound re-check at visit time
            idx, q, sc, dp, kf = idx[keep], q[keep], sc[keep], dp[keep], kf[keep]
            if len(idx) == 0:
                break
            d = self.ring(q) * sc
            if gen == 0 and self.ball > 0:
                ball = np.sqrt((q[:, 0] + 1.0) ** 2 + q[:, 1] ** 2 + q[:, 2] ** 2) - self.ball
                d = _smin(d, ball, blend * self.ball * 0.5)
            gmin = np.full(N, np.inf)
            np.minimum.at(gmin, idx, d)
            better = gmin < plain
            plain = np.where(better, gmin, plain)
            level = np.where(better, gen, level)
            np.minimum.at(best, idx, _smin(dp, d, kf))
            if gen + 1 >= self.iters:
                break
            sc2 = sc * s
            reach_k = blend * tube * sc2
            nidx, nq, nsc, ndp, nbd = [], [], [], [], []
            for c in self.children(q):
                bound = self.subtree(c) * sc2
                m = bound < best[idx] + reach_k
                nidx.append(idx[m]); nq.append(c[m]); nsc.append(sc2[m]); ndp.append(d[m]); nbd.append(bound[m])
            idx, q, sc, dp, bd = (np.concatenate(v) for v in (nidx, nq, nsc, ndp, nbd))
        return best, np.clip(level / 6.0, 0, 1)

    # -- the ring tree as curves ------------------------------------------------
    def child_circle(self, which: str | int, n: int = 400) -> np.ndarray:
        """Closed centre-line of the first generation's child ``a``/``b`` (or 0/1),
        in the root ring's own frame."""
        k = {"a": 0, "b": 1}.get(which, which)
        R, t = self.maps[k]
        phi = np.linspace(0, 2 * np.pi, n, endpoint=False)
        local = np.stack([np.cos(phi), np.sin(phi), np.zeros(n)], -1)
        return t + self.s * local @ R.T

    def ring_tree(self, depth: int = 3, spacing: float = 0.03, closed: bool = True,
                  world: bool = True, n: int | None = None, min_pts: int = 24) -> list[Ring]:
        """The rings of generations 0..``depth`` as curves, breadth first
        (children of ring ``i`` come in pairs, ``a`` then ``b``).

        Each child frame is a similarity of its parent's, so a ring's pose is
        composed down the tree.  ``closed=True`` gives full circles (the rings
        completed across their gap, what linking numbers need); ``closed=False``
        the open arcs the shape is made of.  ``world=True`` uses the frame of
        ``de`` (x and y swapped, z flipped); ``False`` the shape's own frame.
        ``spacing`` is the world distance between points; ``n`` forces a fixed
        point count.
        """
        poses = [(np.eye(3), np.zeros(3), 1.0, -1, -1, 0)]       # M, t, S, parent, sibling, gen
        frontier = [0]
        for gen in range(depth):
            nxt = []
            for pi in frontier:
                M, t, S = poses[pi][:3]
                ia = len(poses)
                for k, (R, tc) in enumerate(self.maps):
                    poses.append((M @ R, t + S * (M @ tc), S * self.s, pi, ia + 1 - k, gen + 1))
                    nxt.append(len(poses) - 1)
            frontier = nxt
        rings = []
        for M, t, S, parent, sib, gen in poses:
            lo, hi = (0.0, 2 * np.pi) if closed else (self.g, 2 * np.pi - self.g)
            m = n or max(min_pts, int(np.ceil((hi - lo) * S / spacing)))
            phi = np.linspace(lo, hi, m, endpoint=not closed)
            local = np.stack([np.cos(phi), np.sin(phi), np.zeros(m)], -1)
            pts = t + S * local @ M.T
            if world:
                pts = np.stack([pts[:, 1], pts[:, 0], -pts[:, 2]], -1)
            rings.append(Ring(gen, parent, sib, S, pts, phi))
        return rings

    def skeleton(self, depth: int = 3, spacing: float = 0.03) -> FiberNetwork:
        """The tree's centre-lines as a capsule network: each ring's open arc
        as a tube of its own tapered thickness, labelled by generation.  With
        ``blend = ball = 0`` this is ``de`` truncated at ``depth + 1``
        generations, up to the polyline's chord error."""
        rings = self.ring_tree(depth, spacing, closed=False)
        radii, labels = [], []
        for r in rings:
            mid = 0.5 * (r.angle[:-1] + r.angle[1:])
            th = np.where(mid <= np.pi, mid, 2 * np.pi - mid)             # = |atan2(y, x)|
            radii.append(self.tube * r.scale * np.exp(self.rate * (np.pi - th)))
            labels.append(r.gen / 6.0)
        return FiberNetwork.from_curves([r.curve for r in rings], radii, closed=False, labels=labels)

    def linking(self, depth: int = 3, spacing: float = 0.02):
        """Linking numbers of the completed rings: ``(L, touching, rings)``,
        see ``linking_matrix``.  Parent and child rings touch (a child's back
        sits on its parent's cut tip), so only the other pairs get a number."""
        from .geometry import linking_matrix
        rings = self.ring_tree(depth, spacing, closed=True, world=False)
        L, touch = linking_matrix([r.curve for r in rings])
        return L, touch, rings
