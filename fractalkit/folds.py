"""Small fold / transform *steps* for building hybrids.

Each is an ``IterMap`` whose ``step`` is one operation and that has no
distance of its own: use it inside a ``Hybrid`` whose ``finisher`` is a map
that does (a bulb, a box, a KIFS ...).  Group several steps into a single
iteration with a nested schedule::

    Hybrid([BoxFold(1.0), SphereFold(0.5), Scale(0.6), Mandelbulb(2)],
           schedule=[[0, 1, 2, 3]], iters=12, finisher=3)

which is ray's ``bulbbox`` (a Mandelbox's folds, then the bulb's squaring).
The derivative ``dr`` is kept consistent by every step (sphere fold and
scale multiply it) so the finisher's estimate stays valid.
"""
from __future__ import annotations

import numpy as np

from .core import IterMap, State, euler_matrix


class _Step(IterMap):
    bound = 2.0

    def finish(self, st):
        raise TypeError(f"{type(self).__name__} has no distance of its own; "
                        "set Hybrid(finisher=...) to a map that does")


class BoxFold(_Step):
    def __init__(self, limit: float = 1.0):
        self.limit = float(limit)

    def step(self, st: State, i: int) -> None:
        st.z = np.clip(st.z, -self.limit, self.limit) * 2.0 - st.z


class SphereFold(_Step):
    """Inside ``min_r`` scale by 1/min_r^2; inside ``fixed_r`` invert."""

    def __init__(self, min_r: float = 0.5, fixed_r: float = 1.0):
        self.min2, self.fix2 = float(min_r) ** 2, float(fixed_r) ** 2

    def step(self, st: State, i: int) -> None:
        r2 = np.einsum("ij,ij->i", st.z, st.z)
        k = np.where(r2 < self.min2, self.fix2 / self.min2, np.where(r2 < self.fix2, self.fix2 / np.maximum(r2, 1e-12), 1.0))
        st.z = st.z * k[:, None]
        st.dr = st.dr * k


class Scale(_Step):
    def __init__(self, s: float = 2.0, add_c: bool = False):
        self.s, self.add_c = float(s), add_c

    def step(self, st: State, i: int) -> None:
        st.z = st.z * self.s + (st.c if self.add_c else 0.0)
        st.dr = st.dr * abs(self.s)


class AbsFold(_Step):
    def __init__(self, axes=(1, 1, 1)):
        self.m = np.asarray(axes, bool)

    def step(self, st: State, i: int) -> None:
        st.z = np.where(self.m, np.abs(st.z), st.z)


class Rotate(_Step):
    """Rotate the orbit by Euler angles (degrees): twists a hybrid."""

    def __init__(self, rx=0.0, ry=0.0, rz=0.0):
        self.R = euler_matrix(rx, ry, rz)

    def step(self, st: State, i: int) -> None:
        st.z = st.z @ self.R.T


class Offset(_Step):
    def __init__(self, v=(0, 0, 0)):
        self.v = np.asarray(v, float)

    def step(self, st: State, i: int) -> None:
        st.z = st.z + self.v
