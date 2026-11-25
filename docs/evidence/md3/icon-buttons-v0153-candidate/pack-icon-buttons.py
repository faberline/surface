#!/usr/bin/env python3
"""Verify exact Icon Button evidence and export compact and full offline archives."""

from argparse import ArgumentParser
import gzip
from hashlib import file_digest, sha1, sha256
import json
import math
from pathlib import Path
import re
from shutil import copy2, copyfileobj, copytree
import subprocess
import sys
import tomllib

from PIL import Image

STAGE = Path("/tmp/surface-md3-icon-buttons-v0153.csjCVwaB")
OBSERVER = Path("/tmp/surface-md3-icon-buttons-stock.BZHOvArh")
BASE = Path("/tmp/surface-md3-chips-v0152.tHzYZc3R")
BASE_TREE = "48d560ab127cc1c692a5ba0eba6fb5a8949234a8"
BASE_SOURCE = "8d2548019eca22046b20992b27f48f4404cb0f5af84f26d6d864d6de498509b1"
VERSION = "0.1.53"
LAST_MATERIAL = 297
SUITES = {"smoke", "ime", "material", "bridge-web", "devtools-headless"}
NATIVE_CAPTURE_NAMES = {"wide-light-icon-buttons.png", "compact-dark-rtl-200.png", "compact-tooltip.png"}
WEB_CAPTURE_NAMES = {
    "material-icon-buttons-wide-light.png", "material-icon-buttons-selected-focus.png",
    "material-icon-buttons-pressed.png", "material-icon-buttons-compact-dark-rtl-200.png",
}
NATIVE_RUNS = ("exact-v0153-r3-1", "exact-v0153-r3-2")
MARKERS = ("workspace-v0153-passed", "gpu-v0153-exact-passed", "native-v0153-r3-passed")
LOGS = ("workspace-v0153-exact.log", "gpu-v0153-exact.log", "native-v0153-r3-build.log")
SOURCES = ("source.sha256", "chips-base-tree", "icon-buttons-tree", "icon-buttons-v0153.patch")
NATIVE_SCRIPTS = ("replay-icon-buttons.py", "render_helpers.py", "gallery_transport.py",
                  "native_runtime.py", "window_check.py")
BROWSER_RECEIPTS = ("headless-window-observation.json", "invocation.json", "inputs-after.json",
                    "verified-icon-buttons-stock.json")
BROWSER_ARTIFACTS = (
    "material-icon-buttons.json", "browser-smoke.json", "browser-ime.json",
    "browser-ime-cancel-redraw.json", "browser-ime-cancel-selected-redraw.json",
    "browser-material-side-sheet.json", "bridge-web-result.json", "devtools-headless-result.json",
)


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text())


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n")


def manifest_entries(path):
    entries = {}
    for line in path.read_text().splitlines():
        checksum, name = line.split("  ", 1)
        relative = Path(name)
        require(re.fullmatch(r"[0-9a-f]{64}", checksum) and name not in entries
                and not relative.is_absolute() and ".." not in relative.parts
                and str(relative) == name, f"Malformed manifest entry in {path}: {name}")
        entries[name] = checksum
    require(entries, f"Empty manifest: {path}")
    return entries


def verify_entries(path, entries):
    for name, checksum in entries.items():
        item = path / name
        require(item.is_file() and not item.is_symlink()
                and item.resolve(strict=True).is_relative_to(path.resolve(strict=True))
                and digest(item) == checksum, f"Different manifest file: {item}")


def object_id(kind, payload):
    header = f"{kind} {len(payload)}\0".encode("ascii")
    return sha1(header + payload).digest()


def source_tree(root, entries):
    """Reconstruct the Git SHA-1 tree without creating any Git objects."""
    top = {}
    for name in entries:
        *parents, basename = Path(name).parts
        directory = top
        for part in parents:
            directory = directory.setdefault(part, {})
            require(isinstance(directory, dict), f"Source file and directory collide: {name}")
        path = root / name
        data = path.read_bytes()
        require(sha256(data).hexdigest() == entries[name], f"Source changed during tree check: {name}")
        mode = b"100755" if path.stat().st_mode & 0o100 else b"100644"
        directory[basename] = (mode, object_id("blob", data))

    def tree_id(directory):
        def sort_key(item):
            name, value = item
            return (name + ("/" if isinstance(value, dict) else "")).encode("utf-8")

        payload = []
        for name, value in sorted(directory.items(), key=sort_key):
            mode, child = (b"40000", tree_id(value)) if isinstance(value, dict) else value
            payload.append(mode + b" " + name.encode("utf-8") + b"\0" + child)
        return object_id("tree", b"".join(payload))

    return tree_id(top).hex()


