"""Signed-style chi4 test: does the seam network of a ring pattern b fail to be the mirror image of the network of
the reflected pattern s.b, and does that failure follow qwla's 4-ring term chi4?

D(b) = min over rotations R of the mean nearest-point distance (degrees) from R.mirror(N(s.b)) to N(b), where N(b)
is the seam network (all hat outlines plus the 12 pentagon loops) of the tiling with ring bits b and s is the
icosahedron reflection of qwla/hat_chirality.py.  D is even under b <-> s.b, as |chi4| is.  D = 0 would mean T(s.b)
is exactly the mirror image of T(b).  For a pattern that a rotation maps to its mirror pattern (an achiral pattern),
N(s.b) is a rotation of N(b) and D is the plain mirror distance of N(b).

    HEXASPHERE_DIR=/path/to/hexasphere python tools/chi4_signed.py /path/to/qwla /path/to/hexasphere/src [n]
"""
import sys, os, json, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
QWLA, HEX = sys.argv[1], sys.argv[2]
NPAT = int(sys.argv[3]) if len(sys.argv) > 3 else 300
sys.path.insert(0, HEX); sys.path.insert(0, QWLA)
_argv, sys.argv = sys.argv, [sys.argv[0]]
import numpy as np
from scipy.spatial import cKDTree
from scipy.optimize import minimize
from scipy.spatial.transform import Rotation
import ring_bits as RB
import hat_chirality as HC
sys.argv = _argv
from fractalkit.seamtree import densify_arc

OUT = os.path.join(os.path.dirname(__file__), "..", "out")


def network_points(ball, fl, T):
    pts = [densify_arc(ball.polygon(ball._outline(sorted(fl.placements[i]))[0]), 0.01, closed=True) for i in T]
    pts += [densify_arc(ball.polygon(ball.pentagon_loop(p)), 0.01, closed=True) for p in ball.pent]
    P = np.vstack(pts)
    return P / np.linalg.norm(P, axis=1, keepdims=True)


def mirror_distance(A, B, starts=8, sub=1200, seed=0):
    """min over rotations of mean nearest distance (degrees) from R.mirror(B) to A."""
    rng = np.random.default_rng(seed)
    kd = cKDTree(A)
    Bs = B[rng.choice(len(B), min(sub, len(B)), replace=False)] * np.array([1, 1, -1])
    cost = lambda v: kd.query(Bs @ Rotation.from_rotvec(v).as_matrix().T)[0].mean()
    best = min(minimize(cost, Rotation.random(random_state=rng.integers(1 << 31)).as_rotvec(), method="Nelder-Mead",
                        options=dict(xatol=1e-3, fatol=1e-6, maxiter=300)).fun for _ in range(starts))
    return float(np.degrees(best))


if __name__ == "__main__":
    t0 = time.time()
    ball, fl, T0, rings, P, edges = RB.setup(); ball.relax()
    paths = HC.helical_paths(P, edges)
    perms = RB.rotation_perms(ball, fl, T0, rings, P)
    g = np.asarray(ball.G[1]); refl = [int(np.argmax(P @ (-(g @ p)))) for p in P]
    act = lambda b, perm: tuple(b[perm.index(i)] for i in range(12))      # b2[perm[i]] = b[i]
    rng = np.random.default_rng(7)
    pats = [tuple(int(x) for x in rng.integers(0, 2, 12)) for _ in range(NPAT)]
    rows = []
    for b in pats:
        sb = act(b, refl)
        orbit = {act(b, p) for p in perms}
        chiral = sb not in orbit
        Nb = network_points(ball, fl, RB.tiling_of(b, T0, rings))
        Ns = Nb if sb == b else network_points(ball, fl, RB.tiling_of(sb, T0, rings))
        rows.append(dict(bits=b, chi4=int(HC.chi4(b, paths)), chi4_reflected=int(HC.chi4(sb, paths)), flipped=int(sum(b)),
                         chiral_pattern=bool(chiral), D=mirror_distance(Nb, Ns)))
        print(len(rows), rows[-1]["chi4"], rows[-1]["chi4_reflected"], chiral, round(rows[-1]["D"], 3), f"{time.time()-t0:.0f}s", flush=True)
    json.dump(rows, open(os.path.join(OUT, "chi4_signed.json"), "w"))
