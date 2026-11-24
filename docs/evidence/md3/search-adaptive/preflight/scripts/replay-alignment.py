"""Replay Search alignment against pinned native headless Gallery and CLI binaries."""

import argparse
import json
from pathlib import Path
import re
import runpy
from tempfile import TemporaryDirectory
from types import SimpleNamespace

from gallery_transport import read_png, require
from native_runtime import digest, focus_by_tab, launch
from render_helpers import capture, focus, key, node, settle, visible, activate
from window_check import onscreen_windows

HERE = Path(__file__).resolve().parent
SUPPORT = runpy.run_path(str(HERE / "replay-search.py"))
PLACEHOLDER = runpy.run_path(str(HERE / "replay-placeholder.py"))
BASE, INPUT, OPENER, BAR, BACK, CLEAR = (
    SUPPORT[name] for name in ("BASE", "INPUT", "OPENER", "BAR", "BACK", "CLEAR")
)
QUERY = "設計"


def extent(box):
    return tuple(round(box[name]) for name in ("x", "y", "width", "height"))


def pixel(rows, x, y):
    return rows[y][4 * x:4 * x + 4]


def glyphs(rows, editor, color):
    x, y, width, height = extent(editor)
    points = [(i, j) for j in range(y + 3, y + height - 3)
              for i in range(x, x + width) if pixel(rows, i, j) == color]
    require(len(points) >= 16, "The field has too little exact glyph ink")
    xs, ys = zip(*points)
    require(max(xs) - min(xs) >= 8, "The exact ink is too narrow to be the query or hint")
    return {
        "ink_pixels": len(points), "ink_x_inclusive": [min(xs), max(xs)],
        "ink_y_inclusive": [min(ys), max(ys)], "left_edge_gap_px": min(xs) - x,
        "right_edge_gap_px": x + width - 1 - max(xs),
    }


def editor_hint(frame):
    return bytes(node(frame, "search-doc-design-supporting")["style"]["foreground"])


def around_editor_focus(rows, neutral, editor, color):
    x, y, width, height = extent(editor)
    focus_differences = [(i, j) for j in range(y, y + height) for i in range(x, x + width)
                         if pixel(rows, i, j) != pixel(neutral, i, j)]
    require(len(focus_differences) > width, "No painted focus on the editor")
    top = [i for i in range(x, x + width) if pixel(rows, i, y) == color]
    require(len(top) > width / 2, "The top focus stroke has no continuous solid span")
    cut = [min(top) - x, x + width - 1 - max(top)]
    return focus_differences, cut


def painted_pill(rows, bar, color):
    x, y, width, height = extent(bar)
    solid = {}
    for scanline in range(y, y + height):
        matches = [i for i in range(x, x + width) if pixel(rows, i, scanline) == color]
        require(matches, f"The pill has no solid painted fill at y={scanline}")
        solid[scanline] = (min(matches), max(matches))
    return solid


def outside_solid_span(points, pill):
    return [(x, y) for x, y in points if not pill[y][0] <= x <= pill[y][1]]


def capture_state(app, plan, name, editor, *, keep):
    # Focus the neighboring Back control for the pixel-only reference. The
    # editor keeps its value and bounds while its caret and outline disappear.
    with TemporaryDirectory(dir=plan.out) as scratch:
        temporary = SimpleNamespace(**{**vars(plan), "out": Path(scratch)})
        destination = plan if keep else temporary
        record = capture(app, destination, name)
        width, height, focused_rows = read_png((destination.out / record["name"]).read_bytes())
        reference = key(app, "tab", reverse=True)
        focus(reference, BACK)
        require(visible(reference, INPUT) == editor, "Moving focus changed the field bounds")
        probe = capture(app, temporary, name + "-back-focused")
        w, h, neutral = read_png((temporary.out / probe["name"]).read_bytes())
        require((w, h) == (width, height), "Reference dimensions changed")
        focus(key(app, "tab"), INPUT)
    return record, focused_rows, neutral, probe["sha256"]