def verify_source(stage, source_sha, tree):
    source = stage / "source"
    manifest = stage / "source.sha256"
    require(digest(manifest) == source_sha, "Candidate source manifest differs from the reviewed digest")
    entries = manifest_entries(manifest)
    verify_entries(source, entries)
    require((stage / "icon-buttons-tree").read_text().strip() == tree
            and source_tree(source, entries) == tree, "Candidate Git tree differs from the reviewed tree")
    require((stage / "chips-base-tree").read_text().strip() == BASE_TREE,
            "Base needs the exact Chips tree")
    version = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    require(version == VERSION, f"Wrong workspace version {version}")
    base_manifest = BASE / "source.sha256"
    require(digest(base_manifest) == BASE_SOURCE, "Chips source manifest changed")
    previous = manifest_entries(base_manifest)
    verify_entries(BASE / "source", previous)
    require(source_tree(BASE / "source", previous) == BASE_TREE, "Chips source modes or tree changed")
    patch = stage / "icon-buttons-v0153.patch"
    patch_paths = set(re.findall(rb"^diff --git a/([^\n ]+) b/[^\n ]+$", patch.read_bytes(), re.M))
    changed = {name.encode() for name in entries.keys() | previous.keys() if entries.get(name) != previous.get(name)}
    require(patch_paths == changed and len(changed) > 0, "Patch paths differ from actual changed source paths")
    for directory, options in ((BASE / "source", []), (source, ["--reverse"])):
        subprocess.run(["git", "-C", str(directory), "apply", "--check", "--whitespace=error-all",
                        *options, str(patch)], check=True)
    return {"version": version, "source_sha256": source_sha, "source_files": len(entries),
            "tree": tree, "base_tree": BASE_TREE, "base_source_sha256": BASE_SOURCE,
            "patch_sha256": digest(patch), "changed_paths": len(changed)}


def test_totals(log):
    lines = log.read_text()
    totals = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", lines, re.M)
    require(totals and all(kind == "ok" and failed == "0" for kind, _, failed, _ in totals),
            f"Missing or failed Rust suites: {log.name}")
    return lines, sum(int(passed) for _, passed, _, _ in totals), sum(int(ignored) for _, _, _, ignored in totals)


def verify_rust(stage):
    workspace = stage / LOGS[0]
    gpu = stage / LOGS[1]
    text, passed, ignored = test_totals(workspace)
    tail = text.rsplit("\ntest result:", 1)[-1]
    wasm = re.findall(r"Finished `dev` profile", tail)
    require(passed > 0 and re.search(r"^provenance: .+ self-test cases passed\)", text, re.M)
            and len(wasm) == 3 and "Doc-tests surface_material_gallery" in text
            and "pressed_face_and_focus_follow_the_same_spring_through_a_toggle_and_cancel ... ok" in text,
            "Incomplete exact workspace, provenance or Wasm evidence")
    metal_text, gpu_tests, gpu_ignored = test_totals(gpu)
    require(gpu_tests == 16 and gpu_ignored == 0 and "backend: Metal" in metal_text,
            "The real offscreen 16-case Metal gate is missing")
    return {"workspace": {"passed": passed, "ignored": ignored, "wasm_checks": len(wasm),
                          "log_sha256": digest(workspace)},
            "metal": {"passed": gpu_tests, "ignored": gpu_ignored, "backend": "Metal",
                      "log_sha256": digest(gpu)}}


def image_receipt(path, shot, size):
    require(shot["name"] == path.name and shot["format"] == "png" and shot["scale"] == 1
            and [shot["width"], shot["height"]] == size and digest(path) == shot["sha256"],
            f"Different native image receipt: {path}")
    with Image.open(path) as opened:
        rgb = opened.convert("RGB")
        require(opened.format == "PNG" and list(rgb.size) == size
                and any(low < high for low, high in rgb.getextrema()), f"Bad native PNG: {path}")
        decoded = sha256(rgb.tobytes()).hexdigest()
    return {"size": size, "sha256": shot["sha256"], "decoded_rgb_sha256": decoded}


