"""Plotting and rendering: 2D slices, a small numpy sphere tracer, and a
surface point cloud.  Everything takes any ``Field``."""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from .core import Field

_AXES = {"x": (1, 2, 0), "y": (0, 2, 1), "z": (0, 1, 2)}  # (u, v, normal) axis indices


def slice_grid(extent: float = 2.0, res: int = 400, axis: str = "z", offset: float = 0.0,
               center=(0.0, 0.0)):
    """Points ``(res, res, 3)`` on the plane ``axis = offset`` plus the two in-plane axes."""
    iu, iv, inorm = _AXES[axis]
    u = np.linspace(center[0] - extent, center[0] + extent, res)
    v = np.linspace(center[1] - extent, center[1] + extent, res)
    U, V = np.meshgrid(u, v)
    P = np.zeros((res, res, 3))
    P[..., iu], P[..., iv], P[..., inorm] = U, V, offset
    return u, v, P


def plot_slice(field: Field, axis: str = "z", offset: float = 0.0, extent: float = 2.0,
               res: int = 400, mode: str = "distance", ax=None, cmap: str | None = None,
               title: str | None = None, center=(0.0, 0.0)):
    """Plot a plane through the field.

    mode: ``distance`` (signed log-distance, zero set outlined), ``trap``
    (orbit trap colouring) or ``mask`` (inside/near the set).
    """
    import matplotlib.pyplot as plt

    u, v, P = slice_grid(extent, res, axis, offset, center)
    d, trap = field.de(P)
    if ax is None:
        _, ax = plt.subplots(figsize=(5, 5))
    ext = [u[0], u[-1], v[0], v[-1]]
    px = 2 * extent / res
    if mode == "trap":
        ax.imshow(np.where(d < 4 * px, trap, np.nan), origin="lower", extent=ext, cmap=cmap or "magma")
    elif mode == "mask":
        ax.imshow(d < px, origin="lower", extent=ext, cmap=cmap or "gray_r")
    else:
        sd = np.sign(d) * np.log10(1 + np.abs(d) / px)
        m = np.abs(sd).max() or 1.0
        ax.imshow(sd, origin="lower", extent=ext, cmap=cmap or "RdBu_r", vmin=-m, vmax=m)
        ax.contour(u, v, d, levels=[px], colors="k", linewidths=0.6)
    iu, iv, inorm = _AXES[axis]
    ax.set_xlabel("xyz"[iu]); ax.set_ylabel("xyz"[iv])
    ax.set_title(title or f"{type(field).__name__}  {axis}={offset:g}", fontsize=9)
    ax.set_aspect("equal")
    return ax


# --------------------------------------------------------------------------
# Sphere tracer
# --------------------------------------------------------------------------


@dataclass
class Camera:
    azimuth: float = 35.0      # degrees about +z
    elevation: float = 25.0    # degrees above the xy plane
    distance: float = 5.0
    fov: float = 40.0          # vertical field of view, degrees; 0 = orthographic
    ortho_height: float = 4.0  # world height seen when fov == 0
    target: tuple = (0.0, 0.0, 0.0)

    def rays(self, w: int, h: int):
        az, el = np.radians(self.azimuth), np.radians(self.elevation)
        tgt = np.asarray(self.target, float)
        eye = tgt + self.distance * np.array([np.cos(el) * np.cos(az), np.cos(el) * np.sin(az), np.sin(el)])
        fwd = tgt - eye
        fwd /= np.linalg.norm(fwd)
        right = np.cross(fwd, [0, 0, 1.0]); right /= np.linalg.norm(right)
        up = np.cross(right, fwd)
        xs = (np.arange(w) + 0.5) / w * 2 - 1
        ys = 1 - (np.arange(h) + 0.5) / h * 2
        X, Y = np.meshgrid(xs * (w / h), ys)
        if self.fov > 0:
            f = np.tan(np.radians(self.fov) / 2)
            D = fwd + (X[..., None] * right + Y[..., None] * up) * f
            D /= np.linalg.norm(D, axis=-1, keepdims=True)
            O = np.broadcast_to(eye, D.shape).copy()
        else:
            s = self.ortho_height / 2
            O = eye + (X[..., None] * right + Y[..., None] * up) * s
            D = np.broadcast_to(fwd, O.shape).copy()
        return O.reshape(-1, 3), D.reshape(-1, 3)


