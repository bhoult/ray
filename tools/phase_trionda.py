"""Phase diagrams of the Trionda sibling clasp over (shrink, lateral), (lean, turn) and (shrink, sink), plus the
chirality control (clasp rate against the seam amplitude).   usage: python tools/phase_trionda.py [quick]"""
import sys, pathlib, json, time
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import numpy as np
import fractalkit as fk
import fractalkit.seamstats as S
from fractalkit import clasp
from multiprocessing import Pool

BASE = dict(shrink=0.628, sink=0.003, lateral=0.792, lean=325.85, turn=7.61)
QUICK = len(sys.argv) > 1 and sys.argv[1] == "quick"


def cell(p):
    t = fk.TriondaHorned(iters=3, tube=0.05, **p)
    m = clasp.metrics(t)
    r = dict(p=p, within=m["within_linked"] / max(m["within_pairs"], 1), within_clear=float(m["within_clear"]),
             pc_clear=float(m["pc_clear"]), pc_linked=m["pc_linked"] / m["cluster"], children=len(t.maps),
             sim=float(np.log(9) / np.log(1 / p["shrink"])))
    if r["within"] == 1 and r["within_clear"] >= 0:
        c2, f2, pc2 = S.clearance(t, 2, step=0.02)
        r.update(clear2=c2, fused2=f2)
    return r


def control(args):
    amp, n, seed = args
    rng = np.random.default_rng(seed)
    tested = ok = 0
    for _ in range(n):
        p = dict(shrink=rng.uniform(0.3, 0.7), sink=rng.uniform(0, 1), lateral=rng.uniform(0.3, 2.5), lean=rng.uniform(0, 360), turn=rng.uniform(0, 120))
        t = fk.TriondaHorned(iters=2, tube=0.05, amp=amp, harmonic=0.035 * amp / 0.21, **p)
        m = clasp.metrics(t, link=False)
        if 0 <= m["within_clear"] <= 0.3:
            tested += 1
            m = clasp.metrics(t)
            ok += m["within_linked"] == m["within_pairs"]
    return amp, tested, ok


if __name__ == "__main__":
    n = 8 if QUICK else 14
    grids = {
        "shrink_lateral": ("shrink", np.linspace(0.35, 0.75, n), "lateral", np.linspace(0.3, 1.6, n)),
        "lean_turn": ("lean", np.linspace(0, 360, n, endpoint=False), "turn", np.linspace(0, 120, n, endpoint=False)),
        "shrink_sink": ("shrink", np.linspace(0.35, 0.75, n), "sink", np.linspace(0.0, 1.0, n)),
    }
    out, t0 = {}, time.time()
    with Pool(2) as pool:
        for name, (kx, xs, ky, ys) in grids.items():
            cells = [dict(BASE, **{kx: x, ky: y}) for y in ys for x in xs]
            res = pool.map(cell, cells, chunksize=4)
            out[name] = dict(kx=kx, ky=ky, xs=xs.tolist(), ys=ys.tolist(), cells=res)
            print(name, "done", f"[{time.time()-t0:.0f}s]", flush=True)
        amps = [0.0, 0.07, 0.14, 0.21, 0.28]
        out["control"] = [dict(zip(("amp", "tested", "clasping"), r)) for r in pool.map(control, [(a, 40 if QUICK else 150, 3) for a in amps])]
        print("control", out["control"], flush=True)
    pathlib.Path("out/phase_trionda.json").write_text(json.dumps(out, default=float))
