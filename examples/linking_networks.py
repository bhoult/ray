"""Hopf fibres and the horned sphere's ring tree as circle networks: shared
structure (both are families of circles -> FiberNetwork) and their linking graphs."""
from _common import *
from matplotlib.colors import ListedColormap

DEPTH = 3
h = fk.HornedSphere()
L_h, touch_h, rings = h.linking(DEPTH, 0.02)
hf = fk.HopfFibers(fk.ring_base([0.0], 12), radius=0.05, max_circle=8.0)
L_f, touch_f = fk.linking_matrix(hf.circles(0.03))


def matrix(ax, L, touch, title, labels=None):
    n = len(L)
    M = np.where(touch, np.nan, np.round(L))
    cmap = plt.get_cmap("RdBu_r").copy(); cmap.set_bad("0.82")
    ax.imshow(M, cmap=cmap, vmin=-1.5, vmax=1.5, interpolation="nearest")
    for i in range(n):
        for j in range(n):
            if i != j and not touch[i, j] and abs(M[i, j]) > 0:
                ax.text(j, i, f"{int(M[i, j]):+d}", ha="center", va="center", fontsize=6)
    ax.set_title(title, fontsize=9); ax.set_xticks([]); ax.set_yticks([])
    if labels is not None:
        ax.set_xlabel(labels, fontsize=8)


fig, axs = plt.subplots(2, 2, figsize=(11, 10))
matrix(axs[0, 0], L_f, touch_f, "Hopf fibres (12 on the Clifford ring): every pair links once",
       "all off-diagonal entries +1; no pair touches")
gens = np.array([r.gen for r in rings])
matrix(axs[0, 1], L_h, touch_h, f"horned sphere ring tree ({len(rings)} rings, depth {DEPTH})",
       "grey = touching (parent-child, linking undefined); colour = linked; white = 0")
for g in range(1, DEPTH + 1):                       # generation boundaries
    k = np.searchsorted(gens, g) - 0.5
    axs[0, 1].axhline(k, color="k", lw=0.4); axs[0, 1].axvline(k, color="k", lw=0.4)

# graphs
ax = axs[1, 0]
n = len(L_f); ang = np.linspace(0, 2 * np.pi, n, endpoint=False); xy = np.stack([np.cos(ang), np.sin(ang)], 1)
for i in range(n):
    for j in range(i + 1, n):
        ax.plot(*xy[[i, j]].T, color="tab:red", lw=0.6, alpha=0.6)
ax.scatter(*xy.T, s=60, c="k", zorder=3); ax.set_aspect("equal"); ax.axis("off")
ax.set_title(f"Hopf linking graph: complete graph K{n} (all {n*(n-1)//2} pairs linked)", fontsize=9)

ax = axs[1, 1]
pos = {}
for g in range(DEPTH + 1):
    idx = [i for i, r in enumerate(rings) if r.gen == g]
    for k, i in enumerate(idx):
        pos[i] = ((k + 0.5) / len(idx) - 0.5, -g)
for i, r in enumerate(rings):
    if r.parent >= 0:
        ax.plot(*zip(pos[i], pos[r.parent]), color="0.6", lw=1.5, zorder=1)             # touching
    if r.sibling > i:
        xs, ys = zip(pos[i], pos[r.sibling]); ax.plot(xs, ys, color="tab:red", lw=2.2, zorder=2)   # linked
for i, r in enumerate(rings):
    ax.scatter(*pos[i], s=140 - 30 * r.gen, c=[plt.cm.plasma(r.gen / 4)], zorder=3, edgecolors="k")
ax.axis("off")
ax.set_title("horned linking graph: grey = touching tree, red = linked siblings\n(a perfect matching in each generation; nothing else links)", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "linking_graphs.png", dpi=130); print("wrote", OUT / "linking_graphs.png")

# ---- the two families as one kind of object: FiberNetworks ------------------------
fig = plt.figure(figsize=(12, 4.8))
a = fig.add_subplot(1, 3, 1, projection="3d")
for c, col in zip(hf.circles(0.03), plt.cm.viridis(np.linspace(0, 1, 12))):
    a.plot(*np.vstack([c, c[:1]]).T, color=col, lw=1.2)
a.set_title("Hopf fibres: circles on one torus", fontsize=9); a.set_axis_off(); a.set_box_aspect((1, 1, 1))
b = fig.add_subplot(1, 3, 2, projection="3d")
for r in h.ring_tree(DEPTH, 0.02, closed=False):
    b.plot(*r.curve.T, color=plt.cm.plasma(r.gen / 4), lw=2.4 - 0.4 * r.gen)
b.set_title("horned sphere: arcs by generation", fontsize=9); b.set_axis_off(); b.set_box_aspect((1, 1, 1))
b.view_init(55, -90)
ax = fig.add_subplot(1, 3, 3)
net_f = fk.FiberNetwork.from_curves(hf.circles(0.04), 0.05)
net_h = h.skeleton(DEPTH, 0.04)
ax.imshow(fk.render(net_h, size=(300, 225), camera=fk.Camera(azimuth=-90, elevation=62, distance=5.2),
                    max_steps=250, palette="plasma", trap_weight=1.4)); ax.axis("off")
ax.set_title(f"the same arcs as a FiberNetwork ({len(net_h.seg)} capsules, tapered)", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "circle_families.png", dpi=130); print("wrote", OUT / "circle_families.png")

# ---- one joint network: horned tree threaded by Hopf fibres -------------------------
scale, shift = 0.55, np.array([0.0, 0.5, 0.0])
rt = h.ring_tree(2, 0.02, closed=True, world=True)
hc = [shift + scale * r.curve for r in rt]
fib = hf.circles(0.03)[::2]                          # 6 fibres
curves = hc + [c * 1.0 for c in fib]
L_j, touch_j = fk.linking_matrix(curves)
nh = len(hc)
fig, ax = plt.subplots(figsize=(6.4, 5.6))
cmap = plt.get_cmap("RdBu_r").copy(); cmap.set_bad("0.82")
ax.imshow(np.where(touch_j, np.nan, np.round(L_j)), cmap=cmap, vmin=-1.5, vmax=1.5, interpolation="nearest")
ax.axhline(nh - 0.5, color="k"); ax.axvline(nh - 0.5, color="k")
ax.set_xticks([nh / 2, nh + len(fib) / 2]); ax.set_xticklabels(["horned rings", "Hopf fibres"])
ax.set_yticks([nh / 2, nh + len(fib) / 2]); ax.set_yticklabels(["horned rings", "Hopf fibres"], rotation=90, va="center")
cross = L_j[:nh, nh:]; ct = touch_j[:nh, nh:]
ax.set_title(f"joint network: cross-family block has {int((np.abs(np.nan_to_num(np.round(cross))) > 0).sum())} linked pairs, {int(ct.sum())} touching\n(depends on how the tree is placed among the fibres)", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "joint_network.png", dpi=130); print("wrote", OUT / "joint_network.png")
print("cross-family linking values:", sorted(set(np.round(np.nan_to_num(cross, nan=9)).astype(int).ravel())), " (9 = touching)")
