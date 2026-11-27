"""Seam trees whose ball type changes with the generation (Trionda root with Hat Trick children, and so on).

``balls`` is a list of ``SeamTree`` objects, one per generation (the last one repeats; ``cyclic=True`` repeats the
whole list).  A network of generation g sits on the sites of its parent's ball; its own site 0 is turned back onto
the parent's site and spun by ``turn`` about that axis, so the mismatch between the two axis orders (3-fold
against 5-fold, say) is absorbed by ``turn`` alone.  The children of one network are the images of one child
under the *parent's* rotation group, so the arrangement keeps the parent's symmetry whatever the child's.  Every
network but the root spends its site 0 on its parent.
"""
from __future__ import annotations

import numpy as np

from .core import Field
from .clasptree import _smin
from .seamtree import rot


def _align(a, b):
    """Rotation taking unit vector a to unit vector b."""
    v = np.cross(a, b); s = np.linalg.norm(v); c = float(a @ b)
    if s < 1e-12:
        if c > 0:
            return np.eye(3)
        p = np.cross(a, [0.3, 0.5, 0.8]); return rot(p, np.pi)
    return rot(v, np.arctan2(s, c))


class MixedSeamTree(Field):
    def __init__(self, balls, shrink: float = 0.4, sink: float = 0.4, turn: float = 10.0, body: float = 0.95,
                 blend: float = 0.5, iters: int = 3, cyclic: bool = False):
        self.balls, self.cyclic = list(balls), cyclic
        self.s, self.sink, self.turn, self.body, self.blend, self.iters = float(shrink), float(sink), float(turn), float(body), float(blend), int(iters)
        self.n_classes = max(b.n_classes for b in self.balls)
        self.n_bands = 1 + self.iters * self.n_classes
        self.maps = {}
        reach, tmax = 1.0, max(b.tube for b in self.balls)
        for a in range(len(self.balls)):
            for b in range(len(self.balls)):
                self.maps[(a, b)] = self._maps(self.balls[a], self.balls[b])
                reach = max(reach, max(float(np.linalg.norm(t)) for _, t, _ in self.maps[(a, b)]))
        self.rb = max(1.0 + tmax, reach / (1.0 - self.s)) + 1e-3
        self.bound = self.rb * 1.02

    def _maps(self, par, kid):
        u0 = par.sites[0]
        R0 = rot(u0, np.radians(self.turn)) @ _align(kid.sites[0], -u0)
        out, seen = [], set()
        for g in par.group:
            k = int(np.argmin(np.linalg.norm(par.sites - g @ u0, axis=1)))
            if k not in seen:
                seen.add(k); out.append((g @ R0, par.sites[k] * (1.0 + self.sink * self.s), k))
        return out

    def idx(self, gen: int) -> int:
        n = len(self.balls)
        return gen % n if self.cyclic else min(gen, n - 1)

    def ball(self, gen: int):
        return self.balls[self.idx(gen)]

    def cmap(self):
        from matplotlib.colors import ListedColormap
        x = np.linspace(0, 1, 2048)
        cols = np.tile((0.08, 0.08, 0.10), (len(x), 1))
        for i, xv in enumerate(x):
            if xv >= 0.15:
                b = min(int((xv - 0.15) / 0.85 * self.n_bands), self.n_bands - 1)
                if b > 0:
                    gen, cls = divmod(b - 1, self.n_classes)
                    cols[i] = self.ball(gen).class_color(gen, cls)
        return ListedColormap(cols)

    def _bound(self, q, tube):
        r = np.linalg.norm(q, axis=1)
        return np.minimum(np.minimum(np.abs(r - 1.0) - tube, np.abs(r - self.body)), r - self.rb)

    def _de(self, P):
        N = len(P); s, blend = self.s, self.blend
        idx = np.arange(N); q = np.asarray(P, float).copy(); sc = np.ones(N)
        dp = np.full(N, 1e9); bd = np.full(N, -1e9)
        best = np.full(N, 1e9); plain = np.full(N, 1e9); band = np.zeros(N)
        for gen in range(self.iters):
            if len(idx) == 0:
                break
            ball = self.ball(gen); tube = ball.tube
            kf = blend * tube * sc
            keep = bd < best[idx] + kf
            idx, q, sc, dp, kf = idx[keep], q[keep], sc[keep], dp[keep], kf[keep]
            if len(idx) == 0:
                break
            dt = (ball.tree.query(q)[0] - tube) * sc
            r = np.linalg.norm(q, axis=1)
            dsh = (r - self.body) * sc
            d = _smin(dt, dsh, 0.5 * kf)
            cand = np.zeros(len(idx))
            ns = dsh < dt
            if ns.any():
                cls = ball.lookup_cls[ball.lookup.query(q[ns] / np.maximum(r[ns], 1e-9)[:, None])[1]]
                cand[ns] = 1 + gen * self.n_classes + cls
            gmin = np.full(N, np.inf); np.minimum.at(gmin, idx, d)
            better = gmin < plain
            plain = np.where(better, gmin, plain)
            win = (d <= gmin[idx] + 1e-12) & better[idx]
            band[idx[win]] = cand[win]
            np.minimum.at(best, idx, _smin(dp, d, kf))
            if gen + 1 >= self.iters:
                break
            sc2 = sc * s
            nxt = self.ball(gen + 1)
            reach_k = blend * nxt.tube * sc2
            nidx, nq, nsc, ndp, nbd = [], [], [], [], []
            for R, t, k in self.maps[(self.idx(gen), self.idx(gen + 1))]:
                if gen > 0 and k == 0:
                    continue
                c = ((q - t) @ R) / s
                bound = self._bound(c, nxt.tube) * sc2
                m = bound < best[idx] + reach_k
                nidx.append(idx[m]); nq.append(c[m]); nsc.append(sc2[m]); ndp.append(d[m]); nbd.append(bound[m])
            idx, q, sc, dp, bd = (np.concatenate(v) for v in (nidx, nq, nsc, ndp, nbd))
        return best, np.where(band > 0, (band + 0.5) / self.n_bands, 0.0)

    # -- curves ---------------------------------------------------------------------
    def networks(self, depth: int = 2):
        """Poses ``(M, t, S, gen, parent)``."""
        poses = [(np.eye(3), np.zeros(3), 1.0, 0, -1)]
        frontier = [0]
        for gen in range(depth):
            nxt = []
            for pi in frontier:
                M, t, S = poses[pi][:3]
                for R, tc, k in self.maps[(self.idx(gen), self.idx(gen + 1))]:
                    if gen > 0 and k == 0:
                        continue
                    poses.append((M @ R, t + S * (M @ tc), S * self.s, gen + 1, pi)); nxt.append(len(poses) - 1)
            frontier = nxt
        return poses

    def clearance(self, depth: int = 2, step: float = 0.02):
        """(smallest gap between unrelated networks' tubes, number of fused pairs, smallest parent-child gap)."""
        from scipy.spatial import cKDTree
        from .clasp import _pts
        nets = self.networks(depth)
        pts = []
        for M, t, S, gen, _ in nets:
            b = self.ball(gen)
            pts.append(_pts([t + S * c @ M.T for c in b.curves], step))
        kd = [cKDTree(p) for p in pts]
        cen = np.array([n[1] for n in nets]); rad = np.array([n[2] * self.rb for n in nets])
        tb = [self.ball(n[3]).tube * n[2] for n in nets]
        best, fused, pc = np.inf, 0, np.inf
        for a in range(len(nets)):
            for b in range(a + 1, len(nets)):
                if np.linalg.norm(cen[a] - cen[b]) > rad[a] + rad[b]:
                    continue
                g = kd[a].query(pts[b])[0].min() - tb[a] - tb[b]
                if nets[b][4] == a or nets[a][4] == b:
                    pc = min(pc, g)
                else:
                    best = min(best, g); fused += g < 0
        return float(best), int(fused), float(pc)
