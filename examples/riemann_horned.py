"""Riemann Sphere (iterated map) and Alexander's Horned Sphere (recursive field)."""
from _common import *

# ---- Riemann sphere: its parameters ------------------------------------------------
RS = fk.RiemannSphere
view = dict(camera=fk.Camera(azimuth=70, elevation=15, distance=4.0), step=0.7, max_steps=250)
riemann = [
    ("default (bell of studs)", RS(iters=8)),
    ("offset (0.6, 0.3): grid slides", RS(offset=(0.6, 0.3), iters=8)),
    ("radius 0.6: pointed heart", RS(radius=0.6, iters=8)),
    ("mandel 1: Mandelbrot-like", RS(mandel=1.0, iters=8)),
    ("c sideways (0.4, 0.9, 0)", RS(c=(0.4, 0.9, 0.0), iters=8)),
    ("iterations 14", RS(iters=14)),
]
grid(riemann, name="riemann_sphere", **view)

# ---- Horned sphere: generations and the clasp ---------------------------------------
top = dict(camera=fk.Camera(azimuth=-90, elevation=62, distance=5.2), step=0.9, max_steps=300, palette="plasma", trap_weight=1.4)
gens = [(f"{n} generation{'s' if n > 1 else ''}", fk.HornedSphere(iters=n)) for n in (1, 2, 3, 5)]
grid(gens + [("8 generations", fk.HornedSphere(iters=8))], ncols=5, size=(220, 165), name="horned_generations", **top)

clasp_cam = dict(camera=fk.Camera(azimuth=-90, elevation=40, distance=2.2, target=(0, 0.75, 0)), step=0.9, max_steps=300, palette="plasma", trap_weight=1.4)
params = [
    ("default, close on the clasp", fk.HornedSphere(iters=9), clasp_cam),
    ("shrink 0.35 (sparse)", fk.HornedSphere(shrink=0.35, clasp=1.4, iters=7), top),
    ("shrink 0.5, clasp 1.2 (dense)", fk.HornedSphere(shrink=0.5, clasp=1.2, iters=7), top),
    ("turn 20", fk.HornedSphere(turn=20, iters=7), top),
    ("tube 0.15, taper 0", fk.HornedSphere(tube=0.15, taper=0.0, iters=6), top),
    ("ball 0 (bare ring)", fk.HornedSphere(ball=0.0, iters=7), top),
]
grid(params, name="horned_params", size=(260, 195))

# ---- the clasp as linked circles (fibre-network view) ------------------------------
h = fk.HornedSphere(turn=45)
fig = plt.figure(figsize=(10.5, 4.6))
ax = fig.add_subplot(1, 2, 1, projection="3d")
phi = np.linspace(0, 2 * np.pi, 400)
root = np.stack([np.cos(phi), np.sin(phi), 0 * phi], -1)
# the shape's frame (x_s, y_s, z_s) = (y, x, -z) in world coordinates
w = lambda c: np.stack([c[:, 1], c[:, 0], -c[:, 2]], -1)
for c, col, lab in ((root, "0.55", "root ring"), (h.child_circle("a"), "tab:red", "child a"), (h.child_circle("b"), "tab:blue", "child b")):
    ax.plot(*w(c).T, color=col, lw=2, label=lab)
ax.set_box_aspect((1, 1, 0.5)); ax.view_init(35, -80); ax.legend(fontsize=8, loc="upper left")
lk = fk.linking_number(h.child_circle("a"), h.child_circle("b"))
ax.set_title(f"first-generation rings: Gauss linking number = {lk:+.3f}", fontsize=9); ax.set_axis_off()
ax2 = fig.add_subplot(1, 2, 2)
turns = np.linspace(0, 90, 19)
lks = [fk.linking_number(*(fk.HornedSphere(turn=t).child_circle(k) for k in "ab")) for t in turns]
ax2.plot(turns, np.abs(lks), "o-"); ax2.set_ylim(0, 1.6); ax2.axhline(1, color="k", lw=0.6, ls="--")
ax2.set_xlabel("turn (degrees)"); ax2.set_ylabel("|linking number| of the two child rings")
ax2.set_title("the clasp is a link for every turn", fontsize=9)
fig.tight_layout(); fig.savefig(OUT / "horned_linking.png", dpi=130); print("wrote", OUT / "horned_linking.png")

# ---- slices ---------------------------------------------------------------------------
fig, axs = plt.subplots(1, 3, figsize=(12, 4))
fk.plot_slice(fk.HornedSphere(iters=8), "z", 0.0, 1.9, 700, ax=axs[0], title="horned sphere, ring plane z=0")
fk.plot_slice(fk.RiemannSphere(iters=14), "y", 0.0, 1.5, 600, mode="trap", ax=axs[1], title="Riemann sphere, y=0 (trap)")
fk.plot_slice(fk.RiemannSphere(iters=14), "x", 0.0, 1.5, 600, mode="mask", ax=axs[2], title="Riemann sphere, x=0 (inside)")
fig.tight_layout(); fig.savefig(OUT / "riemann_horned_slices.png", dpi=130); print("wrote", OUT / "riemann_horned_slices.png")

# ---- mixes with the Hopf fibration -------------------------------------------------
hf = fk.HopfFibers(fk.ring_base([-0.5, 0.0, 0.5], 12), radius=0.05, max_circle=8.0)
mixes = [
    ("horned sphere + Hopf fibres (smooth)", fk.HornedSphere(iters=7).smooth_union(hf.scaled(0.45), 0.05), top),
    ("Riemann sphere threaded by fibres", fk.RiemannSphere(iters=8) | hf.scaled(0.35), view),
    ("Riemann <-> horned morph t=0.5", fk.RiemannSphere(iters=8).blend(fk.HornedSphere(iters=6), 0.5), view),
]
grid(mixes, name="riemann_horned_mix", size=(280, 210))
