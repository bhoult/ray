"""Inversion groups of a ring and its two children.

The equatorial spheres of a ring (radius 1, centre 0) and of its two children (radius s, centres
(cg, +-off, 0)) meet at angles with cos = kappa*s (parent/child) and 1-2(kappa-1)^2 (child/child).
The domain 'inside the parent, outside both children' is then a circular triangle with angles
pi/m, pi/m (parent/child corners) and pi - theta_cc (child/child corner).  It is a reflection
triangle (m, m, n) when kappa = 1 + cos(pi/2n) and s = cos(pi/m)/kappa.  n=2 is the Hopf clasp
(kappa = 1+1/sqrt 2), n=3 the second oloid value (1+sqrt 3/2).  We check the angles, the circle
orthogonal to all three, and that orbits of random reduced words land on it."""
from _common import *
from matplotlib.patches import Circle

rng = np.random.default_rng(0)


def spheres(s, kap):
    sg = kap * s
    cg, off = np.sqrt(1 - sg ** 2), sg - s
    return [(np.zeros(3), 1.0), (np.array([cg, off, 0.0]), s), (np.array([cg, -off, 0.0]), s)]


def cosang(S, i, j):
    (c1, r1), (c2, r2) = S[i], S[j]
    return (r1 * r1 + r2 * r2 - np.sum((c1 - c2) ** 2)) / (2 * r1 * r2)


def inv(x, S, i):
    c, r = S[i]
    v = x - c
    return c + r * r * v / np.sum(v * v)


def ortho_circle(S):
    A = np.array([[-2 * c[0], -2 * c[1], 1] for c, r in S])
    b = np.array([r * r - c[0] ** 2 - c[1] ** 2 for c, r in S])
    C0, C1, K = np.linalg.solve(A, b)
    return np.array([C0, C1]), C0 ** 2 + C1 ** 2 - K


def orbit(S, n=60, N=3000):
    P = []
    for _ in range(N):
        x, last = np.array([0.2, 0.05, 0.0]), -1
        for _ in range(n):
            i = rng.choice([k for k in range(3) if k != last])
            x, last = inv(x, S, i), i
        P.append(x)
    return np.array(P)


def word_err(S, word, order, x=np.array([0.31, -0.17, 0.23])):
    y = x.copy()
    for _ in range(order):
        for i in reversed(word):
            y = inv(y, S, i)
    return np.linalg.norm(y - x)


def kappa_n(n):
    return 1 + np.cos(np.pi / (2 * n))


cases = [(5, 2), (6, 2), (7, 2), (4, 3), (5, 3), (3, 4), (4, 4)]
print(" m n   kappa    s      domain angles (deg)         hyperbolic  R^2(orth)  relation errors (01)^m (12)^n (02)^m   links(d3) touching")
rows = []
for m, n in cases:
    kap, s = kappa_n(n), None
    s = np.cos(np.pi / m) / kap
    S = spheres(s, kap)
    pc, cc = np.degrees(np.arccos(cosang(S, 0, 1))), 180 - np.degrees(np.arccos(cosang(S, 1, 2)))
    hyp = 2 / m + 1 / n < 1
    C, R2 = ortho_circle(S)
    e1, e2, e3 = word_err(S, (0, 1), m), word_err(S, (1, 2), n), word_err(S, (0, 2), m)
    h = fk.HornedSphere(shrink=s, clasp=kap, turn=45.0, iters=3)
    L, touch, rings = h.linking(3, 0.02)
    pcs = {(i, r.parent) for i, r in enumerate(rings) if r.parent >= 0}
    nl = sum(1 for i in range(len(rings)) for j in range(i + 1, len(rings)) if not touch[i, j] and abs(L[i, j]) > .5)
    ne = sum(1 for i in range(len(rings)) for j in range(i + 1, len(rings)) if touch[i, j] and (i, j) not in pcs and (j, i) not in pcs)
    rows.append((m, n, kap, s, S, C, R2))
    print(f"{m:2d} {n:2d}  {kap:.4f}  {s:.4f}  ({pc:6.2f}, {pc:6.2f}, {cc:6.2f})   {str(hyp):9s}  {R2:8.4f}   {e1:.0e} {e2:.0e} {e3:.0e}   {nl:4d}  {ne:4d}")

# a non-discrete comparison: same kappa as (5,2) but s off the discrete value
kap = kappa_n(2)
for s in (0.46, 0.5):
    S = spheres(s, kap)
    pc = np.degrees(np.arccos(cosang(S, 0, 1)))
    print(f"generic s={s}: parent/child angle {pc:.2f} deg; (01)^5 error {word_err(S, (0, 1), 5):.2f}, (01)^m error for m=pi/angle is not an integer")

# figure: circles, images of the triangle under short words, and the limit circle
fig, axs = plt.subplots(1, 3, figsize=(12, 4.2))
for ax, (m, n) in zip(axs, [(5, 2), (4, 3), (7, 2)]):
    r_ = next(r for r in rows if r[0] == m and r[1] == n)
    _, _, kap, s, S, C, R2 = r_
    # images of the three circles under all reduced words up to length 4
    circles = {(): [(S[i][0][:2], S[i][1]) for i in range(3)]}
    def image(circ, S, i):
        c, r = circ
        v = c - S[i][0][:2]; d2 = v @ v; R = S[i][1]
        den = d2 - r * r
        return S[i][0][:2] + R * R * v / den, R * R * r / abs(den)
    frontier = [((), -1)]
    allc = []
    for depth in range(5):
        new = []
        for word, last in frontier:
            for i in range(3):
                if i == last: continue
                w = word + (i,)
                cs = [(S[j][0][:2], S[j][1]) for j in range(3)]
                for k in reversed(w):
                    cs = [image(c, S, k) for c in cs]
                allc.append((w, cs)); new.append((w, i))
        frontier = new
    for w, cs in allc:
        for c, r in cs:
            if r < 5: ax.add_patch(Circle(c, r, fill=False, lw=0.4, color="tab:blue", alpha=0.5))
    for (c, r), col in zip([(S[i][0][:2], S[i][1]) for i in range(3)], ("k", "tab:red", "tab:red")):
        ax.add_patch(Circle(c, r, fill=False, lw=1.6, color=col))
    ax.add_patch(Circle(C, np.sqrt(R2), fill=False, lw=1.6, color="tab:green", ls="--"))
    ax.set_xlim(-1.25, C[0] + np.sqrt(R2) + 0.5)
    ax.set_ylim(-1.25, 1.25)
    ax.set_aspect("equal"); ax.axis("off")
    ax.set_title(f"$(m,n)=({m},{n})$: $\\kappa={kap:.4f}$, $s={s:.4f}$", fontsize=9)
fig.suptitle("parent (black), children (red), their images under words of length $\\leq5$ (blue), limit circle (green)", fontsize=9)
fig.tight_layout()
p = OUT / "hopf_reflection.png"; fig.savefig(p, dpi=140); print("wrote", p)
