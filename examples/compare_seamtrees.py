"""Compare the seam trees: Trionda (T), Trionda sibling-clasp, Hat Trick (I), and the ring horned sphere as reference.

Writes out/seamtree_compare.png, .md and .json.   python compare_seamtrees.py [--quick]
"""
from _common import *
import json, sys, time
import matplotlib.ticker
import fractalkit.seamstats as S
from fractalkit.geometry import linking_matrix

QUICK = "--quick" in sys.argv
t0 = time.time()
log = lambda *a: print(f"[{time.time()-t0:5.0f}s]", *a, flush=True)

TR, CL, HT = fk.TriondaHorned(iters=4), fk.TriondaHorned.clasp(iters=4), fk.HatTrickHorned(iters=3)
HS = fk.HornedSphere()
trees = {"Trionda": (TR, 3), "Trionda clasp": (CL, 3), "Hat Trick": (HT, 2)}      # (tree, depth used for the geometry below)
COL = {"Trionda": "tab:blue", "Trionda clasp": "tab:cyan", "Hat Trick": "tab:red", "Horned ring": "tab:green"}
res = {}

# ---- 1. the ball itself (root network) -------------------------------------------------------------------------
for name in ("Trionda", "Hat Trick"):
    t = trees[name][0]
    pts = S.seam_points(t); A = S.panel_areas(t); P = S.panel_perimeters(t)
    hat = t.classes > 0 if name == "Hat Trick" else np.zeros(len(A), bool)
    spec = S.harmonic_spectrum(pts, 24)
    forbidden = [l for l in range(1, 25) if (l in (1, 2, 5) if name == "Trionda" else l not in (6, 10, 12, 15, 16, 18, 20, 21, 22, 24))]
    r = dict(panels=len(A), group_order=12 if name == "Trionda" else 60, sites=len(t.sites),
             seam_length=S.seam_length(t.curves) if name == "Trionda" else S.seam_length(t.curves) / 2,   # hat outlines list every seam twice
             area_total_over_4pi=float(A.sum() / (4 * np.pi)),
             area_cv=float(A.std() / A.mean()),
             area_cv_hats=float(A[hat].std() / A[hat].mean()) if hat.any() else None,
             hat_to_pent_area=float(A[hat].mean() / A[~hat].mean()) if hat.any() else None,
             compactness=float(np.mean(4 * np.pi * A / P ** 2)),           # 1 = circle (small-panel limit)
             chirality_deg=S.chirality(pts, starts=6 if QUICK else 30),
             leakage_forbidden=float(spec[forbidden].sum() / spec[1:].sum()),
             spectrum=(spec / spec[1:].sum()).tolist())
    res[name] = r
    log(name, "ball done")
res["Trionda"]["seam_length_per_area"] = res["Trionda"]["seam_length"] / (4 * np.pi)
res["Hat Trick"]["seam_length_per_area"] = res["Hat Trick"]["seam_length"] / (4 * np.pi)
# the clasp variant shares the Trionda ball
res["Trionda clasp"] = {k: v for k, v in res["Trionda"].items()}

# ---- 2. the trees -----------------------------------------------------------------------------------------------
for name, (t, depth) in trees.items():
    n, L = S.generation_counts(t, depth)
    nonroot = sum(1 for k in t.map_site if k != 0)
    sim = S.similarity_dimension(max(nonroot, 1), t.s)
    pts = S.tree_points(t, depth, spacing=0.4 * t.s ** depth * 0.25)
    dim, h, N = S.box_dimension(pts, 0.25 * t.s ** depth, 0.5)
    clr, fused, pc = S.clearance(t, min(depth, 2))
    r = res[name]
    r.update(children_root=len(t.maps), children_later=nonroot, shrink=t.s, networks=n.tolist(), seam_len_gen=L.tolist(),
             similarity_dim=sim, box_dim=dim, box_h=h.tolist(), box_N=N.tolist(), clearance=clr, fused_pairs=fused, parent_clearance=pc)
    log(name, "tree done, box dim %.2f" % dim)

# ---- 3. volume of the solid (body ball + tubes) by depth ------------------------------------------------------------
nvol = 40_000 if QUICK else 150_000
for name, mk in (("Trionda", lambda k: fk.TriondaHorned(iters=k, body=0.95)),
                 ("Trionda clasp", lambda k: fk.TriondaHorned.clasp(iters=k, body=0.95 * 1)),
                 ("Hat Trick", lambda k: fk.HatTrickHorned(iters=k, body=0.97))):
    vols = [S.volume(mk(k), nvol) for k in (1, 2, 3)]
    res[name]["volume"] = [v for v, _ in vols]; res[name]["volume_se"] = [e for _, e in vols]
    log(name, "volume", [round(v, 2) for v, _ in vols])

