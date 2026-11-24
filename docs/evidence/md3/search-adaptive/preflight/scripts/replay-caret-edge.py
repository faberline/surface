"""Compare the visible caret at the end and between RTL Search glyphs."""

import argparse
import json
from pathlib import Path
import runpy
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import focus_by_tab, launch
from render_helpers import activate, capture, focus, key, settle, visible
from window_check import onscreen_windows

S = runpy.run_path(str(Path(__file__).with_name("replay-alignment.py")))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("gallery", "cli", "manifest", "out"):
        parser.add_argument(f"--{name}", type=Path, required=True)
    args = parser.parse_args()
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()), "Output must be in /tmp")
    binaries = {name: getattr(args, name).resolve(strict=True) for name in ("gallery", "cli")}
    pins = S["verify_binaries"](args.manifest, binaries)
    plan = SimpleNamespace(**binaries, out=out)
    out.mkdir(parents=True)
    query = S["QUERY"]
    editor_id = S["INPUT"]
    windows = []

    def count(app):
        value = onscreen_windows(app.proc.pid)
        require(value == 0, "Visible window during the headless caret replay")
        windows.append(value)

    with launch(plan, "compact-rtl-caret", 320, 400) as app:
        count(app)
        S["SUPPORT"]["enter"](app, True)
        activate(app, f"{S['BASE']}-style")
        focus_by_tab(app, S["OPENER"])
        focus(key(app, "enter"), editor_id)
        app.wire("Action.fill", {"id": editor_id, "value": query})
        settle(app)
        captures = []
        images = []
        selections = []
        for command, offset, name in (("end", len(query.encode()), "rtl-end-caret"),
                                      ("left", len(query[0].encode()), "rtl-middle-caret"),
                                      ("end", len(query.encode()), "rtl-end-caret-returned")):
            frame = key(app, command)
            focus(frame, editor_id)
            editor = frame["text_inputs"][editor_id]
            require(editor["value"] == query and editor["selection_cursor"] == editor["selection_anchor"] == offset,
                    f"Caret command did not leave the exact query and offset {offset}")
            captures.append(capture(app, plan, name))
            images.append(read_png((out / captures[-1]["name"]).read_bytes())[2])
            selections.append(offset)
            count(app)
        x, y, width, height = S["extent"](visible(frame, editor_id))
    require(pins == S["verify_binaries"](args.manifest, binaries), "Binaries changed")
    require(captures[0]["sha256"] == captures[2]["sha256"], "End caret repaint did not reproduce")
    require(captures[0]["sha256"] != captures[1]["sha256"], "Moving the caret did not visibly change any pixels")
    at_end, middle, _ = images
    changes = [(i, j) for j in range(y, y + height) for i in range(x, x + width)
               if S["pixel"](at_end, i, j) != S["pixel"](middle, i, j)]
    # At 1x this probes the inner edge of the 2 dp focus stroke. Exclude the
    # corners where the rounded shape leaves a few end-caret pixels exposed.
    edge = [(i, j) for i, j in changes if i >= x + width - 16]
    straight_edge = [(i, j) for i, j in edge if y + 6 <= j < y + height - 6]
    interior = [(i, j) for i, j in changes if i < x + width - 16]
    require(len(interior) >= 8, "The interior caret must visibly move")
    report = {"binary_sha256": pins, "editor_bounds_xywh": [x, y, width, height],
              "verified_query": query, "selection_offsets": selections,
              "visible_windows": windows,
              "transport_equality": ["Page.screenshot", "CLI", "MCP"], "captures": captures,
              "changed_middle_region_pixels": len(interior), "changed_right_edge_pixels": len(edge),
              "changed_right_edge_positions_xy": edge,
              "changed_right_straight_edge_pixels": len(straight_edge)}
    (out / "caret-edge-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"Middle region changed at {len(interior)} pixels. Straight right edge changed at {len(straight_edge)} pixels.")


if __name__ == "__main__":
    main()