def verify_state(app, plan, name, frame, editor, *, rtl, typed, keep):
    text_color = bytes(node(frame, INPUT)["style"]["foreground"])
    expected_color = text_color if typed else editor_hint(frame)
    if typed:
        text = frame["text_inputs"][INPUT]
        require(text["value"] == text["display_value"] == node(frame, INPUT)["text"] == QUERY,
                "The typed Search query was not exported as the actual value")
        require(node(frame, INPUT)["name"] == "搜尋文件" and node(frame, INPUT)["role"] == "TextInput",
                "The editor lost its stable accessible label or role")
        require((visible(frame, CLEAR)["width"], visible(frame, CLEAR)["height"]) == (48, 48),
                "The populated search lost its Clear target")
    else:
        PLACEHOLDER["check_empty"](frame)
    shot, focus_rows, neutral, neutral_sha = capture_state(app, plan, name, editor, keep=keep)
    measured = glyphs(neutral, editor, expected_color)
    edge = "right_edge_gap_px" if rtl else "left_edge_gap_px"
    far = "left_edge_gap_px" if rtl else "right_edge_gap_px"
    inset = plan.horizontal_content_inset
    require(inset <= measured[edge] <= inset + (12 if rtl else 6),
            f"Unexpected logical start in {name}: {measured}")
    require(measured[far] > 48, f"No independently distinguishable far edge in {name}: {measured}")
    x0, x1 = measured["ink_x_inclusive"]
    y0, y1 = measured["ink_y_inclusive"]
    measured["focused_frame_glyph_pixels"] = sum(
        pixel(neutral, x, y) == expected_color == pixel(focus_rows, x, y)
        for y in range(y0, y1 + 1) for x in range(x0, x1 + 1)
    )
    require(measured["focused_frame_glyph_pixels"] == measured["ink_pixels"],
            "The editor glyph ink moved or disappeared when focused")
    expected_center = editor["y"] + editor["height"] / 2
    ink_center = (sum(measured["ink_y_inclusive"]) + 1) / 2
    require(abs(ink_center - expected_center) <= 4, "Horizontal alignment broke vertical centering")
    changed, top_cut = around_editor_focus(focus_rows, neutral, editor, text_color)
    require(all(1 <= inset < 8 if rtl else 10 <= inset <= 24 for inset in top_cut),
            f"The expected {'4' if rtl else '24'} dp outline rounding is absent: {top_cut}")
    return {
        "editor": editor, "glyphs": measured, "glyph_rgba": list(expected_color),
        "screenshot": shot, "published_capture": keep, "back_focused_reference_sha256": neutral_sha,
        "focus_diff_pixels": len(changed), "top_stroke_end_insets_px": top_cut,
    }, neutral, changed


def baseline(preserved, profile, editor, hint_color):
    old = next(record for record in preserved["profiles"] if record["profile"] == profile)
    old_capture = next(saved for saved in old["captures"] if saved["name"].endswith("-empty.png"))
    path = preserved["path"] / old_capture["name"]
    require(digest(path) == old_capture["sha256"], f"Baseline bytes changed: {path}")
    require(old["empty_input"] == editor and bytes(old["hint_color"]) == hint_color,
            f"Baseline and current render use different editor bounds or colors in {profile}")
    _, _, rows = read_png(path.read_bytes())
    return rows, old_capture["sha256"], glyphs(rows, editor, hint_color)


