import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

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


# ---------------------------------------------------------------------------
# Riemann sphere and horned sphere: literal scalar ports of the Rust kernels
# ---------------------------------------------------------------------------
import rust_reference as ref  # noqa: E402

RIEMANN_VARIANTS = [dict(), dict(radius=0.7, offset=(0.5, -1.0), c=(0.2, 0.8, -0.3), mandel=0.4)]
HORNED_VARIANTS = [dict(), dict(iters=3),
                   dict(shrink=0.35, clasp=1.5, turn=20, tube=0.2, taper=0.5, blend=0.8, ball=0.3, iters=6)]


@pytest.mark.parametrize("kw", RIEMANN_VARIANTS)
def test_riemann_sphere_matches_rust_transcription(kw):
    P = pts(150, 1.8)
    d, t = fk.RiemannSphere(**kw).de(P)
    ref_kw = dict(radius=kw.get("radius", 1.0), oa=kw.get("offset", (0, 0))[0], ob=kw.get("offset", (0, 0))[1],
                  c=kw.get("c", (0, 1, 0)), w=kw.get("mandel", 0.0))
    R = np.array([ref.riemann_sphere(p, **ref_kw) for p in P])
    assert np.allclose(d, R[:, 0], atol=1e-9) and np.allclose(t, R[:, 1], atol=1e-9)


@pytest.mark.parametrize("kw", HORNED_VARIANTS)
def test_horned_sphere_matches_rust_depth_first_search(kw):
    P = pts(150, 1.8)
    d, t = fk.HornedSphere(**kw).de(P)
    R = np.array([ref.horned_sphere(p, **kw) for p in P])
    assert np.allclose(d, R[:, 0], atol=1e-9) and np.allclose(t, R[:, 1], atol=1e-9)


def test_horned_pruned_search_equals_brute_force_over_all_rings():
    from fractalkit.horned import _smin
    h = fk.HornedSphere(iters=4)
    P = pts(120, 1.6)

    def brute(q, sc, dp, gen):
        d = h.ring(q) * sc
        if gen == 0:
            d = _smin(d, np.sqrt((q[:, 0] + 1) ** 2 + q[:, 1] ** 2 + q[:, 2] ** 2) - h.ball, h.blend * h.ball * 0.5)
        out = _smin(dp, d, h.blend * h.tube * sc)
        if gen + 1 < h.iters:
            for c in h.children(q):
                out = np.minimum(out, brute(c, sc * h.s, d, gen + 1))
        return out

    q0 = np.stack([P[:, 1], P[:, 0], -P[:, 2]], -1)
    assert np.allclose(h(P), brute(q0, 1.0, np.full(len(P), 1e9), 0), atol=1e-12)


def test_riemann_sphere_far_points_are_outside_and_finite():
    f = fk.RiemannSphere()
    P = RNG.normal(size=(500, 3)); P = P / np.linalg.norm(P, axis=1, keepdims=True) * 3.0
    d, t = f.de(P)
    assert np.isfinite(d).all() and (d > 0).all() and ((0 <= t) & (t <= 1)).all()


def test_horned_sphere_distance_is_steep_enough_to_march_at_step_0p9():
    """|grad d| <= ~1 near the surface (the doc: 'passes the steepness test')."""
    h = fk.HornedSphere()
    P = pts(40000, 1.6)
    P = P[np.abs(h(P)) < 0.05][:600]
    dirs = RNG.normal(size=P.shape); dirs /= np.linalg.norm(dirs, axis=1, keepdims=True)
    e = 1e-4
    slope = np.abs(h(P + e * dirs) - h(P - e * dirs)) / (2 * e)
    assert len(P) > 100 and slope.max() < 1.15


def test_horned_first_generation_children_are_linked_like_chain_links():
    for turn in (20.0, 45.0, 70.0):
        h = fk.HornedSphere(turn=turn)
        lk = fk.linking_number(h.child_circle("a"), h.child_circle("b"))
        assert abs(lk) == pytest.approx(1.0, abs=0.05), (turn, lk)


