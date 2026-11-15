"""Literal scalar transcriptions of ray's Rust kernels (one point at a time,
same control flow, float64), used as references for the vectorised ports.
Sources: src/formulas/riemann.rs (riemann_sphere), src/formulas/horned.rs."""
import math

PI = math.pi


def riemann_sphere(p, radius=1.0, oa=0.0, ob=0.0, c=(0.0, 1.0, 0.0), w=0.0, iters=8):
    BAIL, NEAR, HUGE, HOLD, TAIL = 1e3, 2.0, 27.6, PI / 8, 4
    z = list(p)
    r = math.sqrt(sum(v * v for v in z))
    dr = 1.0
    l = math.log(max(r, 1e-6))
    rel = dr / max(r, 1e-6)
    trap = 1e9
    act = r < BAIL
    for i in range(iters + TAIL):
        if i >= iters:
            act = act and r > NEAR
        if not act:
            break
        rs = max(r, 1e-12)
        u = [v * radius / rs for v in z]
        q = 1.0 / max(1.0 - u[1], 1e-4)
        s0, t0 = u[0] * q, u[2] * q
        pw = min(s0 * s0 + t0 * t0 + 1.0, 36.0)
        ss, tt = abs(math.sin(s0 * PI + oa)), abs(math.sin(t0 * PI + ob))
        lp = pw * math.log(rs * rs)
        huge = lp > HUGE
        rp = math.exp(min(lp, HUGE))
        s2 = ss * ss + tt * tt
        f = (rp - 0.25) / (s2 + 1.0)
        nz = [ss * f * 2 + p[0] * w + c[0], (s2 - 1) * f + p[1] * w + c[1], tt * f * 2 + p[2] * w + c[2]]
        g = pw * 2.0 / rs
        hrel = g * dr
        ndr = min(g * (rp + HOLD) * dr + w, 1e30)
        z, dr = nz, ndr
        r = math.sqrt(sum(v * v for v in z))
        rl = max(r, 1e-6)
        l = lp if huge else math.log(rl)
        rel = hrel if huge else dr / rl
        if r < NEAR:
            trap = min(trap, abs(z[1]))
        act = act and r < BAIL and not huge
    return l * 0.5 / max(rel, 1e-12), min(max(trap, 0.0), 1.0)


def _smin(a, b, k):
    if k <= 0:
        return min(a, b)
    h = max(k - abs(a - b), 0.0) / k
    return min(a, b) - h * h * k * 0.25


def horned_sphere(p, shrink=0.45, clasp=1.35, turn=45.0, tube=0.28, taper=1.0, blend=0.5, ball=0.7, iters=8):
    s = shrink
    t = math.radians(turn)
    cp, sp = math.cos(t), math.sin(t)
    sg = min(clasp * s, 0.95)
    g = math.asin(sg)
    cg = math.sqrt(1 - sg * sg)
    c = math.hypot(cg, sg - s)
    rb = max(1.0 + tube, c / (1.0 - s)) + 1e-3
    tip = tube * s ** taper
    rate = taper * math.log(s) / (PI - g)
    off = sg - s

    def ring(q):
        ay = abs(q[1])
        th = math.atan2(ay, q[0])
        if th >= g:
            rxy = math.hypot(q[0], ay)
            return math.hypot(rxy - 1.0, q[2]) - tube * math.exp(rate * (PI - th))
        return math.sqrt((q[0] - cg) ** 2 + (ay - sg) ** 2 + q[2] ** 2) - tip

    def subtree(q):
        rxy = math.hypot(q[0], q[1]) - 1.0
        torus = math.hypot(rxy, q[2]) - tube
        front = math.sqrt((q[0] - cg) ** 2 + q[1] ** 2 + q[2] ** 2) - (off + s * rb)
        return min(torus, front)

    stack = [((p[1], p[0], -p[2]), 1.0, 0.0, -1e9, 1e9)]
    best, plain, level = 1e9, 1e9, 0.0
    while stack:
        (x, y, z), sc, lv, bd, dp = stack.pop()
        kf = blend * tube * sc
        if bd >= best + kf:
            continue
        d = ring((x, y, z)) * sc
        if lv == 0.0 and ball > 0.0:
            d = _smin(d, math.sqrt((x + 1) ** 2 + y * y + z * z) - ball, blend * ball * 0.5)
        if d < plain:
            plain, level = d, lv
        best = min(best, _smin(dp, d, kf))
        if lv + 1.0 >= iters:
            continue
        dx, dy, dz = x - cg, y - off, z
        a = (-dy / s, (sp * dx + cp * dz) / s, (sp * dz - cp * dx) / s)
        dy = y + off
        b = (dy / s, (cp * dx - sp * dz) / s, (-sp * dx - cp * dz) / s)
        sc2 = sc * s
        ba, bb = subtree(a) * sc2, subtree(b) * sc2
        near, bn, far, bf = (a, ba, b, bb) if ba < bb else (b, bb, a, ba)
        reach = best + blend * tube * sc2
        if bf < reach:
            stack.append((far, sc2, lv + 1, bf, d))
        if bn < reach:
            stack.append((near, sc2, lv + 1, bn, d))
    return best, min(max(level / 6.0, 0.0), 1.0)