def native_tooltip(profile):
    tooltip = profile["tooltip"]
    tip, action, ring = (tooltip[key] for key in ("bounds", "action", "focus_ring"))
    for rect in (tip, action, ring):
        require(all(type(rect[key]) in (int, float) and math.isfinite(rect[key])
                    for key in ("x", "y", "width", "height"))
                and min(rect["width"], rect["height"]) > 0, "Invalid native tooltip geometry")
    width, height = profile["viewport"]
    require(0 <= tip["x"] < tip["x"] + tip["width"] <= width
            and 0 <= tip["y"] < tip["y"] + tip["height"] <= height,
            "Native tooltip escaped the viewport")
    require(tip["y"] + tip["height"] <= min(action["y"], ring["y"])
            or tip["y"] >= max(action["y"] + action["height"], ring["y"] + ring["height"]),
            "Native tooltip intersects the action or focus ring")
    return tooltip


def verify_native(stage):
    binaries = stage / "native-v0153-r3-binaries"
    raw = manifest_entries(binaries / "SHA256SUMS")
    require(set(raw) == {"surface-material-gallery", "surface-devtools"}, "Wrong native binary inventory")
    verify_entries(binaries, raw)
    pins = {"gallery": raw["surface-material-gallery"], "cli": raw["surface-devtools"]}
    runs = []
    for name in NATIVE_RUNS:
        root = stage / "native" / name
        report_file = root / "native-gallery-report.json"
        report = read_json(report_file)
        require(report["schema"] == "surface.test.icon-buttons.native.v1"
                and report["rendering"] == "headless"
                and report["transports"] == ["Page.screenshot", "CLI", "MCP"]
                and report["binaries"] == pins, f"Wrong headless native replay: {name}")
        require("matching PNG bytes and no visible windows." in (root.with_suffix(".log")).read_text(),
                f"Native replay did not reach its passing endpoint: {name}")
        require({item.name for item in root.glob("*.png")} == NATIVE_CAPTURE_NAMES,
                f"Wrong native capture inventory: {name}")
        captures = {}
        for profile, (filename, viewport, compact) in zip(report["profiles"], (
                ("wide-light-icon-buttons.png", [960, 800], False),
                ("compact-dark-rtl-200.png", [320, 760], True)), strict=True):
            require(profile["viewport"] == viewport and profile["rtl"] is compact
                    and profile["text_scale"] == (2 if compact else 1)
                    and profile["windows"] == [0, 0, 0] and profile["name"] == "加星號",
                    f"Wrong native viewport, window count or toggle name: {name}")
            tip = native_tooltip(profile)
            captures[filename] = image_receipt(root / filename, profile["png"], viewport)
            if compact:
                captures["compact-tooltip.png"] = image_receipt(
                    root / "compact-tooltip.png", tip["png"], viewport)
                require(captures["compact-tooltip.png"]["decoded_rgb_sha256"]
                        != captures[filename]["decoded_rgb_sha256"], "Native tooltip pixels did not change")
            else:
                require(tip["png"] is None, "Wrong tooltip capture profile")
        runs.append({"name": name, "report_sha256": digest(report_file), "captures": captures})
    require(runs[0]["captures"] == runs[1]["captures"], "Native recurrence did not reproduce all three PNGs")
    return {"binaries": raw, "runs": runs, "visible_windows": 0,
            "transports": ["Page.screenshot", "CLI", "MCP"]}