def test_linking_number_of_hopf_fibres_and_unlinked_circles():
    t = np.linspace(0, 2 * np.pi, 300, endpoint=False)
    c1 = np.stack([np.cos(t), np.sin(t), 0 * t], -1)
    c2 = np.stack([1 + np.cos(t), 0 * t, np.sin(t)], -1)         # passes through c1's disc once
    c3 = c2 + np.array([5.0, 0, 0])
    assert abs(fk.linking_number(c1, c2)) == pytest.approx(1.0, abs=0.02)
    assert abs(fk.linking_number(c1, c3)) < 0.02


def test_new_types_in_spec_language():
    f = fk.from_spec({"op": "union", "args": [{"type": "horned_sphere", "iters": 4},
                                                {"type": "riemann_sphere", "scale": 0.5, "translate": [2, 0, 0]}]})
    assert np.isfinite(f(pts(100))).all()


# ---------------------------------------------------------------------------
# Shared structure: ring tree of the horned sphere, Hopf circles, linking
# ---------------------------------------------------------------------------

def test_ring_tree_first_generation_matches_child_circle():
    h = fk.HornedSphere()
    rings = h.ring_tree(2, closed=True, world=False, n=400)
    assert np.allclose(rings[1].curve, h.child_circle("a")) and np.allclose(rings[2].curve, h.child_circle("b"))
    assert [(r.gen, r.parent, r.sibling) for r in rings[:3]] == [(0, -1, -1), (1, 0, 2), (1, 0, 1)]


@pytest.mark.parametrize("depth", [2, 3])
def test_horned_linking_graph_is_touching_tree_plus_linked_sibling_pairs(depth):
    h = fk.HornedSphere()
    L, touch, rings = h.linking(depth, 0.02)
    n = len(rings)
    off = ~np.eye(n, dtype=bool) & ~touch
    # touching pairs are exactly parent-child (a child's back sits on its parent's cut tip)
    assert touch.sum() // 2 == n - 1 and all(touch[i, rings[i].parent] for i in range(1, n))
    # every other pair has an integer linking number...
    assert np.abs(L - np.round(L))[off].max() < 0.03
    # ...nonzero exactly for sibling pairs, all with one sign
    linked = {(i, j): int(round(L[i, j])) for i in range(n) for j in range(i + 1, n) if off[i, j] and round(L[i, j]) != 0}
    assert len(linked) == (n - 1) // 2 and all(rings[i].sibling == j for i, j in linked)
    assert len(set(linked.values())) == 1 and abs(next(iter(linked.values()))) == 1


def test_hopf_fibres_form_a_complete_linking_graph():
    hf = fk.HopfFibers(fk.ring_base([-0.5, 0.0, 0.5], 4), radius=0.05, max_circle=8.0)
    L, touch = fk.linking_matrix(hf.circles(0.03))
    off = ~np.eye(len(L), dtype=bool)
    assert not touch.any() and np.allclose(L[off], 1.0, atol=0.02)      # every pair links once, same sign


def test_curve_distance_refines_below_sample_spacing():
    t = np.linspace(0, 2 * np.pi, 40, endpoint=False)
    c1 = np.stack([np.cos(t), np.sin(t), 0 * t], -1)
    c2 = c1 + np.array([0.0, 0.0, 0.0123])           # coplanar-ish copy lifted by 0.0123
    assert fk.curve_distance(c1, c2) == pytest.approx(0.0123, abs=1e-6)


def test_skeleton_centreline_lies_inside_the_distance_field():
    """The tree's composed poses reproduce the field's own rings: every
    centre-line vertex has d <= -(local tube radius) in the field without the
    ball and fillets (where d is the min over rings)."""
    h = fk.HornedSphere(blend=0.0, ball=0.0, iters=4)
    for r in h.ring_tree(3, closed=False):
        th = np.where(r.angle <= np.pi, r.angle, 2 * np.pi - r.angle)
        radius = h.tube * r.scale * np.exp(h.rate * (np.pi - th))
        assert (h(r.curve) <= -radius * (1 - 1e-6) + 1e-9).all()


def test_skeleton_network_reproduces_the_field_near_its_surface():
    h = fk.HornedSphere(blend=0.0, ball=0.0, iters=4)
    net = h.skeleton(3, spacing=0.02)
    P = pts(30000, 1.6)
    d = h(P)
    P = P[np.abs(d) < 0.15][:400]
    assert len(P) > 100
    assert np.abs(net(P) - h(P)).max() < 0.01          # chord error + radius steps of the polyline


