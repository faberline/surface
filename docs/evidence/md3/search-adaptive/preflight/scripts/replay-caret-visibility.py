"""Verify the leading LTR and terminal RTL Search carets on headless Metal."""

import argparse
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import focus_by_tab, launch
from render_helpers import activate, capture, focus, key, node, settle, visible
from window_check import onscreen_windows

ALIGN = runpy.run_path(str(Path(__file__).with_name("replay-alignment.py")))
SUPPORT = ALIGN["SUPPORT"]
QUERY = ALIGN["QUERY"]
INPUT = ALIGN["INPUT"]


def column_witness(active, control, x_range, y_range, ink):
    """Find the longest uninterrupted vertical run produced only by the caret."""
    witness = None
    for x in x_range:
        streak = 0
        for y in y_range:
            a, b = (ALIGN["pixel"](image, x, y) for image in (active, control))
            if a != ink or b == ink:
                streak = 0
                continue
            streak += 1
            if witness is None or streak > witness["length_px"]:
                witness = {"x": x, "y_inclusive": [y - streak + 1, y], "length_px": streak,
                           "caret_rgba": list(a), "control_rgba": list(b)}
    return witness


def verify_profile(plan, compact):
    width, height = (320, 400) if compact else (960, 800)
    profile = "compact-dark-rtl-200-divided" if compact else "docked-light-ltr-contained"
    windows = []

    def count(app):
        value = onscreen_windows(app.proc.pid)
        require(value == 0, "A native window became visible")
        windows.append(value)

    with launch(plan, profile, width, height) as app:
        count(app)
        SUPPORT["enter"](app, compact)
        if compact:
            activate(app, f"{ALIGN['BASE']}-style")
        focus_by_tab(app, ALIGN["OPENER"])
        opening = key(app, "enter")
        SUPPORT["check_open"](opening, width, height, compact, compact)
        app.wire("Action.fill", {"id": INPUT, "value": QUERY})
        frame = settle(app)
        focus(frame, INPUT)
        bounds = visible(frame, INPUT)
        require(bounds["height"] == 48, "The editor hit area changed vertically")
        ink = bytes(node(frame, INPUT)["style"]["foreground"])
        edge_key, middle_key = ("end", "left") if compact else ("home", "right")
        edge_byte = len(QUERY.encode()) if compact else 0
        middle_byte = len(QUERY[0].encode())
        states = ((edge_key, edge_byte, "edge"), (middle_key, middle_byte, "middle"),
                  (edge_key, edge_byte, "edge-returned"))
        records = []
        images = []
        for command, location, label in states:
            frame = key(app, command)
            focus(frame, INPUT)
            editor = frame["text_inputs"][INPUT]
            require(visible(frame, INPUT) == bounds, "Key changed the editor hit area")
            require(editor["value"] == editor["display_value"] == QUERY and not editor["composing"],
                    "Caret movement changed the Search query")
            require(editor["selection_anchor"] == editor["selection_cursor"] == location,
                    f"Wrong caret offset at {label}: {editor}")
            record = capture(app, plan, f"{profile}-{label}")
            records.append({"caret_byte": location, **record})
            images.append(read_png((plan.out / record["name"]).read_bytes())[2])
            count(app)

    edge, middle, _ = images
    require(records[0]["sha256"] == records[2]["sha256"], "Returning to the same caret did not reproduce the pixels")
    x, y, w, h = ALIGN["extent"](bounds)
    edge_strip = range(x + w - 16, x + w) if compact else range(x, x + 16)
    middle_strip = range(x, x + w - 16) if compact else range(x + 16, x + w)
    straight_rows = range(y + 6, y + h - 6)
    witnesses = {"edge": column_witness(edge, middle, edge_strip, straight_rows, ink),
                 "middle": column_witness(middle, edge, middle_strip, straight_rows, ink)}
    minimum = 24 if compact else 12
    for label, witness in witnesses.items():
        require(witness is not None and witness["length_px"] >= minimum
                and witness["y_inclusive"][0] <= y + h // 2 <= witness["y_inclusive"][1],
                f"{profile} {label} lacks a visibly separate caret: {witness}")
    return {"profile": profile, "editor_bounds": bounds, "query": QUERY, "captures": records,
            "visible_windows": windows, "separate_caret_pixels": witnesses}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("gallery", "cli", "manifest", "out"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Output must be in /tmp")
    binaries = {name: getattr(args, name).resolve(strict=True) for name in ("gallery", "cli")}
    pins = ALIGN["verify_binaries"](args.manifest, binaries)
    plan = SimpleNamespace(**binaries, out=out)
    out.mkdir(parents=True)
    profiles = [verify_profile(plan, compact) for compact in (True, False)]
    require(ALIGN["verify_binaries"](args.manifest, binaries) == pins, "Binary pins changed")
    report = {"schema": "surface.test.search-caret-visibility-native.v1", "binary_sha256": pins,
              "transport_equality": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles}
    (out / "native-search-caret-visibility-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"PASS both Search edge/middle carets paint a separate vertical stroke: {out}")


if __name__ == "__main__":
    main()
