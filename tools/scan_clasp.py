"""Random search for a Trionda sibling clasp: all 6 sibling pairs linked, tubes clear of each other and of the parent.
usage: python tools/scan_clasp.py [n_samples] [seed]"""
import sys, pathlib, time
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import numpy as np
import fractalkit as fk
from fractalkit.geometry import linking_matrix, curve_distance
from fractalkit.seamtree import densify_arc
from scipy.spatial import cKDTree


def metrics(f, link=True):
    """Depth-1 clasp metrics of a SeamTree.  Children on the same site form a *cluster* (the sibling trio of a
    Trionda vertex); we count linked loop pairs within clusters, between clusters and with the parent, and the
    tube clearances (distance between seam curves minus the two tube radii; < 0 means the tubes fuse)."""
    tube = f.tube
    nets = f.networks(1)
    S = [c for _, c in f.seams(1)]; nc = len(f.curves)
    pts = [np.vstack([densify_arc_world(c) for c in S[i*nc:(i+1)*nc]]) for i in range(len(nets))]
    trees = [cKDTree(p) for p in pts]
    site = {i: f.map_site[i - 1] for i in range(1, len(nets))}
    def gap(a, b): return trees[a].query(pts[b])[0].min() - tube * (nets[a][2] + nets[b][2])
    kids = range(1, len(nets))
    within = [(a, b) for a in kids for b in kids if a < b and site[a] == site[b]]
    across = [(a, b) for a in kids for b in kids if a < b and site[a] != site[b]]
    out = dict(within_clear=min(gap(a, b) for a, b in within), across_clear=min(gap(a, b) for a, b in across),
               pc_clear=min(gap(0, b) for b in kids))
    if link:
        loops, who = f.panel_loops(1)
        L, touch = linking_matrix(loops, max_densify=5)
        linked = {}
        for i in range(len(loops)):
            for j in range(i + 1, len(loops)):
                a, b = who[i][0], who[j][0]
                if a == b or touch[i, j] or abs(L[i, j]) < 0.5: continue
                linked.setdefault((a, b), []).append(round(L[i, j]))
        out.update(within_linked=sum(k in linked for k in within), within_total=len(within),
                   across_linked=sum(k in linked for k in across), pc_linked=sum((0, b) in linked for b in kids),
                   loop_pairs_within=sum(len(linked[k]) for k in within if k in linked))
    return out


def densify_world(c): return c
def densify_arc_world(c, step=0.01):
    """Points of a world-frame polyline, a point every ~step (no sphere projection)."""
    seg = np.linalg.norm(np.diff(c, axis=0), axis=1)
    m = np.maximum(1, np.ceil(seg / step).astype(int))
    return np.vstack([c[i] + (c[i + 1] - c[i]) * (np.arange(k) / k)[:, None] for i, k in enumerate(m)] + [c[-1:]])


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 400
    rng = np.random.default_rng(int(sys.argv[2]) if len(sys.argv) > 2 else 0)
    good, t0 = [], time.time()
    for it in range(n):
        p = dict(shrink=rng.uniform(0.3, 0.7), sink=rng.uniform(0, 1), lateral=rng.uniform(0.3, 2.5), lean=rng.uniform(0, 360), turn=rng.uniform(0, 120))
        f = fk.TriondaHorned(iters=2, tube=0.05, **p)
        m = metrics(f, link=False)
        if m["within_clear"] < 0.0 or m["within_clear"] > 0.3:       # fused, or too far apart to clasp
            continue
        m = metrics(f)
        if m["within_linked"] == m["within_total"]:
            good.append((p, m)); print({k: round(v, 2) for k, v in p.items()}, {k: round(float(v), 3) for k, v in m.items()}, flush=True)
    print(f"{len(good)} clasping of {n} samples, {time.time()-t0:.0f}s")