def verify_browser(observer, archive, seal, source, native):
    manifest = archive / "SHA256SUMS"
    require(digest(manifest) == seal, "Stock archive differs from the reviewed seal")
    archive_entries = manifest_entries(manifest)
    verify_entries(archive, archive_entries)
    require(len(archive_entries) == len([item for item in archive.rglob("*") if item.is_file()]) - 1,
            "Stock archive inventory differs")
    for name in ("observe-icon-buttons.py", "prepare-pins.py", "verify-existing.py", "archive-proof.py"):
        require(digest(observer / name) == archive_entries[name], f"Stock helper differs from approved observer: {name}")
    result = subprocess.run([sys.executable, "-E", "-B", str(observer / "archive-proof.py"), "--verify",
                             str(archive)], check=True, capture_output=True, text=True)
    replay = json.loads(result.stdout)
    receipt = read_json(archive / "verified-icon-buttons-offline.json")
    pins = receipt["release"]
    profile = read_json(archive / "run/verified-icon-buttons-stock.json")
    run = archive / "run"
    windows = read_json(run / "headless-window-observation.json")
    require(pins["version"] == VERSION and pins["last_material_step"] == LAST_MATERIAL
            and pins["source_sha256"] == digest(archive / "browser-source.sha256") == source["source_sha256"]
            and pins["source_files"] == source["source_files"] and replay["manifest_sha256"] == seal,
            "Stock source does not bind the candidate")
    require(manifest_entries(archive / "native-binaries.sha256") == native["binaries"],
            "Stock native executable pair differs from the two exact native replays")
    require(set(receipt["cases"]) == SUITES and receipt["cases"]["ime"] == {"steps": 20, "checks": 77}
            and receipt["cases"]["material"]["steps"] == LAST_MATERIAL - 1
            and profile["total_checks"] == receipt["total_checks"] == replay["checks"]
            and profile["page_errors"] == 0, "Incomplete stock suite or page errors")
    adapter = receipt["adapter"]
    require(adapter["vendor"] == "apple" and adapter["architecture"].startswith("metal")
            and adapter["isFallbackAdapter"] is False and windows["stock_exit"] == 0
            and windows["problems"] == windows["visible_owned_windows"] == windows["remaining_observed_processes"] == [],
            "Missing real Apple Metal or clean headless observation")
    require(set(receipt["icons_pngs"]) == WEB_CAPTURE_NAMES
            and digest(run / "artifacts/material-icon-buttons.json") == receipt["icons_report_sha256"],
            "Stock Icon Button PNGs or typed report differ")
    icons_pixels = receipt["icons_pixels"]
    require(set(icons_pixels) == {"focus_pixels", "light_command_ink", "dark_command_ink",
                                 "pressed_corner_pixels"} and
            all(type(value) is int and value > 0 for value in icons_pixels.values()),
            "Offline Icon Button pixel verification is missing")
    for filename in BROWSER_ARTIFACTS:
        require(f"run/artifacts/{filename}" in archive_entries, f"Missing browser artifact: {filename}")
    return {"archive_seal": seal, "verified_files": replay["files"], "checks": receipt["total_checks"],
            "adapter": adapter, "last_material": LAST_MATERIAL, "cases": receipt["cases"],
            "report_sha256": receipt["icons_report_sha256"], "screenshots": receipt["icons_pngs"],
            "window_receipt_sha256": digest(run / "headless-window-observation.json"),
            "icons_pixels": icons_pixels}


def copy(source, dest):
    dest.parent.mkdir(exist_ok=True, parents=True)
    copy2(source, dest)
    require(digest(dest) == digest(source), f"File changed while copying: {source}")


def compress(source, dest):
    dest.parent.mkdir(exist_ok=True, parents=True)
    with source.open("rb") as contents, dest.open("wb") as raw:
        with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
            copyfileobj(contents, compressed)
    with gzip.open(dest, "rb") as restored:
        require(file_digest(restored, "sha256").hexdigest() == digest(source),
                f"Compressed bytes differ: {source}")


def seal(directory):
    files = sorted(path for path in directory.rglob("*") if path.is_file())
    manifest = directory / "SHA256SUMS"
    require(not manifest.exists(), "A proof manifest already exists")
    manifest.write_text("".join(f"{digest(path)}  {path.relative_to(directory)}\n" for path in files))
    entries = manifest_entries(manifest)
    verify_entries(directory, entries)
    return {"manifest_sha256": digest(manifest), "files": len(entries)}


def copy_native(stage, out):
    folder = out / "native"
    copy(stage / LOGS[2], folder / "build.log")
    copy(stage / "native-v0153-r3-binaries/SHA256SUMS", folder / "binary-sha256")
    for helper in NATIVE_SCRIPTS:
        copy(stage / "native" / helper, folder / "scripts" / helper)
    for index, name in enumerate(NATIVE_RUNS, start=1):
        source = stage / "native" / name
        run = folder / f"replay-{index}"
        for file in sorted(NATIVE_CAPTURE_NAMES | {"native-gallery-report.json"}):
            copy(source / file, run / file)
        copy(source.with_suffix(".log"), run / "runner.log")


