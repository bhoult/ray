"""Seam trees mixed with the existing fractals: morphs with the Riemann sphere, unions with Hopf fibres."""
from _common import *

V = fk.trionda_horned.TETRA
PENT = fk.hattrick_horned.load_hattrick()[2]
cam = lambda d=6.0, az=35, el=25: dict(camera=fk.Camera(azimuth=az, elevation=el, distance=d), step=0.7, max_steps=250,
                                      palette="viridis", trap_weight=0.9)
rs = fk.RiemannSphere(iters=8)
tr = fk.TriondaHorned(iters=3, body=0.95)
ht = fk.HatTrickHorned(iters=2, body=0.97)
cl = fk.TriondaHorned.clasp(iters=2, body=0.95)

# ---- 7a. morphs: blend averages the two distance estimates (a morph, not a boolean) -----------------------------------
items = [("Riemann sphere (alone)", rs, cam(4.0)),
         ("Riemann -> Trionda, t = 0.3", rs.blend(tr, 0.3), cam()),
         ("Riemann -> Trionda, t = 0.5", rs.blend(tr, 0.5), cam()),
         ("Riemann -> Trionda, t = 0.7", rs.blend(tr, 0.7), cam()),
         ("Riemann -> Hat Trick, t = 0.5", rs.blend(ht, 0.5), cam()),
         ("Riemann -> Trionda clasp, t = 0.5", rs.blend(cl, 0.5), cam())]
grid(items, name="mix_riemann_seamtrees", size=(220, 165))

# ---- 7b. unions with Hopf fibres: fibres over the ball's own axes, so they are the axes' Hopf lifts -----------------------
fibV = fk.HopfFibers(base=V, radius=0.1)                               # 4 fibres over the Trionda vertices: pairwise linked
fibE = fk.HopfFibers(base=np.array([V[i] + V[j] for i in range(4) for j in range(i + 1, 4)]) / np.sqrt(8 / 3), radius=0.1)   # 6 over its edge midpoints
fibP = fk.HopfFibers(base=PENT, radius=0.1)                            # 12 over the Hat Trick's pentagon centres
items = [("Trionda + 4 vertex fibres", fk.TriondaHorned(iters=2, body=0.95).smooth_union(fibV.scaled(0.4), 0.05), cam(5.5)),
         ("Trionda + 6 edge-midpoint fibres", fk.TriondaHorned(iters=2, body=0.95).smooth_union(fibE.scaled(0.4), 0.05), cam(5.5)),
         ("Trionda clasp + 4 vertex fibres", cl.smooth_union(fibV.scaled(0.5), 0.05), cam(6.5)),
         ("Hat Trick + 12 pentagon fibres", ht.smooth_union(fibP.scaled(0.3), 0.05), cam(6.0)),
         ("4 vertex fibres alone", fibV.scaled(0.5), cam(5.0)),
         ("12 pentagon fibres alone", fibP.scaled(0.4), cam(5.0))]
grid(items, name="mix_hopf_seamtrees", size=(220, 165))
