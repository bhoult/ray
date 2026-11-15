import pathlib, sys
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
import fractalkit as fk

OUT = pathlib.Path(__file__).resolve().parents[1] / "out"
OUT.mkdir(exist_ok=True)


def panel(ax, field, title, size=(240, 180), **kw):
    ax.imshow(fk.render(field, size=size, **kw))
    ax.set_title(title, fontsize=9)
    ax.axis("off")


def grid(items, ncols=3, size=(240, 180), name="out", **kw):
    nrows = -(-len(items) // ncols)
    fig, axs = plt.subplots(nrows, ncols, figsize=(3.4 * ncols, 2.7 * nrows))
    axs = np.atleast_1d(axs).ravel()
    for ax, item in zip(axs, items):
        title, f, *extra = item          # (title, field[, per-panel render kwargs])
        panel(ax, f, title, size, **{**kw, **(extra[0] if extra else {})})
    for ax in axs[len(items):]:
        ax.axis("off")
    fig.tight_layout()
    p = OUT / f"{name}.png"
    fig.savefig(p, dpi=130)
    plt.close(fig)
    print("wrote", p)
    return p


def linking_plot(ax, L, touch, title, xlabel=None):
    """Heat map of a linking matrix: colour = linking number, grey = touching (undefined)."""
    n = len(L)
    M = np.where(touch, np.nan, np.round(L))
    cmap = plt.get_cmap("RdBu_r").copy(); cmap.set_bad("0.82")
    ax.imshow(M, cmap=cmap, vmin=-1.5, vmax=1.5, interpolation="nearest")
    ax.set_title(title, fontsize=9); ax.set_xticks([]); ax.set_yticks([])
    if xlabel:
        ax.set_xlabel(xlabel, fontsize=8)
