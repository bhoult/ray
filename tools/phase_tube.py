"""Fused tube pairs at depth 2 over (shrink, tube) for the Trionda clasp preset; linking checked in the clear cells.
usage: python tools/phase_tube.py"""
import sys, pathlib, json, time
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import numpy as np
import fractalkit as fk
import fractalkit.seamstats as S
from fractalkit import clasp
from multiprocessing import Pool

BASE = dict(sink=0.003, lateral=0.792, lean=325.85, turn=7.61)


def cell(a):
    s, tube = a
    t = fk.TriondaHorned(iters=3, tube=tube, shrink=s, **BASE)
    c2, f2, pc = S.clearance(t, 2, step=0.02)
    r = dict(shrink=s, tube=tube, clear2=c2, fused2=f2)
    if f2 == 0:
        lk, pr, nc = clasp.cluster_linking(t, 2)
        r.update(linked=lk, pairs=pr, clusters=nc)
    return r


if __name__ == "__main__":
    shr = np.linspace(0.3, 0.7, 9); tubes = np.array([0.003, 0.005, 0.0075, 0.01, 0.015, 0.02, 0.03, 0.05])
    with Pool(2) as p:
        res = p.map(cell, [(s, t) for t in tubes for s in shr], chunksize=3)
    pathlib.Path("out/phase_tube.json").write_text(json.dumps(dict(shrink=shr.tolist(), tube=tubes.tolist(), cells=res), default=float))
    ok = [r for r in res if r["fused2"] == 0]
    print(len(ok), "clear cells;", sum(r.get("linked") == r.get("pairs") for r in ok), "of them linked in every cluster to depth 2")
