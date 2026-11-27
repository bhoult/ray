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
