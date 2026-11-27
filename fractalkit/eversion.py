"""Bednorz & Bednorz (arXiv 1711.10466) twisted-cylinder family, n=2 stage.

Mesh-based, because intermediate eversion surfaces are immersions with
double curves and have no signed distance field.  The surface is
parametrised by (h, phi); the *paint* lives on the parameter domain, so it
follows the surface through the homotopy.  A face is drawn with the colour
of the side we see: sign(n . view) flips the inside/outside paint.
"""
import numpy as np
from scipy.spatial import cKDTree


def twisted_cylinder(h, phi, t=0.0, n=2, p=1.0, q=0.0):
    """Eq. (4) of the paper (p=1, q=0 gives eq. (3))."""
    x = t*np.cos(phi) + p*np.sin((n-1)*phi) - h*np.sin(phi)
    y = t*np.sin(phi) + p*np.cos((n-1)*phi) + h*np.cos(phi)
    z = h*np.sin(n*phi) - (t/n)*np.cos(n*phi) - q*t*h
    return np.stack([x, y, z], -1)


def mesh(t, H=3.0, nh=121, nphi=240, **kw):
    """Triangle mesh of the cylinder; phi is periodic. Returns V, F, (h,phi) per vertex."""
    h = np.linspace(-H, H, nh)
    phi = np.linspace(0, 2*np.pi, nphi, endpoint=False)
    hh, pp = np.meshgrid(h, phi, indexing="ij")
    V = twisted_cylinder(hh, pp, t, **kw).reshape(-1, 3)
    idx = np.arange(nh*nphi).reshape(nh, nphi)
    a = idx[:-1, :]; b = idx[:-1, np.r_[1:nphi, 0]]
    c = idx[1:, :];  d = idx[1:, np.r_[1:nphi, 0]]
    F = np.concatenate([np.stack([a, b, d], -1).reshape(-1, 3),
                        np.stack([a, d, c], -1).reshape(-1, 3)])
    return V, F, np.stack([hh.ravel(), pp.ravel()], -1)


def normals(V, F):
    """Parametrisation-oriented face normals (unnormalised cross product)."""
    return np.cross(V[F[:, 1]]-V[F[:, 0]], V[F[:, 2]]-V[F[:, 0]])


def jacobian_normal(h, phi, t, n=2, eps=1e-6, **kw):
    """n = r_h x r_phi, analytic-by-difference; zero means not an immersion."""
    rh = (twisted_cylinder(h+eps, phi, t, n, **kw)-twisted_cylinder(h-eps, phi, t, n, **kw))/(2*eps)
    rp = (twisted_cylinder(h, phi+eps, t, n, **kw)-twisted_cylinder(h, phi-eps, t, n, **kw))/(2*eps)
    return np.cross(rh, rp)


def min_normal(t, H=3.0, N=400, **kw):
    h = np.linspace(-H, H, N); phi = np.linspace(0, 2*np.pi, N, endpoint=False)
    hh, pp = np.meshgrid(h, phi)
    return np.linalg.norm(jacobian_normal(hh, pp, t, **kw), axis=-1).min()


def _edge_tri(P0, P1, T):
    """Moeller-Trumbore for segments P0->P1 against triangles T (m,3,3); bool per row."""
    e1 = T[:, 1]-T[:, 0]; e2 = T[:, 2]-T[:, 0]
    d = P1-P0
    pv = np.cross(d, e2); det = (e1*pv).sum(1)
    ok = np.abs(det) > 1e-14
    inv = np.where(ok, 1/np.where(ok, det, 1), 0)
    tv = P0-T[:, 0]
    u = (tv*pv).sum(1)*inv
    qv = np.cross(tv, e1)
    v = (d*qv).sum(1)*inv
    s = (e2*qv).sum(1)*inv
    return ok & (u > 0) & (v > 0) & (u+v < 1) & (s > 0) & (s < 1)


def self_intersections(V, F, chunk=400000):
    """Pairs of triangles (sharing no vertex) that intersect, and edge-crossing points (double curve).

    Broad phase: bounding spheres (centroid trees; triangles larger than the 99th-percentile are
    queried individually), then a bounding-box test, then edge-vs-triangle tests, all in chunks."""
    T = V[F]
    cen = T.mean(1)
    rad = np.linalg.norm(T-cen[:, None], axis=-1).max(1)
    lo, hi = T.min(1), T.max(1)
    thr = np.quantile(rad, 0.99)
    small = np.nonzero(rad <= thr)[0]; big = np.nonzero(rad > thr)[0]
    isbig = np.zeros(len(F), bool); isbig[big] = True
    tree = cKDTree(cen[small])
    cand = [small[tree.query_pairs(2*thr, output_type="ndarray")]]
    if len(big):
        full = cKDTree(cen)
        ex = []
        for i in big:
            for j in full.query_ball_point(cen[i], rad[i]+thr):
                if j != i and not (isbig[j] and j < i):
                    ex.append((i, j))
        if ex:
            cand.append(np.array(ex))
    cand = np.vstack(cand)
    hitpairs, pts = [], []
    for a in range(0, len(cand), chunk):
        pr = cand[a:a+chunk]
        ok = ((lo[pr[:, 0]] <= hi[pr[:, 1]]) & (lo[pr[:, 1]] <= hi[pr[:, 0]])).all(1)
        pr = pr[ok]
        sh = (F[pr[:, 0]][:, :, None] == F[pr[:, 1]][:, None, :]).any((1, 2))
        pr = pr[~sh]
        hit = np.zeros(len(pr), bool)
        for (ia, ib) in ((0, 1), (1, 2), (2, 0)):
            for A, B in ((0, 1), (1, 0)):
                ta = T[pr[:, A]]; tb = T[pr[:, B]]
                r = _edge_tri(ta[:, ia], ta[:, ib], tb)
                hit |= r
                if r.any():
                    pts.append(0.5*(ta[r, ia]+ta[r, ib]))
        hitpairs.append(pr[hit])
    pairs = np.vstack(hitpairs) if hitpairs else np.zeros((0, 2), int)
    pts = np.concatenate(pts) if pts else np.zeros((0, 3))
    return pairs, pts


