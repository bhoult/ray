"""The clasp of a horned-style tree as a Moebius object: when is it a Hopf pair?

The two children of a ring are circles in perpendicular planes whose centre line lies in
both planes, at distance d = 2 s (kappa - 1) with radius s.  Their Moebius invariants are
(|1 - 2 (kappa - 1)^2|, 0): a Hopf pair exactly at kappa = 1 + 1/sqrt 2, the oloid pair at
kappa = 3/2.  The script also counts the linked pairs of the depth-3 tree against kappa."""
from _common import *

S, DEPTH, TURN = 0.4, 3, 45.0
K_HOPF, K_OLOID = 1 + 1 / np.sqrt(2), 1.5


def clasp_invariants(kappa, turn=TURN):
    h = fk.HornedSphere(shrink=S, clasp=kappa, turn=turn, iters=2)
    cs = []
    for r in h.ring_tree(1, 0.03, closed=True, world=True):
        if r.gen == 1:
            k = len(r.curve)
            cs.append(fk.geometry.circle_through(r.curve[0], r.curve[k // 3], r.curve[2 * k // 3]))
    return fk.circle_moebius_invariants(*cs)


def link_counts(kappa):
    h = fk.HornedSphere(shrink=S, clasp=kappa, turn=TURN, iters=3)
    L, touch, rings = h.linking(DEPTH, 0.02)
    n = len(rings)
    pc = {(i, r.parent) for i, r in enumerate(rings) if r.parent >= 0}
    links = sib = extra_touch = 0
    for i in range(n):
        for j in range(i + 1, n):
            if touch[i, j]:
                extra_touch += (i, j) not in pc and (j, i) not in pc
            elif abs(L[i, j]) > 0.5:
                links += 1
                sib += rings[i].parent == rings[j].parent
    return links, sib, extra_touch


kaps = np.array([1.1, 1.2, 1.35, 1.5, 1.6, 1.65, 1.7, K_HOPF, 1.75, 1.8, 1.9, 2.0, 2.2])
rows = [(k, *clasp_invariants(k), *link_counts(k)) for k in kaps]
print("kappa   s1        s2        links  sibling  extra-touching")
for k, s1, s2, nl, ns, ne in rows:
    print(f"{k:.4f}  {s1:.6f}  {s2:.1e}  {nl:5d}  {ns:7d}  {ne:6d}")

kk = np.linspace(1.0, 2.3, 400)
fig, (a, b) = plt.subplots(1, 2, figsize=(11, 3.8))
a.plot(kk, np.abs(1 - 2 * (kk - 1) ** 2), "k-", lw=1.6, label=r"$s_1=|1-2(\kappa-1)^2|$ (closed form)")
a.plot([r[0] for r in rows], [r[1] for r in rows], "o", color="tab:orange", ms=5, label="computed from the rings")
a.axvline(K_OLOID, color="tab:green", ls="--", lw=1); a.axvline(K_HOPF, color="tab:red", ls="--", lw=1)
a.text(K_OLOID - 0.02, 1.55, "oloid\n$s_1=1/2$", color="tab:green", ha="right", fontsize=8)
a.text(K_HOPF + 0.02, 1.55, "Hopf pair\n$s_1=0$", color="tab:red", fontsize=8)
a.set_xlabel(r"clasp $\kappa$"); a.set_ylabel("Moebius invariant of the clasp"); a.legend(fontsize=8, loc="lower left")
a.set_title("first Moebius invariant of the clasped children ($s_2=0$ throughout)", fontsize=9)
ks = [r[0] for r in rows]
b.bar(range(len(ks)), [r[3] for r in rows], color="tab:blue", label="all linked pairs")
b.bar(range(len(ks)), [r[4] for r in rows], color="tab:gray", label="of which siblings")
for i, r in enumerate(rows):
    if r[5]:
        b.text(i, r[3] + 0.8, "*", ha="center", color="tab:red")
b.set_xticks(range(len(ks))); b.set_xticklabels([f"{k:.2f}" for k in ks], rotation=60, fontsize=7)
b.set_xlabel(r"clasp $\kappa$"); b.set_ylabel("linked pairs (depth 3)"); b.legend(fontsize=8)
b.set_title("* = extra touching pairs besides parent-child", fontsize=9)
fig.tight_layout()
p = OUT / "hopf_clasp.png"
fig.savefig(p, dpi=140)
print("wrote", p)
