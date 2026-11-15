"""A horned-style tree whose clasped pairs are Hopf pairs."""
from _common import *
from fractalkit.geometry import hopf_fiber_points, stereographic, circle_through

SHRINK = 0.4
top = dict(camera=fk.Camera(azimuth=-90, elevation=62, distance=5.2), step=0.9, max_steps=300,
           palette="plasma", trap_weight=1.4)

# ---- 1. the pair, and where it goes ----------------------------------------------
t0 = fk.HopfTree(shrink=SHRINK, psi=np.radians(110))
ca, na, cb, nb, r = fk.hopf_pair()
fig = plt.figure(figsize=(11.5, 5))
a = fig.add_subplot(1, 2, 1, projection="3d")
ang = np.linspace(0, 2 * np.pi, 200)
for phi, col, lab in ((0.0, "tab:red", "fibre over base point A"), (np.pi, "tab:blue", "fibre over -A")):
    pts_ = stereographic(hopf_fiber_points(np.pi / 4, phi, ang)) / r          # normalised to radius 1
    a.plot(*pts_.T, color=col, lw=2, label=lab)
a.plot([0, 0], [0, 0], [-1.6, 1.6], "k--", lw=0.8, label="half-turn axis")
pa = t0.pair_back; pb = np.diag([-1, -1, 1]) @ pa
a.scatter(*np.array([pa, pb]).T, color="k", s=40, label="chosen back points", zorder=5)
a.set_title("a Hopf pair on the Clifford torus (unit circles)", fontsize=9)
a.legend(fontsize=7, loc="upper left"); a.set_box_aspect((1, 1, 1)); a.view_init(55, 30); a.set_axis_off()
b = fig.add_subplot(1, 2, 2, projection="3d")
phi = np.linspace(0, 2 * np.pi, 400)
ring = np.stack([np.cos(phi), np.sin(phi), 0 * phi], -1)
arc = phi > t0.g; arc &= phi < 2 * np.pi - t0.g
b.plot(*ring[arc].T, color="0.4", lw=3, label="parent ring (cut open)")
for k, col in ((0, "tab:red"), (1, "tab:blue")):
    c = t0.child_circle(k, 300)
    b.plot(*np.vstack([c, c[:1]]).T, color=col, lw=2)
tips = np.array(t0.back_points())
b.scatter(*tips.T, color="k", s=40, zorder=5, label="cut tips = back points")
b.set_title(f"carried onto the parent's tips (shrink {SHRINK}, rho {t0.rho:.2f}, g {np.degrees(t0.g):.0f} deg)", fontsize=9)
b.legend(fontsize=7, loc="upper left"); b.set_box_aspect((1, 1, 0.5)); b.view_init(35, 20); b.set_axis_off()
fig.tight_layout(); fig.savefig(OUT / "hopf_tree_pair.png", dpi=130); print("wrote", OUT / "hopf_tree_pair.png")

# ---- 2. gallery ---------------------------------------------------------------------
items = [("horned sphere (reference)", fk.HornedSphere(iters=7, tube=0.2), top)]
for psid, hang in ((60, 1), (110, 1), (215, 1), (215, -1)):
    items.append((f"Hopf tree  psi={psid}  hang={hang:+d}", fk.HopfTree(shrink=SHRINK, psi=np.radians(psid), hang=hang, tube=0.2, iters=7), top))
items.append(("Hopf tree psi=220  hang=+1", fk.HopfTree(shrink=SHRINK, psi=np.radians(220), tube=0.2, iters=7), top))
grid(items, name="hopf_tree_gallery", size=(260, 195))

# ---- 3. phase diagram over psi ------------------------------------------------------
from scipy.spatial.distance import cdist


def tube_max(tree, depth=3, spacing=0.004):
    rings = tree.ring_tree(depth, spacing, closed=False, world=False)
    best = np.inf
    for i in range(len(rings)):
        for j in range(i + 1, len(rings)):
            if rings[i].parent == j or rings[j].parent == i:
                continue
            best = min(best, cdist(rings[i].curve, rings[j].curve).min() / (rings[i].scale + rings[j].scale))
    return best


psis = np.arange(0, 361, 10)
rho, nlink, nsib, tmax = [], [], [], []
for p in psis:
    t = fk.HopfTree(shrink=SHRINK, psi=np.radians(p))
    L, touch, rings = t.linking(3, 0.02)
    n = len(rings); off = ~np.eye(n, dtype=bool) & ~touch
    linked = [(i, j) for i in range(n) for j in range(i + 1, n) if off[i, j] and round(L[i, j]) != 0]
    rho.append(t.rho); nlink.append(len(linked)); nsib.append(sum(rings[i].sibling == j for i, j in linked))
    tmax.append(tube_max(t))
    print(f"psi {p:3d}  rho {t.rho:.3f}  linked {len(linked):2d} (siblings {nsib[-1]})  tube_max {tmax[-1]:.4f}", flush=True)
