"""Build fields from plain dict / JSON specs, in the spirit of ray's scene
files (``{"type": "mandelbulb", "scale": 0.6}``).

    spec = {"op": "smooth_union", "k": 0.15, "args": [
              {"type": "menger", "iters": 4},
              {"type": "hopf_fibers", "base": 30, "scale": 0.7}]}
    field = from_spec(spec)

Leaf keys: ``type`` + constructor params, plus the transforms ``scale``,
``rotation`` (degrees xyz), ``translate``.  Operator keys: ``op`` in
{union, intersect, subtract, smooth_union, blend, hybrid} and ``args``
(``hybrid`` takes ``schedule``, ``iters``, ``finisher``).
"""
from __future__ import annotations

import json
from functools import reduce

from . import fractals as F
from . import geometry as G
from .horned import HornedSphere
from .trionda_horned import TriondaHorned
from .hattrick_horned import HatTrickHorned
from .hopftree import HopfTree
from .core import Field, Hybrid

REGISTRY = {
    "mandelbulb": F.Mandelbulb, "juliabulb": lambda **k: F.Mandelbulb(julia=k.pop("c", (0.2, -0.64, 0.2)), **k),
    "mandelbox": F.Mandelbox, "kifs": F.KIFS, "menger_kifs": lambda iters=8, **k: F.Menger_kifs(iters),
    "sierpinski": F.Sierpinski, "apollonian": F.Apollonian, "pseudo_kleinian": F.PseudoKleinian,
    "menger": F.Menger, "riemann_sphere": F.RiemannSphere, "horned_sphere": HornedSphere, "trionda_horned": TriondaHorned, "hattrick_horned": HatTrickHorned, "hopf_tree": HopfTree, "quaternion_julia": F.QuaternionJulia, "hopfbrot": F.Hopfbrot,
    "sphere": G.Sphere, "box": G.Box, "torus": G.Torus, "plane": G.Plane,
    "hopf_fibers": G.HopfFibers, "torus_knot": G.FiberNetwork.torus_knot,
    "lattice": G.FiberNetwork.lattice, "gyroid": G.gyroid, "clifford_torus": G.clifford_torus,
}


def register(name: str, factory) -> None:
    """Add your own field type to the spec language."""
    REGISTRY[name] = factory


def _transform(f: Field, spec: dict) -> Field:
    if "scale" in spec:
        f = f.scaled(spec["scale"])
    if "rotation" in spec:
        f = f.rotated(*spec["rotation"])
    if "translate" in spec:
        f = f.translated(spec["translate"])
    return f


def from_spec(spec) -> Field:
    if isinstance(spec, str):
        spec = json.loads(spec)
    spec = dict(spec)
    if "op" in spec:
        op = spec.pop("op")
        args = [from_spec(a) for a in spec.pop("args")]
        if op == "union":
            f = reduce(lambda a, b: a | b, args)
        elif op == "intersect":
            f = reduce(lambda a, b: a & b, args)
        elif op == "subtract":
            f = reduce(lambda a, b: a - b, args)
        elif op == "smooth_union":
            k = spec.pop("k", 0.1)
            f = reduce(lambda a, b: a.smooth_union(b, k), args)
        elif op == "blend":
            t = spec.pop("t", 0.5)
            f = args[0].blend(args[1], t)
        elif op == "hybrid":
            f = Hybrid(args, spec.pop("schedule", None), spec.pop("iters", 14), spec.pop("finisher", 0))
        else:
            raise ValueError(f"unknown op {op!r}")
        return _transform(f, spec)
    kind = spec.pop("type")
    tf = {k: spec.pop(k) for k in ("scale", "rotation", "translate") if k in spec}
    f = REGISTRY[kind](**spec)
    return _transform(f, tf)
