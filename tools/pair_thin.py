"""Thin-tube version of the pair scan: for settings where the root's two children link (out/pair_scan.json), shrink the
tubes and ask whether the clasp survives clear tubes at depth 2, and whether the clasp repeats at depth 2.
Writes out/pair_thin.json.   python tools/pair_thin.py
"""
import sys, os, json, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np
import fractalkit as fk
import fractalkit.seamstats as S
from fractalkit.geometry import linking_matrix
from fractalkit.seamtree import densify_arc
OUT = os.path.join(os.path.dirname(__file__), "..", "out")


def sibling_links(t, depth=2):
    """For every network of generations 0..depth-1: do its two children link (all panel loops, coarse)?"""
    nets = t.networks(depth)
    kids = {}
    for i, n in enumerate(nets):
        if n[4] >= 0: kids.setdefault(n[4], []).append(i)
    loops = lambda i: [nets[i][1] + nets[i][2] * densify_arc(p, 0.1, closed=True)[:-1] @ nets[i][0].T for p in t.panels]
    out = []
    for par, (a, b) in kids.items():
        A, B = loops(a), loops(b); L, touch = linking_matrix(A + B, max_densify=2); n = len(A)
        out.append(bool(((np.abs(np.nan_to_num(L[:n, n:])) > .5) & ~touch[:n, n:]).any()))
    return out


if __name__ == "__main__":
    base = [r for r in json.load(open(os.path.join(OUT, "pair_scan.json"))) if r["kind"] == "A" and r["sib_linked_pairs"] > 0]
    rows, t0 = [], time.time()
    for r in base:
        for kind in "ABCD":
            for tube in (0.004, 0.008):
                t = fk.PairHatTrick(kind=kind, shrink=r["shrink"], sink=r["sink"], turn=r["turn"], tube=tube, iters=3)
                clear, fused, pc = S.clearance(t, 2, 0.01)
                nets = t.networks(1)
                from scipy.spatial import cKDTree
                pts = [np.vstack([nets[i][1] + nets[i][2] * densify_arc(c, 0.01) @ nets[i][0].T for c in t.curves]) for i in (1, 2)]
                sib_gap = float(cKDTree(pts[0]).query(pts[1])[0].min() - tube * (nets[1][2] + nets[2][2]))
                row = dict(kind=kind, shrink=r["shrink"], sink=r["sink"], turn=r["turn"], tube=tube, sib_gap=sib_gap,
                           clearance2=float(clear), fused2=int(fused), pc_clearance=float(pc))
                if fused == 0 and sib_gap > 0:
                    row["sibling_links_by_generation"] = sibling_links(t, 2)
                rows.append(row)
                print({k: (round(v, 4) if isinstance(v, float) else v) for k, v in row.items()}, f"{time.time()-t0:.0f}s", flush=True)
    json.dump(rows, open(os.path.join(OUT, "pair_thin.json"), "w"), indent=1)