fig, axs = plt.subplots(1, 2, figsize=(11, 4))
axs[0].plot(psis, nlink, "o-", label="linked pairs (all)"); axs[0].plot(psis, nsib, "s--", label="of which siblings")
axs[0].set_xlabel("psi (degrees)"); axs[0].set_ylabel("linked ring pairs, depth 3 (15 rings)"); axs[0].legend()
axs[0].set_title("linking graph size vs where the back point sits (hang=+1)", fontsize=9)
axs[1].semilogy(psis, tmax, "o-"); axs[1].axhline(0.2, color="r", ls=":", label="tube = 0.2 (gallery)")
axs[1].set_xlabel("psi (degrees)"); axs[1].set_ylabel("largest tube without overlap"); axs[1].legend()
axs[1].set_title("thick tubes (>= 0.2) for psi up to ~100 and from ~215 (hang=+1)", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "hopf_tree_phase.png", dpi=130); print("wrote", OUT / "hopf_tree_phase.png")

# ---- 4. linking structure: horned vs Hopf tree in the window vs rich regimes ----------
cases = [("horned sphere", fk.HornedSphere()),
         ("Hopf tree psi=110 (sibling links only)", fk.HopfTree(shrink=SHRINK, psi=np.radians(110))),
         ("Hopf tree psi=215 (thick, embedded)", fk.HopfTree(shrink=SHRINK, psi=np.radians(215))),
         ("Hopf tree psi=180 (needs thin tubes)", fk.HopfTree(shrink=SHRINK, psi=np.pi))]
fig, axs = plt.subplots(1, 4, figsize=(17, 4.9))
for ax, (name, tree) in zip(axs, cases):
    L, touch, rings = tree.linking(3, 0.02)
    nl = sum(1 for i in range(15) for j in range(i + 1, 15) if not touch[i, j] and round(L[i, j]) != 0)
    linking_plot(ax, L, touch, f"{name}\n{nl} linked pairs", "grey = touching parent-child")
    for g in (1, 3, 7):
        ax.axhline(g - 0.5, color="k", lw=0.4); ax.axvline(g - 0.5, color="k", lw=0.4)
fig.tight_layout(rect=(0, 0, 1, 0.94)); fig.savefig(OUT / "hopf_tree_linking.png", dpi=130); print("wrote", OUT / "hopf_tree_linking.png")

# ---- 4b. the thick rich tree: which pairs link ------------------------------------------
rich = fk.HopfTree(shrink=SHRINK, psi=np.radians(215))
L, touch, rings = rich.linking(3, 0.02)
fig, ax = plt.subplots(figsize=(7, 5.2))
pos = {}
for g in range(4):
    idx = [i for i, r in enumerate(rings) if r.gen == g]
    for k, i in enumerate(idx):
        pos[i] = ((k + 0.5) / len(idx) - 0.5, -g)
for i, r in enumerate(rings):
    if r.parent >= 0:
        ax.plot(*zip(pos[i], pos[r.parent]), color="0.75", lw=1.2, zorder=1)
for i in range(15):
    for j in range(i + 1, 15):
        if not touch[i, j] and round(L[i, j]) != 0:
            sib = rings[i].sibling == j
            ax.annotate("", xy=pos[j], xytext=pos[i], zorder=2,
                        arrowprops=dict(arrowstyle="-", color="tab:red" if L[i, j] > 0 else "tab:blue", lw=2.2 if sib else 1.1,
                                        connectionstyle=f"arc3,rad={0.0 if sib else 0.28}"))
for i, r in enumerate(rings):
    ax.scatter(*pos[i], s=150 - 30 * r.gen, c=[plt.cm.plasma(r.gen / 4)], zorder=3, edgecolors="k")
ax.axis("off")
ax.set_title("psi=215: red = linking +1, blue = -1, grey = touching tree\nthick = siblings; curved = links between non-siblings", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "hopf_tree_graph.png", dpi=130); print("wrote", OUT / "hopf_tree_graph.png")

# ---- 5. the rich regime as a wire network ------------------------------------------------
wire = fk.HopfTree(shrink=SHRINK, psi=np.pi, tube=0.04)
net = wire.skeleton(3, 0.03)
L, touch, rings = wire.linking(3, 0.02)
fig = plt.figure(figsize=(12, 5))
a = fig.add_subplot(1, 2, 1, projection="3d")
for r_ in wire.ring_tree(3, 0.02, closed=False):
    a.plot(*r_.curve.T, color=plt.cm.plasma(r_.gen / 4), lw=2.2 - 0.4 * r_.gen)
a.set_box_aspect((1, 1, 0.6)); a.set_axis_off(); a.view_init(40, -60)
a.set_title("Hopf tree, psi=180: arcs by generation", fontsize=9)
b = fig.add_subplot(1, 2, 2)
b.imshow(fk.render(net, size=(320, 240), camera=fk.Camera(azimuth=-90, elevation=62, distance=5.0),
                   max_steps=250, palette="plasma", trap_weight=1.4)); b.axis("off")
b.set_title(f"as a wire network ({len(net.seg)} capsules, tube 0.04)", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "hopf_tree_wire.png", dpi=130); print("wrote", OUT / "hopf_tree_wire.png")