def test_fiber_network_per_segment_radius_and_labels():
    seg = np.array([[[0, 0, 0], [1, 0, 0]], [[0, 1, 0], [1, 1, 0]]], float)
    P = np.array([[0.5, 0.0, 0.2], [0.5, 1.0, 0.2]])
    a = fk.FiberNetwork(seg, 0.05)
    b = fk.FiberNetwork(seg, np.array([0.05, 0.1]), labels=[0.2, 0.9])
    assert np.allclose(a(P), [0.15, 0.15]) and np.allclose(b(P), [0.15, 0.1])
    assert np.allclose(b.de(P)[1], [0.2, 0.9])


# ---------------------------------------------------------------------------
# ClaspTree generalisation and the Hopf tree
# ---------------------------------------------------------------------------
from fractalkit.clasptree import ClaspTree  # noqa: E402


def test_generic_clasp_tree_reproduces_the_horned_sphere_exactly():
    """The base class's generic child maps / generic bounds give the same
    field as the horned sphere's closed-form children and tighter bounds."""
    h = fk.HornedSphere(iters=6)
    g = ClaspTree()
    g._setup(h.s, h.g, h.maps, h.tube, 1.0, h.blend, h.ball, h.iters)
    P = pts(400, 1.8)
    (dh, th), (dg, tg) = h.de(P), g.de(P)
    assert np.allclose(dh, dg, atol=1e-12) and np.allclose(th, tg)


@pytest.mark.parametrize("psi,hang,hand", [(0.5, 1, 1), (1.5, -1, 1), (2.0, 1, -1), (4.0, -1, -1)])
def test_hopf_tree_places_back_points_on_the_tips(psi, hang, hand):
    t = fk.HopfTree(psi=psi, hang=hang, hand=hand)
    tips = np.array([[t.cg, t.sg, 0.0], [t.cg, -t.sg, 0.0]])
    assert np.allclose(t.back_points(), tips, atol=1e-12)
    assert t.sg == pytest.approx(t.s * t.rho)


@pytest.mark.parametrize("psi,hang,hand", [(0.5, 1, 1), (1.5, -1, 1), (2.0, 1, -1), (4.0, -1, -1)])
def test_hopf_tree_siblings_are_hopf_fibres(psi, hang, hand):
    """Map the two child circles back through the inverse similarity and the
    inverse stereographic projection: each lies over ONE point of S^2 (it is a
    fibre) and the two points are antipodal (a Hopf pair)."""
    t = fk.HopfTree(psi=psi, hang=hang, hand=hand)
    Q, o, m = t.to_parent
    bases = []
    for k in (0, 1):
        q = t.child_circle(k, n=200)
        pair = (q - m) @ Q / t.s + o                      # inverse of  x -> s Q (x - o) + m
        s3 = fk.inverse_stereographic(pair * t.fibre_radius)
        b = fk.hopf_map(s3)
        assert np.allclose(np.linalg.norm(s3, axis=1), 1.0) and b.std(axis=0).max() < 1e-9
        bases.append(b.mean(axis=0))
    assert bases[0] @ bases[1] == pytest.approx(-1.0, abs=1e-9)


def test_hopf_tree_deeper_sibling_pairs_are_hopf_pairs_too():
    t = fk.HopfTree(psi=1.2)
    rings = t.ring_tree(2, closed=True, world=False, n=200)
    # ring 1's frame: children are rings 3 and 4; bring them into ring 1's frame, then check as above
    M, off, S = np.eye(3), np.zeros(3), 1.0
    R, tc = t.maps[0]
    M, off, S = R, tc, t.s                                 # ring 1 -> root: q = off + S M l
    Q, o, m = t.to_parent
    for k, idx in ((0, 3), (1, 4)):
        local = ((rings[idx].curve - off) / S) @ M          # in ring 1's own frame
        pair = (local - m) @ Q / t.s + o
        b = fk.hopf_map(fk.inverse_stereographic(pair * t.fibre_radius))
        assert b.std(axis=0).max() < 1e-9


