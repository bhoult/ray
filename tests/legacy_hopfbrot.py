"""The standalone Hopfbrot Field kernel from before it became an IterMap.

Kept verbatim (renamed ``HopfbrotField``) so the tests can check that the
``IterMap`` version in ``fractalkit.fractals`` gives identical results.
"""
import numpy as np
from fractalkit.core import Field


class HopfbrotField(Field):
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
