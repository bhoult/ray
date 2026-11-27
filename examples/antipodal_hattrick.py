"""The Hat Trick as a binary horned tree (children on an antipodal pair of pentagons), and the chi4 comparison figure."""
from _common import *
import json

def view(tree, az=35, el=25, dist=6.0, **kw):
    return dict(camera=fk.Camera(azimuth=az, elevation=el, distance=dist), step=0.7, max_steps=250,
                palette=tree.cmap(), trap_weight=0.85, **kw)

ap = lambda **k: fk.AntipodalHatTrick(body=0.97, **k)
a = [("binary Hat Trick, depth 1", ap(iters=1)), ("depth 2", ap(iters=2)), ("depth 3", ap(iters=3)),
     ("depth 3, other side", ap(iters=3)), ("depth 3, shrink 0.6", ap(iters=3, shrink=0.6)), ("depth 3, turn 36", ap(iters=3, turn=36))]
ev = {3: dict(az=215, el=-20)}
items = [(n, t, view(t, dist=5.5, **ev.get(i, {}))) for i, (n, t) in enumerate(a)]
grid(items, name="antipodal_hattrick_gallery", size=(220, 165))

# chi4 comparison (needs out/chi4_comparison.json from tools/chi4_comparison.py)
p = OUT / "chi4_comparison.json"
if p.exists():
    r = json.loads(p.read_text())
    m = np.array([x["mirror_distance"] for x in r]); c = np.abs([x["chi4"] for x in r]); f = np.array([x["flipped"] for x in r])
    fig, ax = plt.subplots(figsize=(5.6, 3.6))
    jit = np.random.default_rng(0).uniform(-0.18, 0.18, len(f))
    for sel, col, lab in ((c[1:] == 0, "tab:blue", "chi4 = 0"), (c[1:] > 0, "tab:orange", "chi4 != 0")):
        ax.scatter(f[1:][sel] + jit[1:][sel], m[1:][sel], s=14, c=col, label=lab, alpha=.8)
    ax.scatter([0], [m[0]], marker="*", s=90, c="k", label="standard tiling")
    ax.set_xlabel("rings flipped"); ax.set_ylabel("mirror distance of the seam network (deg)")
    ax.set_title("seam-network chirality of %d ring patterns" % (len(r) - 1), fontsize=9); ax.legend(fontsize=7)
    fig.tight_layout(); fig.savefig(OUT / "chi4_comparison.png", dpi=130); plt.close(fig)
