"""Self-intersection census along the full closed-sphere eversion (Bednorz-Bednorz, n=2)."""
import sys, os, json
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from fractalkit import eversion as E
OUT = os.path.join(os.path.dirname(__file__), "..", "out")

if __name__ == "__main__":
    ss = np.linspace(0, 1, 41); rows = []
    for s in ss:
        par = E.schedule(s); V, F, U = E.sphere_mesh(101, 200, **par)
        pr, pt = E.self_intersections(V, F)
        rows.append(dict(s=float(s), t=par["t"], pairs=int(len(pr)), volume=float(E.signed_volume(V, F))))
        print(rows[-1], flush=True)
    json.dump(rows, open(os.path.join(OUT, "eversion_sphere_census.json"), "w"), indent=1)
    fig, ax = plt.subplots(1, 2, figsize=(9, 3.2), facecolor=(.97, .97, .975))
    ax[0].plot(ss, [r["volume"] for r in rows], "o-"); ax[0].axhline(0, c="gray", lw=.7); ax[0].set_ylabel("signed volume")
    ax[1].semilogy(ss, [max(r["pairs"], 0.8) for r in rows], "s-"); ax[1].set_ylabel("intersecting triangle pairs")
    for a in ax: a.set_xlabel("movie parameter s")
    fig.tight_layout(); fig.savefig(os.path.join(OUT, "eversion_sphere_census.png"), dpi=120)
