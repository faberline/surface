#!/usr/bin/env python3
"""Verify or pack the local v0.1.52 Chips proof without launching a test."""

import argparse
from hashlib import file_digest, sha1
import json
from math import ceil, floor
from pathlib import Path
import re
from runpy import run_path
import shutil
import stat
import subprocess
from tempfile import TemporaryDirectory

from PIL import Image


HERE = Path(__file__).resolve().parent
TEMP = Path("/tmp").resolve()
STAGE = TEMP / "surface-md3-chips-v0152.tHzYZc3R"
BASE = TEMP / "surface-md3-tabs-v0151.jUrwFmHq"
STOCK = TEMP / "surface-md3-chips-stock-observer.6d5si1tA"
BASE_SHA = "f928486f4fd04824a503424b0ebf53caa5b2784ba363dae928c7fca17c202cd0"
SOURCE_SHA = "8d2548019eca22046b20992b27f48f4404cb0f5af84f26d6d864d6de498509b1"
PATCH_SHA = "0f9a7f35624e31282329b1d07528f033b4e1b60708dbd01c33707f613175438b"
BASE_TREE = "187930ac394178c821649c76b0f29c2f9c751b9e"
CHIPS_TREE = "48d560ab127cc1c692a5ba0eba6fb5a8949234a8"
CASES = ("smoke", "ime", "material", "bridge-web", "devtools-headless")
REPLAYS = ("exact-v0152-r2-1", "exact-v0152-r2-2")
NATIVE_PNG = ("wide-light-chips.png", "compact-dark-rtl-200.png")
NATIVE_SCRIPTS = ("replay-chips.py", "gallery_transport.py", "native_runtime.py", "render_helpers.py", "window_check.py")
BRIEF_MARKERS = ("native-v0152-r2-built", "native-v0152-r2-passed", "native-chips-ax-passed")
PASS_MARKERS = ("workspace-v0152-passed", "gpu-v0152-exact-passed")
WORKSPACE_LOGS = ("workspace-source-before.log", "workspace-source-after.log", "workspace-v0152-exact.log")
PROFILE_FILES = (*NATIVE_PNG, "native-gallery-report.json")
STOCK_CODE = {
    "observe-chips.py": "a9c4b530a4a38b84d0b7d0e4bb53b41908bddf95925f085a4566288dace4dc95",
    "verify-existing.py": "22b89e022e41ca92b7e5094ae3f5cb51e643f32f87ecba04e873728b6e25dd06",
    "archive-proof.py": "ad5536e809b6da658a7d3fc0732ad0409812fdd82129358b64ef155ee83190ae",
    "retained/SHA256SUMS": "5a9c99e2bf64a76375d81d5f16ab58228410be56ba32225b01930df949a0ac6f",
}


class Failed(Exception):
    pass


def need(ok, message):
    if not ok:
        raise Failed(message)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def load(path):
    return json.loads(path.read_text())


def save(path, payload):
    path.write_text(json.dumps(payload, indent=2, ensure_ascii=False) + "\n")


def manifest(path):
    result = {}
    for line in path.read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  (.+)", line)
        need(match, f"bad checksum in {path.name}")
        checksum, name = match.groups()
        rel = Path(name)
        need(not rel.is_absolute() and ".." not in rel.parts and name == rel.as_posix()
             and name not in result, f"unsafe or duplicate checksum name: {name}")
        result[name] = checksum
    need(result, f"empty manifest {path.name}")
    return result


def verify(root, pins):
    for name, checksum in pins.items():
        path = root / name
        need(path.is_file() and not path.is_symlink() and sha(path) == checksum, f"file digest differs: {path}")


def git_hash(kind, data):
    return sha1(f"{kind} {len(data)}\0".encode() + data).digest()


