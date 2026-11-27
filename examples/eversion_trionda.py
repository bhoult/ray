"""Evert a Trionda ball: the four panels and six seams are paint on the sphere and ride the
Bednorz-Bednorz eversion.  Outside paint = saturated panel colours, inside paint = the same
panels in a dark, cool variant, so the two sides are always told apart.

    python examples/eversion_trionda.py            # strip of 9 frames -> out/eversion_trionda.png
"""
import sys, os, colorsys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection
from fractalkit import eversion as E
from fractalkit.trionda_horned import TriondaHorned

OUT = os.path.join(os.path.dirname(__file__), "..", "out"); os.makedirs(OUT, exist_ok=True)
BALL = TriondaHorned(iters=1)
OUTSIDE = np.array([colorsys.hsv_to_rgb(c/4*0.75+0.07, 0.62, 0.98) for c in range(4)])
INSIDE = np.array([colorsys.hsv_to_rgb(0.55+0.04*c, 0.35, 0.42) for c in range(4)])
SEAM = np.array([0.06, 0.06, 0.08])


def paint(U, F, seam_width=0.035):
    """Panel class and seam flag per face from the sphere point it carries."""
    C = U[F].mean(1); C /= np.linalg.norm(C, axis=1, keepdims=True)
    cls = BALL.lookup_cls[BALL.lookup.query(C)[1]]
    seam = BALL.tree.query(C)[0] < seam_width
    return cls.astype(int), seam


def draw(ax, s, elev=22, azim=-58, L=1.75, nth=121, nph=240):
    par = E.schedule(s)
    V, F, U = E.sphere_mesh(nth, nph, **par)
    cls, seam = paint(U, F)
    e, a = np.radians(elev), np.radians(azim)
    cam = np.array([np.cos(e)*np.cos(a), np.cos(e)*np.sin(a), np.sin(e)])      # towards the viewer
    N = E.normals(V, F); N /= np.linalg.norm(N, axis=1, keepdims=True)+1e-12
    front = N @ cam > 0                                                         # +n side faces us
    col = np.where(front[:, None], OUTSIDE[cls], INSIDE[cls])
    col[seam] = SEAM
    col *= (0.6+0.4*np.abs(N @ cam))[:, None]
    right = np.array([-np.sin(a), np.cos(a), 0]); up = np.cross(cam, right)
    P = np.stack([V @ right, V @ up], -1); D = V @ cam
    tri = P[F]; o = np.argsort(D[F].mean(1))                                    # far first (D = depth towards viewer)
    ax.add_collection(PolyCollection(tri[o], facecolors=col[o], edgecolors=col[o], linewidths=0.25))
    ax.set_xlim(-L, L); ax.set_ylim(-L, L); ax.set_aspect("equal"); ax.axis("off")
    ax.set_title(f"s={s:.2f}  t={par['t']:+.2f}", fontsize=9)


if __name__ == "__main__":
    ss = [0, .14, .29, .43, .5, .57, .71, .86, 1.0]
    fig, axs = plt.subplots(1, len(ss), figsize=(2.6*len(ss), 2.9), facecolor=(.97, .97, .975))
    for ax, s in zip(axs, ss):
        draw(ax, s)
    fig.suptitle("Trionda ball everted (Bednorz-Bednorz map): warm saturated = +n side, dark cool = -n side", fontsize=10)
    fig.savefig(os.path.join(OUT, "eversion_trionda.png"), dpi=110, bbox_inches="tight")
    print("wrote out/eversion_trionda.png")
