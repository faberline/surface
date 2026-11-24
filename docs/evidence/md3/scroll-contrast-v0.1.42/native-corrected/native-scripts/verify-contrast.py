"""Headless verification for the disposable macOS contrast probe."""

import argparse
import json
import math
from pathlib import Path
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import activate, capture, focus, key, node, settle, visible
from window_check import onscreen_windows

PAGE = "scroll-popup-demo"
EDITOR = f"{PAGE}-quantity-editor"
ERROR = f"{PAGE}-quantity-error"
FACE = f"{PAGE}-quantity-face"
RING = f"{PAGE}-quantity-focus"
METHOD = "Test.setDisplayContrast"
THEMES = ("light", "dark")
REPORT_NAME = "native-contrast-report.json"
IMAGE_NAMES = {f"{theme}-{mode}.png" for theme in THEMES for mode in ("normal", "increased", "restored")}
IMAGE_NAMES.update(f"stock-{theme}-normal.png" for theme in THEMES)


def startup(app):
    schema = "surface.test.macos-display-contrast-startup.v1"
    log = (app.session_path.parent / "gallery.log").read_text().splitlines()
    records = [json.loads(line) for line in log if line.startswith("{") and schema in line]
    require(len(records) == 1 and records[0]["schema"] == schema, "Missing OS startup receipt")
    sample = records[0]["system_increased"]
    require(isinstance(sample, bool)
            and records[0]["accepted_mode"] == ("increased" if sample else "normal"),
            "The Runtime mode differs from the actual OS startup value")
    return records[0]


def set_mode(app, mode, redraw=None):
    result = app.wire(METHOD, {"mode": mode})
    require(result["schema"] == "surface.test.set-display-contrast.v1" and result["mode"] == mode,
            f"The Runtime did not accept {mode}")
    if redraw is not None:
        require(result["redraw"] == redraw, f"Unexpected redraw receipt for {mode}")


def draft(frame):
    focus(frame, EDITOR)
    editor = node(frame, EDITOR)
    text = frame["text_inputs"][EDITOR]
    error = node(frame, ERROR)["text"]
    saved = node(frame, f"{PAGE}-saved")["text"]
    require(editor["role"] == "TextInput" and editor["name"] == "數量"
            and editor.get("invalid") and editor.get("described_by_text") == ERROR and error,
            "The quantity field lost the invalid description")
    require(text["value"] == text["display_value"] == "1203" and not text["composing"],
            "The rejected draft disappeared")
    require(saved.startswith("已選 1200 "), f"The rejected draft was saved: {saved!r}")
    return {"focus": EDITOR, "text_input": text, "error": error, "saved": saved}


def changed_pixels(rect, previous, current):
    x0, x1 = math.ceil(rect["x"]), math.floor(rect["x"] + rect["width"])
    y0, y1 = math.ceil(rect["y"]), math.floor(rect["y"] + rect["height"])
    require(0 <= y0 < y1 <= len(previous) == len(current)
            and 0 <= x0 < x1 <= len(previous[0]) // 4 == len(current[0]) // 4,
            "The measured layout lies outside the paired capture")
    return sum(previous[y][x*4:x*4+4] != current[y][x*4:x*4+4]
               for y in range(y0, y1) for x in range(x0, x1))


def focus_ink(frame, rows):
    bounds = visible(frame, FACE)
    ring = visible(frame, RING)
    x = math.floor(bounds["x"] - 4.)
    y = math.floor(bounds["y"] + bounds["height"] / 2.)
    require(0 <= x and x + 3 < len(rows[0]) // 4 and 0 <= y < len(rows),
            "Focus sample is outside the frame")
    pixel = rows[y][x * 4:x * 4 + 4]
    expected = bytes(node(frame, RING)["focus_color"])
    gap = rows[y][(x + 3) * 4:(x + 4) * 4]
    require(pixel == expected and pixel != gap, "The focused field did not paint its outline and gap")
    return {"bounds": ring, "sample_xy": [x, y], "rgba": list(pixel)}


def edit_invalid_quantity(app, theme):
    activate(app, "section-0")
    activate(app, f"{PAGE}-show")
    if theme == "dark":
        activate(app, f"{PAGE}-theme")
    for setting in ("rtl", "large-text", "bottom"):
        frame = activate(app, f"{PAGE}-{setting}")
        require(node(frame, f"{PAGE}-{setting}")["selected"], f"Setting {setting} is inactive")
    require(bool(node(frame, f"{PAGE}-theme").get("checked")) == (theme == "dark"),
            f"The fixture did not use the {theme} color theme")
    frame = activate(app, f"{PAGE}-opener")
    if frame["interaction"]["focused"] != EDITOR:
        focus_by_tab(app, EDITOR)
    app.wire("Action.fill", {"id": EDITOR, "value": "1203"})
    return key(app, "enter")