def git_tree(root, files):
    """Recompute a Git object ID without touching the index or object store."""
    tree = {}
    for name in files:
        path = root / name
        parts = Path(name).parts
        item = tree
        for part in parts[:-1]:
            item = item.setdefault(part, {})
        mode = b"100755" if path.stat().st_mode & stat.S_IXUSR else b"100644"
        item[parts[-1]] = (mode, git_hash("blob", path.read_bytes()))

    def encode(subtree):
        # Git sorts directory names as though suffixed with a slash.
        names = sorted(subtree, key=lambda name: name.encode() + (b"/" if isinstance(subtree[name], dict) else b""))
        parts = []
        for name in names:
            item = subtree[name]
            mode, oid = (b"40000", encode(item)) if isinstance(item, dict) else item
            parts.append(mode + b" " + name.encode() + b"\0" + oid)
        return git_hash("tree", b"".join(parts))

    return encode(tree).hex()


def source_pins(root):
    verify(root, {"source.sha256": SOURCE_SHA, "chips-v0152.patch": PATCH_SHA})
    entries = manifest(root / "source.sha256")
    need(len(entries) == 624 and (root / "tabs-base-tree").read_text().strip() == BASE_TREE
         and (root / "chips-tree").read_text().strip() == CHIPS_TREE, "source or tree pin differs")
    patch = (root / "chips-v0152.patch").read_text()
    paths = re.findall(r"^diff --git a/(\S+) b/\1$", patch, re.M)
    need(len(paths) == len(set(paths)) == 24
         and paths == (root / "actual-chips.paths").read_text().splitlines(), "patch path inventory differs")
    return entries, paths


def inspect_source(stage, base):
    entries, paths = source_pins(stage)
    verify(base, {"source.sha256": BASE_SHA})
    old = manifest(base / "source.sha256")
    need(len(old) == 616 and not old.keys() - entries.keys(), "wrong Tabs base inventory")
    need({name for name, checksum in entries.items() if old.get(name) != checksum} == set(paths),
         "patch paths differ from changed source bytes")
    for root, files, tree in ((base / "source", old, BASE_TREE), (stage / "source", entries, CHIPS_TREE)):
        verify(root, files)
        need(git_tree(root, files) == tree, f"Git tree differs: {root}")
    verify(stage / "web-stock", entries)
    with TemporaryDirectory(prefix="patch-replay.", dir=HERE) as tmp:
        applied = Path(tmp)
        for name in old:
            copy(base / "source" / name, applied / name)
        with (stage / "chips-v0152.patch").open("rb") as patch:
            result = subprocess.run(["patch", "--batch", "--forward", "-p1"], cwd=applied,
                                    stdin=patch, capture_output=True, check=False)
        need(result.returncode == 0, f"portable patch failed: {result.stderr.decode(errors='replace')}")
        need(inventory(applied) == entries and git_tree(applied, entries) == CHIPS_TREE,
             "portable patch did not recreate the exact candidate")
    return {"source_files": len(entries), "changed_paths": len(paths), "source_sha256": SOURCE_SHA,
            "patch_sha256": PATCH_SHA, "tabs_base_tree": BASE_TREE, "chips_tree": CHIPS_TREE}


def png(path, size=None):
    with Image.open(path) as raw:
        need(raw.format == "PNG" and (size is None or raw.size == tuple(size)), f"wrong PNG format or size: {path.name}")
        image = raw.convert("RGBA")
        need(any(lo < hi for lo, hi in image.getextrema()), f"blank PNG: {path.name}")
    return image