@pytest.mark.parametrize("psi", [1.0, 1.6, 2.0])
def test_hopf_tree_linking_matches_horned_pattern_in_the_valid_window(psi):
    L, touch, rings = fk.HopfTree(psi=psi).linking(3, 0.02)
    n = len(rings)
    off = ~np.eye(n, dtype=bool) & ~touch
    assert touch.sum() // 2 == n - 1 and all(touch[i, rings[i].parent] for i in range(1, n))
    linked = [(i, j, round(L[i, j])) for i in range(n) for j in range(i + 1, n) if off[i, j] and round(L[i, j]) != 0]
    assert np.abs(L - np.round(L))[off].max() < 0.03
    assert len(linked) == (n - 1) // 2 and all(rings[i].sibling == j for i, j, _ in linked)
    assert {v for *_, v in linked} == {1}                    # Hopf fibres link with sign +1


def test_hopf_tree_beyond_the_window_has_extra_links_between_non_siblings():
    L, touch, rings = fk.HopfTree(psi=np.pi).linking(3, 0.02)
    n = len(rings)
    off = ~np.eye(n, dtype=bool) & ~touch
    assert np.abs(L - np.round(L))[off].max() < 0.03
    linked = [(i, j, round(L[i, j])) for i in range(n) for j in range(i + 1, n) if off[i, j] and round(L[i, j]) != 0]
    assert len(linked) > (n - 1) // 2 and any(rings[i].sibling != j for i, j, _ in linked)
    assert {v for *_, v in linked} == {-1, 1}


def test_hopf_tree_skeleton_reproduces_its_field():
    t = fk.HopfTree(psi=1.2, tube=0.1, blend=0.0, ball=0.0, iters=4)
    net = t.skeleton(3, 0.02)
    P = pts(40000, 1.6)
    P = P[np.abs(t(P)) < 0.1][:300]
    assert len(P) > 100 and np.abs(net(P) - t(P)).max() < 0.01


def test_hopf_tree_rejects_a_clasp_that_cannot_close():
    with pytest.raises(ValueError):
        fk.HopfTree(shrink=0.7, psi=np.pi)           # 0.7 * 1.707 > 0.95


def test_hopf_tree_in_the_spec_language():
    f = fk.from_spec({"type": "hopf_tree", "psi": 1.2, "iters": 5, "scale": 0.8})
    assert np.isfinite(f(pts(100))).all()


# ---- Hopfbrot as an IterMap -------------------------------------------------

import legacy_hopfbrot as legacy  # noqa: E402

HB_CASES = [dict(power=3, tilt=90, slice=0.0, iters=10), dict(power=3, tilt=45, slice=0.0, iters=10),
            dict(power=3, tilt=0, slice=0.2, iters=10), dict(power=5, tilt=70, slice=-0.1, iters=6),
            dict(power=2, tilt=90, slice=0.0, iters=8)]


@pytest.mark.parametrize("kw", HB_CASES)
def test_hopfbrot_itermap_matches_the_standalone_kernel(kw):
    P = pts(4000, 1.8)
    d0, t0 = legacy.HopfbrotField(**kw).de(P)
    d1, t1 = fk.Hopfbrot(**kw).de(P)
    assert np.allclose(d0, d1, rtol=1e-12, atol=1e-12) and np.allclose(t0, t1)


def test_hopfbrot_is_an_itermap_with_a_four_dimensional_orbit():
    h = fk.Hopfbrot(3, 90.0, 0.0, 6)
    assert isinstance(h, fk.IterMap)
    st = h.run(pts(50))
    assert st.z.shape == (50, 3) and st.extra["w4"].shape == (50,)
    assert h.orbit4(st).shape == (50, 4)
    P = pts(50)
    assert np.allclose(h.with_iters(3)(P), fk.Hopfbrot(3, 90.0, 0.0, 3)(P))


def test_hopfbrot_in_a_hybrid_alone_equals_itself_without_the_tail():
    P = pts(2000, 1.8)
    h = fk.Hopfbrot(3, 90.0, 0.0, 10)
    hy = fk.Hybrid([h], iters=10)
    st = h.start(P)
    for i in range(10):
        if not st.active.any():
            break
        h.step(st, i)
    d, _ = h.finish(st)
    assert np.allclose(hy(P), d, rtol=1e-12, atol=1e-12)


