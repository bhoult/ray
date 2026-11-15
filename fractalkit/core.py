"""Core protocol for fractalkit.

Everything is a ``Field``: an object that maps points ``P`` (``(..., 3)``
float array) to ``(d, trap)``, a distance estimate and an orbit trap in
0..1, exactly the contract of the Rust kernels in ``src/formulas`` of ray.

Two ways to build a field:

* ``IterMap`` -- an *iterated map* given as separate ``start / step /
  finish`` pieces.  Because the step is separable, steps of different maps
  can be interleaved (``Hybrid``), the way Mandelbulber hybrids work.
* Any ``Field`` subclass implementing ``_de(P)`` on an ``(N, 3)`` array --
  primitives, fibre networks, 4D slices, and so on.

Fields compose with operators (``|``, ``&``, ``-``, ``smooth_union``,
``blend``) and transforms (``scaled``, ``rotated``, ``translated``,
``repeated``, ``warped``, ``shell``).
"""
from __future__ import annotations

from dataclasses import dataclass, field as dc_field
from typing import Callable, Sequence

import numpy as np

# --------------------------------------------------------------------------
# Field base class
# --------------------------------------------------------------------------


class Field:
    """A signed (or unsigned) distance estimate with an orbit trap."""

    #: radius of a sphere (about the origin) that bounds the set; used by the
    #: renderers to pick a view and by the box counter to pick a domain.
    bound: float = 2.0

    # -- subclass hook ----------------------------------------------------
    def _de(self, P: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        raise NotImplementedError

    # -- public API -------------------------------------------------------
    def de(self, P) -> tuple[np.ndarray, np.ndarray]:
        """Distance estimate and trap at points ``P`` of shape ``(..., 3)``."""
        P = np.asarray(P, dtype=np.float64)
        shape = P.shape[:-1]
        d, t = self._de(P.reshape(-1, 3))
        return d.reshape(shape), t.reshape(shape)

    def __call__(self, P) -> np.ndarray:
        return self.de(P)[0]

    def gradient(self, P, eps: float = 1e-4) -> np.ndarray:
        """Central-difference gradient of the distance (a surface normal)."""
        P = np.asarray(P, dtype=np.float64)
        g = np.empty_like(P)
        for i in range(3):
            e = np.zeros(3)
            e[i] = eps
            g[..., i] = self(P + e) - self(P - e)
        n = np.linalg.norm(g, axis=-1, keepdims=True)
        return g / np.maximum(n, 1e-30)

    # -- composition operators -------------------------------------------
    def __or__(self, other: "Field") -> "Field":
        return Union(self, other)

    def __and__(self, other: "Field") -> "Field":
        return Intersect(self, other)

    def __sub__(self, other: "Field") -> "Field":
        return Subtract(self, other)

    def smooth_union(self, other: "Field", k: float = 0.1) -> "Field":
        return SmoothUnion(self, other, k)

    def blend(self, other: "Field", t: float = 0.5) -> "Field":
        """Linear blend of distance estimates (a morph, not a boolean)."""
        return Blend(self, other, t)

    # -- transforms -------------------------------------------------------
    def scaled(self, s: float) -> "Field":
        return Transform(self, scale=s)

    def translated(self, v: Sequence[float]) -> "Field":
        return Transform(self, translate=v)

    def rotated(self, rx: float = 0.0, ry: float = 0.0, rz: float = 0.0) -> "Field":
        """Rotate by Euler angles in degrees (applied x, then y, then z)."""
        return Transform(self, rotation=euler_matrix(rx, ry, rz))

    def repeated(self, period: Sequence[float] | float) -> "Field":
        return Repeat(self, period)

    def warped(self, fn: Callable[[np.ndarray], np.ndarray]) -> "Field":
        """Pre-warp the sample points. The estimate is no longer exact."""
        return Warp(self, fn)

    def shell(self, thickness: float) -> "Field":
        return Shell(self, thickness)


def euler_matrix(rx: float, ry: float, rz: float) -> np.ndarray:
    ax, ay, az = np.radians([rx, ry, rz])
    cx, sx, cy, sy, cz, sz = np.cos(ax), np.sin(ax), np.cos(ay), np.sin(ay), np.cos(az), np.sin(az)
    Rx = np.array([[1, 0, 0], [0, cx, -sx], [0, sx, cx]])
    Ry = np.array([[cy, 0, sy], [0, 1, 0], [-sy, 0, cy]])
    Rz = np.array([[cz, -sz, 0], [sz, cz, 0], [0, 0, 1]])
    return Rz @ Ry @ Rx


# --------------------------------------------------------------------------
# Boolean / blend operators
# --------------------------------------------------------------------------


class _Binary(Field):
    def __init__(self, a: Field, b: Field):
        self.a, self.b = a, b
        self.bound = max(a.bound, b.bound)


class Union(_Binary):
    def _de(self, P):
        da, ta = self.a._de(P)
        db, tb = self.b._de(P)
        m = da <= db
        return np.where(m, da, db), np.where(m, ta, tb)


class Intersect(_Binary):
    def __init__(self, a, b):
        super().__init__(a, b)
        self.bound = min(a.bound, b.bound)

    def _de(self, P):
        da, ta = self.a._de(P)
        db, tb = self.b._de(P)
        m = da >= db
        return np.where(m, da, db), np.where(m, ta, tb)


class Subtract(_Binary):
    """``a - b``: a with b carved out."""

    def __init__(self, a, b):
        super().__init__(a, b)
        self.bound = a.bound

    def _de(self, P):
        da, ta = self.a._de(P)
        db, _ = self.b._de(P)
        return np.maximum(da, -db), ta


class SmoothUnion(_Binary):
    """Polynomial smooth minimum (iq).  ``k`` is the blend width."""

    def __init__(self, a, b, k):
        super().__init__(a, b)
        self.k = max(float(k), 1e-9)

    def _de(self, P):
        da, ta = self.a._de(P)
        db, tb = self.b._de(P)
        h = np.clip(0.5 + 0.5 * (db - da) / self.k, 0.0, 1.0)
        d = db * (1 - h) + da * h - self.k * h * (1 - h)
        return d, tb * (1 - h) + ta * h


class Blend(_Binary):
    def __init__(self, a, b, t):
        super().__init__(a, b)
        self.t = float(t)

    def _de(self, P):
        da, ta = self.a._de(P)
        db, tb = self.b._de(P)
        return (1 - self.t) * da + self.t * db, (1 - self.t) * ta + self.t * tb


# --------------------------------------------------------------------------
# Unary transforms
# --------------------------------------------------------------------------


class Transform(Field):
    """Uniform scale, rotation, translation (applied in that order to the
    *object*: scale about the origin, rotate, then move)."""

    def __init__(self, f: Field, scale=1.0, rotation=None, translate=(0, 0, 0)):
        self.f, self.s = f, float(scale)
        self.R = np.eye(3) if rotation is None else np.asarray(rotation, float)
        self.t = np.asarray(translate, float)
        self.bound = f.bound * self.s + float(np.linalg.norm(self.t))

    def _de(self, P):
        q = ((P - self.t) @ self.R) / self.s  # inverse of rotate (R orthogonal)
        d, t = self.f._de(q)
        return d * self.s, t


class Repeat(Field):
    def __init__(self, f: Field, period):
        self.f = f
        self.p = np.broadcast_to(np.asarray(period, float), (3,)).copy()
        self.bound = np.inf

    def _de(self, P):
        q = P - self.p * np.round(P / self.p)
        return self.f._de(q)


class Warp(Field):
    def __init__(self, f: Field, fn):
        self.f, self.fn = f, fn
        self.bound = f.bound * 1.5

    def _de(self, P):
        return self.f._de(self.fn(P))


class Shell(Field):
    def __init__(self, f: Field, thickness: float):
        self.f, self.t = f, float(thickness)
        self.bound = f.bound + thickness

    def _de(self, P):
        d, t = self.f._de(P)
        return np.abs(d) - self.t, t


# --------------------------------------------------------------------------
# Iterated maps and hybrids
# --------------------------------------------------------------------------


@dataclass
class State:
    """Running state of an escape-time / folding iteration (vectorised)."""

    c: np.ndarray                     # (N,3) the sample point (the "+c")
    z: np.ndarray                     # (N,3) current orbit point
    dr: np.ndarray                    # (N,)  running derivative / scale
    trap: np.ndarray                  # (N,)  orbit trap accumulator
    active: np.ndarray                # (N,)  bool, still iterating
    extra: dict = dc_field(default_factory=dict)  # per-map scratch (e.g. fold count)

    @property
    def r(self) -> np.ndarray:
        return np.linalg.norm(self.z, axis=-1)

    def commit(self, mask, z=None, dr=None):
        """Write new z / dr only where ``mask`` is true."""
        if z is not None:
            self.z = np.where(mask[:, None], z, self.z)
        if dr is not None:
            self.dr = np.where(mask, dr, self.dr)


class IterMap(Field):
    """An iterated 3D map with separable start / step / finish.

    Subclasses implement ``step`` and ``finish`` (and optionally ``start``).
    ``bail`` > 0 deactivates points whose orbit leaves that radius.
    """

    bail: float = 0.0
    iters: int = 12

    def start(self, P: np.ndarray) -> State:
        n = len(P)
        return State(c=P, z=P.copy(), dr=np.ones(n), trap=np.full(n, 1e9),
                     active=np.ones(n, bool) if self.bail <= 0 else (np.linalg.norm(P, axis=-1) < self.bail))

    def step(self, st: State, i: int) -> None:  # pragma: no cover - abstract
        raise NotImplementedError

    def finish(self, st: State) -> tuple[np.ndarray, np.ndarray]:  # pragma: no cover
        raise NotImplementedError

    def run(self, P: np.ndarray, iters: int | None = None) -> State:
        st = self.start(P)
        for i in range(self.iters if iters is None else iters):
            if not st.active.any():
                break
            self.step(st, i)
            if self.bail > 0:
                st.active &= st.r < self.bail
        return st

    def _de(self, P):
        return self.finish(self.run(P))

    def with_iters(self, n: int):
        import copy
        m = copy.copy(self)
        m.iters = int(n)
        return m


class Hybrid(Field):
    """Interleave the steps of several ``IterMap`` s.

    ``schedule`` is a repeating pattern of indices into ``maps``, one entry
    per iteration; ``[0, 0, 1]`` does two steps of ``maps[0]`` then one of
    ``maps[1]``.  An entry may itself be a list, run in order *within* one
    iteration: ``[[0, 1]]`` does map 0's step then map 1's every iteration.
    The distance is finished by ``maps[finisher]`` (a different finisher
    gives a different, still-valid-looking estimate; use a small step size
    when marching a hybrid).  All maps share the ``c`` (the sample point),
    so a bulb step adds the same ``+c`` a box step does.  The bound defaults
    to the largest of the maps'; pass ``bound=`` to tighten the view.
    """

    def __init__(self, maps: Sequence[IterMap], schedule: Sequence | None = None,
                 iters: int = 14, finisher: int = 0, bail: float | None = None,
                 bound: float | None = None):
        self.maps = list(maps)
        self.schedule = list(schedule) if schedule is not None else list(range(len(maps)))
        self.iters = iters
        self.finisher = finisher
        # escape only if some map in the mix escapes
        bails = [m.bail for m in self.maps if m.bail > 0]
        self.bail = bail if bail is not None else (max(bails) if bails else 0.0)
        self.bound = bound if bound is not None else max(m.bound for m in self.maps)

    def _de(self, P):
        st = self.maps[self.finisher].start(P)   # the finisher knows which scratch it needs
        if self.bail > 0:
            st.active = np.linalg.norm(P, axis=-1) < self.bail
        for i in range(self.iters):
            if not st.active.any():
                break
            entry = self.schedule[i % len(self.schedule)]
            for j in ([entry] if isinstance(entry, (int, np.integer)) else entry):
                self.maps[j].step(st, i)
            if self.bail > 0:
                st.active &= st.r < self.bail
        return self.maps[self.finisher].finish(st)