def inspect_native(native, binary_dir=None):
    pins = manifest(native / "binary-sha256")
    need(pins.keys() == {"surface-material-gallery", "surface-devtools"}, "native binary inventory differs")
    if binary_dir is not None:
        verify(binary_dir, pins)
    report1, report2 = (native / run / "native-gallery-report.json" for run in ("replay-1", "replay-2"))
    need(sha(report1) == sha(report2), "native repeat report differs")
    report = load(report1)
    need(report["schema"] == "surface.test.chips.native.v1" and report["rendering"] == "headless"
         and report["binaries"] == {"gallery": pins["surface-material-gallery"], "cli": pins["surface-devtools"]}
         and report["transports"] == ["Page.screenshot", "CLI", "MCP"], "native replay provenance differs")
    profiles = report["profiles"]
    need(len(profiles) == 2, "both native profiles are required")
    captures = {}
    for profile, name, size, rtl in zip(profiles, NATIVE_PNG, ([960, 800], [320, 760]), (False, True), strict=True):
        photo = profile["png"]
        file1, file2 = (native / run / name for run in ("replay-1", "replay-2"))
        need(profile["viewport"] == size and profile["rtl"] is rtl and profile["text_scale"] == (2 if rtl else 1)
             and profile["windows"] == [0, 0, 0], "native profile or window samples differ")
        need(photo["name"] == name and photo["format"] == "png" and photo["scale"] == 1
             and [photo["width"], photo["height"]] == size and sha(file1) == sha(file2) == photo["sha256"],
             f"native PNG recurrence or metadata differs: {name}")
        need(profile["menu_trigger"] == "chip-demo-filter-type" + ("" if rtl else "-menu")
             and profile["focus_after_delete"] == "chip-demo-mei", "native input/focus limits differ")
        split = profile["split"]
        a, b = split["body"], split["trailing"]
        first, second = (b, a) if rtl else (a, b)
        need(min(a["height"], a["width"], b["height"], b["width"]) >= 48
             and abs(first["x"] + first["width"] - second["x"]) < 0.6
             and split["face"]["height"] >= 32, "native split targets or face were lost")
        image = png(file1, size)
        face = photo["face_pixels"]
        need(len(face) == (1 if rtl else 3), "native token samples incomplete")
        for sample in face.values():
            need(list(image.getpixel(tuple(sample["point"]))) == sample["rgba"], "native fill sample differs")
        label = photo["ink"]["chip-demo-lin-label" if rtl else "chip-demo-chen-label"]
        verify_native_ink(image, label, 80 if rtl else 20)
        captures[name] = {"size": size, "png_sha256": photo["sha256"], "ink_pixels": label["ink_pixels"]}
    return {"rendering": "headless", "runs": 2, "window_samples": 12, "visible_windows": 0,
            "transports": report["transports"], "report_sha256": sha(report1), "binaries": pins, "captures": captures}


def verify_native_ink(image, label, minimum_height):
    rect = label["bounds"]
    left, top = ceil(rect["x"]), ceil(rect["y"])
    right, bottom = floor(rect["x"] + rect["width"]), floor(rect["y"] + rect["height"])
    need(rect["height"] >= minimum_height and label["rgb_tolerance"] == 64
         and 0 <= left < right <= image.width and 0 <= top < bottom <= image.height,
         "native label has invalid ink limits")
    rgb, alpha = label["rgba"][:3], label["rgba"][3]
    pixels = image.load()
    points = []
    for y in range(top, bottom):
        for x in range(left, right):
            color = pixels[x, y]
            if color[3] == alpha and max(abs(a - b) for a, b in zip(color[:3], rgb)) <= 64:
                points.append((x, y))
    need(len(points) == label["ink_pixels"] and len(points) >= 16, "native label ink count differs")
    xs, ys = zip(*points)
    need([min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1] == label["ink_bounds"],
         "native label ink bounds differ")


def ax_check(log):
    path = log / "native-chips-ax.log"
    body = path.read_text()
    need(re.search(r"^test macos_accessibility::tests::accesskit_split_chip_keeps_two_named_actions_in_separate_grid_cells \.\.\. ok$", body, re.M)
         and re.search(r"^test result: ok\. 1 passed; 0 failed; 0 ignored;", body, re.M),
         "headless native AccessKit projection test missing or failed")
    return {"passed": 1, "log_sha256": sha(path)}


def workspace_check(log, files):
    for name in WORKSPACE_LOGS[:2]:
        lines = (log / name).read_text().splitlines()
        need(len(lines) == len(files) and {line[:-4] for line in lines if line.endswith(": OK")} == files.keys(),
             f"wrong before/after source inventory: {name}")
    path = log / WORKSPACE_LOGS[-1]
    body = path.read_text()
    tests = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", body, re.M)
    need(tests and sum(int(p) for p, _, _ in tests) == 1390 and all(int(f) == 0 for _, f, _ in tests),
         "workspace tests incomplete or failing")
    need(re.search(r"^provenance: .+ \(14 self-test cases passed\)$", body, re.M)
         and "Doc-tests" in body and len(re.findall(r"^\s*Finished `dev` profile", body[body.rfind("test result:"):], re.M)) == 3
         and not re.search(r"^error[:\[]|^test result: FAILED", body, re.M), "workspace check or Wasm completion missing")
    return {"rust_tests_passed": sum(int(p) for p, _, _ in tests), "rust_tests_ignored": sum(int(i) for _, _, i in tests),
            "wasm_check_commands_finished": 3, "log_sha256": sha(path)}


