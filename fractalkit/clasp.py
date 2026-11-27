"""Does a seam tree clasp?  Linking and clearance of the first generation, for any ``SeamTree``.

Children that hang on the same site of the root form a *cluster* (a Trionda vertex carries a trio, a cube vertex
under the full group six).  A cluster *clasps* when every pair of its children has a linked pair of panel loops
(Gauss linking number, nonzero) while their tubes stay apart.  Clusters on different sites of a symmetric tree are
equivalent, so linking is computed for the cluster on site 0 only; clearances are computed for all pairs.
"""
from __future__ import annotations

import numpy as np
from scipy.spatial import cKDTree

from .geometry import curve_distance, densify, linking_number


def _pts(curves, step):
    out = []
    for c in curves:
        seg = np.linalg.norm(np.diff(c, axis=0), axis=1)
        m = np.maximum(1, np.ceil(seg / step).astype(int))
        out += [c[j] + (c[j + 1] - c[j]) * (np.arange(k) / k)[:, None] for j, k in enumerate(m)]
    return np.vstack(out + [curves[-1][-1:]])


def _gauss(A, B, max_densify=8):
    gap = curve_distance(A, B)
    lmax = max(np.linalg.norm(np.roll(A, -1, 0) - A, axis=1).max(), np.linalg.norm(np.roll(B, -1, 0) - B, axis=1).max())
    if gap < 0.05 * lmax:
        return np.nan                                       # touching: linking number undefined
    m = int(np.clip(np.ceil(2 * lmax / gap), 1, max_densify))
    return linking_number(densify(A, m), densify(B, m)) if m > 1 else linking_number(A, B)


def metrics(tree, link: bool = True, step: float = 0.02) -> dict:
    """Depth-1 clasp metrics.  Keys: n_children, cluster (children on site 0), within_clear / across_clear / pc_clear
    (tube gaps; < 0 fused), and if ``link``: within_linked / within_pairs (pairs of the site-0 cluster with a linked
    loop pair) and pc_linked (children whose loops link a parent loop)."""
    nets = tree.networks(1)
    nc = len(tree.curves)
    S = [c for _, c in tree.seams(1)]
    pts = [_pts(S[i * nc:(i + 1) * nc], step) for i in range(len(nets))]
    kd = [cKDTree(p) for p in pts]
    gap = lambda a, b: kd[a].query(pts[b])[0].min() - tree.tube * (nets[a][2] + nets[b][2])
    kids = list(range(1, len(nets)))
    site = {i: tree.map_site[i - 1] for i in kids}
    cluster = [i for i in kids if site[i] == site[kids[0]]]
    within = [(a, b) for a in kids for b in kids if a < b and site[a] == site[b]]
    across = [(a, b) for a in kids for b in kids if a < b and site[a] != site[b]]
    out = dict(n_children=len(kids), cluster=len(cluster),
               within_clear=min((gap(a, b) for a, b in within), default=np.inf),
               across_clear=min((gap(a, b) for a, b in across), default=np.inf),
               pc_clear=min(gap(0, b) for b in kids))
    if link:
        loops, who = tree.panel_loops(1)
        by_net = {}
        for i, (n, k) in enumerate(who):
            by_net.setdefault(n, []).append(loops[i])

        def linked(a, b):
            return any(abs(L) > 0.5 for A in by_net[a] for B in by_net[b] if not np.isnan(L := _gauss(A, B)))
        pairs = [(a, b) for a, b in within if a in cluster and b in cluster]
        out.update(within_pairs=len(pairs), within_linked=sum(linked(a, b) for a, b in pairs),
                   pc_linked=sum(linked(0, b) for b in cluster))
    return out


def cluster_linking(tree, depth: int = 2, max_clusters: int | None = None):
    """Linking of sibling clusters at every generation up to ``depth``: the children of one network that hang on
    the same site (clusters of one child are skipped).  Returns ``(linked pairs, pairs, clusters)`` summed over all
    parents of generations 0..depth-1.  Pairs are counted linked when some pair of their panel loops has a nonzero
    Gauss linking number."""
    from .seamtree import densify_arc
    nets = tree.networks(depth)
    sites = {0: None}
    kids = {}
    for i, n in enumerate(nets):
        if n[4] >= 0:
            kids.setdefault(n[4], []).append(i)
    loops = {}

    def loops_of(i):
        if i not in loops:
            M, t, S = nets[i][:3]
            loops[i] = [t + S * densify_arc(p, 0.05, closed=True)[:-1] @ M.T for p in tree.panels]
        return loops[i]
    linked = pairs = nclu = 0
    for pi, ch in kids.items():
        g = nets[pi][3]
        ms = [k for k in tree.map_site if not (g > 0 and k == 0)]
        assert len(ms) == len(ch)
        for site in set(ms):
            cl = [c for c, k in zip(ch, ms) if k == site]
            if len(cl) < 2:
                continue
            nclu += 1
            for a in range(len(cl)):
                for b in range(a + 1, len(cl)):
                    pairs += 1
                    linked += any(abs(L) > 0.5 for A in loops_of(cl[a]) for B in loops_of(cl[b]) if not np.isnan(L := _gauss(A, B)))
            if max_clusters and nclu >= max_clusters:
                return linked, pairs, nclu
    return linked, pairs, nclu
