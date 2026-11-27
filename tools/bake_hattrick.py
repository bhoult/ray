"""Bake the Hat Trick ball (hexasphere) into fractalkit/data/hattrick.npz: panel outlines, colour classes, pentagon centres.
usage: python tools/bake_hattrick.py /path/to/hexasphere/src"""
import sys, pathlib
import numpy as np
sys.path.insert(0, sys.argv[1])
from hexasphere.hattrick import HatBall
b = HatBall.from_frequency(3); b.relax()
P = b.panels()
loops, kinds, cls = [], [], []
pent_ids = list(b.pent)
for p in P:
    loops.append(np.array(p["outline"])); kinds.append(p["kind"])
    cls.append(0 if p["kind"] == "pentagon" else 1 + next(i for i, q in enumerate(pent_ids) if p["id"] in b.pentagon_hats(q)))
centres = np.array([b.X[b.ids[("v", q)]] if ("v", q) in b.ids else b.V[q] for q in pent_ids]) if False else np.array([b.V[q] for q in pent_ids])
lens = np.array([len(l) for l in loops])
np.savez_compressed(pathlib.Path(__file__).resolve().parents[1] / "fractalkit" / "data" / "hattrick.npz",
                    pts=np.vstack(loops), lens=lens, cls=np.array(cls), pent=np.array([k == "pentagon" for k in kinds]), centres=centres)
print(len(loops), lens.sum(), centres.shape, np.linalg.norm(centres, axis=1)[:3])