def metal_check(log, captures):
    body = log.read_text()
    need(re.search(r"^test result: ok\. 16 passed; 0 failed; 0 ignored;", body, re.M)
         and "backend: Metal" in body and "gpu_offscreen_raster_preserves_crop_clip_alpha_and_draw_order" in body
         and "gpu_offscreen_full_screen_fab_moves_and_clips_real_dialog_pixels" in body,
         "final real Metal GPU result is missing or failed")
    files = sorted(captures.glob("*.png"))
    need(len(files) == 21 and {"gpu-offscreen.png", "gpu-list-ripple-full.png", "gpu-transform-rounded.png"}
         <= {p.name for p in files}, "final exact Metal screenshot inventory is incomplete")
    shots = {}
    for file in files:
        # The all-clear capture fixture can be a solid image by design.
        with Image.open(file) as raw:
            raw.load()
            need(raw.format == "PNG", f"invalid Metal screenshot: {file.name}")
            shots[file.name] = {"size": list(raw.size), "sha256": sha(file)}
    return {"tests_passed": 16, "backend": "Metal", "log_sha256": sha(log), "captures": shots}


def stock_check(stock, run, expected_native):
    verify(stock, {"browser-source.sha256": SOURCE_SHA})
    verify(stock, STOCK_CODE)
    retained = stock / "retained"
    verify(retained, manifest(retained / "SHA256SUMS"))
    pin = load(stock / "release-pin.json")
    need(pin == {"version": "0.1.52", "last_material_step": 293, "source_sha256": SOURCE_SHA,
                 "source_files": 624, "wide_canvas_height": 800, "rtl_canvas_height": 1020}, "stock release pin differs")
    need(manifest(stock / "native-binaries.sha256") == expected_native, "stock and standalone native binary pins differ")
    original = run_path(str(stock / "verify-existing.py"))["existing"](run, stock)
    need(tuple(original["cases"]) == CASES and original["cases"]["material"]["steps"] == 292,
         "stock suite or Material count differs")
    original["independent_chips_raster"] = stock_raster(run / "artifacts")
    return json.loads(json.dumps(original))


def stock_raster(artifacts):
    report = load(artifacts / "material-chips.json")
    wide, keys, pointer, compact = (report[key] for key in ("wide", "keyboard", "pointer", "compact"))
    need([item["kind"] for item in wide["variants"]] == ["assist-flat", "assist-elevated", "filter-flat",
         "filter-elevated", "input", "suggestion-flat", "suggestion-elevated"] and len(wide["ax"]) == 12
         and all(item["ancestry"] == ["gridcell", "row", "grid"] for item in wide["ax"]),
         "stock Chips variant or accessibility ancestry evidence differs")
    need([pointer[key]["removals"] for key in ("after_body", "after_canceled_touch", "after_remove",
         "after_delete", "after_backspace")] == [0, 0, 1, 2, 3], "stock remove, cancel or keyboard path differs")

    def image(scene, rect):
        return png(artifacts / scene["capture"], rect).convert("RGB")

    def point(pixels, x, y):
        return list(pixels.getpixel((floor(2 * x), floor(2 * (y + 44)))))

    def count(pixels, rect, rgb):
        x, y, width, height = rect
        box = (round(2 * x), round(2 * (y + 44)), round(2 * (x + width)), round(2 * (y + height + 44)))
        need(0 <= box[0] < box[2] <= pixels.width and 0 <= box[1] < box[3] <= pixels.height,
             "Chips sample is outside actual raster")
        colors = dict((color, amount) for amount, color in pixels.crop(box).getcolors((box[2] - box[0]) * (box[3] - box[1])))
        return colors.get(tuple(rgb), 0)

    wide_shape, narrow_shape = (1920, 1688), (640, 2128)
    light = image(wide, wide_shape)
    shapes = [item["face"] for item in wide["variants"]]
    indexes = (0, 1, 2, 3, 6)
    fills = [point(light, shapes[i][0] + 3, shapes[i][1] + shapes[i][3] / 2) for i in indexes]
    palette = wide["palette"]
    need(fills == [palette["surface"][:3], palette["surface_container_low"][:3], palette["secondary_container"][:3],
                   palette["surface_container_low"][:3], palette["surface_container_low"][:3]], "stock token pixels differ")
    x, y, _, height = shapes[0]
    press = point(image(pointer, wide_shape), x + 3, y + height / 2)
    need(press == pointer["held"] != pointer["before"] == fills[0], "stock mouse press ink differs")
    focus = count(image(keys, wide_shape), keys["focus_rect"], palette["secondary"][:3])
    need(focus == keys["focus_pixels"] > 20, "stock recipient focus outline differs")
    dark = image(compact, narrow_shape)
    x, y, width, height = compact["long_label"]
    lines = [count(dark, (x, y + offset, width, min(40, height - offset)), compact["label_ink"])
             for offset in range(0, round(height), 40)]
    need(len(lines) >= 2 and min(lines) > 20 and lines == compact["label_line_ink"], "stock compact label ink differs")
    return {"wide_fills": fills, "pressed_pixel": press, "focus_ink_pixels": focus, "wrapped_line_ink": lines}


