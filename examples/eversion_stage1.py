"""Two-sided paint on the Bednorz & Bednorz twisted cylinder (n=2), several times t."""
import sys, os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import numpy as np, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.collections import PolyCollection
from fractalkit import eversion as E

OUT = os.path.join(os.path.dirname(__file__), "..", "out"); os.makedirs(OUT, exist_ok=True)
INSIDE, OUTSIDE = np.array([.85, .25, .2]), np.array([.2, .45, .8])   # paint on the two sides

def draw(ax, t, elev=25, azim=-60, H=1.6, nh=60, nphi=120):
    V, F, hp = E.mesh(t, H=H, nh=nh, nphi=nphi)
    # view direction (camera -> scene), orthographic
    e, a = np.radians(elev), np.radians(azim)
    cam = np.array([np.cos(e)*np.cos(a), np.cos(e)*np.sin(a), np.sin(e)])
    N = E.normals(V, F); N /= np.linalg.norm(N, axis=1, keepdims=True)+1e-12
    facing = N @ cam > 0                      # parametrisation normal toward the viewer?
    # paint on the domain: side colour, modulated by phi sector so the seams are visible
    phi = hp[F].mean(1)[:, 1]; hh = hp[F].mean(1)[:, 0]
    sector = (np.floor(phi/(2*np.pi)*8) % 2)*0.18
    stripe = (np.floor((hh+H)/(2*H)*8) % 2)*0.10
    col = np.where(facing[:, None], OUTSIDE, INSIDE) + (sector+stripe)[:, None]
    col = np.clip(col, 0, 1)
    shade = 0.55+0.45*np.abs(N @ cam)
    col = col*shade[:, None]
    # project and depth-sort (painter's)
    right = np.array([-np.sin(a), np.cos(a), 0]); up = np.cross(cam, right)
    P = np.stack([V @ right, V @ up], -1); D = V @ cam
    tri = P[F]; order = np.argsort(-D[F].mean(1))     # far first
    ax.add_collection(PolyCollection(tri[order], facecolors=col[order], edgecolors="none"))
    ax.set_xlim(P[:, 0].min()-.1, P[:, 0].max()+.1); ax.set_ylim(P[:, 1].min()-.1, P[:, 1].max()+.1)
    ax.set_aspect("equal"); ax.axis("off")
    ax.set_title(f"t = {t:+.2f}", fontsize=10)

if __name__ == "__main__":
    ts = [-1.5, -1.0, -0.56, 0.0, 0.56, 1.0, 1.5]
    fig, axs = plt.subplots(1, len(ts), figsize=(3*len(ts), 3.4), facecolor=(.97, .97, .975))
    for ax, t in zip(axs, ts):
        draw(ax, t)
    fig.suptitle("Bednorz–Bednorz twisted cylinder, n=2: blue = +n side (parametrisation normal), red = -n side; paint follows the surface", fontsize=10)
    fig.savefig(os.path.join(OUT, "eversion_stage1.png"), dpi=110, bbox_inches="tight")
