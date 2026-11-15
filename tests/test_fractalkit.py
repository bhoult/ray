import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import numpy as np
import pytest

import fractalkit as fk
from fractalkit import symbolic
from fractalkit.geometry import hopf_fiber_points, stereographic

RNG = np.random.default_rng(1)


def pts(n=2000, e=1.5):
    return RNG.uniform(-e, e, (n, 3))


def test_menger_closed_form_matches_kifs():
    P = pts()
    a, b = fk.Menger(4)(P), fk.Menger_kifs(4)(P)
    assert (np.sign(a) == np.sign(b)).mean() > 0.98
    assert np.abs(a - b).max() < 0.35 * 3 ** -4 * 30  # same set; estimates differ only near holes


def test_menger_is_1_lipschitz():
    P, Q = pts(), pts()
    f = fk.Menger(4)
    assert (np.abs(f(P) - f(Q)) <= np.linalg.norm(P - Q, axis=1) * 1.001 + 1e-9).all()


def test_box_counting_dimensions_match_exact_when_grid_is_aligned():
    dm, _ = fk.box_counting_dimension(fk.Menger(4), base=3, levels=4)
    assert dm == pytest.approx(np.log(20) / np.log(3), abs=0.01)       # 2.7268
    ds, _ = fk.box_counting_dimension(fk.Sierpinski(6), base=2, levels=7, max_cells=64)
    assert ds == pytest.approx(2.0, abs=0.05)


def test_box_counting_misaligned_or_thickened_is_biased_high():
    dm, _ = fk.box_counting_dimension(fk.Menger(4), base=2, levels=7, threshold=0.87)
    assert dm > np.log(20) / np.log(3) + 0.05


def test_hybrid_of_one_map_is_that_map():
    P = pts(500)
    b = fk.Mandelbulb(8, iters=8)
    h = fk.Hybrid([b], iters=8)
    assert np.allclose(b(P), h(P))


def test_hybrid_mixes():
    P = pts(500)
    h = fk.Hybrid([fk.Mandelbulb(8), fk.Mandelbox(2)], [0, 1], iters=10, finisher=1)
    d, t = h.de(P)
    assert np.isfinite(d).all() and (np.abs(d - fk.Mandelbox(2)(P)) > 0).any()


@pytest.mark.parametrize("n", [2, 3, 5])
def test_symbolic_bulb_matches_numeric_step(n):
    P = pts(300, 1.2)
    sym = symbolic.bulb_map_numeric(n)(P)
    m = fk.Mandelbulb(n)
    st = m.start(P.copy())
    st.c = np.zeros_like(P)        # drop the +c so we compare the bare power map
    m.step(st, 0)
    assert np.allclose(sym, st.z, atol=1e-8)


def test_bulb_derivative_estimate_is_the_radial_stretch_and_a_lower_bound():
    """dr = n r^(n-1) is the map's radial stretch, so the Jacobian's spectral
    norm is never below it -- and exceeds it near the poles (a factor ~2-6 for
    n = 2..8 in a sample).  That is why bulb DEs must be marched with step < 1."""
    P = pts(60, 1.2)
    r = np.linalg.norm(P, axis=1)
    for n in (2, 3):
        ratio = symbolic.bulb_jacobian_norm(n)(P) / (n * r ** (n - 1))
        assert ratio.min() >= 1 - 1e-9 and ratio.max() > 1.05


def test_hopf_map_is_onto_the_sphere():
    _, zero = symbolic.hopf_map_symbolic()
    assert zero == 0


def test_hopf_fibres_are_circles_and_tubes_are_centred_on_them():
    f = fk.HopfFibers(10, radius=0.05)
    t = np.linspace(0, 2 * np.pi, 40)
    # fibre over base point #k: reproduce its parameters
    for k in f.keep[:3]:
        x, y, z = f.base[k]
        eta, phi = 0.5 * np.arccos(x), np.arctan2(z, y)
        P = stereographic(hopf_fiber_points(eta, phi, t))
        assert np.allclose(f(P), -0.05, atol=1e-6)


def test_hopf_fibres_are_linked_by_circle_pair_linking_number():
    # two fibres: linking number of two circles via Gauss integral ~ +-1
    def fibre(base, n=400):
        x, y, z = base
        eta, phi = 0.5 * np.arccos(x), np.arctan2(z, y)
        return stereographic(hopf_fiber_points(eta, phi, np.linspace(0, 2 * np.pi, n, endpoint=False)))
    A, B = fibre(fk.geometry.fibonacci_sphere(7)[2]), fibre(fk.geometry.fibonacci_sphere(7)[5])
    dA, dB = np.roll(A, -1, 0) - A, np.roll(B, -1, 0) - B
    r = A[:, None] - B[None]
    gauss = np.einsum("ijk,ijk->ij", np.cross(dA[:, None], dB[None]), r) / np.linalg.norm(r, axis=-1) ** 3
    assert abs(gauss.sum() / (4 * np.pi)) == pytest.approx(1.0, abs=0.05)


def test_clifford_torus_is_the_projection_of_the_equal_modulus_torus():
    th, ph = RNG.uniform(0, 2 * np.pi, (2, 200))
    x4 = np.stack([np.cos(th), np.sin(th), np.cos(ph), np.sin(ph)], -1) / np.sqrt(2)
    assert np.allclose(fk.clifford_torus()(stereographic(x4)), 0, atol=1e-9)


def test_operators_and_transforms():
    a, b = fk.Sphere(1), fk.Sphere(1).translated((1, 0, 0))
    p = np.array([[0.5, 0, 0], [3, 0, 0]])
    assert np.allclose((a | b)(p), np.minimum(a(p), b(p)))
    assert np.allclose((a & b)(p), np.maximum(a(p), b(p)))
    assert np.allclose(a.scaled(2)(p), np.linalg.norm(p, axis=1) - 2)
    assert (a.smooth_union(b, 0.3)(p) <= (a | b)(p) + 1e-12).all()
    r = fk.Box((1, 2, 3)).rotated(0, 0, 90)   # long axis y -> x
    assert r(np.array([[2.0, 0, 0]]))[0] == pytest.approx(0.0, abs=1e-9)
    assert r(np.array([[0, 2.0, 0]]))[0] == pytest.approx(1.0, abs=1e-9)


def test_implicit_gyroid_distance_is_small_on_surface():
    g = fk.gyroid(2 * np.pi)
    P = pts(4000, 3)
    d = g(P)
    near = np.abs(d) < 0.02
    assert near.sum() > 20


def test_from_spec_roundtrip():
    f = fk.from_spec({"op": "smooth_union", "k": 0.1, "args": [
        {"type": "menger", "iters": 3},
        {"type": "hopf_fibers", "base": 8, "scale": 0.7}]})
    d, t = f.de(pts(100))
    assert d.shape == (100,) and np.isfinite(d).all()
    h = fk.from_spec({"op": "hybrid", "iters": 6, "args": [{"type": "mandelbulb", "power": 8}, {"type": "mandelbox"}]})
    assert np.isfinite(h(pts(50))).all()


def test_render_smoke():
    img = fk.render(fk.Menger(3), size=(48, 36))
    assert img.shape == (36, 48, 3) and img.std() > 0.01
