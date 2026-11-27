"""A *Hat Trick* horned sphere: the seam-tree recursion on the Hat Trick ball.

The building block is the Hat Trick ball of ``hexasphere`` (72 panels: 60
aperiodic hats and 12 pentagons, rotation group I of order 60), baked into
``data/hattrick.npz`` by ``tools/bake_hattrick.py``.  Copies hang on the 12
pentagon centres (the 5-fold axes); the root has 12 children and every later
network 11.  Colour: each hat takes the colour of the pentagon whose patch it
belongs to, pentagons are pale.
"""
from __future__ import annotations

import colorsys
import pathlib

import numpy as np

from .seamtree import SeamTree, group_from_sites

_DATA = pathlib.Path(__file__).parent / "data" / "hattrick.npz"


def load_hattrick():
    z = np.load(_DATA)
    ends = np.cumsum(z["lens"])
    panels = np.split(z["pts"], ends[:-1])
    return panels, z["cls"], z["centres"]


class HatTrickHorned(SeamTree):
    n_classes = 13

    def __init__(self, shrink: float = 0.3, sink: float = 0.4, turn: float = 10.0,
                 tube: float = 0.025, body: float = 0.0, blend: float = 0.5, iters: int = 3):
        panels, cls, centres = load_hattrick()
        self.setup(panels, panels, cls, centres, group_from_sites(centres, 5, 60), 5,
                   shrink, sink, turn, tube, body, blend, iters)

    def class_color(self, gen: int, cls: int):
        if cls == 0:
            return colorsys.hsv_to_rgb(0.1, 0.08, max(0.5, 0.97 - 0.13 * gen))
        h = ((cls - 1) / 12 + gen * 0.11) % 1.0
        return colorsys.hsv_to_rgb(h, 0.6, max(0.45, 0.95 - 0.1 * gen))


def _site_triples():
    """Child-site pairs (u, v) of adjacent pentagons, with the parent pentagon p, by how p sits relative to them.

    Kinds: ``A`` p adjacent to both u and v (a triangle of the icosahedron), ``B`` adjacent to one and 117 degrees
    from the other, ``C`` 117 degrees from both, ``D`` 117 degrees from one and antipodal to the other."""
    c = load_hattrick()[2]
    c = c / np.linalg.norm(c, axis=1, keepdims=True)
    G = np.round(np.degrees(np.arccos(np.clip(c @ c.T, -1, 1))), 1)
    out = {}
    for u in range(12):
        for v in range(12):
            if u < v and abs(G[u, v] - 63.4) < 0.2:
                for p in (0,):                       # the baked pentagon 0 carries the parent
                    if p in (u, v):
                        continue
                    key = tuple(sorted((G[p, u], G[p, v])))
                    kind = {(63.4, 63.4): "A", (63.4, 116.6): "B", (116.6, 116.6): "C", (116.6, 180.0): "D"}.get(key)
                    if kind and kind not in out:
                        out[kind] = (c[p], c[u], c[v])
    return out


class PairHatTrick(SeamTree):
    """Two children per ball, on a pair of *adjacent* pentagons (63 degrees apart), so siblings are close.

    ``kind`` A to D says where the parent's pentagon sits relative to the pair (see ``_site_triples``).  As in
    ``AntipodalHatTrick`` the sites are ``[p, u, v]``, the root leaves ``p`` free, and every ball has 2 children."""
    n_classes = 13
    root_site0 = False
    class_color = HatTrickHorned.class_color

    def __init__(self, kind: str = "A", shrink: float = 0.6, sink: float = 0.4, turn: float = 10.0,
                 tube: float = 0.025, body: float = 0.0, blend: float = 0.5, iters: int = 3,
                 lateral: float = 0.0, lean: float = 0.0):
        panels, cls, centres = load_hattrick()
        I = group_from_sites(centres, 5, 60)
        p, u, v = _site_triples()[kind]
        sites = np.array([p, u, v])
        G = [g for g in I if np.min(np.linalg.norm(sites - g @ p, axis=1)) < 1e-6]
        self.kind = kind
        self.setup(panels, panels, cls, sites, G, 5, shrink, sink, turn, tube, body, blend, iters,
                   lateral=lateral, lean=lean)

    @classmethod
    def clasp(cls, **kw):
        """Sibling clasp: the two children of every ball link each other, to depth 2 with clear tubes (tools/pair_thin.py).
        Margins are thin: sibling gap 0.010, depth-2 clearance 0.005 at tube 0.004."""
        return cls(**{**dict(kind="D", shrink=0.7, sink=0.4, turn=0.0, tube=0.004), **kw})


class AntipodalHatTrick(SeamTree):
    """Two children per ball, on an antipodal pair of pentagons: the horned sphere's two ends on the Hat Trick ball.

    Sites are ``[p, b, -b]``: ``p`` is the pentagon that carries the parent (never used by a child) and ``b`` is a
    neighbour of ``p``, so ``{b, -b}`` is one of the 5 antipodal pairs that avoid ``p`` (all equivalent under the
    turns about ``p``).  The group is the part of I that sends ``p`` into ``{p, b, -b}``.  The root has no child on
    ``p`` either, so every ball has exactly 2 children and the similarity dimension is log 2 / log(1/shrink)."""
    n_classes = 13
    root_site0 = False
    class_color = HatTrickHorned.class_color

    def __init__(self, shrink: float = 0.45, sink: float = 0.4, turn: float = 10.0,
                 tube: float = 0.025, body: float = 0.0, blend: float = 0.5, iters: int = 3,
                 lateral: float = 0.0, lean: float = 0.0):
        panels, cls, centres = load_hattrick()
        I = group_from_sites(centres, 5, 60)
        p = centres[0]
        b = centres[1 + int(np.argmax(centres[1:] @ p))]
        sites = np.array([p, b, -b])
        G = [g for g in I if np.min(np.linalg.norm(sites - g @ p, axis=1)) < 1e-6]
        self.setup(panels, panels, cls, sites, G, 5, shrink, sink, turn, tube, body, blend, iters,
                   lateral=lateral, lean=lean)
