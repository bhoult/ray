"""Sibling clasp for the Hat Trick on adjacent pentagons: do the two children of the root link each other?

Kinds A-D (where the parent's pentagon sits relative to the child pair), over shrink, sink, turn.  Linking uses all
72 panel loops of each child (coarse: step 0.1, 2 refinements).  Also the sibling tube gap and the depth-2 clearance.
Writes out/pair_scan.json.    python tools/pair_scan.py
"""
import sys, os, json, time, itertools
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np
import fractalkit as fk
import fractalkit.seamstats as S
from fractalkit.geometry import linking_matrix
from fractalkit.seamtree import densify_arc
from scipy.spatial import cKDTree

OUT = os.path.join(os.path.dirname(__file__), "..", "out")


def cell(kind, shrink, sink, turn, tube=0.025):
    t = fk.PairHatTrick(kind=kind, shrink=shrink, sink=sink, turn=turn, tube=tube, iters=2)
    nets = t.networks(1)
    loops = lambda i: [nets[i][1] + nets[i][2] * densify_arc(p, 0.1, closed=True)[:-1] @ nets[i][0].T for p in t.panels]
    A, B = loops(1), loops(2)
    L, touch = linking_matrix(A + B, max_densify=2); n = len(A)
    sub, tc = np.nan_to_num(L[:n, n:]), touch[:n, n:]
    pts = [np.vstack([nets[i][1] + nets[i][2] * densify_arc(c, 0.02) @ nets[i][0].T for c in t.curves]) for i in (1, 2)]
    gap = float(cKDTree(pts[0]).query(pts[1])[0].min() - tube * (nets[1][2] + nets[2][2]))
    return dict(kind=kind, shrink=shrink, sink=sink, turn=turn, sib_linked_pairs=int(((np.abs(sub) > .5) & ~tc).sum()),
                sib_touching=int(tc.sum()), sib_gap=gap)


if __name__ == "__main__":
    rows, t0 = [], time.time()
    for kind, s_, k_, u_ in itertools.product("ABCD", [0.5, 0.6, 0.7, 0.8], [0.0, 0.2, 0.4, 0.6], [0.0, 36.0]):
        r = cell(kind, s_, k_, u_); rows.append(r)
        print({k: (round(v, 3) if isinstance(v, float) else v) for k, v in r.items()}, f"{time.time()-t0:.0f}s", flush=True)
    json.dump(rows, open(os.path.join(OUT, "pair_scan.json"), "w"), indent=1)
