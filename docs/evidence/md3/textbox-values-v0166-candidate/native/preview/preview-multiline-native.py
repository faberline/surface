#!/usr/bin/env python3
import json
from pathlib import Path
import sys
from types import SimpleNamespace

sys.path.insert(0, "/tmp/surface-md3-counter-native-r2.NqEd6dD0")
from gallery_transport import require
from native_runtime import focus_by_tab, launch
from render_helpers import activate, capture, focus, key, layout, node, settle
from window_check import onscreen_windows

pin_dir, out = (Path(arg).resolve() for arg in sys.argv[1:])
assert pin_dir.is_relative_to(Path("/private/tmp"))
assert out.is_relative_to(Path("/private/tmp"))
out.mkdir(exist_ok=False)
binary = pin_dir / "binaries"
plan = SimpleNamespace(
    gallery=binary / "surface-material-gallery",
    cli=binary / "surface-devtools",
    out=out,
)
prefix = "multiline-text-field-demo-"
grow = prefix + "grow"
fixed = prefix + "fixed"
outcomes = []
for profile, width in (("wide", 960), ("compact", 320)):
    samples = []
    shots = []
    with launch(plan, profile, width, 900) as app:
        def sample():
            visible = onscreen_windows(app.proc.pid)
            require(visible == 0, "Native test window became visible")
            samples.append(visible)

        sample()
        frame = activate(app, prefix + "show")
        focus(frame, prefix + "back")
        if profile == "compact":
            for setting in ("theme", "rtl", "large-text"):
                activate(app, prefix + setting)
        focus_by_tab(app, grow)
        frame = settle(app)
        before = node(frame, grow)["text"]
        grow_box = layout(frame, grow + "-value-box")["bounds"]
        fixed_box = layout(frame, fixed + "-value-box")["bounds"]
        require(fixed_box["height"] == 3 * grow_box["height"], "Fixed row count differs")
        shots.append(capture(app, plan, profile + "-outlined-r1"))
        sample()
        frame = key(app, "enter")
        focus(frame, grow)
        require(node(frame, grow)["text"] == before + "\n", "Enter did not add exactly one line")
        require(not node(frame, prefix + "feedback").get("text"), "Enter submitted")
        enlarged = layout(frame, grow + "-value-box")["bounds"]["height"]
        require(enlarged == 2 * grow_box["height"], "Grow did not add the empty final row")
        activate(app, prefix + "appearance")
        focus_by_tab(app, fixed)
        frame = settle(app)
        require(layout(frame, fixed + "-value-box")["bounds"]["height"] == fixed_box["height"],
                "Changing appearance changed the number of fixed rows")
        shots.append(capture(app, plan, profile + "-filled-r1"))
        sample()
        frame = activate(app, prefix + "submit")
        require(bool(node(frame, prefix + "feedback").get("text")), "Submit did not send")
        sample()
    require(app.proc.poll() is not None, "Gallery was not reaped")
    outcomes.append({"profile": profile, "width": width, "samples": samples, "shots": shots,
                     "reaped": True, "row_height": grow_box["height"], "grow_after_enter": enlarged})
(out / "preview.json").write_text(json.dumps({"transport_bytes_match": True, "profiles": outcomes}, indent=2))
print(json.dumps({"profiles": len(outcomes), "original_pngs": sum(len(x["shots"]) for x in outcomes),
                  "window_samples": sum(len(x["samples"]) for x in outcomes)}, sort_keys=True))