def render(field: Field, size=(320, 240), camera: Camera | None = None, max_steps: int = 200,
           step: float = 0.9, eps: float = 1.0e-3, palette: str = "magma", trap_weight: float = 0.65,
           light=(0.5, -0.4, 0.8), background=(0.97, 0.97, 0.975), tmax: float = 20.0) -> np.ndarray:
    """Sphere-trace ``field`` and return an ``(h, w, 3)`` float image in 0..1.

    ``eps`` is a *cone* tolerance (hit when ``d < eps * t``), so distant
    pixels do not need the fine detail.  Colour is the orbit trap through
    ``palette`` with Lambert shading and a cheap step-count ambient occlusion.
    """
    import matplotlib

    w, h = size
    cam = camera or Camera(distance=max(3.0, 2.2 * min(field.bound, 6.0)))
    O, D = cam.rays(w, h)
    n = len(O)
    t = np.zeros(n)
    # skip empty space: start on the bounding sphere around the origin
    if np.isfinite(field.bound):
        b = float(field.bound) * 1.02
        oc = O
        bq = (oc * D).sum(-1)
        cq = (oc * oc).sum(-1) - b * b
        disc = bq * bq - cq
        has = disc > 0
        t = np.where(has, np.maximum(-bq - np.sqrt(np.maximum(disc, 0)), 0), tmax + 1)
    alive = t < tmax
    hit = np.zeros(n, bool)
    steps = np.zeros(n)
    for it in range(max_steps):
        idx = np.nonzero(alive)[0]
        if len(idx) == 0:
            break
        d, _ = field.de(O[idx] + D[idx] * t[idx, None])
        hit_now = d < eps * np.maximum(t[idx], 0.5)
        hit[idx[hit_now]] = True
        steps[idx] = it
        t[idx] += np.where(hit_now, 0.0, np.maximum(d, 0) * step)
        alive[idx[hit_now]] = False
        alive[t > tmax] = False
    img = np.tile(np.asarray(background, float), (n, 1))
    # vertical sky gradient
    img *= (1.0 - 0.07 * (np.linspace(0, 1, h))[:, None].repeat(w, 1).reshape(-1, 1))   # a little greyer towards the bottom
    hi = np.nonzero(hit)[0]
    if len(hi):
        Ph = O[hi] + D[hi] * t[hi, None]
        _, trap = field.de(Ph)
        N = field.gradient(Ph, eps=max(eps, 1e-4) * 2)
        L = np.asarray(light, float); L /= np.linalg.norm(L)
        diff = np.clip(N @ L, 0, 1)
        rim = np.clip(1 + (N * D[hi]).sum(-1), 0, 1) ** 2
        ao = np.clip(1 - steps[hi] / (0.6 * max_steps), 0.15, 1)
        cmap = palette if callable(palette) else matplotlib.colormaps[palette]
        base = cmap(np.clip(trap, 0, 1) * trap_weight + 0.15)[:, :3]
        col = base * (0.25 + 0.85 * diff[:, None]) * ao[:, None] + 0.15 * rim[:, None]
        img[hi] = col
    return np.clip(img, 0, 1).reshape(h, w, 3)


# --------------------------------------------------------------------------
# Point cloud
# --------------------------------------------------------------------------


def surface_points(field: Field, extent: float | None = None, res: int = 96, thickness: float = 1.0,
                   chunk: int = 400_000):
    """Grid points whose distance estimate is within ``thickness`` cells of
    the surface.  Returns ``(points, trap)``."""
    e = extent or min(field.bound, 6.0)
    ax = np.linspace(-e, e, res)
    h = ax[1] - ax[0]
    G = np.stack(np.meshgrid(ax, ax, ax, indexing="ij"), -1).reshape(-1, 3)
    pts, traps = [], []
    for s in range(0, len(G), chunk):
        g = G[s:s + chunk]
        d, t = field.de(g)
        m = np.abs(d) < thickness * h * 0.87
        pts.append(g[m]); traps.append(t[m])
    return np.concatenate(pts), np.concatenate(traps)


def scatter3d(field: Field, ax=None, res: int = 96, extent: float | None = None, s: float = 1.0,
              cmap: str = "viridis", elev: float = 22, azim: float = 35, max_points: int = 120_000):
    import matplotlib.pyplot as plt

    p, t = surface_points(field, extent, res)
    if len(p) > max_points:
        sel = np.random.default_rng(0).choice(len(p), max_points, replace=False)
        p, t = p[sel], t[sel]
    if ax is None:
        ax = plt.figure(figsize=(5, 5)).add_subplot(projection="3d")
    ax.scatter(p[:, 0], p[:, 1], p[:, 2], c=t, s=s, cmap=cmap, linewidths=0, depthshade=True)
    e = extent or min(field.bound, 6.0)
    ax.set_xlim(-e, e); ax.set_ylim(-e, e); ax.set_zlim(-e, e)
    ax.set_box_aspect((1, 1, 1)); ax.view_init(elev, azim)
    ax.set_axis_off()
    return ax
