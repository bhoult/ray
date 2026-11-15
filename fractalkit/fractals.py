"""Fractal distance estimators, ported from ray's Rust kernels
(``src/render/vm.rs`` and ``src/formulas/*.rs``).

Iterated 3D maps are ``IterMap`` s (mixable in a ``Hybrid``):
``Mandelbulb`` (also the Juliabulb), ``Mandelbox``, ``KIFS`` (the Menger
sponge in Knighty's folding form is one preset), ``Sierpinski``,
``Apollonian``, ``PseudoKleinian``.

Stand-alone fields: ``Menger`` (closed form), ``QuaternionJulia``,
``Hopfbrot`` (4D map cut to 3D).
"""
from __future__ import annotations

import numpy as np

from .core import Field, IterMap, State, euler_matrix

# --------------------------------------------------------------------------
# Bulbs
# --------------------------------------------------------------------------


class Mandelbulb(IterMap):
    """z <- r^n (sin(n th) cos(n ph), cos(n th), sin(n th) sin(n ph)) + c.

    With ``julia`` set to a 3-vector, ``c`` is that constant instead of the
    sample point: the Juliabulb.
    """

    bail = 2.0
    bound = 1.4

    def __init__(self, power: float = 8.0, iters: int = 12, julia=None):
        self.power, self.iters = float(power), int(iters)
        self.julia = None if julia is None else np.asarray(julia, float)
        if self.julia is not None:
            self.bound = 1.4

    def step(self, st: State, i: int) -> None:
        n, a = self.power, st.active
        z = st.z
        r = np.maximum(np.linalg.norm(z, axis=-1), 1e-6)
        theta = np.arctan2(np.hypot(z[:, 0], z[:, 2]), z[:, 1]) * n
        phi = np.arctan2(z[:, 2], z[:, 0]) * n
        rp1 = r ** (n - 1.0)
        ndr = rp1 * st.dr * n + (0.0 if self.julia is not None else 1.0)
        zr = rp1 * r
        c = st.c if self.julia is None else self.julia
        st_ = np.sin(theta)
        nz = np.stack([zr * st_ * np.cos(phi), zr * np.cos(theta), zr * st_ * np.sin(phi)], -1) + c
        st.commit(a, nz, ndr)
        st.trap = np.where(a, np.minimum(st.trap, st.r), st.trap)

    def finish(self, st: State):
        r = np.maximum(st.r, 1e-6)
        return np.log(r) * r * 0.5 / np.maximum(st.dr, 1e-12), np.clip(st.trap * 0.5, 0, 1)


# --------------------------------------------------------------------------
# Folding fractals
# --------------------------------------------------------------------------


class Mandelbox(IterMap):
    bound = 6.0

    def __init__(self, scale: float = 2.0, min_r: float = 0.5, iters: int = 12):
        self.scale, self.min_r, self.iters = float(scale), float(min_r), int(iters)
        self.bound = 3.0 * abs(scale) + 1.0

    def step(self, st: State, i: int) -> None:
        z = np.clip(st.z, -1.0, 1.0) * 2.0 - st.z           # box fold
        r2 = np.einsum("ij,ij->i", z, z)
        st.trap = np.minimum(st.trap, r2)
        min2 = self.min_r ** 2
        k = np.where(r2 < min2, 1.0 / min2, 1.0 / np.maximum(r2, 1e-12))
        k = np.where(r2 < 1.0, k, 1.0)                       # sphere fold
        z = z * k[:, None]
        dr = st.dr * k
        z = z * self.scale + st.c
        dr = dr * abs(self.scale) + 1.0
        st.z, st.dr = z, dr

    def finish(self, st: State):
        return st.r / np.abs(st.dr), np.clip(np.sqrt(st.trap), 0, 1)


class KIFS(IterMap):
    """Kaleidoscopic IFS (Knighty's Menger form): abs, sort axes, rotate,
    scale by ``s`` about ``off``; distance to the unit box, scaled back.

    Defaults give the Menger sponge.  ``rot`` is a 3x3 matrix or Euler
    angles in degrees ``(rx, ry, rz)``.
    """

    bound = 1.8

    def __init__(self, s: float = 3.0, off=(1.0, 1.0, 1.0), rot=None, iters: int = 8):
        self.s, self.off, self.iters = float(s), np.asarray(off, float), int(iters)
        if rot is None:
            self.rot = np.eye(3)
        else:
            rot = np.asarray(rot, float)
            self.rot = rot if rot.shape == (3, 3) else euler_matrix(*rot)

    def step(self, st: State, i: int) -> None:
        k = self.s - 1.0
        z = -np.sort(-np.abs(st.z), axis=-1)                 # x >= y >= z
        z = z @ self.rot.T
        z = z * self.s - self.off * k
        low = z[:, 2] < -0.5 * self.off[2] * k
        z[:, 2] = np.where(low, z[:, 2] + self.off[2] * k, z[:, 2])
        st.z = z
        st.dr = st.dr * self.s
        st.trap = np.minimum(st.trap, np.einsum("ij,ij->i", z, z) / st.dr ** 2)

    def finish(self, st: State):
        q = np.abs(st.z) - 1.0
        boxd = np.linalg.norm(np.maximum(q, 0), axis=-1) + np.minimum(q.max(-1), 0)
        return boxd / st.dr, np.clip(np.sqrt(st.trap) * 1.5, 0, 1)