# ---- 4. linking of parent and child loops -------------------------------------------------------------------------------
def pc_linking(t, pick):
    loops, who = t.panel_loops(1)
    root = [i for i, w in enumerate(who) if w[0] == 0 and pick(w[1])]
    kid = [i for i, w in enumerate(who) if w[0] == 1 and pick(w[1])]
    use = root + kid
    L, touch = linking_matrix([loops[i] for i in use], max_densify=5)
    cnt = sum(1 for a in range(len(root)) for b in range(len(root), len(use)) if not touch[a, b] and abs(L[a, b]) > 0.5)
    return cnt, len(root) * len(kid)
res["Trionda"]["pc_linked_loops"] = pc_linking(TR, lambda k: True)
res["Trionda clasp"]["pc_linked_loops"] = pc_linking(CL, lambda k: True)
res["Hat Trick"]["pc_linked_loops"] = pc_linking(HT, lambda k: HT.classes[k] == 0)           # the 12 pentagon loops only
L, touch, rings = HS.linking(2, 0.02)
n = len(rings); sib = [(i, j) for i in range(n) for j in range(i + 1, n) if rings[i].sibling == j]
res["Horned ring"] = dict(shrink=HS.s, children_root=2, children_later=2, networks=[1, 2, 4], similarity_dim=S.similarity_dimension(2, HS.s),
                          sibling_linked=int(sum(abs(L[i, j]) > 0.5 for i, j in sib)), sibling_pairs=len(sib))
log("linking done")

# ---- figure ------------------------------------------------------------------------------------------------------------------
fig, axs = plt.subplots(2, 3, figsize=(15, 8.2))
ax = axs[0, 0]
w = 0.4
for k, nm in enumerate(("Trionda", "Hat Trick")):
    ax.bar(np.arange(1, 25) + (k - 0.5) * w, np.maximum(res[nm]["spectrum"][1:], 1e-6), w, color=COL[nm], label=nm)
ax.set_yscale("log"); ax.set_xlabel("degree l"); ax.set_ylabel("share of seam power (l >= 1)"); ax.legend(fontsize=8)
ax.set_title("seam spectrum: T forbids l = 1, 2, 5; I forbids l < 6 and most of the rest", fontsize=9)
ax = axs[0, 1]
for k, nm in enumerate(("Trionda", "Hat Trick")):
    t = trees[nm][0]; A = S.panel_areas(t)
    if nm == "Hat Trick":
        for kind, lab, off in ((t.classes > 0, "hats (60)", -0.15), (t.classes == 0, "pentagons (12)", 0.15)):
            ax.scatter(np.full(kind.sum(), 1 + off) + np.random.default_rng(0).uniform(-.05, .05, kind.sum()), A[kind] / (4 * np.pi) * len(A), s=14, color=COL[nm], alpha=.6, label=lab)
    else:
        ax.scatter(np.full(len(A), 0.0), A / (4 * np.pi) * len(A), s=30, color=COL[nm], label="4 panels")
ax.set_xticks([0, .85, 1.15]); ax.set_xticklabels(["Trionda", "hats", "pentagons"]); ax.set_ylabel("panel area / mean panel area of an equal split")
ax.set_title("panel areas: Trionda equal; Hat Trick hats equal, 2.3x the pentagons", fontsize=9); ax.legend(fontsize=7)
ax = axs[0, 2]
x = np.arange(4); wd = 0.2
for k, nm in enumerate(("Horned ring", "Trionda", "Trionda clasp", "Hat Trick")):
    n_ = res[nm]["networks"] + [np.nan] * (4 - len(res[nm]["networks"]))
    ax.bar(x + (k - 1.5) * wd, n_, wd, color=COL[nm], label=nm)
ax.set_yscale("log"); ax.set_xticks(x); ax.set_xlabel("generation"); ax.set_ylabel("networks (rings for the horned sphere)"); ax.legend(fontsize=8)
ax.set_title("how fast each tree branches", fontsize=9)
ax = axs[1, 0]
for nm in ("Trionda", "Trionda clasp", "Hat Trick"):
    h, N = np.array(res[nm]["box_h"]), np.array(res[nm]["box_N"])
    ax.loglog(1 / h, N, "o-", ms=3, color=COL[nm], label=f"{nm}: box dim {res[nm]['box_dim']:.2f} (similarity {res[nm]['similarity_dim']:.2f})")
