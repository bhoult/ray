#!/usr/bin/env python3
"""The demo video's director. It drives the viewer of tools/demo/session.sh
through its control socket and records each segment with render_video (the
view drawn whole a frame at a time at 1920 x 1080 with SAMPLES samples a
pixel, so it plays smoothly however slowly the frames were drawn) into
WORKDIR/clips/NAME.mp4. tools/demo/assemble.py captions the clips and joins
them. Scenes are opened from saves/ and never saved.
    python3 tools/demo/make_demo.py WORKDIR [--scout] [segment ...]
(default: all; --scout: at a third of the size and 2 samples, into
WORKDIR/scout, to try a segment out quickly)
"""
import math
import os
import shutil
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from viewer import Viewer  # noqa: E402

# (RAY_DEMO_SOCKET: session.sh's, when it was given one)
SOCKET = os.environ.get("RAY_DEMO_SOCKET") or os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "ray-demo.sock")
SCOUT = False
# the clips' size, and the samples a pixel in each frame (16: the edges
# smooth; a path-traced take asks for more)
SIZE = (1920, 1080)
SAMPLES = 16
# the share of the GPU's and the CPU's time the takes keep busy (the
# computer stays usable meanwhile)
MAX_LOAD = 0.9


class Take:
    """One render_video take, `length` seconds of video: started on entry,
    it ends by itself after exactly that many frames; `at(t)` waits until
    the video is t seconds long; `end()` waits for its file and keeps it."""

    def __init__(self, v, work, name, length, samples=SAMPLES):
        self.v, self.work, self.name, self.length = v, work, name, length
        self.samples = 2 if SCOUT else samples

    def __enter__(self):
        self.t0 = time.time()
        w, h = (SIZE[0] // 3, SIZE[1] // 3) if SCOUT else SIZE
        self.file = self.v("render_video", width=w, height=h, samples=self.samples, seconds=self.length, max_load=MAX_LOAD)["file"]
        self.said = 0
        return self

    def progress(self, st):
        """Says how far it is, now and then."""
        steady = (st.get("recording") or {}).get("steady")
        if steady and time.time() - self.said > 60:
            eta = steady.get("eta_seconds") or 0
            print(f"  {self.name}: {steady['seconds']:5.2f} of {self.length} s of video, {time.time() - self.t0:5.0f} s in, {steady['seconds_a_frame']:.1f} s a frame, about {eta / 60:.0f} min to go", flush=True)
            self.said = time.time()

    def at(self, t):
        while True:
            st = self.v("status")
            rec = st.get("recording")
            if rec is None or ((rec.get("steady") or {}).get("seconds", 0.0)) >= t - 1e-4:
                return
            self.progress(st)
            time.sleep(0.1)

    def end(self):
        # (whole once the viewer names it its last video)
        while True:
            st = self.v("status")
            if st.get("recording") is None:
                if st.get("last_video") and os.path.realpath(st["last_video"]) == os.path.realpath(self.file):
                    break
                if st.get("recording_error"):
                    raise RuntimeError(f"{self.name}: {st['recording_error']}")
            self.progress(st)
            time.sleep(0.5)
        out = os.path.join(self.work, "scout" if SCOUT else "clips", self.name + ".mp4")
        os.makedirs(os.path.dirname(out), exist_ok=True)
        shutil.move(self.file, out)
        print(f"  {self.name}: {self.length} s of video in {time.time() - self.t0:.0f} s -> {out}", flush=True)

    def __exit__(self, *exc):
        # (stopped on an error too, so the next take starts clean)
        if exc[0] is not None and self.v("status").get("recording"):
            self.v("render_video", stop=True)


def setup(v, scene, game=False, settings=None, **camera):
    """Opens a scene (or a screenshot: the moment it was taken) out of game
    mode, with the camera set, and waits for the GPU; then into game mode
    when asked (the walker standing where the camera is; a screenshot taken
    in game mode keeps its walker as it stood)."""
    if v("status").get("game"):
        v("command", name="game_mode")
    v("open", path=scene)
    if v("status").get("game") and not (game and not camera):
        v("command", name="game_mode")
    if camera:
        v("camera", **camera)
    if settings:
        v("settings", set=settings)
    v.ready(samples=1, timeout=120)
    if game and not v("status").get("game"):
        v("command", name="game_mode")
        time.sleep(1.5)
    if game:
        g = v("status").get("game") or {}
        print(f"  walker {g.get('height_m', 0):.4g} m tall at {g.get('feet')}", flush=True)


def orbit(take, center, radius, heights, angles, times, legs=8):
    """The camera round `center` along an arc (angles in degrees from +z,
    heights from start to end) between scene times, looking at it. The legs
    go at once, queued: each starts where and exactly when the last ends (sent
    one by one as each ended, one could arrive a step late, and that step
    moved nothing: a frame shown twice)."""
    (a0, a1), (h0, h1), (t0, t1) = angles, heights, times
    for i in range(1, legs + 1):
        k = i / legs
        a = math.radians(a0 + (a1 - a0) * k)
        pos = [center[0] + radius * math.sin(a), h0 + (h1 - h0) * k, center[2] + radius * math.cos(a)]
        if i == 1:
            take.at(t0)
        take.v("glide", position=pos, look_at=center, seconds=(t1 - t0) / legs, even=True, queue=i > 1)


def showcase(v, take):
    setup(v, "saves/showcase.json", position=[2.4, 3.0, 9.6], look_at=[0, 1.1, -0.5])
    with take(8) as t:
        orbit(t, [0, 1.1, -0.5], 10.0, (3.0, 2.2), (14, -32), (0, 8))
        t.end()


def glass(v, take):
    setup(v, "saves/glass.json", position=[1.7, 1.75, 2.4], look_at=[0, 0.85, 0])
    with take(7) as t:
        orbit(t, [0, 0.85, 0], 2.95, (1.75, 1.35), (35, -28), (0, 7))
        t.end()


def city(v, take):
    setup(v, "saves/night-city.json", position=[7.5, 16, 92], look_at=[-3, 26, -120])
    with take(7) as t:
        v("glide", position=[5.0, 9.0, 40.0], look_at=[-3, 15, -120], seconds=7, even=True)
        t.end()


def clouds(v, take):
    setup(v, "saves/clouds.json", position=[6, 9, 30], look_at=[-2, 10, -6])
    with take(7) as t:
        v("glide", position=[1.0, 10.5, 15.0], look_at=[-3, 11, -8], seconds=7, even=True)
        t.end()


def island(v, take):
    """Across the mouth of the bay, low over the water: the sloop slides
    past, the lighthouse grows on its headland and the sun goes behind it."""
    setup(v, "saves/island.json", position=[72, 7.5, 150], look_at=[5, 9, 55])
    with take(6) as t:
        v("glide", position=[-8, 10, 132], look_at=[-10, 11, 45], seconds=6, even=True)
        t.end()


def water(v, take):
    setup(v, "saves/water.json", settings={"animate_water": True}, position=[1.5, 4.6, 10.5], look_at=[-0.5, 1.2, -3.0])
    with take(6) as t:
        v("glide", position=[-3.2, 2.7, 7.0], look_at=[0, 0.7, -3.0], seconds=6)
        t.end()


def landscape(v, take):
    setup(v, "saves/landscape.json", position=[-60, 34, 95], look_at=[10, 18, 0])
    with take(6) as t:
        v("glide", position=[-28, 27, 52], look_at=[16, 15, -20], seconds=6, even=True)
        t.end()


def materials(v, take):
    setup(v, "saves/materials.json", position=[0, 4.4, 3.2], look_at=[0, 0.5, -4.6])
    with take(5) as t:
        v("glide", position=[2.6, 2.5, 1.6], look_at=[0, 0.6, -3.4], seconds=5)
        t.end()


def gallery(v, take):
    """Along the fractal gallery (an arc of radius 7 round the origin),
    looking out at the plinths a little ahead."""
    setup(v, "saves/fractals.json", position=[-4.0, 1.9, 1.1], look_at=[-6.9, 1.7, -0.8])
    legs, (a0, a1), T = 10, (-108, 100), 7.0
    with take(T) as t:
        for i in range(1, legs + 1):
            a = math.radians(a0 + (a1 - a0) * i / legs)
            ahead = a + math.radians(14)
            pos = [4.2 * math.sin(a), 1.9, -4.2 * math.cos(a)]
            look = [7.0 * math.sin(ahead), 1.75, -7.0 * math.cos(ahead)]
            v("glide", position=pos, look_at=look, seconds=T / legs, even=True, queue=i > 1)
        t.end()


def sponge(v, take):
    """Game mode in the Menger sponge's central tunnel (half a metre square,
    clear through): the walker, made to fit it, walks to the middle, drops
    down the central shaft, looks up it and rockets up and out of the top."""
    floor, eye = 1.281, 0.40
    setup(v, "saves/fractals.json", game=True, settings={"headlight": 1.0}, position=[-6.659, floor + eye, -1.45], yaw_degrees=0, pitch_degrees=-6)
    with take(16) as t:
        t.at(1.0)
        v("hold", keys=["w"], seconds=2.6)
        t.at(4.0)
        v("glide", pitch_degrees=72, turn_degrees=40, seconds=2.5)
        t.at(7.0)
        v("glide", turn_degrees=150, seconds=4.0)
        t.at(9.0)
        v("hold", keys=["rmb"], seconds=5.0)
        t.at(12.0)
        v("glide", pitch_degrees=-30, seconds=3.0)
        t.end()


def bulb(v, take):
    """On top of the Mandelbulb: the walker shrinks with its feet on it until
    the bulb is a landscape, and walks across it."""
    setup(v, "saves/fractals.json", game=True, position=BULB_TOP, yaw_degrees=-20, pitch_degrees=-18)
    with take(8) as t:
        t.at(0.5)
        v("hold", keys=["f"], seconds=3.0)
        t.at(3.5)
        v("hold", keys=["w"], seconds=4.5)
        v("glide", turn_degrees=-50, pitch_degrees=-8, seconds=4.5)
        t.end()


# half a metre above the Mandelbulb's top (y 4.1 there, by raycast)
BULB_TOP = [0.213, 4.6, -6.2]


def kleinian(v, take):
    setup(v, "saves/fractals2.json", game=True, settings={"headlight": 1.0})
    with take(9) as t:
        v("glide", turn_degrees=100, pitch_degrees=8, seconds=3.5)
        t.at(3.5)
        v("hold", keys=["w"], seconds=3.0)
        t.at(5.5)
        v("hold", keys=["rmb"], seconds=2.5)
        v("glide", pitch_degrees=35, turn_degrees=-40, seconds=3.0)
        t.end()


def physics(v, take):
    """The gravity gun at the crate pyramid, twice."""
    setup(v, "saves/physics.json", game=True, position=[-3.0, 1.66, 5.2], look_at=[-3.0, 1.05, 0])
    with take(8) as t:
        t.at(0.6)
        v("hold", keys=["lmb"], seconds=1.0)
        t.at(4.0)
        v("glide", turn_degrees=8, pitch_degrees=-12, seconds=0.8)
        t.at(5.0)
        v("hold", keys=["lmb"], seconds=0.6)
        t.end()


def bonsai(v, take):
    """Swimming to the bonsai's sandbank, a dive and back up."""
    setup(v, "saves/bonsai.json", game=True, position=[1.6, 1.35, 8.0], look_at=[0.15, 1.6, 0])
    with take(9) as t:
        v("hold", keys=["w"], seconds=3.5)
        t.at(3.5)
        v("glide", pitch_degrees=-55, seconds=0.6)
        t.at(4.2)
        v("hold", keys=["space"], seconds=0.2)
        v("hold", keys=["w"], seconds=1.2)
        t.at(5.2)
        v("glide", pitch_degrees=35, seconds=0.8)
        t.at(5.6)
        v("hold", keys=["w"], seconds=3.4)
        t.end()


def cornell(v, take):
    """Path traced: many more samples a frame."""
    setup(v, "saves/cornell.json", position=[0, 1, 3.6], look_at=[0, 1, 0])
    with take(5, samples=CORNELL_SAMPLES) as t:
        v("glide", position=[0.3, 1.02, 2.95], look_at=[0, 0.95, 0], seconds=5)
        t.end()


CORNELL_SAMPLES = 1024


def nurbs(v, take):
    setup(v, "saves/nurbs-booleans.json", position=[0.5, 3.0, 7.5], look_at=[0, 0.8, 0])
    with take(6) as t:
        orbit(t, [0, 0.8, 0], 7.5, (3.0, 2.3), (4, -32), (0, 6))
        t.end()


# the video's order: (name, caption, record); the stills and cards are
# assemble.py's
SEGMENTS = [
    ("showcase", "Every shape is maths, ray traced with no meshes: distance fields, booleans, metaballs, a gyroid", showcase),
    ("glass", "NURBS surfaces of revolution in clear and smoked glass, refracting", glass),
    ("nurbs", "NURBS in booleans, cut and blended like any other shape", nurbs),
    ("landscape", "A 512 m eroded landscape and lake; fractals in the distance, coloured by orbit traps", landscape),
    ("city", "A city at the blue hour: about 1,500 shapes", city),
    ("clouds", "Volume clouds, lit through; balloons with NURBS envelopes", clouds),
    ("island", "An island at sunset: noise terrain sunk into a coastline, the sun low on the sea, a lighthouse lit", island),
    ("water", "The sea: waves, refraction and caustics; things float or sink by their density", water),
    ("cornell", "Path tracing: the Cornell box", cornell),
    ("physics", "Game mode: a gravity gun that charges while held", physics),
    ("bonsai", "Swimming and diving by the bonsai's sandbank", bonsai),
    ("gallery", "A gallery of fractals, each a distance estimator", gallery),
    ("sponge", "Game mode in the Menger sponge: down a tunnel, into the central shaft, out by rocket", sponge),
    ("kleinian", "Inside a Kleinian limit set, less than a millimetre tall", kleinian),
    ("bulb", "Shrinking onto the Mandelbulb until it is a landscape", bulb),
]


def main():
    global SCOUT
    work = sys.argv[1]
    SCOUT = "--scout" in sys.argv
    want = [a for a in sys.argv[2:] if a != "--scout"] or [s[0] for s in SEGMENTS]
    v = Viewer(SOCKET)
    v.up()
    for name, _, record in SEGMENTS:
        if name in want:
            print(f"{name}:", flush=True)
            # (one that fails is said and left out; the others go on)
            try:
                record(v, lambda length, **kw: Take(v, work, name, length, **kw))
            except Exception as e:
                print(f"  {name} FAILED: {e}", flush=True)


if __name__ == "__main__":
    main()
