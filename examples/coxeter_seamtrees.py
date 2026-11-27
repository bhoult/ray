"""Coxeter seam trees: the straight tetra / cube / icosa balls with the rotation group against the full reflection group.

Needs out/coxeter_scan_free.json and out/coxeter_scan_mirror.json from tools/coxeter_scan.py.
Writes out/coxeter_clasp_rates.png and out/coxeter_gallery.png.
"""
from _common import *
import json

free = json.loads((OUT / "coxeter_scan_free.json").read_text())
mirror = json.loads((OUT / "coxeter_scan_mirror.json").read_text())

# ---- clasp rate by ball, site and group -------------------------------------------------------------------------------------
fig, axs = plt.subplots(1, 2, figsize=(12, 4.2), sharey=True)
for ax, res, title in ((axs[0], free, "random shift, turn and lean"), (axs[1], mirror, "no turn, lean 0 (full group: each child fixed by a mirror)")):
    labels = sorted({(r["kind"], r["site"]) for r in res}, key=lambda x: (["tetra", "cube", "icosa"].index(x[0]), ["vertex", "face", "edge"].index(x[1])))
    x = np.arange(len(labels))
    for k, (full, col, name) in enumerate(((False, "tab:blue", "rotation group"), (True, "tab:orange", "full reflection group"))):
        vals, tested = [], []
        for lab in labels:
            r = next((r for r in res if (r["kind"], r["site"]) == lab and r["full"] == full), None)
            vals.append(r["clasping"] / max(r["tested"], 1) if r else np.nan); tested.append(r["tested"] if r else 0)
        ax.bar(x + (k - 0.5) * 0.38, vals, 0.38, color=col, label=name)
        for xi, v, n in zip(x, vals, tested):
            ax.text(xi + (k - 0.5) * 0.38, (0 if np.isnan(v) else v) + 0.01, f"n={n}", ha="center", fontsize=6, rotation=90)
    ax.set_xticks(x); ax.set_xticklabels([f"{a}\n{b}" for a, b in labels], fontsize=8); ax.set_title(title, fontsize=9)
    ax.set_ylim(0, 1.05)
axs[0].set_ylabel("share of placements (tubes not fused) whose\nsibling cluster has every pair linked"); axs[0].legend(fontsize=8)
fig.tight_layout(); fig.savefig(OUT / "coxeter_clasp_rates.png", dpi=130); print("wrote", OUT / "coxeter_clasp_rates.png")

# ---- the best clasping placements, and a full-group tree for comparison ----------------------------------------------------------
def best(res, kind, site, full):
    r = next(r for r in res if r["kind"] == kind and r["site"] == site and r["full"] == full)
    return r["best"][1] if r["best"] else None


cam = lambda t, az=35, el=25, d=6.0: dict(camera=fk.Camera(azimuth=az, elevation=el, distance=d), step=0.7, max_steps=250, palette=t.cmap(), trap_weight=0.85)
items = []
for title, res, kind, site, full in (("tetra, edge sites, rotations", mirror, "tetra", "edge", False),
                                      ("cube, vertex sites, rotations", mirror, "cube", "vertex", False),
                                      ("cube, edge sites, rotations", mirror, "cube", "edge", False)):
    p = best(res, kind, site, full)
    t = fk.PolyBall(kind, site, full, iters=2, body=0.95, tube=0.05, **p)
    items.append((title, t, cam(t)))
for title, kind, site in (("tetra, vertex sites, reflection group", "tetra", "vertex"), ("cube, vertex sites, reflection group", "cube", "vertex"),
                          ("icosa, vertex sites, reflection group", "icosa", "vertex")):
    t = fk.PolyBall(kind, site, True, iters=2, body=0.95, tube=0.05, shrink=0.4, sink=0.4, lateral=0.0, turn=0.0)
    items.append((title, t, cam(t, d=6.5)))
grid(items, name="coxeter_gallery", size=(220, 165))

# ---- the depth-2 clasp with thin tubes ------------------------------------------------------------------------------------------------
dc = fk.TriondaHorned.deep_clasp(iters=3, body=0.95)
items = [("Trionda deep clasp, depth 2", dc.with_iters(3) if hasattr(dc, "with_iters") else dc, cam(dc, d=7.0)),
         ("Trionda deep clasp, other side", dc, cam(dc, az=215, el=-20, d=7.0))]
grid(items, ncols=2, name="deep_clasp", size=(300, 225))