ax.set_xlabel("1 / box size"); ax.set_ylabel("occupied boxes"); ax.xaxis.set_minor_formatter(matplotlib.ticker.NullFormatter()); ax.legend(fontsize=7); ax.set_title("box counting of the seams (depth 3 / 3 / 2)", fontsize=9)
ax = axs[1, 1]
for nm in ("Trionda", "Trionda clasp", "Hat Trick"):
    ax.errorbar([1, 2, 3], res[nm]["volume"], res[nm]["volume_se"], marker="o", color=COL[nm], label=nm, capsize=3)
ax.set_xlabel("generations"); ax.set_ylabel("volume of the solid"); ax.legend(fontsize=8); ax.set_xticks([1, 2, 3])
ax.set_title("volume of ball + tubes (Monte Carlo), by depth", fontsize=9)
ax = axs[1, 2]
names = ["Trionda", "Trionda clasp", "Hat Trick"]
ax.bar(np.arange(3) - 0.2, [res[n_]["clearance"] for n_ in names], 0.4, color=[COL[n_] for n_ in names], label="tubes of unrelated networks")
ax.bar(np.arange(3) + 0.2, [res[n_]["parent_clearance"] for n_ in names], 0.4, color=[COL[n_] for n_ in names], alpha=.45, label="parent-child")
ax.axhline(0, color="k", lw=.8); ax.set_xticks(range(3)); ax.set_xticklabels(names); ax.set_ylabel("tube clearance (< 0: fused)")
ax.set_title("clearance to depth 2 (tube 0.07 / 0.05 / 0.025)", fontsize=9); ax.legend(fontsize=7)
fig.tight_layout(); fig.savefig(OUT / "seamtree_compare.png", dpi=130)

# ---- table --------------------------------------------------------------------------------------------------------------------------
def f(v, p=3): return "-" if v is None else (f"{v:.{p}f}" if isinstance(v, float) else str(v))
rows = [
    ("rotation group order", lambda r: r.get("group_order")), ("panels", lambda r: r.get("panels")),
    ("seam length (unit sphere)", lambda r: r.get("seam_length")), ("seam length / area", lambda r: r.get("seam_length_per_area")),
    ("panel area CV", lambda r: r.get("area_cv")), ("hat : pentagon area", lambda r: r.get("hat_to_pent_area")),
    ("panel compactness (1 = disc)", lambda r: r.get("compactness")), ("chirality (deg to mirror image)", lambda r: r.get("chirality_deg")),
    ("seam power in forbidden degrees", lambda r: r.get("leakage_forbidden")),
    ("shrink", lambda r: r.get("shrink")), ("children: root / later", lambda r: f"{r['children_root']} / {r['children_later']}" if "children_root" in r else None),
    ("networks per generation", lambda r: r.get("networks")), ("similarity dimension", lambda r: r.get("similarity_dim")),
    ("box-counting dimension of the seams", lambda r: r.get("box_dim")),
    ("tube clearance, unrelated / parent-child", lambda r: f"{r['clearance']:+.3f} / {r['parent_clearance']:+.3f}" if "clearance" in r else None),
    ("fused pairs (depth 2)", lambda r: r.get("fused_pairs")),
    ("volume at depth 1 / 2 / 3", lambda r: " / ".join(f"{v:.2f}" for v in r["volume"]) if "volume" in r else None),
    ("parent-child loop pairs linked", lambda r: "%d of %d" % tuple(r["pc_linked_loops"]) if "pc_linked_loops" in r else None),
    ("sibling pairs linked", lambda r: "%d of %d" % (r["sibling_linked"], r["sibling_pairs"]) if "sibling_linked" in r else None),
]
cols = ["Horned ring", "Trionda", "Trionda clasp", "Hat Trick"]
md = ["| | " + " | ".join(cols) + " |", "|---|" + "---|" * len(cols)]
for lab, fn in rows:
    cells = []
    for c in cols:
        try: v = fn(res.get(c, {}))
        except (KeyError, TypeError): v = None
        cells.append(f(v) if not isinstance(v, (list, np.ndarray)) else ", ".join(map(str, v)))
    md.append(f"| {lab} | " + " | ".join(cells) + " |")
(OUT / "seamtree_compare.md").write_text("\n".join(md) + "\n")
(OUT / "seamtree_compare.json").write_text(json.dumps(res, indent=1, default=float))
print("\n".join(md)); log("wrote", OUT / "seamtree_compare.png")
