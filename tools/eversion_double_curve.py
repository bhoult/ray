"""Double-curve census of the Bednorz-Bednorz n=2 family: components and size vs t."""
import sys, os, json
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from scipy.spatial import cKDTree
from scipy.sparse.csgraph import connected_components
from scipy.sparse import coo_matrix
from fractalkit import eversion as E

OUT = os.path.join(os.path.dirname(__file__), "..", "out")

def census(t, H=1.6, nh=81, nphi=160, link=0.12):
    V, F, hp = E.mesh(t, H=H, nh=nh, nphi=nphi)
    pr, pts = E.self_intersections(V, F)
    if len(pts) == 0:
        return dict(t=t, pairs=0, comps=0, big=0)
    pts = np.unique(np.round(pts, 6), axis=0)
    tr = cKDTree(pts); e = tr.query_pairs(link, output_type="ndarray")
    g = coo_matrix((np.ones(len(e)), (e[:, 0], e[:, 1])), shape=(len(pts),)*2)
    nc, lab = connected_components(g, directed=False)
    sizes = np.bincount(lab)
    big = int((sizes > 0.02*len(pts)).sum())
    return dict(t=float(t), pairs=int(len(pr)), comps=int(nc), big=big, pts=int(len(pts)))

if __name__ == "__main__":
    ts = np.round(np.arange(-1.5, 1.5001, 0.1), 3)
    rows = [census(t) for t in ts]
    for r in rows: print(r)
    json.dump(rows, open(os.path.join(OUT, "eversion_double_curve.json"), "w"), indent=1)
    fig, ax = plt.subplots(1, 2, figsize=(9, 3.2), facecolor=(.97, .97, .975))
    ax[0].plot(ts, [r["pairs"] for r in rows], "o-"); ax[0].set_ylabel("intersecting triangle pairs")
    ax[1].plot(ts, [r["big"] for r in rows], "s-"); ax[1].set_ylabel("double-curve pieces (>2% of samples)")
    for a in ax:
        a.set_xlabel("t")
        for tv in (-1, -.5616, .5616, 1): a.axvline(tv, c="gray", ls=":", lw=.8)
    fig.tight_layout(); fig.savefig(os.path.join(OUT, "eversion_double_curve.png"), dpi=120)
