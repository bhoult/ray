"""Non-fractal geometry that mixes with the fractals: primitives, fibre
networks (capsule graphs, knots, lattices), the Hopf fibration and
sympy-built implicit surfaces.

Every class is a ``Field``, so ``Menger() & HopfFibers(40).scaled(...)``,
``a.smooth_union(b, 0.1)``, ``Hybrid`` outputs and so on all just work.
"""
from __future__ import annotations

from typing import Callable, Sequence

import numpy as np

from .core import Field

# --------------------------------------------------------------------------
# Primitives
# --------------------------------------------------------------------------


class Sphere(Field):
    def __init__(self, r: float = 1.0):
        self.r, self.bound = float(r), float(r)

    def _de(self, P):
        return np.linalg.norm(P, axis=-1) - self.r, np.zeros(len(P))


class Box(Field):
    def __init__(self, half=(1.0, 1.0, 1.0), rounding: float = 0.0):
        self.h = np.broadcast_to(np.asarray(half, float), (3,)).copy()
        self.rounding = float(rounding)
        self.bound = float(np.linalg.norm(self.h)) + rounding

    def _de(self, P):
        q = np.abs(P) - self.h
        d = np.linalg.norm(np.maximum(q, 0), axis=-1) + np.minimum(q.max(-1), 0)
        return d - self.rounding, np.zeros(len(P))


class Torus(Field):
    """Torus about the z axis: major radius R, minor radius r."""

    def __init__(self, R: float = 1.0, r: float = 0.3):
        self.R, self.r, self.bound = float(R), float(r), float(R + r)

    def _de(self, P):
        q = np.hypot(np.hypot(P[:, 0], P[:, 1]) - self.R, P[:, 2])
        return q - self.r, np.zeros(len(P))


class Plane(Field):
    def __init__(self, normal=(0, 0, 1), offset: float = 0.0):
        n = np.asarray(normal, float)
        self.n, self.o = n / np.linalg.norm(n), float(offset)
        self.bound = np.inf

    def _de(self, P):
        return P @ self.n - self.o, np.zeros(len(P))


# --------------------------------------------------------------------------
# Fibre networks
# --------------------------------------------------------------------------


