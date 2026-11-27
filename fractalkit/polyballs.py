"""Seam trees on the straight (achiral) polyhedral balls: tetrahedron, cube, icosahedron.

The seams are the edges of the polyhedron as great arcs, the panels its faces, so the ball has the full
Coxeter reflection group (tetra: T_d = A3, order 24; cube: O_h = B3, order 48; icosa: I_h = H3, order 120).
``full=True`` generates the children with that group, mirrors included, so every child has a mirror-image
twin and the tree is achiral; ``full=False`` uses only its rotation subgroup (T, O, I) as for the chiral balls.

``site`` chooses which axes carry children: ``"vertex"``, ``"face"`` (face centres) or ``"edge"`` (edge midpoints,
2-fold axes).  ``lateral`` and ``lean`` shift each child sideways as in ``SeamTree``; if the shift lies in a mirror
plane (``lean`` = 0 or 60) the mirrors fix the child and the cluster at a site has 3 members instead of 6.
"""
from __future__ import annotations

import itertools

import numpy as np
from scipy.spatial import ConvexHull

from .seamtree import SeamTree, rot, closure, reflection

PHI = (1 + 5 ** 0.5) / 2


def _vertices(kind: str) -> np.ndarray:
    if kind == "tetra":
        V = np.array([[1, 1, 1], [1, -1, -1], [-1, 1, -1], [-1, -1, 1]], float)
    elif kind == "cube":
        V = np.array(list(itertools.product((-1, 1), repeat=3)), float)
    elif kind == "icosa":
        V = np.array([(0, s1, s2 * PHI) for s1 in (-1, 1) for s2 in (-1, 1)]
                     + [(s1, s2 * PHI, 0) for s1 in (-1, 1) for s2 in (-1, 1)]
                     + [(s2 * PHI, 0, s1) for s1 in (-1, 1) for s2 in (-1, 1)], float)
    else:
        raise ValueError(kind)
    return V / np.linalg.norm(V, axis=1, keepdims=True)


def faces_of(V: np.ndarray):
    """Faces of the convex hull as vertex-index loops (coplanar triangles merged), in cyclic order."""
    h = ConvexHull(V)
    groups: dict = {}
    for simp, eq in zip(h.simplices, h.equations):
        key = tuple(np.round(eq[:3], 5))
        groups.setdefault(key, set()).update(simp.tolist())
    faces = []
    for key, idx in groups.items():
        idx = sorted(idx)
        n = np.array(key); c = V[idx].mean(0)
        e1 = np.cross(n, [0.3, 0.5, 0.8]); e1 /= np.linalg.norm(e1); e2 = np.cross(n, e1)
        ang = np.arctan2((V[idx] - c) @ e2, (V[idx] - c) @ e1)
        faces.append([idx[i] for i in np.argsort(ang)])
    return faces


def rotation_group(kind: str):
    if kind == "tetra":
        V = _vertices("tetra")
        return closure([rot(V[0], 2 * np.pi / 3), rot([0, 0, 1], np.pi)], 12)
    if kind == "cube":
        return closure([rot([0, 0, 1], np.pi / 2), rot([1, 1, 1], 2 * np.pi / 3)], 24)
    V = _vertices("icosa")
    nb = [j for j in range(1, 12) if V[0] @ V[j] > 0.4]
    fc = V[0] + V[nb[0]] + V[next(j for j in nb[1:] if V[nb[0]] @ V[j] > 0.4)]
    return closure([rot(V[0], 2 * np.pi / 5), rot(fc, 2 * np.pi / 3)], 60)


def full_group(kind: str):
    R = rotation_group(kind)
    if kind == "tetra":
        return closure(R + [reflection([1, -1, 0])], 24)
    return closure(R + [-np.eye(3)], 2 * len(R))


class PolyBall(SeamTree):
    def __init__(self, kind: str = "tetra", site: str = "vertex", full: bool = False, shrink: float = 0.4,
                 sink: float = 0.4, turn: float = 0.0, tube: float = 0.05, body: float = 0.0, blend: float = 0.5,
                 iters: int = 3, lateral: float = 0.0, lean: float = 0.0):
        self.kind, self.site_kind, self.full = kind, site, full
        V = _vertices(kind)
        F = faces_of(V)
        edges = sorted({tuple(sorted((f[i], f[(i + 1) % len(f)]))) for f in F for i in range(len(f))})
        curves = [np.array([V[a], V[b]]) for a, b in edges]
        panels = [V[f] for f in F]
        if site == "vertex":
            sites = V
        elif site == "face":
            sites = np.array([V[f].mean(0) for f in F])
        else:
            sites = np.array([V[a] + V[b] for a, b in edges])
        sites = sites / np.linalg.norm(sites, axis=1, keepdims=True)
        G = full_group(kind) if full else rotation_group(kind)
        self.n_classes = len(F)
        self.setup(curves, panels, np.arange(len(F)), sites, G, 0, shrink, sink, turn, tube, body, blend, iters,
                   lateral=lateral, lean=lean)