def replay(plan, compact, preserved):
    width, height = (320, 400) if compact else (960, 800)
    profile = "compact-dark-rtl-200-divided" if compact else "docked-light-ltr-contained"
    windows = []

    def count(app):
        hits = onscreen_windows(app.proc.pid)
        require(hits == 0, "The headless process opened a visible window")
        windows.append(hits)

    with launch(plan, profile, width, height) as app:
        count(app)
        SUPPORT["enter"](app, compact)
        if compact:
            activate(app, f"{BASE}-style")
        focus_by_tab(app, OPENER)
        opening = key(app, "enter")
        measured = SUPPORT["check_open"](opening, width, height, compact, compact)
        editor = measured["editor"]
        require(editor["height"] == 48, "The input changed its 48 dp hit target")
        empty, neutral, focus_pixels = verify_state(
            app, plan, f"{profile}-empty", opening, editor, rtl=compact, typed=False, keep=True,
        )
        count(app)
        old_rows, old_sha, old_ink = baseline(preserved, profile, editor, editor_hint(opening))
        comparison = {"baseline_empty_png_sha256": old_sha, "baseline_empty_glyphs": old_ink}
        if compact:
            require(old_ink["left_edge_gap_px"] <= 6 and old_ink["right_edge_gap_px"] > 48,
                    "The preserved compact baseline did not reproduce the original physical-left hint")
            require(empty["glyphs"]["ink_y_inclusive"] == old_ink["ink_y_inclusive"],
                    "Moving the compact placeholder changed its vertical ink bounds")
        else:
            ink = empty["glyphs"]
            require(ink["ink_x_inclusive"] == [x + plan.horizontal_content_inset for x in old_ink["ink_x_inclusive"]]
                    and ink["ink_y_inclusive"] == old_ink["ink_y_inclusive"]
                    and ink["ink_pixels"] == old_ink["ink_pixels"],
                    "The LTR hint changed beyond the declared horizontal inset")
            bar_color = bytes(node(opening, BAR)["style"]["background"])
            pill = painted_pill(neutral, measured["bar"], bar_color)
            new_outside = outside_solid_span(focus_pixels, pill)
            old_color = bytes(node(opening, INPUT)["style"]["foreground"])
            x, y, w, h = extent(editor)
            old_focus = [(i, j) for j in range(y, y + h) for i in range(x, x + w)
                         if pixel(old_rows, i, j) == old_color]
            old_outside = outside_solid_span(old_focus, pill)
            require(old_outside, "The old ring's painted-pill crossing was not reproduced")
            require(not new_outside, f"Updated focus paint escaped the painted pill: {new_outside[:8]}")
            comparison.update({"baseline_focus_ink_outside_pill": len(old_outside),
                               "baseline_outside_sample_xy": old_outside[0],
                               "candidate_focus_pixels_outside_pill": len(new_outside)})

        app.wire("Action.fill", {"id": INPUT, "value": QUERY})
        populated = settle(app)
        focus(populated, INPUT)
        typed, _, _ = verify_state(app, plan, f"{profile}-typed", populated, visible(populated, INPUT),
                                   rtl=compact, typed=True, keep=compact)
        count(app)
        restored = activate(app, CLEAR)
        require(visible(restored, INPUT) == editor, "Clear moved the editor hit area")
        cleared, _, _ = verify_state(app, plan, f"{profile}-cleared", restored, editor,
                                    rtl=compact, typed=False, keep=compact)
        require(cleared["screenshot"]["sha256"] == empty["screenshot"]["sha256"],
                "Clear did not restore the exact empty pixels or outline")
        focus(key(app, "escape"), OPENER)
        count(app)
    return {"profile": profile, "viewport": [width, height], "visible_window_counts": windows,
            "empty": empty, "typed": typed, "cleared": cleared, "baseline": comparison}


def verify_binaries(manifest, binaries):
    entries = {}
    for line in manifest.read_text().splitlines():
        sha, filename = line.split(maxsplit=1)
        require(re.fullmatch("[a-f0-9]{64}", sha), "Invalid pinned binary digest")
        entries[Path(filename.strip().lstrip("*")).name] = sha
    values = {}
    for name, binary in binaries.items():
        values[name] = digest(binary)
        require(entries.get(binary.name) == values[name], f"Unpinned or changed {name} binary")
    return values


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for arg in ("gallery", "cli", "manifest", "baseline", "out"):
        parser.add_argument(f"--{arg}", type=Path, required=True)
    parser.add_argument("--horizontal-content-inset", type=int, default=0,
                        help="Expected deliberate per-side horizontal inset in logical pixels")
    args = parser.parse_args()
    require(args.horizontal_content_inset >= 0, "The expected content inset cannot be negative")
    gallery, cli, manifest, old = (p.resolve(strict=True) for p in (args.gallery, args.cli, args.manifest, args.baseline))
    out = args.out.resolve()
    require(out.is_relative_to(Path("/tmp").resolve()) and not out.is_relative_to(old), "Use a separate, new /tmp output directory")
    manifest_sha = digest(manifest)
    binaries = {"gallery": gallery, "cli": cli}
    pins = verify_binaries(manifest, binaries)
    baseline_report = old / "native-blank-search-report.json"
    preserved = {**json.loads(baseline_report.read_text()), "path": old}
    plan = SimpleNamespace(gallery=gallery, cli=cli, out=out, horizontal_content_inset=args.horizontal_content_inset)
    out.mkdir(parents=True)
    profiles = [replay(plan, compact, preserved) for compact in (False, True)]
    require(pins == verify_binaries(manifest, binaries) and manifest_sha == digest(manifest), "The binary seal changed")
    require(len(list(out.glob("*.png"))) == 4, "The verifier did not retain exactly four representative PNGs")
    report = {
        "schema": "surface.test.search-alignment-native.v1", "headless": True,
        "binary_manifest_sha256": manifest_sha, "binary_sha256": pins,
        "horizontal_content_inset_dp": args.horizontal_content_inset,
        "baseline_manifest_sha256": digest(baseline_report),
        "transport_equality": ["Page.screenshot", "CLI", "MCP"], "profiles": profiles,
    }
    (out / "native-search-alignment-report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(f"PASS four PNGs, two headless profiles, and exact glyph/focus geometry: {out}")


if __name__ == "__main__":
    main()