# ---------------------------------------------------------------------------------------------
# Closed sphere: the paper's full pipeline  (theta,phi) -> (h,phi) -> r -> r' (7) -> r'' (8)
# ---------------------------------------------------------------------------------------------
Q_DEFAULT = 2/3


def evert_map(theta, phi, t, q=0.0, xi=1.0, eta=1.0, alpha=1.0, beta=1.0, lam=1.0, omega=2.0, n=2):
    """Point on the surface for sphere coordinates (theta, phi) and parameters of Table 1.

    lam = 1: the cylinder (4) with p = 1 - |q t|, h = omega sin(theta)/cos(theta)^n.
    lam < 1: eqs. (12) and (15) (needs p = 0, xi = 0, alpha = 0, as in the last stage).
    Both are followed by the damping (7) and the inversion (8)."""
    c = np.cos(theta); s = np.sin(theta); kap = (n-1)/(2*n)
    if lam >= 1 - 1e-12:
        p = 1 - abs(q*t)
        h = omega*s/c**n
        x = t*np.cos(phi) + p*np.sin((n-1)*phi) - h*np.sin(phi)
        y = t*np.sin(phi) + p*np.cos((n-1)*phi) + h*np.cos(phi)
        z = h*np.sin(n*phi) - (t/n)*np.cos(n*phi) - q*t*h
    else:
        x = (t*(1-lam+lam*c**n)*np.cos(phi) - lam*omega*s*np.sin(phi))/c**n
        y = (t*(1-lam+lam*c**n)*np.sin(phi) + lam*omega*s*np.cos(phi))/c**n
        z = (lam*(omega*s*(np.sin(n*phi) - q*t)/c**n - (t/n)*np.cos(n*phi))
             - (1-lam)*eta**(1+kap)*t*abs(t)**(2*kap)*s/c**(2*n))
    # damping (7)
    D = xi + eta*(x*x + y*y)
    x1, y1, z1 = x*D**(-kap), y*D**(-kap), z/D
    # inversion (8), written so that alpha -> 0 is stable
    r2 = x1*x1 + y1*y1
    g = 2*np.sqrt(alpha*beta)
    den = alpha + beta*r2
    E = np.exp(np.clip(g*z1, -700, 700))
    A = (alpha - beta*r2)/den
    gz = g*z1
    ex = np.where(np.abs(gz) < 1e-12, z1, np.expm1(np.clip(gz, -700, 700))/np.where(g == 0, 1, g))
    z2 = np.sqrt(alpha*beta)*(1-r2)/(den*(alpha+beta)) + A*ex
    return np.stack([x1*E/den, y1*E/den, z2], -1)


def schedule(s, Q=Q_DEFAULT, w=(.12, .12, .12, .24, .12, .12, .12)):
    """Full eversion movie, s in [0,1]: sphere(t=-1/Q) -> ... -> sphere(t=+1/Q), Table 1 read
    bottom-to-top for t<0, then top-to-bottom for t>0, parameters changed linearly."""
    w = np.asarray(w, float); w = w/w.sum(); edge = np.r_[0, np.cumsum(w)]
    k = int(np.clip(np.searchsorted(edge, s, side="right")-1, 0, 6)); u = (s-edge[k])/w[k]
    u = float(np.clip(u, 0, 1)); T = 1/Q
    d = dict(t=-T, q=Q, xi=0., eta=1., alpha=0., beta=1., lam=1.)
    if k == 0:   d.update(lam=u)                                   # sphere -> inverted wormhole
    elif k == 1: d.update(xi=u, alpha=u)                           # -> unfolded wormhole
    elif k == 2: d.update(xi=1., alpha=1., q=Q*(1-u))              # -> closed wormhole
    elif k == 3: d.update(xi=1., alpha=1., q=0., t=-T+2*T*u)       # t sweeps through 0
    elif k == 4: d.update(xi=1., alpha=1., q=Q*u, t=T)
    elif k == 5: d.update(xi=1-u, alpha=1-u, t=T)
    else:        d.update(lam=1-u, t=T)
    return d


def sphere_mesh(nth=161, nph=320, eps=2e-3, **par):
    th = np.linspace(-np.pi/2+eps, np.pi/2-eps, nth)
    ph = np.linspace(0, 2*np.pi, nph, endpoint=False)
    tt, pp = np.meshgrid(th, ph, indexing="ij")
    V = evert_map(tt, pp, **par).reshape(-1, 3)
    idx = np.arange(nth*nph).reshape(nth, nph); nx = np.r_[1:nph, 0]
    a, b, c, d = idx[:-1], idx[:-1, nx], idx[1:], idx[1:, nx]
    F = np.concatenate([np.stack([a, d, b], -1).reshape(-1, 3), np.stack([a, c, d], -1).reshape(-1, 3)])
    U = np.stack([np.cos(tt)*np.cos(pp), np.cos(tt)*np.sin(pp), np.sin(tt)], -1).reshape(-1, 3)
    return V, F, U           # U: the point of the unit sphere each vertex carries (paint lives there)


def signed_volume(V, F):
    T = V[F]
    return np.einsum("ij,ij->i", T[:, 0], np.cross(T[:, 1], T[:, 2])).sum()/6