def test_hopfbrot_mixes_with_a_fold_in_a_hybrid():
    P = pts(3000, 1.8)
    hy = fk.Hybrid([fk.Hopfbrot(3, 90.0, 0.0), fk.Mandelbulb(8.0)], schedule=[0, 0, 1], iters=12, bail=8.0)
    d = hy(P)
    assert np.isfinite(d).all() and d.std() > 0
    other = fk.Hybrid([fk.Hopfbrot(3, 90.0, 0.0), fk.Mandelbulb(8.0)], schedule=[0, 1, 1], iters=12, bail=8.0)
    assert not np.allclose(d, other(P))
    # as a non-finisher the hidden coordinate is created lazily
    hy2 = fk.Hybrid([fk.Mandelbulb(8.0), fk.Hopfbrot(3, 90.0, 0.0)], schedule=[0, 1], iters=8, finisher=0, bail=8.0)
    assert np.isfinite(hy2(P)).all()


def test_hopfbrot_in_the_spec_language_and_with_operators():
    f = fk.from_spec({"type": "hopfbrot", "power": 3, "iters": 6})
    assert isinstance(f, fk.IterMap) and np.isfinite(f(pts(100))).all()
    g = fk.Hopfbrot(3).scaled(0.5) | fk.Sphere(0.2)
    assert np.isfinite(g(pts(100))).all()


def test_riemann_sphere_can_step_inside_a_hybrid_it_does_not_finish():
    P = pts(1500, 1.8)
    hy = fk.Hybrid([fk.RiemannSphere(1.0, iters=8), fk.Hopfbrot(3, 90.0, 0.0)], schedule=[0, 1], iters=8,
                   finisher=1, bail=30.0)
    assert np.isfinite(hy(P)).all()


def _fit_circle(c):
    k = len(c)
    return fk.geometry.circle_through(c[0], c[k // 3], c[2 * k // 3])


def _child_circles(h):
    return [_fit_circle(r.curve) for r in h.ring_tree(1, 0.03, closed=True, world=True) if r.gen == 1]


def test_moebius_invariants_of_two_hopf_fibres_are_equal():
    t = np.linspace(0, 2 * np.pi, 400, endpoint=False)
    cs = [_fit_circle(fk.geometry.stereographic(fk.geometry.hopf_fiber_points(e, p, t)))
          for e, p in [(0.5, 0.2), (0.9, 2.0), (0.7, 4.0)]]
    for i in range(3):
        for j in range(i + 1, 3):
            s = fk.circle_moebius_invariants(cs[i], cs[j])
            assert abs(s[0] - s[1]) < 1e-9


def test_horned_clasp_moebius_invariants_have_the_closed_form():
    for kap in (1.2, 1.35, 1.5, 1 + 1 / np.sqrt(2), 1.9):
        for turn in (45.0, -45.0, 100.0):
            a, b = _child_circles(fk.HornedSphere(shrink=0.4, clasp=kap, turn=turn, iters=2))
            s = fk.circle_moebius_invariants(a, b)
            assert abs(s[0] - abs(1 - 2 * (kap - 1) ** 2)) < 1e-9 and s[1] < 1e-9


def test_hopf_tree_at_psi_pi_is_the_horned_sphere_with_hopf_clasp():
    from scipy.spatial.distance import cdist
    kap = 1 + 1 / np.sqrt(2)
    H = fk.HopfTree(shrink=0.4, psi=np.pi, hang=1, hand=1)
    h = fk.HornedSphere(shrink=0.4, clasp=kap, turn=-45.0, iters=3)
    rh, rr = H.ring_tree(3, 0.04, closed=True, world=True), h.ring_tree(3, 0.04, closed=True, world=True)
    for g in (1, 2, 3):
        A = np.vstack([r.curve for r in rh if r.gen == g])
        B = np.vstack([r.curve for r in rr if r.gen == g])
        D = cdist(A, B)
        assert max(D.min(1).max(), D.min(0).max()) < 1e-9
