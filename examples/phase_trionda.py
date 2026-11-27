"""Phase diagrams of the Trionda sibling clasp (data from tools/phase_trionda.py), and the chirality control.
Writes out/phase_trionda.png, out/chirality_control.png, out/phase_windows.json."""
from _common import *
import json
from fractalkit import clasp

D = json.loads((OUT / "phase_trionda.json").read_text())
BASE = dict(shrink=0.628, sink=0.003, lateral=0.792, lean=325.85, turn=7.61)
names = {"shrink": "shrink", "lateral": "lateral shift (child radii)", "lean": "lean (deg)", "turn": "turn (deg)", "sink": "sink"}
grids = ["shrink_lateral", "lean_turn", "shrink_sink"]
fig, axs = plt.subplots(3, 3, figsize=(14, 11))
windows = []
for r, g in enumerate(grids):
    G = D[g]; xs, ys = np.array(G["xs"]), np.array(G["ys"]); nx, ny = len(xs), len(ys)
    cells = G["cells"]
    W = np.array([c["within"] for c in cells]).reshape(ny, nx)
    WC = np.array([c["within_clear"] for c in cells]).reshape(ny, nx)
    F2 = np.array([c.get("fused2", np.nan) for c in cells], float).reshape(ny, nx)
    ext = [xs[0] - (xs[1] - xs[0]) / 2, xs[-1] + (xs[1] - xs[0]) / 2, ys[0] - (ys[1] - ys[0]) / 2, ys[-1] + (ys[1] - ys[0]) / 2]
    kw = dict(origin="lower", extent=ext, aspect="auto")
    im = axs[r, 0].imshow(W, cmap="viridis", vmin=0, vmax=1, **kw); fig.colorbar(im, ax=axs[r, 0], label="sibling pairs linked (share)")
    im = axs[r, 1].imshow(np.clip(WC, -0.1, 0.1), cmap="RdBu", vmin=-0.1, vmax=0.1, **kw); fig.colorbar(im, ax=axs[r, 1], label="sibling tube clearance (<0 fused)")
    im = axs[r, 2].imshow(np.log10(1 + F2), cmap="magma_r", vmin=0, vmax=2.6, **kw); fig.colorbar(im, ax=axs[r, 2], label="log10(1 + fused pairs at depth 2)")
    axs[r, 2].set_facecolor("0.9")
    for c, (i, j) in zip(cells, [(i, j) for i in range(ny) for j in range(nx)]):
        if c.get("fused2") == 0:
            windows.append((c["within_clear"], c["p"], g))
            axs[r, 2].plot(xs[j], ys[i], "c*", ms=9)
    for a in axs[r]:
        a.set_xlabel(names[G["kx"]]); a.set_ylabel(names[G["ky"]]); a.plot(BASE[G["kx"]], BASE[G["ky"]], "k+", ms=10, mew=2)
    if G["kx"] == "shrink":
        # similarity dimension of the 9-child tree: log 9 / log(1/s) = 3 at s = 1/sqrt(...)
        axs[r, 2].axvline(9 ** (-1 / 3), color="k", ls="--", lw=1)
axs[0, 0].set_title("depth-1 clasp: every sibling pair linked", fontsize=9)
axs[0, 1].set_title("tubes of siblings", fontsize=9)
axs[0, 2].set_title("depth 2: fused tube pairs (stars: none, dashed: similarity dim 3)", fontsize=9)
fig.suptitle("Trionda sibling clasp: black + is the preset", fontsize=11)
fig.tight_layout(); fig.savefig(OUT / "phase_trionda.png", dpi=130); print("wrote", OUT / "phase_trionda.png")

windows.sort(key=lambda w: -w[0])
print(len(windows), "cells with clasp at depth 1 and no fused pairs at depth 2")
rep = []
for wc, p, g in windows[:6]:
    t = fk.TriondaHorned(iters=3, tube=0.05, **p)
    lk, pr, nc = clasp.cluster_linking(t, 2)
    c2 = fk.seamstats.clearance(t, 2, step=0.02)
    rep.append(dict(grid=g, params=p, within_clear=wc, depth2_linked=lk, depth2_pairs=pr, depth2_clusters=nc, clear2=c2[0], fused2=c2[1]))
    print(rep[-1], flush=True)
