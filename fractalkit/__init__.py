"""fractalkit: a modular numpy model of the fractals in ray, for exploring
and plotting them (and mixing them with other geometry) from Python."""
from .core import (Blend, Field, Hybrid, Intersect, IterMap, Repeat, Shell, SmoothUnion,
                   State, Subtract, Transform, Union, Warp, euler_matrix)
from .fractals import (Apollonian, Hopfbrot, KIFS, Mandelbox, Mandelbulb, Menger, Menger_kifs,
                       PseudoKleinian, QuaternionJulia, RiemannSphere, Sierpinski)
from .clasptree import ClaspTree, Ring
from .horned import HornedSphere
from .seamtree import SeamTree
from .trionda_horned import TriondaHorned
from .hattrick_horned import HatTrickHorned
from .trionda_ifs import TriondaIFS
from .polyballs import PolyBall
from .mixedtree import MixedSeamTree
from .hopftree import HopfTree, hopf_pair
from .folds import AbsFold, BoxFold, Offset, Rotate, Scale, SphereFold
from .geometry import (Box, FiberNetwork, HopfFibers, Implicit, Plane, Sphere, Torus,
                       clifford_torus, gyroid, hopf_map, inverse_stereographic, linking_matrix, linking_number, circle_moebius_invariants, curve_distance, ring_base, stereographic)
from .render import Camera, plot_slice, render, scatter3d, surface_points
from .scene import REGISTRY, from_spec, register
from .analysis import box_counting_dimension, distance_stats, occupancy

__all__ = [n for n in dir() if not n.startswith("_")]
