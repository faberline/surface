#!/usr/bin/env python3
"""Check the native replay helper with stored originals and synthetic mutations."""

import dis
import json
from pathlib import Path
from runpy import run_path
import subprocess
from tempfile import TemporaryDirectory
from types import CodeType, FunctionType

ROOT = Path(__file__).resolve().parent
Q70 = Path("/tmp/surface-md3-field-boundary-v0170.X35mBqoS")
Q71 = Path("/tmp/surface-md3-field-contrast-v0171.xE0ruhQh")
REPLAY = run_path(str(ROOT / "replay-disabled-contrast.py"))
BASE = REPLAY["BASE"]
R1 = Path("/tmp/surface-md3-field-disabled-contrast-native.9p4yBUuy")


def must_reject(fn, detail):
    try:
        fn()
    except RuntimeError as error:
        assert detail in str(error), str(error)
        return str(error)
    raise AssertionError(f"Wrong positive result for {detail}")


def missing_globals(namespace):
    """Check globals throughout the replay module and nested functions."""
    module = namespace["__name__"]
    defined = set(namespace) | set(namespace["__builtins__"])
    instructions = 0
    missing = set()

    def visit(code):
        nonlocal instructions
        for instruction in dis.get_instructions(code):
            if instruction.opname == "LOAD_GLOBAL":
                instructions += 1
                if instruction.argval not in defined:
                    missing.add((code.co_name, instruction.argval))
        for child in code.co_consts:
            if isinstance(child, CodeType):
                visit(child)

    for value in namespace.values():
        if isinstance(value, FunctionType) and value.__module__ == module:
            visit(value.__code__)
    return instructions, sorted(missing)


def launcher_controls():
    original = run_path(str(R1 / "replay-disabled-contrast.py"))
    prior, prior_missing = missing_globals(original)
    checked, missing = missing_globals(REPLAY)
    assert prior_missing == [("play", "focus_by_tab")] and not missing
    expected_helpers = {
        "gallery_transport": ("read_png", "require"),
        "native_runtime": ("digest", "focus_by_tab", "focused", "launch", "same_box"),
        "render_helpers": ("activate", "capture", "find", "focus", "key", "node", "settle", "visible"),
        "window_check": ("onscreen_windows",),
    }
    for module, names in expected_helpers.items():
        for name in names:
            assert callable(REPLAY[name]) and REPLAY[name].__module__ == module, name
    for name in ("bottom", "crop", "near", "pins", "pixel", "right"):
        assert callable(REPLAY["PRIOR"][name]), name
    return {"global_instructions_checked": checked, "r1_global_instructions": prior,
            "r1_missing": prior_missing, "r3_missing": missing,
            "production_helpers_checked": expected_helpers}


def shell_controls():
    results = {}
    with TemporaryDirectory(prefix="forced-colors-argv-", dir=ROOT) as temp:
        path = Path(temp)
        # This shell fixture only records argv. It does not start a Gallery.
        echo = path / "fixture arguments only"
        echo.write_text("#!/bin/sh\nprintf '%s\\n' \"$@\"\n")
        echo.chmod(0o700)
        for profile, (width, height, mode, _, _) in REPLAY["PROFILES"].items():
            guard = path / f"guard-{profile}"
            guard.write_text(REPLAY["guard_program"](echo, profile))
            guard.chmod(0o700)
            subprocess.run(["/bin/sh", "-n", guard], check=True)
            good = ["--headless", "--width", str(width), "--height", str(height)]
            args = [good[1:], [*good, "--forced-colors", mode], [*good[:3], "--smoke", str(height)]]
            for bad in args:
                result = subprocess.run([guard, *bad], capture_output=True)
                assert result.returncode == 71 and result.stdout == result.stderr == b""
            actual = subprocess.run([guard, *good], text=True, capture_output=True, check=True)
            expected = [*good, "--forced-colors", mode]
            assert actual.stdout.splitlines() == expected and not actual.stderr
            results[profile] = {"confirmed": expected, "bypasses_rejected": len(args)}
    return results


