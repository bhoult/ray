"""Compare the seam-network mirror distance with qwla's 4-ring term chi4 on Hat Trick tilings.

For random ring patterns b (12 bits): re-tile the ball (qwla's ring flips), take the outlines of all hats and the
12 pentagon loops as the seam network, and compute fractalkit.seamstats.chirality of that network.  Compare with
chi4(b) (qwla/hat_chirality.py).  Writes out/chi4_comparison.json and .png.

    python tools/chi4_comparison.py /path/to/qwla /path/to/hexasphere/src [n_patterns]
"""
import sys, os, json, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
QWLA, HEX = sys.argv[1], sys.argv[2]
NPAT = int(sys.argv[3]) if len(sys.argv) > 3 else 120
sys.path.insert(0, HEX); sys.path.insert(0, QWLA)
_argv, sys.argv = sys.argv, [sys.argv[0]]
import numpy as np
import ring_bits as RB
import hat_chirality as HC
sys.argv = _argv
import fractalkit.seamstats as S
from fractalkit.seamtree import densify_arc

OUT = os.path.join(os.path.dirname(__file__), "..", "out")


def network_points(ball, fl, T):
    """Dense points of every hat outline in tiling T plus the 12 pentagon loops (unit sphere)."""
    pts = []
    for i in T:
        loop = ball._outline(sorted(fl.placements[i]))[0]
        pts.append(densify_arc(ball.polygon(loop), 0.01, closed=True))
    for p in ball.pent:
        pts.append(densify_arc(ball.polygon(ball.pentagon_loop(p)), 0.01, closed=True))
    P = np.vstack(pts)
    return P / np.linalg.norm(P, axis=1, keepdims=True)


if __name__ == "__main__":
    t0 = time.time()
    ball, fl, T0, rings, P, edges = RB.setup()
    ball.relax()
    paths = HC.helical_paths(P, edges)
    print("setup", round(time.time() - t0), "s", flush=True)
    rng = np.random.default_rng(1)
    pats = [tuple([0] * 12)] + [tuple(int(x) for x in rng.integers(0, 2, 12)) for _ in range(NPAT)]
    rows = []
    for b in pats:
        T = RB.tiling_of(b, T0, rings)
        pts = network_points(ball, fl, T)
        rows.append(dict(bits=b, chi4=int(HC.chi4(b, paths)), mirror_distance=S.chirality(pts, starts=8, sub=1200),
                         flipped=int(sum(b)), n_points=len(pts)))
        print(len(rows), rows[-1]["chi4"], round(rows[-1]["mirror_distance"], 3), f"{time.time()-t0:.0f}s", flush=True)
    json.dump(rows, open(os.path.join(OUT, "chi4_comparison.json"), "w"))