def Menger_kifs(iters: int = 8) -> KIFS:
    return KIFS(3.0, (1, 1, 1), None, iters)


class Sierpinski(IterMap):
    """Sierpinski tetrahedron by folding: 3 reflections, scale 2 about (1,1,1)."""

    bound = 1.8

    def __init__(self, iters: int = 10):
        self.iters = int(iters)

    def start(self, P):
        st = super().start(P)
        st.extra["folds"] = np.zeros(len(P))
        return st

    def step(self, st: State, i: int) -> None:
        z = st.z.copy()
        f = st.extra.setdefault("folds", np.zeros(len(z)))
        for a, b in ((0, 1), (0, 2), (1, 2)):
            m = (z[:, a] + z[:, b]) < 0
            f += m
            za, zb = z[:, a].copy(), z[:, b].copy()
            z[:, a] = np.where(m, -zb, za)
            z[:, b] = np.where(m, -za, zb)
        st.z = z * 2.0 - 1.0
        st.dr = st.dr * 2.0

    def finish(self, st: State):
        x, y, z = st.z[:, 0], st.z[:, 1], st.z[:, 2]
        t = np.maximum(np.abs(x + y) - z, np.abs(x - y) + z) - 1.0
        folds = st.extra.get("folds", 0.0)
        return t * 0.57735026 / st.dr, np.clip(folds / (3 * max(self.iters, 1)), 0, 1)


class Apollonian(IterMap):
    """iq's Apollonian packing: fold into [-1,1]^3 then invert in the sphere."""

    bound = 1.5

    def __init__(self, fold: float = 1.0, iters: int = 8):
        self.fold, self.iters = float(fold), int(iters)

    def step(self, st: State, i: int) -> None:
        h = st.z * 0.5 + 0.5
        z = (h - np.floor(h)) * 2.0 - 1.0
        r2 = np.maximum(np.einsum("ij,ij->i", z, z), 1e-12)
        st.trap = np.minimum(st.trap, r2)
        k = self.fold / r2
        st.z = z * k[:, None]
        st.dr = st.dr * k

    def finish(self, st: State):
        return np.abs(st.z[:, 1]) * 0.25 / st.dr, np.clip(st.trap * 6.0, 0, 1)


class PseudoKleinian(IterMap):
    """Knighty's pseudo-Kleinian: box fold then inversion, repeated."""

    bound = 2.0

    def __init__(self, cbox=(0.9, 0.9, 0.9), size: float = 1.0, iters: int = 10):
        self.cbox, self.size, self.iters = np.asarray(cbox, float), float(size), int(iters)

    def step(self, st: State, i: int) -> None:
        z = np.clip(st.z, -self.cbox, self.cbox) * 2.0 - st.z
        r2 = np.maximum(np.einsum("ij,ij->i", z, z), 1e-12)
        st.trap = np.minimum(st.trap, r2)
        k = np.maximum(self.size / r2, 1.0)
        st.z = z * k[:, None]
        st.dr = st.dr * k

    def finish(self, st: State):
        rxy = np.hypot(st.z[:, 0], st.z[:, 1])
        d = np.maximum(rxy - 0.92784, np.abs(rxy * st.z[:, 2]) / np.maximum(st.r, 1e-12))
        return d / st.dr, np.clip(np.sqrt(st.trap), 0, 1)


# --------------------------------------------------------------------------
# Closed-form / stand-alone
# --------------------------------------------------------------------------


class Menger(Field):
    """Menger sponge filling [-1,1]^3, closed-form per-level distance."""

    bound = 1.8

    def __init__(self, iters: int = 5):
        self.iters = min(int(iters), 12)

    def _de(self, P):
        q = np.abs(P) - 1.0
        d = np.linalg.norm(np.maximum(q, 0), axis=-1) + np.minimum(q.max(-1), 0)
        level = np.zeros(len(P))
        for i in range(self.iters):
            s = 3.0 ** i
            fold = np.empty_like(P)
            a = P * (s / 2.0)
            a = a - np.floor(a)
            fold = np.abs(1.0 - np.abs(2 * a - 1.0) * 3.0)
            x, y, z = fold[:, 0], fold[:, 1], fold[:, 2]
            cross = np.minimum(np.minimum(np.maximum(x, y), np.maximum(y, z)), np.maximum(z, x))
            hole = (cross - 1.0) / (3.0 * s)
            level = np.where(hole > d, i + 1, level)
            d = np.maximum(d, hole)
        return d, level / max(self.iters, 1)