def fake_frame(profile, form, phase):
    width, height, mode, rtl, scale = REPLAY["PROFILES"][profile]
    canvas, active, gray = REPLAY["COLORS"][mode]
    rows = [bytearray(bytes(canvas) * width) for _ in range(height)]
    regions = {}

    def glyph(rect, color):
        size = 12
        x0 = int(rect["x"] + (rect["width"] - size - 4 if rtl else 4))
        y0 = int(rect["y"] + rect["height"] / 2) - 3
        for y in range(y0, y0 + 6):
            rows[y][4*x0:4*(x0+size)] = bytes(color) * size

    for part, y, ink in (("disabled", 240, gray), ("read-only", 132, active)):
        height = (104 if form == "filled" else 80) if rtl else 56
        face = {"x":24, "y":y, "width":width - 48 if rtl else 504,
                "height":height}
        label = {"x": (face["x"] + face["width"] - 16 - 80 if rtl else 40),
                 "y": y + (8 if form == "filled" else -8 * scale), "width":80, "height":16 * scale}
        body = REPLAY["body_rect"](face, scale, form)
        regions[part] = {"face":face, "label":label, "body":body}
        if part == "disabled" and form == "filled":
            paper = REPLAY["blend_paper"](canvas, active, 10 / 255)
            x0, x1 = face["x"], face["x"] + face["width"]
            for row in rows[y:y + face["height"]]:
                row[4*x0:4*x1] = bytes(paper) * (x1 - x0)
        glyph(label, ink)
        glyph(body, ink)
        if part == "read-only":
            bottom = y + face["height"]
            thickness = 3 if phase == "read-only-focused" else 1
            for row in rows[bottom - thickness:bottom]:
                x0, x1 = face["x"], face["x"] + face["width"]
                row[4*x0:4*x1] = bytes(active) * (x1 - x0)
    return rows, regions, (canvas, active, gray)