def inventory(root):
    return {str(path.relative_to(root)): sha(path) for path in sorted(root.rglob("*"))
            if path.is_file() and path != root / "SHA256SUMS"}


def copy(source, destination):
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def seal_native(stage, out):
    copy(stage / "native-v0152-r2-binaries/SHA256SUMS", out / "binary-sha256")
    copy(stage / "native-v0152-r2-build.log", out / "build.log")
    for name in NATIVE_SCRIPTS:
        copy(stage / "native" / name, out / "scripts" / name)
    for idx, run in enumerate(REPLAYS, 1):
        for name in PROFILE_FILES:
            copy(stage / "native" / run / name, out / f"replay-{idx}" / name)
        copy(stage / "native" / f"{run}.log", out / f"replay-{idx}" / "runner.log")


def missing_gates(stage, stock, run):
    paths = [stage / name for name in (*BRIEF_MARKERS, *PASS_MARKERS, *WORKSPACE_LOGS, "gpu-v0152-exact.log")]
    paths += [stock / name for name in ("release-pin.json", "inputs-before.json", "sealed-inputs.sha256")]
    paths += [run / name for name in ("runner.log", "inputs-after.json", "headless-window-observation.json",
                                      "verified-chips-stock.json", "artifacts/material-chips.json")]
    return [str(path) for path in paths if not path.is_file()]


def preflight(stage, base, stock, run):
    source = inspect_source(stage, base)
    need(all((stage / marker).is_file() for marker in BRIEF_MARKERS), "standalone native build/replay or AccessKit marker missing")
    with TemporaryDirectory(prefix="native-preflight.", dir=HERE) as tmp:
        seal_native(stage, Path(tmp))
        native = inspect_native(Path(tmp), stage / "native-v0152-r2-binaries")
    accesskit = ax_check(stage)
    missing = missing_gates(stage, stock, run)
    result = {"source": source, "native": native, "accesskit": accesskit, "missing_gates": missing}
    if all((stage / name).is_file() for name in (PASS_MARKERS[0], *WORKSPACE_LOGS)):
        result["workspace"] = workspace_check(stage, manifest(stage / "source.sha256"))
    if all((stage / name).is_file() for name in (PASS_MARKERS[1], "gpu-v0152-exact.log")):
        result["metal"] = metal_check(stage / "gpu-v0152-exact.log", stage / "gpu-v0152-exact")
    return result


def final_check(stock, run, checked):
    need(not checked["missing_gates"], "missing exact gates: " + ", ".join(checked["missing_gates"]))
    return {**checked, "stock_macos": stock_check(stock, run, checked["native"]["binaries"])}


