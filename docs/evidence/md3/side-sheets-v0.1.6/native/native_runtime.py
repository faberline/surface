from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time

from gallery_transport import Gallery, require


def digest(path):
    sha256 = hashlib.sha256()
    with path.open("rb") as handle:
        for data in iter(lambda: handle.read(1024 * 1024), b""):
            sha256.update(data)
    return sha256.hexdigest()


def read_trace(path):
    lines = path.read_bytes().split(b"\n")
    return [json.loads(line) for line in lines[:-1] if line]


def presses(trace, stable_id):
    return [
        row for row in trace
        if row["kind"] == "Message"
        and (row["data"].get("source") or {}).get("id") == stable_id
    ]


def find_input(rows, event):
    return [row for row in rows if row["kind"] == "Input" and row["data"]["event"] == event]


def wait_for_persisted_trace(app, path, ready):
    # The stock native host uses BufWriter. Read-only presentations advance
    # that buffer, without sending any additional application input.
    for _ in range(64):
        rows = read_trace(path)
        if ready(rows):
            return rows
        app.dom()
    raise RuntimeError("The native trace did not persist the expected boundary")


@contextmanager
def launch(plan, name, width, height, compact):
    if plan.headless:
        command = [plan.gallery, "--headless", "--width", str(width), "--height", str(height)]
    elif compact:
        command = [plan.profile_launcher, str(width), str(height)]
    else:
        command = [plan.gallery]
    with tempfile.TemporaryDirectory(dir="/tmp", prefix="surface-side-sheet-session-") as tmp:
        private = Path(tmp)
        session = private / "session"
        trace = private / "trace.jsonl"
        env = os.environ.copy()
        env.pop("SURFACE_BRIDGE_SESSION", None)
        if plan.headless:
            env.pop("DISPLAY", None)
            env.pop("WAYLAND_DISPLAY", None)
        env.update({
            "SURFACE_TRACE": str(trace),
            "SURFACE_DEVTOOLS_SESSION": str(session),
            "SURFACE_REDUCED_MOTION": "1",
        })
        with (private / "gallery.log").open("wb") as log:
            proc = subprocess.Popen(command, env=env, stdout=log, stderr=subprocess.STDOUT)
            try:
                app = Gallery(plan, session, proc)
                deadline = time.monotonic() + 22
                while time.monotonic() < deadline:
                    require(proc.poll() is None, f"{name} Gallery exited before readiness")
                    if session.is_file():
                        # Never send a token from a session readable by other users.
                        require(stat.S_IMODE(session.stat().st_mode) == 0o600, "Session mode is not 0600")
                        try:
                            snapshot = app.dom()
                        except (OSError, ValueError, RuntimeError):
                            time.sleep(0.1)
                            continue
                        break
                    time.sleep(0.1)
                else:
                    raise RuntimeError(f"{name} Gallery did not become ready")
                viewport = snapshot["layout"]["viewport"]
                require((viewport["width"], viewport["height"]) == (width, height),
                        f"{name} logical viewport was {viewport}")
                caps = app.cli(["capabilities"])
                require("Page.screenshot" in caps["methods"], "Production screenshot capability is missing")
                yield app, trace
            finally:
                if proc.poll() is None:
                    proc.terminate()
                    try:
                        proc.wait(timeout=4)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait(timeout=4)


def focused(snapshot):
    return snapshot["interaction"]["focused"]


def focus_by_tab(app, target):
    steps = []
    for _ in range(36):
        app.key("tab")
        current_focus = focused(app.dom())
        steps.append(current_focus)
        if current_focus == target:
            return steps
    raise RuntimeError(f"Tab did not reach {target}: {steps!r}")


def same_box(parent, child, tolerance=0.5):
    return all(child[k] >= parent[k] - tolerance
               and child[k] + child[d] <= parent[k] + parent[d] + tolerance
               for k, d in (("x", "width"), ("y", "height")))