def pixel_controls():
    check = REPLAY["verify_paint"]
    results = []
    for profile in REPLAY["PROFILES"]:
        for form in REPLAY["FORMS"]:
            for phase in REPLAY["PHASES"]:
                rows, regions, _ = fake_frame(profile, form, phase)
                actual = check(rows, regions, profile, form, phase)
                assert all(i["exact_pixels"] == 72 for part in REPLAY["LOCKED"] for i in actual[part].values())
                results.append((profile, form, phase))
    p, f, s = "compact-dark-rtl", "filled", "read-only-focused"
    checks = {}
    for role, region, key in (("disabled", "body", "wrong_disabled_value"),
                               ("disabled", "label", "wrong_disabled_label"),
                               ("read-only", "body", "wrong_readonly_value")):
        rows, regions, (_, active, gray) = fake_frame(p, f, s)
        target, replacement = (gray, active) if role == "disabled" else (active, gray)
        box = regions[role][region]
        for y in range(int(box["y"]), int(box["y"] + box["height"])):
            rows[y][4*int(box["x"]):4*int(box["x"]+box["width"])] = rows[y][
                4*int(box["x"]):4*int(box["x"]+box["width"])].replace(bytes(target), bytes(replacement))
        failure = "pixels blended" if region == "label" else "exact"
        checks[key] = must_reject(lambda: check(rows, regions, p, f, s), f"{role}-{region} has too few {failure}")
    rows, regions, _ = fake_frame(p, f, "idle")
    checks["idle_boundary_as_focus"] = must_reject(lambda: check(rows, regions, p, f, s),
                                                  "lost 3 dp CanvasText boundary")
    rows, regions, _ = fake_frame(p, f, s)
    regions["disabled"]["body"]["x"] += 3
    checks["shifted_body_bounds"] = must_reject(lambda: check(rows, regions, p, f, s),
                                               "measured glyph bounds or scale changed")
    rows, regions, _ = fake_frame(p, f, s)
    body = regions["disabled"]["body"]
    left, top, right, bottom = REPLAY["PRIOR"]["crop"](body, len(rows[0]) // 4, len(rows))
    for y in range(top, bottom):
        line = rows[y][4*left:4*right]
        rgba = [line[i:i+4] for i in range(0, len(line), 4)]
        rows[y][4*left:4*right] = b"".join(reversed(rgba))
    checks["mirrored_to_wrong_side"] = must_reject(lambda: check(rows, regions, p, f, s),
                                                  "disabled-body painted on the wrong logical side")
    return {"synthetic_positive_profiles_forms_phases": len(results), "rejections": checks}


def recorded_caption():
    png = ROOT / "r2-wide-light-outlined-idle.png"
    sha = "cbea94f34801da7bc0ee674955c53c679353b8ec99d111a5834eb0ca6419775e"
    assert REPLAY["digest"](png) == sha
    canvas, active, gray = REPLAY["COLORS"]["light"]
    w, h, rows = REPLAY["read_png"](png.read_bytes())
    assert (w, h) == (960, 900)
    label = {"x":38, "y":320, "width":59, "height":24}
    body = {"x":39, "y":348, "width":84, "height":26}
    sample = REPLAY["pixel_mask"]
    solid_rejection = must_reject(lambda: sample(rows, label, gray, active, False, "disabled-label"),
                                  "Got 0")
    actual_label = sample(rows, label, gray, active, False, "disabled-label", paper=canvas)
    actual_body = sample(rows, body, gray, active, False, "disabled-body")
    assert actual_label["exact_pixels"] == 0 and actual_label["caption_fit_pixels"] == 176
    assert actual_label["caption_strong_pixels"] == 47 and actual_body["exact_pixels"] == 152
    rejections = {}
    for key, new_ink in (("wrong_red", (105, 0, 0, 255)), ("grayscale_fallback", (45, 45, 45, 255))):
        altered = [bytearray(row) for row in rows]
        for y in range(label["y"], label["y"] + label["height"]):
            for x in range(label["x"], label["x"] + label["width"]):
                value = REPLAY["PRIOR"]["pixel"](rows, x, y)
                coverage = REPLAY["caption_coverage"](value, gray, canvas, 1)
                if coverage > 0.:
                    altered[y][4*x:4*x+4] = bytes(REPLAY["blend_paper"](canvas, new_ink, coverage))
        rejections[key] = must_reject(lambda: sample(altered, label, gray, active, False, "disabled-label",
                                                   paper=canvas), "too few pixels blended")
    return {"source_png_sha256": sha, "label": actual_label, "body": actual_body,
            "old_solid_check": solid_rejection, "wrong_ink_rejections": rejections}


def main():
    REPLAY["checked_base"]()
    former_report = json.loads((Q70 / "native-v0170-verified.json").read_text())
    old = run_path(str(BASE / "replay-boundaries.py"))
    actual = old["verify_stored"]((Q70 / "native-exact-replay").resolve(), former_report["source_manifest_sha256"],
                                  former_report["binary_manifest_sha256"],
                                  former_report["replay_manifest_sha256"])
    assert actual == former_report
    s = "888bf446dba55df082f0f8c8c9181e5a76fbad76c63e3cc002c9f3d211d07ed8"
    b = "2932f41e240c5d8eb0898b1d40b2c41d6f1f278c301d80186df33c04ffc9be56"
    pins = subprocess.run([
        str(Path("/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3")),
        "-E", "-B", str(ROOT / "replay-disabled-contrast.py"), "--verify-inputs-only",
        "--source-root", str(Q71 / "development-snapshot-r1/source"),
        "--source-manifest", str(Q71 / "development-snapshot-r1/source.sha256"),
        "--source-manifest-sha", s,
        "--binary-manifest", str(Q71 / "native-development/SHA256SUMS"), "--binary-manifest-sha", b,
    ], capture_output=True, text=True, check=True)
    exact = json.loads(pins.stdout)
    assert exact["source_files"] == 690 and exact["source_manifest_sha256"] == s and exact["binary_manifest_sha256"] == b
    print(json.dumps({"q70_prior_actual_sealed_readback": actual, "q71_exact_inputs_only": exact,
                      "launcher_references": launcher_controls(), "shell_fixture_only": shell_controls(),
                      "synthetic_pixel_controls": pixel_controls(), "actual_r2_partial_screen": recorded_caption(),
                      "q71_runtime_exercised": False}, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