def package_check(package):
    need(manifest(package / "SHA256SUMS") == inventory(package), "sealed package inventory or bytes differ")
    files, changes = source_pins(package)
    log = package / "rust"
    need(all((log / marker).is_file() for marker in (*BRIEF_MARKERS, *PASS_MARKERS)), "package lacks exact gate markers")
    stock = package / "stock-macos"
    native = inspect_native(package / "native")
    result = {"source": {"source_files": len(files), "changed_paths": len(changes), "source_sha256": SOURCE_SHA,
                         "patch_sha256": PATCH_SHA, "tabs_base_tree": BASE_TREE, "chips_tree": CHIPS_TREE},
              "native": native, "accesskit": ax_check(log), "missing_gates": [],
              "workspace": workspace_check(log, files),
              "metal": metal_check(log / "gpu-v0152-exact.log", package / "gpu"),
              "stock_macos": stock_check(stock, stock / "run", native["binaries"])}
    archive = run_path(str(stock / "archive-proof.py"))["verify"](stock)
    need(archive["source_sha256"] == SOURCE_SHA and archive["checks"] == result["stock_macos"]["total_checks"],
         "nested stock archive does not bind the exact source and full result")
    need(result == load(package / "verified-chips-v0152.json"), "recomputed receipt differs from the packaged receipt")
    return {"package_files": len(inventory(package)), "package_sha256": sha(package / "SHA256SUMS"),
            "rust_tests": result["workspace"]["rust_tests_passed"], "metal_tests": result["metal"]["tests_passed"],
            "stock_checks": result["stock_macos"]["total_checks"], "source_sha256": SOURCE_SHA,
            "stock_archive_sha256": archive["manifest_sha256"]}


def pack(stage, stock, run, out, checked):
    out = out.resolve()
    need(out.is_relative_to(TEMP) and out != TEMP and not out.exists(), "use a new package path inside /tmp")
    out.mkdir(mode=0o700)
    try:
        for name in ("source.sha256", "chips-v0152.patch", "tabs-base-tree", "chips-tree", "actual-chips.paths"):
            copy(stage / name, out / name)
        for name in (*WORKSPACE_LOGS, *BRIEF_MARKERS, *PASS_MARKERS, "native-chips-ax.log", "gpu-v0152-exact.log"):
            copy(stage / name, out / "rust" / name)
        for image in sorted((stage / "gpu-v0152-exact").glob("*.png")):
            copy(image, out / "gpu" / image.name)
        seal_native(stage, out / "native")
        run_path(str(stock / "archive-proof.py"))["seal"](run, out / "stock-macos")
        for name in ("README.md", "chips-proof.py"):
            copy(HERE / name, out / name)
        save(out / "verified-chips-v0152.json", checked)
        (out / "SHA256SUMS").write_text("".join(f"{digest}  {name}\n" for name, digest in inventory(out).items()))
        return package_check(out)
    except Exception:
        # Keep failed inputs for diagnosis but never leave a positive seal.
        (out / "SHA256SUMS").unlink(missing_ok=True)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--check", action="store_true", help="verify a live exact run or an existing package")
    action.add_argument("--pack", type=Path, help="write a checked new portable directory inside /tmp")
    parser.add_argument("--package", type=Path, help="check a portable package without a product source checkout")
    parser.add_argument("--stage", type=Path, default=STAGE)
    parser.add_argument("--base", type=Path, default=BASE)
    parser.add_argument("--stock", type=Path, default=STOCK)
    parser.add_argument("--run", type=Path, help="original stock run directory, default STOCK/run")
    args = parser.parse_args()
    if args.package is not None and not args.check:
        parser.error("--package requires --check")
    try:
        if args.package is not None:
            result = package_check(args.package)
        else:
            run = args.run if args.run is not None else args.stock / "run"
            checked = preflight(args.stage, args.base, args.stock, run)
            if checked["missing_gates"]:
                print(json.dumps(checked, indent=2, ensure_ascii=False))
                raise Failed("portable proof incomplete; the listed exact gates must finish")
            result = final_check(args.stock, run, checked)
            if args.pack is not None:
                result = pack(args.stage, args.stock, run, args.pack, result)
        print(json.dumps(result, indent=2, ensure_ascii=False))
    except (Failed, KeyError, OSError, RuntimeError, ValueError) as error:
        raise SystemExit(str(error)) from error


if __name__ == "__main__":
    main()
