"""Binary Hat Trick tree on adjacent pentagons with a sibling clasp, and the signed chi4 figure."""
from _common import *
import json

def view(tree, az=35, el=25, dist=6.0, **kw):
    return dict(camera=fk.Camera(azimuth=az, elevation=el, distance=dist), step=0.7, max_steps=300,
                palette=tree.cmap(), trap_weight=0.85, **kw)

cl = lambda **k: fk.PairHatTrick.clasp(body=0.97, **k)
a = [("pair clasp, depth 1", cl(iters=1)), ("depth 2", cl(iters=2)), ("depth 3", cl(iters=3)),
     ("depth 3, other side", cl(iters=3)), ("depth 3, top", cl(iters=3)), ("depth 2, seams only", fk.PairHatTrick.clasp(iters=2))]
ev = {3: dict(az=215, el=-20), 4: dict(az=0, el=85)}
items = [(n, t, view(t, dist=5.5, **ev.get(i, {}))) for i, (n, t) in enumerate(a)]
items[5] = (items[5][0], items[5][1], {**items[5][2], "palette": "viridis", "trap_weight": 0.3})
grid(items, name="pair_hattrick_gallery", size=(220, 165))

p = OUT / "chi4_signed.json"
if p.exists():
    r = json.loads(p.read_text())
    D = np.array([x["D"] for x in r]); c = np.abs([x["chi4"] for x in r]); f = np.array([x["flipped"] for x in r])
    fig, ax = plt.subplots(figsize=(5.6, 3.6)); jit = np.random.default_rng(0).uniform(-0.18, 0.18, len(f))
    for sel, col, lab in ((c == 0, "tab:blue", "chi4 = 0"), (c > 0, "tab:orange", "chi4 != 0")):
        ax.scatter(f[sel] + jit[sel], D[sel], s=12, c=col, label=lab, alpha=.7)
    ax.set_xlabel("rings flipped"); ax.set_ylabel("D: network of b vs mirror of network of s.b (deg)")
    ax.set_title("signed-style chi4 test, %d patterns" % len(r), fontsize=9); ax.legend(fontsize=7)
    fig.tight_layout(); fig.savefig(OUT / "chi4_signed.png", dpi=130); plt.close(fig)
