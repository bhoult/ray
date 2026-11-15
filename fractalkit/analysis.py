"""Numerical analysis of a field with numpy/scipy: occupancy grids, box
counting dimension, distance statistics."""
from __future__ import annotations

import numpy as np
from scipy import stats

from .core import Field


def occupancy(field: Field, extent: float, res: int, threshold: float = 0.0, chunk: int = 500_000) -> np.ndarray:
    """Boolean ``(res,res,res)`` grid of cells whose centre has ``d < threshold * cell``.

    ``threshold = 0`` is "centre inside the solid" (right for signed fields
    such as the Menger sponge); larger values thicken the set, which a thin
    surface needs but which biases the dimension upward at coarse scales.
    """
    ax = (np.arange(res) + 0.5) / res * 2 * extent - extent
    h = 2 * extent / res
    X, Y, Z = np.meshgrid(ax, ax, ax, indexing="ij")
    G = np.stack([X, Y, Z], -1).reshape(-1, 3)
    out = np.empty(len(G), bool)
    for s in range(0, len(G), chunk):
        d, _ = field.de(G[s:s + chunk])
        out[s:s + chunk] = d < h * threshold
    return out.reshape(res, res, res)


def box_counting_dimension(field: Field, extent: float = 1.0, base: int = 2, levels: int = 7,
                           threshold: float = 0.0, min_cells: int | None = None,
                           max_cells: int | None = None):
    """Box-counting dimension of the set ``{d < 0}`` (or of a thickened set).

    The grid has ``base**levels`` cells per side and is OR-pooled by
    ``base`` at each coarser level.  **Align ``base`` with the fractal**:
    3 for the Menger sponge, 2 for the Sierpinski tetrahedron; then
    ``threshold = 0`` recovers the exact dimension (2.7268 / 2.0000).  A
    misaligned base or a positive threshold gives a biased (high) estimate.

    Returns ``(dimension, counts)``, counts mapping cells-per-side -> boxes.
    The fit uses scales ``min_cells <= n <= max_cells`` cells per side
    (default ``base**2`` up to the finest).  Set ``max_cells`` to where the
    fractal's own structure stops: at finer scales a solid of ``iters``
    levels looks 3-dimensional.
    """
    res = base ** levels
    occ = occupancy(field, extent, res, threshold)
    counts = {}
    for j in range(levels, 0, -1):
        n = base ** j
        k = res // n
        counts[n] = int(occ.reshape(n, k, n, k, n, k).any(axis=(1, 3, 5)).sum())
    lo, hi = (min_cells or base ** 2), (max_cells or res)
    ns = np.array(sorted(n for n in counts if lo <= n <= hi))
    N = np.array([counts[n] for n in ns], float)
    fit = stats.linregress(np.log(ns), np.log(N))
    return float(fit.slope), counts


def distance_stats(field: Field, extent: float = 1.5, res: int = 64) -> dict:
    """Quick summary of d over a box: min/max, fraction negative, fraction near the set."""
    ax = np.linspace(-extent, extent, res)
    G = np.stack(np.meshgrid(ax, ax, ax, indexing="ij"), -1)
    d = field(G)
    h = ax[1] - ax[0]
    return dict(min=float(d.min()), max=float(d.max()), inside=float((d < 0).mean()),
                near=float((np.abs(d) < h).mean()))