def render_readme(out, verified, full, browser):
    counts = verified["browser"]
    source = verified["source"]
    local = verified["rust"]
    lines = [
        "# Surface Material 3 Icon Button v0.1.53 candidate",
        "",
        f"The candidate tree is `{source['tree']}`.",
        f"The checked source manifest covers {source['source_files']} files.",
        f"Apply the patch to Chips tree `{BASE_TREE}`.",
        "",
        f"The workspace passed {local['workspace']['passed']:,} Rust tests, provenance, formatting, Clippy and three Wasm checks.",
        f"All {local['metal']['passed']} offscreen Metal tests ran and passed.",
        "",
        "Two native headless replays reproduced the same three PNGs.",
        "Page.screenshot, CLI and MCP returned the same bytes within each replay.",
        "The observer recorded zero visible windows.",
        "",
        f"Five stock macOS headless suites passed {counts['checks']:,} checks.",
        "They reached M297 with 296 completed Material steps and 77 IME checks.",
        "Chrome used Apple Metal. The stock observer recorded zero visible windows and no surviving test processes.",
        "",
        "## Original captures",
        "",
    ]
    for name in sorted(NATIVE_CAPTURE_NAMES):
        lines.append(f"- [Native {name}](native/replay-1/{name})")
    for name in sorted(WEB_CAPTURE_NAMES):
        lines.append(f"- [Chrome {name}](stock-macos/artifacts/{name})")
    lines += [
        "",
        "## Verification files",
        "",
        "- [Candidate receipt](verified-icon-buttons-v0153.json)",
        "- [Native replay 1](native/replay-1/native-gallery-report.json)",
        "- [Native replay 2](native/replay-2/native-gallery-report.json)",
        "- [Chrome typed result](stock-macos/artifacts/material-icon-buttons.json)",
        "- [Stock offline receipt](stock-macos/verified-icon-buttons-offline.json)",
        "- [Stock window observation](stock-macos/headless-window-observation.json)",
        "",
        "Use `shasum -a 256 -c SHA256SUMS` from this directory to check the compact copy.",
        "The bundled packer checks both the original headless archive and the compact export inputs.",
        f"The full local archive manifest has SHA-256 `{full['manifest_sha256']}`.",
        f"The original stock archive manifest has SHA-256 `{browser['archive_seal']}`.",
        "",
        "Coverage stays partial. Hosted Linux and live screen-reader speech require separate evidence.",
        "",
    ]
    (out / "README.md").write_text("\n".join(lines))


def export(stage, archive, out, verified):
    require(not out.exists() and out.parent.resolve(strict=True).is_relative_to(Path("/tmp").resolve()),
            "Choose a fresh output directory in /tmp")
    out.mkdir()
    full = out / "full"
    full.mkdir()
    for name in (*SOURCES, *MARKERS):
        copy(stage / name, full / name)
    copy(BASE / "source.sha256", full / "chips-base.sha256")
    for name in LOGS[:2]:
        copy(stage / name, full / "rust" / name)
    for name in ("workspace-source-before.log", "workspace-source-after.log",
                 "gpu-source-before.log", "gpu-source-after.log", "native-accesskit-toggle-exact.log"):
        if (stage / name).is_file():
            copy(stage / name, full / "rust" / name)
    copy_native(stage, full)
    for image in sorted((stage / "gpu-v0153-exact").glob("*.png")):
        copy(image, full / "gpu" / image.name)
    copytree(archive, full / "stock-macos")
    stock = full / "stock-macos"
    require(digest(stock / "SHA256SUMS") == verified["browser"]["archive_seal"], "Copied stock archive seal differs")
    verify_entries(stock, manifest_entries(stock / "SHA256SUMS"))
    copy(Path(__file__), full / "pack-icon-buttons.py")
    source = verified["source"]
    require(digest(full / "source.sha256") == source["source_sha256"] and
            digest(full / "icon-buttons-v0153.patch") == source["patch_sha256"],
            "Copied candidate source or patch differs")
    for index, run in enumerate(verified["native"]["runs"], start=1):
        replay = full / "native" / f"replay-{index}"
        require(digest(replay / "native-gallery-report.json") == run["report_sha256"],
                "Copied native report differs")
        verify_entries(replay, {name: image["sha256"] for name, image in run["captures"].items()})
    for name, key in zip(LOGS[:2], ("workspace", "metal"), strict=True):
        require(digest(full / "rust" / name) == verified["rust"][key]["log_sha256"],
                "Copied test log differs")
    write_json(full / "verified-icon-buttons-v0153.json", verified)
    full_seal = seal(full)

    portable = out / "candidate"
    portable.mkdir()
    for name in ("source.sha256", "chips-base.sha256", "chips-base-tree", "icon-buttons-tree",
                 "pack-icon-buttons.py", "verified-icon-buttons-v0153.json"):
        copy(full / name, portable / name)
    copy(full / "SHA256SUMS", portable / "full-local-archive.sha256")
    compress(full / "icon-buttons-v0153.patch", portable / "surface-v0.1.53.patch.gz")
    for name in MARKERS:
        copy(full / name, portable / "rust" / name)
    for log in sorted((full / "rust").glob("*.log")):
        compress(log, portable / "rust" / f"{log.name}.gz")
    copytree(full / "native", portable / "native")
    image_examples = ("gpu-shapes.png", "gpu-offscreen.png", "gpu-motion-midpoint.png")
    for name in image_examples:
        if (full / "gpu" / name).is_file():
            copy(full / "gpu" / name, portable / "gpu" / name)

    browser = portable / "stock-macos"
    copy(stock / "SHA256SUMS", browser / "full-stock-archive.sha256")
    for file in sorted(stock.iterdir()):
        if file.is_file() and file.name != "SHA256SUMS":
            copy(file, browser / file.name)
    for name in BROWSER_RECEIPTS:
        copy(stock / "run" / name, browser / name)
    copytree(stock / "run/guard-ledger", browser / "guard-ledger")
    compress(stock / "run/runner.log", browser / "runner.log.gz")
    for name in sorted(WEB_CAPTURE_NAMES | set(BROWSER_ARTIFACTS)):
        copy(stock / "run/artifacts" / name, browser / "artifacts" / name)
    render_readme(portable, verified, full_seal, verified["browser"])
    return {"full": {"directory": str(full), **full_seal},
            "compact": {"directory": str(portable), **seal(portable)}}


