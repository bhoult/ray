"""Talks to a viewer over its control socket (one JSON line each way), for
the demo director: v = Viewer(path); v('open', path=...); v.wait_ready()."""
import json, socket, time


class Viewer:
    def __init__(self, sock):
        self.sock = sock

    def __call__(self, cmd, **args):
        s = socket.socket(socket.AF_UNIX)
        s.settimeout(120)
        s.connect(self.sock)
        s.sendall((json.dumps({"cmd": cmd, "args": args}) + "\n").encode())
        buf = b""
        while not buf.endswith(b"\n"):
            chunk = s.recv(1 << 20)
            if not chunk:
                break
            buf += chunk
        s.close()
        r = json.loads(buf)
        if not r.get("ok"):
            raise RuntimeError(f"{cmd}: {r.get('error')}")
        return r.get("result")

    def up(self, timeout=60):
        """Waits for the socket."""
        t0 = time.time()
        while time.time() - t0 < timeout:
            try:
                return self("status")
            except (OSError, RuntimeError):
                time.sleep(0.3)
        raise RuntimeError("the viewer did not come up")

    def ready(self, samples=8, timeout=60):
        """Waits until the GPU has joined in (shader built) and the image has
        `samples` samples."""
        t0 = time.time()
        while time.time() - t0 < timeout:
            st = self("status")
            if st.get("gpu_share", 0) > 0 and st.get("samples", 0) >= samples:
                return st
            time.sleep(0.2)
        return self("status")

    def shot(self, path, width=480, samples=4, timeout=10):
        import base64
        r = self("screenshot", width=width, samples=samples, timeout=timeout)
        open(path, "wb").write(base64.b64decode(r["png_base64"]))
        return r