def replay(plan, theme):
    observations = []

    def check_windows(app, stage):
        count = onscreen_windows(app.proc.pid)
        require(count == 0, f"A Gallery window appeared at {stage}")
        observations.append({"stage": stage, "visible_windows": count})

    with launch(plan, theme, 320, 400) as app:
        check_windows(app, "startup")
        sample = startup(app)
        require(METHOD not in app.cli(["capabilities"])["methods"],
                "The injection is listed as a public method")
        set_mode(app, "normal")
        normal = edit_invalid_quantity(app, theme)
        expected = draft(normal)
        first = capture(app, plan, f"{theme}-normal")
        check_windows(app, "normal capture")

        for params in ({}, {"mode": 42}, {"mode": "forced"}):
            app.wire(METHOD, params, expected_error="invalid_params")
        require(settle(app) == normal, "Malformed requests affected the Gallery")
        set_mode(app, "increased", True)
        increased = settle(app)
        require(draft(increased) == expected, "Contrast changed the draft or focus")
        set_mode(app, "increased", False)
        require(settle(app) == increased, "The same mode changed the frame")
        second = capture(app, plan, f"{theme}-increased")
        check_windows(app, "increased capture")

        set_mode(app, "normal", True)
        require(settle(app) == normal, "The restored DOM differs from the initial DOM")
        last = capture(app, plan, f"{theme}-restored")
        require(last["sha256"] == first["sha256"], "The restored PNG differs from the initial PNG")
        check_windows(app, "restored capture")
        previous, current = [read_png((plan.out / shot["name"]).read_bytes())[2]
                             for shot in (first, second)]
        pixels = {}
        for target in (f"{PAGE}-dialog", EDITOR, RING):
            rect = visible(normal, target)
            require(rect == visible(increased, target), "Contrast changed the layout")
            changed = changed_pixels(rect, previous, current)
            require(changed >= 16, f"No meaningful GPU pixel change in {target}")
            pixels[target] = {"bounds": rect, "changed_pixels": changed}
        check_windows(app, "complete")
        return {"theme": theme, "macos_startup": sample, "draft": expected,
                "captures": [first, second, last], "changed_regions": pixels,
                "focus_ink": {"normal": focus_ink(normal, previous),
                              "increased": focus_ink(increased, current)},
                "window_observations": observations}


def stock_replay(plan, theme, reference):
    with launch(plan, f"stock-{theme}", 320, 400) as app:
        before = onscreen_windows(app.proc.pid)
        require(before == 0, "A stock Gallery window appeared")
        require(METHOD not in app.cli(["capabilities"])["methods"],
                "Stock advertised the test-only method")
        frame = edit_invalid_quantity(app, theme)
        require(draft(frame) == reference["draft"], "The stock editor state differs from the probe")
        app.wire(METHOD, {"mode": "increased"}, expected_error="unknown_method")
        require(settle(app) == frame, "Stock changed after the rejected method")
        shot = capture(app, plan, f"stock-{theme}-normal")
        require((plan.out / shot["name"]).read_bytes()
                == (plan.out / reference["captures"][0]["name"]).read_bytes(),
                "Stock and probe Normal rendered different PNGs")
        after = onscreen_windows(app.proc.pid)
        require(after == 0, "A stock Gallery window appeared during capture")
        return {"theme": theme, "draft": reference["draft"], "capture": shot,
                "visible_windows": [before, after], "test_method": "unknown_method"}


def main():
    parser = argparse.ArgumentParser()
    for name in ("stock-gallery", "probe-gallery", "cli", "out"):
        parser.add_argument(f"--{name}", required=True, type=Path)
    options = parser.parse_args()
    cli = options.cli.resolve(strict=True)
    out = options.out.resolve()
    out.mkdir(parents=True)
    probe = SimpleNamespace(gallery=options.probe_gallery.resolve(strict=True), cli=cli, out=out)
    stock = SimpleNamespace(gallery=options.stock_gallery.resolve(strict=True), cli=cli, out=out)
    tested = [replay(probe, theme) for theme in THEMES]
    report = {
        "schema": "surface.test.macos-offscreen-contrast-release.v1",
        "stock_gallery_sha256": digest(stock.gallery), "probe_gallery_sha256": digest(probe.gallery),
        "cli_sha256": digest(cli),
        "headless_viewport": [320, 400], "png_transports": ["Page.screenshot", "CLI", "MCP"],
        "probe": tested,
        "stock": [stock_replay(stock, case["theme"], case) for case in tested],
    }
    require({path.name for path in out.iterdir()} == IMAGE_NAMES, "Unexpected native capture set")
    (out / REPORT_NAME).write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n")
    print("PASS eight transport-matched PNGs; stock matches probe Normal; fourteen window checks all zero")


if __name__ == "__main__":
    main()