def _chunked_min(P: np.ndarray, K: int, fn: Callable[[np.ndarray], np.ndarray], budget: int = 3_000_000):
    """Evaluate fn(P_chunk) -> (n, K) distances in chunks and return the
    per-point min and argmin."""
    n = len(P)
    step = max(1, budget // max(K, 1))
    d = np.empty(n)
    idx = np.empty(n, dtype=np.int64)
    for s in range(0, n, step):
        m = fn(P[s:s + step])
        j = m.argmin(axis=1)
        idx[s:s + step] = j
        d[s:s + step] = m[np.arange(len(m)), j]
    return d, idx


class FiberNetwork(Field):
    """A graph of straight capsules (segments with round caps).

    ``segments`` is ``(M, 2, 3)``; ``radius`` one number or one per segment
    (tapered tubes).  The trap is the nearest segment's ``labels`` entry
    (default: its index, mapped to 0..1), so colouring by trap shows the
    individual fibres, or whatever the labels say (e.g. the generation).
    """

    def __init__(self, segments: np.ndarray, radius=0.03, labels=None):
        self.seg = np.asarray(segments, float).reshape(-1, 2, 3)
        self.radius = np.broadcast_to(np.asarray(radius, float), (len(self.seg),)).copy()   # scalar or per segment
        self.labels = None if labels is None else np.asarray(labels, float)
        self.bound = float(np.abs(self.seg).max() * np.sqrt(3) + self.radius.max())

    # -- constructors -----------------------------------------------------
    @classmethod
    def from_polyline(cls, pts: np.ndarray, radius: float = 0.03, closed: bool = False):
        pts = np.asarray(pts, float)
        a, b = pts[:-1], pts[1:]
        if closed:
            a, b = np.vstack([a, pts[-1:]]), np.vstack([b, pts[:1]])
        return cls(np.stack([a, b], 1), radius)

    @classmethod
    def from_curves(cls, curves, radii=0.03, closed=True, labels=None):
        """One network from several polylines.  ``radii`` and ``labels`` are
        per curve (a number each) or per curve a per-segment array."""
        segs, rad, lab = [], [], []
        for i, c in enumerate(curves):
            c = np.asarray(c, float)
            a, b = (np.vstack([c, c[:1]])[:-1], np.vstack([c, c[:1]])[1:]) if closed else (c[:-1], c[1:])
            segs.append(np.stack([a, b], 1))
            r = radii[i] if hasattr(radii, "__len__") else radii
            rad.append(np.broadcast_to(np.asarray(r, float), (len(a),)))
            lv = (labels[i] if labels is not None else i)
            lab.append(np.broadcast_to(np.asarray(lv, float), (len(a),)))
        lab = np.concatenate(lab)
        if labels is None:
            lab = lab / max(len(curves) - 1, 1)
        return cls(np.concatenate(segs), np.concatenate(rad), lab)

    @classmethod
    def from_curve(cls, fn: Callable[[np.ndarray], np.ndarray], n: int = 400,
                   radius: float = 0.03, closed: bool = True):
        """Tube around a parametric curve ``fn(t) -> (n,3)`` with t in [0, 2pi)."""
        t = np.linspace(0, 2 * np.pi, n, endpoint=not closed)
        return cls.from_polyline(fn(t), radius, closed=closed)

    @classmethod
    def torus_knot(cls, p: int = 2, q: int = 3, R: float = 1.0, r: float = 0.4,
                   radius: float = 0.06, n: int = 600):
        def fn(t):
            rr = R + r * np.cos(q * t)
            return np.stack([rr * np.cos(p * t), rr * np.sin(p * t), r * np.sin(q * t)], -1)
        return cls.from_curve(fn, n, radius)

    @classmethod
    def lattice(cls, n: int = 3, spacing: float = 1.0, radius: float = 0.04, diagonals: bool = False):
        """Simple cubic lattice of ``n^3`` nodes joined along the axes
        (and optionally the body diagonals), centred on the origin."""
        ax = (np.arange(n) - (n - 1) / 2) * spacing
        X, Y, Z = np.meshgrid(ax, ax, ax, indexing="ij")
        nodes = np.stack([X, Y, Z], -1)
        segs = []
        for axis in range(3):
            sl_a = [slice(None)] * 3
            sl_b = [slice(None)] * 3
            sl_a[axis], sl_b[axis] = slice(0, -1), slice(1, None)
            segs.append(np.stack([nodes[tuple(sl_a)].reshape(-1, 3), nodes[tuple(sl_b)].reshape(-1, 3)], 1))
        if diagonals:
            segs.append(np.stack([nodes[:-1, :-1, :-1].reshape(-1, 3), nodes[1:, 1:, 1:].reshape(-1, 3)], 1))
        return cls(np.concatenate(segs), radius)

    # -- distance ---------------------------------------------------------
    def _de(self, P):
        a, b = self.seg[:, 0], self.seg[:, 1]
        ab = b - a
        L2 = np.maximum((ab * ab).sum(-1), 1e-30)

        def chunk(Q):
            ap = Q[:, None, :] - a[None]
            t = np.clip((ap * ab[None]).sum(-1) / L2[None], 0, 1)
            return np.linalg.norm(ap - t[..., None] * ab[None], axis=-1) - self.radius[None]

        d, idx = _chunked_min(P, len(self.seg), chunk)
        if self.labels is not None:
            return d, self.labels[idx]
        return d, idx / max(len(self.seg) - 1, 1)


# --------------------------------------------------------------------------
# Hopf fibration
# --------------------------------------------------------------------------


def stereographic(x4: np.ndarray) -> np.ndarray:
    """S^3 -> R^3 stereographic projection from the pole (0,0,0,1)."""
    x4 = np.asarray(x4, float)
    return x4[..., :3] / (1.0 - x4[..., 3:4])


def inverse_stereographic(u: np.ndarray) -> np.ndarray:
    """R^3 -> S^3, the inverse of ``stereographic`` (pole (0,0,0,1) is infinity)."""
    u = np.asarray(u, float)
    r2 = (u * u).sum(-1, keepdims=True)
    return np.concatenate([2 * u, r2 - 1.0], -1) / (r2 + 1.0)


def hopf_fiber_points(eta: float, phi: float, t: np.ndarray) -> np.ndarray:
    """Points of the Hopf fibre over the base point with Hopf coordinates
    (eta, phi) on S^3: ``(cos eta e^{it}, sin eta e^{i(t+phi)})``."""
    return np.stack([np.cos(eta) * np.cos(t), np.cos(eta) * np.sin(t),
                     np.sin(eta) * np.cos(t + phi), np.sin(eta) * np.sin(t + phi)], -1)


def circle_through(a, b, c):
    """Centre, radius and unit normal of the circle through 3 points of R^3."""
    a, b, c = (np.asarray(v, float) for v in (a, b, c))
    u, v = b - a, c - a
    w = np.cross(u, v)
    w2 = float(w @ w)
    if w2 < 1e-18:
        return None
    centre = a + (np.cross(w, u) * float(v @ v) + np.cross(v, w) * float(u @ u)) / (2 * w2)
    return centre, float(np.linalg.norm(centre - a)), w / np.sqrt(w2)


def fibonacci_sphere(n: int) -> np.ndarray:
    i = np.arange(n) + 0.5
    u = 1 - 2 * i / n
    th = np.pi * (1 + 5 ** 0.5) * i
    s = np.sqrt(1 - u * u)
    return np.stack([s * np.cos(th), s * np.sin(th), u], -1)


def ring_base(latitudes: Sequence[float], per_ring: int = 12) -> np.ndarray:
    """Base points of S^2 on rings of constant ``x = cos(2 eta)``.

    Fibres over one ring lie on a single torus (a Clifford-type torus for
    ``x = 0``), so these give the classic nested-tori picture of the
    fibration.  ``latitudes`` are values of ``x`` in (-1, 1).
    """
    pts = []
    for x in latitudes:
        s = np.sqrt(1 - x * x)
        a = np.linspace(0, 2 * np.pi, per_ring, endpoint=False)
        pts.append(np.stack([np.full(per_ring, x), s * np.cos(a), s * np.sin(a)], -1))
    return np.concatenate(pts)


class HopfFibers(Field):
    """Tubes around Hopf fibres, stereographically projected to R^3.

    Each fibre of S^3 -> S^2 maps to a circle (a line only for the fibre
    through the projection pole, which is skipped); any two are linked once.
    ``base`` picks the base points on S^2: an int gives a Fibonacci
    spiral of that many; or pass an ``(n,3)`` array of unit vectors, e.g.
    a single latitude ring to get one nested torus of fibres.

    ``rotation`` is an extra angle (radians) applied in the S^3 ``t``
    direction before projecting, which makes no difference to the set
    (fibres are invariant) but is handy for animations of single points.
    The trap is the base-point index in 0..1 so colouring by trap shows
    the linking structure.
    """

    def __init__(self, base: int | np.ndarray = 24, radius: float = 0.05, max_circle: float = 6.0):
        pts = fibonacci_sphere(base) if isinstance(base, (int, np.integer)) else np.asarray(base, float)
        self.base = pts
        self.radius = float(radius)
        centres, radii, normals, keep = [], [], [], []
        for i, (x, y, z) in enumerate(pts):
            eta = 0.5 * np.arccos(np.clip(x, -1, 1))          # base = (cos2eta, sin2eta cos phi, sin2eta sin phi)
            phi = np.arctan2(z, y)
            t = np.array([0.0, 2 * np.pi / 3, 4 * np.pi / 3])
            q = stereographic(hopf_fiber_points(eta, phi, t))
            circ = circle_through(*q)
            if circ is None or circ[1] > max_circle or not np.all(np.isfinite(circ[0])):
                continue
            centres.append(circ[0]); radii.append(circ[1]); normals.append(circ[2]); keep.append(i)
        self.C, self.R, self.N = np.array(centres), np.array(radii), np.array(normals)
        self.keep = np.array(keep)
        self.bound = float(np.max(np.linalg.norm(self.C, axis=1) + self.R) + radius) if len(self.R) else 1.0

    def circles(self, spacing: float = 0.05, min_pts: int = 24):
        """The fibres as closed polylines, one ``(n,3)`` array each, with
        about ``spacing`` between points (more points on bigger circles)."""
        out = []
        for c, r, n in zip(self.C, self.R, self.N):
            u = np.cross(n, [1.0, 0, 0]) if abs(n[0]) < 0.9 else np.cross(n, [0, 1.0, 0])
            u /= np.linalg.norm(u); v = np.cross(n, u)
            m = max(min_pts, int(np.ceil(2 * np.pi * r / spacing)))
            a = np.linspace(0, 2 * np.pi, m, endpoint=False)
            out.append(c + r * (np.cos(a)[:, None] * u + np.sin(a)[:, None] * v))
        return out

    def _de(self, P):
        def chunk(Q):
            v = Q[:, None, :] - self.C[None]
            h = (v * self.N[None]).sum(-1)
            rho = np.linalg.norm(v - h[..., None] * self.N[None], axis=-1)
            return np.hypot(rho - self.R[None], h) - self.radius

        d, idx = _chunked_min(P, len(self.R), chunk)
        return d, self.keep[idx] / max(len(self.base) - 1, 1)


def densify(curve: np.ndarray, m: int, closed: bool = True) -> np.ndarray:
    """Subdivide every segment of a polyline into ``m`` equal pieces."""
    c = np.asarray(curve, float)
    nxt = np.roll(c, -1, 0) if closed else c[1:]
    base = c if closed else c[:-1]
    t = (np.arange(m) / m)[None, :, None]
    out = (base[:, None, :] * (1 - t) + nxt[:, None, :] * t).reshape(-1, 3)
    return out if closed else np.vstack([out, c[-1:]])


def linking_number(A: np.ndarray, B: np.ndarray, chunk: int = 400) -> float:
    """Gauss linking number of two closed polylines ``(n,3)``, ``(m,3)``
    (a float: close to an integer when the curves are well separated
    compared with their segment length; ``linking_matrix`` takes care of that)."""
    A, B = np.asarray(A, float), np.asarray(B, float)
    dA, dB = np.roll(A, -1, 0) - A, np.roll(B, -1, 0) - B
    mA, mB = A + dA / 2, B + dB / 2
    total = 0.0
    for s in range(0, len(A), chunk):
        r = mA[s:s + chunk, None] - mB[None]
        num = np.einsum("ijk,ijk->ij", np.cross(dA[s:s + chunk, None], dB[None]), r)
        total += (num / np.linalg.norm(r, axis=-1) ** 3).sum()
    return float(total / (4 * np.pi))


def curve_distance(A: np.ndarray, B: np.ndarray, rounds: int = 4, sub: int = 40) -> float:
    """Smallest distance between two closed polylines, refined: the nearest
    sample pair is re-sampled ``sub`` times finer in a window, ``rounds`` times
    (plain sample-to-sample distance overestimates by up to half a spacing)."""
    from scipy.spatial.distance import cdist

    A, B = (np.vstack([np.asarray(c, float), np.asarray(c, float)[:1]]) for c in (A, B))

    def window(P, k):
        seg = P[max(k - 2, 0):min(k + 3, len(P))]
        t = np.linspace(0, len(seg) - 1, (len(seg) - 1) * sub + 1)
        return np.stack([np.interp(t, np.arange(len(seg)), seg[:, c]) for c in range(3)], -1)

    for _ in range(rounds):
        D = cdist(A, B)
        i, j = np.unravel_index(D.argmin(), D.shape)
        A, B = window(A, i), window(B, j)
    return float(cdist(A, B).min())


def linking_matrix(curves, touch_tol: float = 0.05, max_densify: int = 8):
    """Pairwise Gauss linking numbers of closed polylines.

    Returns ``(L, touching)``.  A pair *touches* when the refined distance
    between the curves is below ``touch_tol`` times the longer of their longest
    segments (their chord error is far smaller than that, so a true contact
    always passes): the linking number of curves that share a point is
    undefined, and ``L`` holds ``nan`` there.  For every other pair the curves
    are densified until their segments are at most half the gap, up to
    ``max_densify`` times, and the Gauss integral is taken: the values are
    integers up to discretisation error (check ``|L - round(L)|``).
    """
    n = len(curves)
    seglen = [float(np.linalg.norm(np.roll(c, -1, 0) - c, axis=1).max()) for c in curves]
    L = np.zeros((n, n))
    touching = np.zeros((n, n), bool)
    for i in range(n):
        for j in range(i + 1, n):
            lmax = max(seglen[i], seglen[j])
            gap = curve_distance(curves[i], curves[j])
            if gap < touch_tol * lmax:
                touching[i, j] = touching[j, i] = True
                L[i, j] = L[j, i] = np.nan
                continue
            m = int(np.clip(np.ceil(2 * lmax / gap), 1, max_densify))
            A, B = (densify(curves[i], m), densify(curves[j], m)) if m > 1 else (curves[i], curves[j])
            L[i, j] = L[j, i] = linking_number(A, B)
    return L, touching


def hopf_map(x4: np.ndarray) -> np.ndarray:
    """The Hopf map S^3 -> S^2, (z1, z2) -> (|z1|^2-|z2|^2, 2 z1 conj(z2))."""
    z1 = x4[..., 0] + 1j * x4[..., 1]
    z2 = x4[..., 2] + 1j * x4[..., 3]
    w = 2 * z1 * np.conj(z2)
    return np.stack([np.abs(z1) ** 2 - np.abs(z2) ** 2, w.real, w.imag], -1)


# --------------------------------------------------------------------------
# Implicit surfaces from sympy
# --------------------------------------------------------------------------


class Implicit(Field):
    """Surface ``F(x,y,z) = 0`` from a sympy expression (or a callable).

    The distance is the first-order estimate ``F / |grad F|`` (exact for
    planes, good near the surface, poor far away); ``trap`` is |F| squashed.
    Use ``shell`` to thicken a zero set into a wall.
    """

    def __init__(self, expr, symbols=None, bound: float = 2.0):
        import sympy as sp

        if callable(expr) and not isinstance(expr, sp.Basic):
            raise TypeError("pass a sympy expression")
        x, y, z = symbols if symbols is not None else sp.symbols("x y z")
        self.expr = expr
        self._f = sp.lambdify((x, y, z), expr, "numpy")
        grad = [sp.diff(expr, v) for v in (x, y, z)]
        self._g = sp.lambdify((x, y, z), grad, "numpy")
        self.bound = float(bound)

    def _de(self, P):
        x, y, z = P[:, 0], P[:, 1], P[:, 2]
        f = np.broadcast_to(self._f(x, y, z), x.shape)
        g = np.stack([np.broadcast_to(c, x.shape) for c in self._g(x, y, z)], -1)
        n = np.maximum(np.linalg.norm(g, axis=-1), 1e-9)
        return f / n, np.tanh(np.abs(f))


def gyroid(period: float = 2 * np.pi, bound: float = 3.0) -> Implicit:
    import sympy as sp
    x, y, z = sp.symbols("x y z")
    k = 2 * sp.pi / period
    e = sp.sin(k * x) * sp.cos(k * y) + sp.sin(k * y) * sp.cos(k * z) + sp.sin(k * z) * sp.cos(k * x)
    return Implicit(e, (x, y, z), bound)


def clifford_torus() -> Torus:
    """Stereographic image of the Clifford torus |z1| = |z2| in S^3: a round
    torus of revolution about the z axis with R = sqrt(2), r = 1 (checked
    numerically in the tests).  The Hopf fibres over a latitude of S^2 lie
    on nested tori of this kind."""
    return Torus(R=np.sqrt(2.0), r=1.0)
