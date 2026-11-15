"""Box-counting dimensions (scipy regression) against the exact values (sympy)."""
from _common import *
from fractalkit import symbolic

cases = [  # (name, field, grid base aligned with the fractal, levels, exact)
    ("Menger sponge", fk.Menger(4), 3, 4, symbolic.kifs_scale_dimension(3, 20)),
    ("Sierpinski tetrahedron", fk.Sierpinski(6), 2, 7, symbolic.kifs_scale_dimension(2, 4)),
]
fig, axs = plt.subplots(1, len(cases), figsize=(4.6 * len(cases), 3.6))
for ax, (name, f, base, lv, exact) in zip(axs, cases):
    dim, counts = fk.box_counting_dimension(f, extent=1.0, base=base, levels=lv, max_cells=base ** 6 if base == 2 else None)
    ns = np.array(sorted(counts)); N = np.array([counts[n] for n in ns])
    ax.loglog(ns, N, "o-", label=f"measured slope {dim:.3f}")
    ax.loglog(ns, N[0] * (ns / ns[0]) ** exact, "k--", lw=0.8, label=f"exact {exact:.3f}")
    ax.set_xlabel("cells per side"); ax.set_ylabel("occupied boxes"); ax.set_title(name); ax.legend()
    print(f"{name}: measured {dim:.3f}  exact {exact:.3f}")
fig.tight_layout(); fig.savefig(OUT / "dimension.png", dpi=130); print("wrote", OUT / "dimension.png")