(OUT / "phase_windows.json").write_text(json.dumps(dict(n_windows=len(windows), checked=rep), indent=1, default=float))

# ---- chirality control -------------------------------------------------------------------------------------------------------
C = D["control"]
fig, ax = plt.subplots(figsize=(5.5, 3.8))
ax.bar([f"{c['amp']:.2f}" for c in C], [c["clasping"] / max(c["tested"], 1) for c in C], color="tab:blue")
for i, c in enumerate(C):
    ax.text(i, c["clasping"] / max(c["tested"], 1) + 0.005, f"{c['clasping']}/{c['tested']}", ha="center", fontsize=8)
ax.set_xlabel("seam amplitude of the ball (0 = achiral tetrahedron, 0.21 = Trionda)"); ax.set_ylabel("share of placements with a clasping trio")
fig.tight_layout(); fig.savefig(OUT / "chirality_control.png", dpi=130); print("wrote", OUT / "chirality_control.png")

# ---- depth-2 window: shrink x tube ----------------------------------------------------------------------------------------------
P = json.loads((OUT / "phase_tube.json").read_text())
shr, tub = np.array(P["shrink"]), np.array(P["tube"])
F = np.array([c["fused2"] for c in P["cells"]], float).reshape(len(tub), len(shr))
CL = np.array([c["clear2"] for c in P["cells"]], float).reshape(len(tub), len(shr))
fig, axs = plt.subplots(1, 2, figsize=(11, 4.2))
im = axs[0].imshow(np.log10(1 + F), origin="lower", aspect="auto", cmap="magma_r", vmin=0, vmax=2.6,
                   extent=[shr[0] - 0.025, shr[-1] + 0.025, -0.5, len(tub) - 0.5]); fig.colorbar(im, ax=axs[0], label="log10(1 + fused pairs at depth 2)")
axs[0].set_yticks(range(len(tub))); axs[0].set_yticklabels([f"{t:g}" for t in tub]); axs[0].set_xlabel("shrink"); axs[0].set_ylabel("tube radius")
for c in P["cells"]:
    if c["fused2"] == 0 and c.get("linked") == c.get("pairs"):
        axs[0].plot(c["shrink"], list(tub).index(c["tube"]), "c*", ms=10)
axs[0].set_title("stars: no fused pairs at depth 2 and all 120 sibling pairs\nlinked in the 40 clusters of depths 1-2", fontsize=9)
im = axs[1].imshow(np.clip(CL, -0.06, 0.02), origin="lower", aspect="auto", cmap="RdBu", vmin=-0.06, vmax=0.06,
                   extent=[shr[0] - 0.025, shr[-1] + 0.025, -0.5, len(tub) - 0.5]); fig.colorbar(im, ax=axs[1], label="smallest tube gap between unrelated networks")
axs[1].set_yticks(range(len(tub))); axs[1].set_yticklabels([f"{t:g}" for t in tub]); axs[1].set_xlabel("shrink")
axs[1].set_title("depth-2 clearance (tubes of unrelated networks)", fontsize=9)
fig.suptitle("Trionda sibling clasp at depth 2: thin tubes (lateral 0.79, lean 326, turn 7.6, sink 0.003)", fontsize=10)
fig.tight_layout(); fig.savefig(OUT / "phase_tube.png", dpi=130); print("wrote", OUT / "phase_tube.png")
ok = sorted((c for c in P["cells"] if c["fused2"] == 0 and c.get("linked") == c.get("pairs")), key=lambda c: -c["clear2"])
print("best depth-2 clasp cells:", [(round(c["shrink"], 3), c["tube"], round(c["clear2"], 4)) for c in ok[:5]])