class QuaternionJulia(Field):
    """Julia set of q -> q^2 + c in the quaternions, cut at w = 0 (iq's form)."""

    bound = 1.4

    def __init__(self, c=(-0.2, 0.8, 0.0, 0.0), iters: int = 12, w: float = 0.0):
        self.c, self.iters, self.w = np.asarray(c, float), int(iters), float(w)

    def _de(self, P):
        n = len(P)
        a, b, cc = P[:, 0].copy(), P[:, 1].copy(), P[:, 2].copy()
        d = np.full(n, self.w)
        md2 = np.ones(n)
        mz2 = a * a + b * b + cc * cc + d * d
        trap = mz2.copy()
        act = mz2 < 16.0
        c = self.c
        for _ in range(self.iters):
            if not act.any():
                break
            nmd2 = md2 * mz2 * 4.0
            na = a * a - b * b - cc * cc - d * d + c[0]
            nb = 2 * a * b + c[1]
            nc = 2 * a * cc + c[2]
            nd = 2 * a * d + c[3]
            a, b, cc, d = (np.where(act, v, o) for v, o in ((na, a), (nb, b), (nc, cc), (nd, d)))
            md2 = np.where(act, nmd2, md2)
            mz2 = a * a + b * b + cc * cc + d * d
            trap = np.where(act, np.minimum(trap, mz2), trap)
            act &= mz2 < 16.0
        mz2 = np.maximum(mz2, 1e-12)
        return np.sqrt(mz2 / md2) * np.log(mz2) * 0.25, np.clip(np.sqrt(trap), 0, 1)


class Hopfbrot(Field):
    """The Mandelbulb power map in 4D with Hopf coordinates, cut to 3D.

    c = (x + shift, y, z cos t - slice sin t, z sin t + slice cos t);
    w <- |w|^n U(n a1, n a2, n phi) + c with
    U = (cos phi e^{i a1}, sin phi e^{i a2}).  ``tilt`` is in degrees.
    """

    bound = 2.0

    def __init__(self, power: float = 3.0, tilt: float = 90.0, slice: float = 0.0, iters: int = 10):
        self.power, self.tilt, self.slice, self.iters = float(power), float(tilt), float(slice), int(iters)
        self.shift = max(3.0 - self.power, 0.0) * -0.5
        self.escape = float(np.exp(np.log(1e8) / self.power))

    def _de(self, P):
        n = self.power
        t = np.radians(self.tilt)
        ct, st_ = np.cos(t), np.sin(t)
        x, y, z = P[:, 0], P[:, 1], P[:, 2]
        c = np.stack([x + self.shift, y, z * ct - self.slice * st_, z * st_ + self.slice * ct], -1)
        w = c.copy()
        r = np.linalg.norm(w, axis=-1)
        dr = np.ones(len(P))
        trap = np.full(len(P), 1e9)
        act = r < self.escape
        for i in range(self.iters + 4):
            if i >= self.iters:
                act &= r > 2.0
            if not act.any():
                break
            r1 = np.hypot(w[:, 0], w[:, 1])
            r2 = np.hypot(w[:, 2], w[:, 3])
            a1 = np.arctan2(w[:, 1], w[:, 0]) * n
            a2 = np.arctan2(w[:, 3], w[:, 2]) * n
            ph = np.arctan2(r2, r1) * n
            rs = np.maximum(r, 1e-12)
            rp = rs ** n
            cp, sp = np.cos(ph) * rp, np.sin(ph) * rp
            nw = np.stack([cp * np.cos(a1), cp * np.sin(a1), sp * np.cos(a2), sp * np.sin(a2)], -1) + c
            ndr = np.minimum(rp / rs * n * dr + 1.0, 1e30)
            w = np.where(act[:, None], nw, w)
            dr = np.where(act, ndr, dr)
            r = np.where(act, np.linalg.norm(w, axis=-1), r)
            trap = np.where(act & (r < 2.0), np.minimum(trap, np.hypot(w[:, 2], w[:, 3])), trap)
            act &= r < self.escape
        r = np.maximum(r, 1e-6)
        return r * np.log(r) * 0.5 / np.maximum(dr, 1e-12), np.clip(trap, 0, 1)
