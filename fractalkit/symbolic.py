"""sympy companions: derive and cross-check the formulas symbolically."""
from __future__ import annotations

import numpy as np
import sympy as sp

x, y, z = sp.symbols("x y z", real=True)


def bulb_map(n: int):
    """Symbolic Mandelbulb power map (ray's axis convention: y is the pole).

    Built with complex powers: (y + i rho)^n = r^n e^{i n theta} and
    (x + i z)^n = rho^n e^{i n phi}, rho = sqrt(x^2+z^2), r = |(x,y,z)|.
    Returns a sympy Matrix [x', y', z'].
    """
    rho = sp.sqrt(x**2 + z**2)
    A = sp.expand((y + sp.I * rho) ** n)
    B = sp.expand((x + sp.I * z) ** n)
    cos_n_theta_rn = sp.re(A)
    sin_n_theta_rn = sp.im(A)
    cos_n_phi_rhon = sp.re(B)
    sin_n_phi_rhon = sp.im(B)
    # sin(n theta) cos(n phi) r^n = (Im A) (Re B) / rho^n
    xp = sin_n_theta_rn * cos_n_phi_rhon / rho**n
    zp = sin_n_theta_rn * sin_n_phi_rhon / rho**n
    return sp.Matrix([sp.simplify(xp), sp.simplify(cos_n_theta_rn), sp.simplify(zp)])


def bulb_map_numeric(n: int):
    """Lambdified symbolic power map: ``P (N,3) -> (N,3)``."""
    f = sp.lambdify((x, y, z), bulb_map(n), "numpy")

    def run(P):
        P = np.atleast_2d(P)
        cols = f(P[:, 0], P[:, 1], P[:, 2])
        return np.stack([np.broadcast_to(np.asarray(c, float).reshape(-1), (len(P),)) for c in cols], -1)

    return run


def bulb_jacobian_norm(n: int):
    """Symbolic Jacobian of the power map and a lambdified spectral norm,
    to check the derivative estimate ``dr ~ n r^(n-1)`` that the DE uses."""
    M = bulb_map(n)
    J = M.jacobian([x, y, z])
    f = sp.lambdify((x, y, z), J, "numpy")

    def norm(P):
        P = np.atleast_2d(P)
        out = []
        for p in P:
            out.append(np.linalg.norm(np.array(f(*p), float), 2))
        return np.array(out)

    return norm


def hopf_map_symbolic():
    """The Hopf map S^3 -> S^2 in real coordinates, and a proof that its
    image lies on the unit 2-sphere when x1^2+..+x4^2 = 1."""
    x1, x2, x3, x4 = sp.symbols("x1 x2 x3 x4", real=True)
    z1, z2 = x1 + sp.I * x2, x3 + sp.I * x4
    w = sp.expand(2 * z1 * sp.conjugate(z2))
    X = sp.expand(x1**2 + x2**2 - x3**2 - x4**2)
    Y, Z = sp.re(w), sp.im(w)
    norm_sq = sp.simplify(sp.expand(X**2 + Y**2 + Z**2) - (x1**2 + x2**2 + x3**2 + x4**2) ** 2)
    return (X, Y, Z), norm_sq  # norm_sq == 0  =>  |h(x)| = |x|^2


def kifs_scale_dimension(s: float, n_copies: int) -> float:
    """Similarity dimension of an IFS with n_copies maps of ratio 1/s."""
    return float(sp.N(sp.log(n_copies) / sp.log(s)))