def inputs(args):
    stage = args.stage.resolve(strict=True)
    required = [stage / name for name in (*SOURCES, *LOGS, *MARKERS)]
    required += [stage / "native-v0153-r3-binaries/SHA256SUMS"]
    required += [stage / "native" / run / "native-gallery-report.json" for run in NATIVE_RUNS]
    missing = [str(path.relative_to(stage)) for path in required if not path.is_file()]
    for value, flag in ((args.source_sha256, "--source-sha256"), (args.tree, "--tree"),
                        (args.stock_archive, "--stock-archive"), (args.stock_seal, "--stock-seal")):
        if not value:
            missing.append(flag)
    if args.stock_archive and not (args.stock_archive / "SHA256SUMS").is_file():
        missing.append(str(args.stock_archive / "SHA256SUMS"))
    return stage, missing


def main():
    parser = ArgumentParser(description=__doc__)
    parser.add_argument("--stage", type=Path, default=STAGE)
    parser.add_argument("--observer", type=Path, default=OBSERVER)
    parser.add_argument("--source-sha256", help="approved final candidate source manifest SHA-256")
    parser.add_argument("--tree", help="approved final 40-digit candidate tree")
    parser.add_argument("--patch-sha256", help="optional approved patch SHA-256")
    parser.add_argument("--stock-archive", type=Path, help="sealed stock observer archive")
    parser.add_argument("--stock-seal", help="approved stock archive manifest SHA-256")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true", help="read only and report pending or verified")
    mode.add_argument("--out", type=Path, help="write a full and compact archive under a new /tmp path")
    args = parser.parse_args()
    try:
        stage, missing = inputs(args)
        if missing:
            present = [name for name in MARKERS if (stage / name).is_file()]
            print(json.dumps({"status": "pending", "present_gates": present, "missing": missing}, indent=2))
            return 2
        for value, digits in ((args.source_sha256, 64), (args.tree, 40), (args.stock_seal, 64)):
            require(re.fullmatch(rf"[a-f0-9]{{{digits}}}", value), "An approved digest is malformed")
        stock = args.stock_archive.resolve(strict=True)
        source = verify_source(stage, args.source_sha256, args.tree)
        if args.patch_sha256:
            require(source["patch_sha256"] == args.patch_sha256, "Candidate patch differs from the reviewed digest")
        rust = verify_rust(stage)
        native = verify_native(stage)
        browser = verify_browser(args.observer.resolve(strict=True), stock, args.stock_seal, source, native)
        verified = {"status": "verified", "source": source, "rust": rust, "native": native, "browser": browser}
        if args.out:
            verified["outputs"] = export(stage, stock, args.out, verified)
        print(json.dumps(verified, indent=2, ensure_ascii=False))
        return 0
    except subprocess.CalledProcessError as error:
        details = (error.stderr or "").strip()
        print(f"Rejected offline replay ({error.returncode}). {details}", file=sys.stderr)
        return 1
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"Rejected proof. {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
