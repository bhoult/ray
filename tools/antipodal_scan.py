"""The Hat Trick as a binary horned tree: children on an antipodal pair of pentagons.

For each (shrink, sink, turn): sibling and parent-child linking of the pentagon loops, and tube clearance at depth 2.
Writes out/antipodal_scan.json.   python tools/antipodal_scan.py [--quick]
"""
import sys, os, json, time, itertools
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np
import fractalkit as fk
import fractalkit.seamstats as S
from fractalkit.geometry import linking_matrix

OUT = os.path.join(os.path.dirname(__file__), "..", "out")


def linked_pairs(tree, nets_pairs, depth=2):
    """For each (a, b) of network indices: do any two pentagon loops link?  (touching pairs count as not linked.)"""
    from fractalkit.seamtree import densify_arc
    nets = tree.networks(depth)
    pent = [k for k in range(len(tree.panels)) if tree.classes[k] == 0]
    cache = {}
    def loops(i):
        if i not in cache:
            M, t, Sc = nets[i][:3]
            cache[i] = [t + Sc * densify_arc(tree.panels[k], 0.05, closed=True)[:-1] @ M.T for k in pent]
        return cache[i]
    out = {}
    for a, b in nets_pairs:
        A, B = loops(a), loops(b)
        L, touch = linking_matrix(A + B, max_densify=5)
        n = len(A)
        sub, tch = L[:n, n:], touch[:n, n:]
        out[(a, b)] = bool(((np.abs(np.nan_to_num(sub)) > 0.5) & ~tch).any())
    return out


def cell(shrink, sink, turn, tube=0.025):
    t = fk.AntipodalHatTrick(shrink=shrink, sink=sink, turn=turn, tube=tube, iters=3)
    nets = t.networks(2)
    kids = {}
    for i, n in enumerate(nets):
        if n[4] >= 0: kids.setdefault(n[4], []).append(i)
    sib = [tuple(c) for c in kids.values() if len(c) == 2]
    pc = [(n[4], i) for i, n in enumerate(nets) if n[4] >= 0]
    L = linked_pairs(t, sib[:1] + pc[:2])                 # root siblings, root->child
    clear, fused, pcl = S.clearance(t, 2, 0.02)
    return dict(shrink=shrink, sink=sink, turn=turn, sibling_linked=L[sib[0]], pc_linked=int(sum(L[p] for p in pc[:2])),
                clearance=float(clear), fused=int(fused), pc_clearance=float(pcl),
                sim_dim=float(np.log(2) / np.log(1 / shrink)))


if __name__ == "__main__" and "--all-panels" not in sys.argv:
    quick = "--quick" in sys.argv
    shr = [0.3, 0.45, 0.6, 0.75] if quick else [0.3, 0.4, 0.5, 0.6, 0.7, 0.8]
    snk = [0.0, 0.4] if quick else [0.0, 0.2, 0.4, 0.6]
    trn = [0.0, 36.0] if quick else [0.0, 10.0, 36.0]
    rows = []
    t0 = time.time()
    for s_, k_, u_ in itertools.product(shr, snk, trn):
        r = cell(s_, k_, u_); rows.append(r)
        print({k: (round(v, 3) if isinstance(v, float) else v) for k, v in r.items()}, f"{time.time()-t0:.0f}s", flush=True)
    json.dump(rows, open(os.path.join(OUT, "antipodal_scan.json"), "w"), indent=1)


def all_panels(shrink, sink, turn, tube=0.025):
    """Depth-1 linking with every one of the 72 panel loops: (parent-child loop pairs linked, sibling loop pairs linked, sibling gap)."""
    from fractalkit.seamtree import densify_arc
    t = fk.AntipodalHatTrick(shrink=shrink, sink=sink, turn=turn, tube=tube, iters=2)
    nets = t.networks(1)
    def loops(i):
        M, tt, Sc = nets[i][:3]
        return [tt + Sc * densify_arc(p, 0.08, closed=True)[:-1] @ M.T for p in t.panels]
    def count(a, b):
        A, B = loops(a), loops(b); L, touch = linking_matrix(A + B, max_densify=3); n = len(A)
        return int(((np.abs(np.nan_to_num(L[:n, n:])) > 0.5) & ~touch[:n, n:]).sum())
    pc = count(0, 1) + count(0, 2)
    sib = count(1, 2)
    return dict(shrink=shrink, sink=sink, turn=turn, pc_loop_pairs=pc, sibling_loop_pairs=sib)


if __name__ == "__main__" and "--all-panels" in sys.argv:
    rows = []
    for s_, k_ in itertools.product([0.3, 0.45, 0.6, 0.75], [0.0, 0.4]):
        r = all_panels(s_, k_, 10.0); rows.append(r); print(r, flush=True)
    json.dump(rows, open(os.path.join(OUT, "antipodal_allpanels.json"), "w"), indent=1)
