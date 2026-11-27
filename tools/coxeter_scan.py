"""Random search for clasping clusters on the Coxeter balls, rotation group vs full reflection group.
usage: python tools/coxeter_scan.py [samples] [out.json]"""
import sys, pathlib, json, time
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import numpy as np
import fractalkit as fk
from fractalkit import clasp
from multiprocessing import Pool


def run(args):
    kind, site, full, n, seed = args[:5]
    mirror = len(args) > 5 and args[5]       # mirror mode: no turn, no lean, so mirrors fix the children
    rng = np.random.default_rng(seed)
    best, nclasp, ntested, nclear, csize = None, 0, 0, 0, 0
    for _ in range(n):
        p = dict(shrink=rng.uniform(0.3, 0.75), sink=rng.uniform(0, 1), lateral=rng.uniform(0.3, 2.5),
                 lean=rng.uniform(0, 360), turn=rng.uniform(0, 120))
        if mirror:
            p.update(lean=0.0, turn=0.0)
        t = fk.PolyBall(kind, site, full, iters=2, tube=0.05, **p)
        m = clasp.metrics(t, link=False)
        csize = m["cluster"]
        if not (0 <= m["within_clear"] <= 0.3) or m["cluster"] < 2:
            continue
        ntested += 1
        m = clasp.metrics(t)
        if m["within_linked"] == m["within_pairs"] and m["within_pairs"] > 0:
            nclasp += 1
            score = min(m["within_clear"], m["pc_clear"])
            if best is None or score > best[0]:
                best = (score, p, {k: float(v) for k, v in m.items()})
    return dict(kind=kind, site=site, full=full, mirror=bool(mirror), samples=n, cluster=csize, tested=ntested, clasping=nclasp, best=best)


if __name__ == "__main__":
    n = int(sys.argv[1]) if len(sys.argv) > 1 else 100
    jobs = [(k, s, f, n, 7) for k in ("tetra", "cube") for s in ("vertex", "face", "edge") for f in (False, True)]
    jobs += [("icosa", "vertex", f, max(n // 6, 8), 7) for f in (False, True)]          # 60-120 children each: slow
    jobs += [(k, s, f, n, 11, True) for k in ("tetra", "cube") for s in ("vertex", "face", "edge") for f in (False, True)]
    jobs += [("icosa", "vertex", f, max(n // 6, 8), 11, True) for f in (False, True)]
    if len(sys.argv) > 3 and sys.argv[3] == "mirror":
        jobs = [j for j in jobs if len(j) > 5]
    t0 = time.time()
    with Pool(2) as pool:
        res = []
        for r in pool.imap(run, jobs):
            res.append(r)
            print(f"{r['kind']:5s} {r['site']:6s} {'full' if r['full'] else 'rot '} {'mirror' if r['mirror'] else 'free  '}: cluster {r['cluster']}  tested {r['tested']:3d}/{r['samples']}  clasping {r['clasping']:3d}"
                  + (f"  best clear {r['best'][0]:+.3f}" if r['best'] else ""), f"[{time.time()-t0:.0f}s]", flush=True)
    pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "out/coxeter_scan.json").write_text(json.dumps(res, indent=1, default=float))
