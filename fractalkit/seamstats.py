"""Numbers for seam networks and seam trees (``SeamTree`` and friends): what they are made of, and how to compare them.

Per ball (the root network, on the unit sphere): seam length, panel areas, seam-density spherical-harmonic
spectrum (selection rules of the symmetry group), chirality.  Per tree: networks and seam length per generation,
similarity and box-counting dimension, volume of the solid, linking of parent and child loops.
"""
from __future__ import annotations

import numpy as np
from scipy.spatial import cKDTree
from scipy.special import sph_harm_y
from scipy.spatial.transform import Rotation

from .seamtree import densify_arc


def seam_length(curves) -> float:
    """Total length (unit sphere) of seam polylines, great-arc segments.  Curves shared by two panels
    must be listed once (Trionda: the six arcs; for outlines of every panel the result counts seams twice)."""
    return float(sum(np.arccos(np.clip((c[:-1] * c[1:]).sum(1), -1, 1)).sum() for c in curves))


def spherical_polygon_area(poly: np.ndarray) -> float:
    """Area of a small spherical polygon (unit vectors), as a fan of triangles about its centre."""
    c = poly.mean(0); c /= np.linalg.norm(c)
    tot = 0.0
    for i in range(len(poly)):
        a, b = poly[i], poly[(i + 1) % len(poly)]
        tot += 2 * np.arctan2(abs(c @ np.cross(a, b)), 1 + c @ a + a @ b + b @ c)
    return float(tot)


def panel_areas(tree) -> np.ndarray:
    return np.array([spherical_polygon_area(densify_arc(p, 0.05, closed=True)[:-1]) for p in tree.panels])


def panel_perimeters(tree) -> np.ndarray:
    return np.array([seam_length([np.vstack([p, p[:1]])]) for p in tree.panels])


def seam_points(tree, step: float = 0.004) -> np.ndarray:
    """Dense sample of the root network's seams on the unit sphere."""
    return tree.tree.data


def harmonic_spectrum(points: np.ndarray, lmax: int = 24) -> np.ndarray:
    """Power per degree l of the (normalised) seam density: sum over m of |mean_p Y_lm(p)|^2.
    A symmetry group forbids the degrees it has no invariant for (Trionda, T: l = 1, 2, 5 vanish; Hat Trick, I:
    everything below l = 6, and 7, 8, 9, 11, ...)."""
    th = np.arccos(np.clip(points[:, 2], -1, 1)); ph = np.arctan2(points[:, 1], points[:, 0])
    out = np.zeros(lmax + 1)
    for l in range(lmax + 1):
        out[l] = sum(abs(sph_harm_y(l, m, th, ph).mean()) ** 2 for m in range(-l, l + 1))
    return out


def chirality(points: np.ndarray, starts: int = 40, sub: int = 1500, seed: int = 0) -> float:
    """Distance (degrees, mean nearest-point) from the seam network to its own mirror image, minimised over
    rotations (random starts + Nelder-Mead).  0 for an achiral network."""
    from scipy.optimize import minimize
    rng = np.random.default_rng(seed)
    P = points[rng.choice(len(points), min(sub, len(points)), replace=False)]
    kd = cKDTree(points)
    M = P * np.array([1, 1, -1])

    def cost(v):
        return kd.query(M @ Rotation.from_rotvec(v).as_matrix().T)[0].mean()
    best = min((minimize(cost, Rotation.random(random_state=rng.integers(1 << 31)).as_rotvec(), method="Nelder-Mead",
                         options=dict(xatol=1e-3, fatol=1e-6, maxiter=300)).fun for _ in range(starts)))
    return float(np.degrees(best))


def similarity_dimension(branching: float, shrink: float) -> float:
    return float(np.log(branching) / np.log(1 / shrink))


def generation_counts(tree, depth: int = 3):
    """(networks per generation, seam length per generation) of the tree's curves to ``depth``."""
    nets = tree.networks(depth)
    n = np.zeros(depth + 1, int); L = np.zeros(depth + 1)
    base = seam_length(tree.curves)
    for M, t, S, gen, _ in nets:
        n[gen] += 1; L[gen] += S * base
    return n, L


def boxcount_points(points: np.ndarray, sizes) -> np.ndarray:
    """Number of occupied cubes of each side length in ``sizes`` (points must be dense relative to the smallest)."""
    lo = points.min(0)
    return np.array([len(np.unique(np.floor((points - lo) / h).astype(np.int64), axis=0)) for h in sizes])


def box_dimension(points: np.ndarray, hmin: float, hmax: float, n: int = 12):
    """Box-counting dimension of a point sample: slope of log N vs log(1/h) over [hmin, hmax]."""
    h = np.geomspace(hmin, hmax, n)
    N = boxcount_points(points, h)
    slope = np.polyfit(np.log(1 / h), np.log(N), 1)[0]
    return float(slope), h, N


def tree_points(tree, depth: int, spacing: float) -> np.ndarray:
    """Dense world-frame sample of all seams of generations 0..depth, a point every ``spacing``."""
    out = []
    for gen, c in tree.seams(depth):
        seg = np.linalg.norm(np.diff(c, axis=0), axis=1)
        m = np.maximum(1, np.ceil(seg / spacing).astype(int))
        out.append(np.vstack([c[i] + (c[i + 1] - c[i]) * (np.arange(k) / k)[:, None] for i, k in enumerate(m)] + [c[-1:]]))
    return np.vstack(out)


def volume(field, n: int = 200_000, seed: int = 0) -> tuple[float, float]:
    """Monte Carlo volume of ``{d < 0}`` in the bounding cube and its standard error."""
    rng = np.random.default_rng(seed)
    b = float(field.bound)
    P = rng.uniform(-b, b, (n, 3))
    ins = np.concatenate([field.de(P[i:i + 20000])[0] < 0 for i in range(0, n, 20000)])
    p = ins.mean()
    return float(p * (2 * b) ** 3), float(np.sqrt(p * (1 - p) / n) * (2 * b) ** 3)


def clearance(tree, depth: int = 2, step: float = 0.01):
    """Smallest gap between the tubes of two networks that are neither parent and child nor the same
    (distance between their seam curves minus both tube radii; < 0: the tubes fuse).  Also returns the number of
    such pairs that fuse and the smallest gap between a network and its parent.  Pairs are tested by KD-tree."""
    nets = tree.networks(depth)
    nc = len(tree.curves)
    S = [c for _, c in tree.seams(depth)]
    pts = []
    for i in range(len(nets)):
        out = []
        for c in S[i * nc:(i + 1) * nc]:
            seg = np.linalg.norm(np.diff(c, axis=0), axis=1)
            m = np.maximum(1, np.ceil(seg / step).astype(int))
            out += [c[j] + (c[j + 1] - c[j]) * (np.arange(k) / k)[:, None] for j, k in enumerate(m)]
        pts.append(np.vstack(out))
    kd = [cKDTree(p) for p in pts]
    cen = np.array([n[1] for n in nets]); rad = np.array([n[2] * tree.rb for n in nets])
    best, fused, pc = np.inf, 0, np.inf
    for a in range(len(nets)):
        for b in range(a + 1, len(nets)):
            if np.linalg.norm(cen[a] - cen[b]) > rad[a] + rad[b]:
                continue                                       # bounding balls apart
            g = kd[a].query(pts[b])[0].min() - tree.tube * (nets[a][2] + nets[b][2])
            if nets[b][4] == a or nets[a][4] == b:
                pc = min(pc, g)
            else:
                best = min(best, g); fused += g < 0
    return float(best), int(fused), float(pc)
