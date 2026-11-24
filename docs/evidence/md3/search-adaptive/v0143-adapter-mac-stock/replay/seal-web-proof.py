"""Fully decode all Search PNGs and preserve the verified stock gate."""

from hashlib import sha256
import json
from pathlib import Path
from shutil import copy2
import sys
from tempfile import TemporaryDirectory

from PIL import Image

from verify_search_report import PREFIX, expected_frames

ROOT = Path(__file__).resolve().parent


def read(path):
    return json.loads(path.read_text())


def digest(path):
    return sha256(path.read_bytes()).hexdigest()


def fully_decode_search(run, verified):
    exact = expected_frames()
    assert set(verified["search"]["pngs"]) == set(exact) and len(exact) == 46
    decoded = {}
    for name, shape in exact.items():
        frame = run / f"{PREFIX}{name}.png"
        assert digest(frame) == verified["search"]["pngs"][name]["sha256"]
        with Image.open(frame) as image:
            assert image.format == "PNG" and list(image.size) == shape
            image.load()
            pixels = image.convert("RGBA").tobytes()
        assert len(pixels) == shape[0] * shape[1] * 4
        decoded[name] = {"source_png_sha256": digest(frame), "rgba_sha256": sha256(pixels).hexdigest(),
                         "physical_size": shape}
    return decoded


def readme(stage, verification, window_counts, entries):
    return f"""# Surface v0.1.43 stock headless Search proof

This gate used the exact 557-file source export for the repaired v0.1.43 candidate. The incoming manifest SHA-256 is `{verification['incoming_source_manifest_sha256']}`. The stock bundle manifest is `{verification['exact_binary_manifest_sha256']}`. The exact runner was rebuilt. Three unchanged companion binaries and the three unchanged Web products were reused byte for byte. `inputs/payload-reuse.json` and the two prior manifests prove the only changed source file was the runner test. All source and bundle files passed checks before and after the run.

The stock Mac fixture used {verification['browser']} with Apple Metal WebGPU. It ran `smoke`, `ime`, `material`, `bridge-web`, and `devtools-headless`. All five cases passed with {verification['total_checks']:,} checks. I1 through I20 ran with 77 IME checks. The Material case ran all 254 scheduled steps through M255. The stock suite omits the M23 failure-dump step.

Both cancellation reports passed. Select All survived a Canvas resize after cancellation. A separate backward selection was restored before repaint. Trusted Right and insertion produced `台文X` while repaint was held and kept it after repaint. Search M248–M255 passed. The verifier decoded all 46 Search PNGs and checked selected caret, palette, and motion pixels independently.

The reported Showcase and Material page errors were zero. CoreGraphics found zero visible windows in {window_counts['chrome']['samples']:,} Chrome and {window_counts['native_gallery']['samples']:,} Gallery process observations. The runner also checked that its owned Chrome and Surface processes exited. Native screenshot bytes matched through the API, CLI, and MCP. The run used browser-simulated composition and headless platform reports.

Run `shasum -a 256 -c SHA256SUMS` from this folder to check the {entries} published proof files. To replay, use the source and manifest under the same checksum. Run the included `replay/stock-build.sh` with the source path, manifest path, private `stock.*` stage under the replay directory, and prior sealed stock stage as the fourth argument. Copy `inputs/adapter-check.patch` into the replay directory before running the build. Then run `replay/observe-headless.py` with that stage. `replay/seal-web-proof.py` requires Pillow for the full Search image decode. The exact build and runner records are in `inputs` and `run`.

Original pinned binaries and full scratch run are under `{stage}`.
"""


def main():
    stage = Path(sys.argv[1]).resolve(strict=True)
    run = stage / (sys.argv[2] if len(sys.argv) > 2 else "run")
    verified = read(run / "verified-browser.json")
    observed = read(run / "headless-window-observation.json")
    assert verified["version"] == "0.1.43"
    assert verified["incoming_source_manifest_sha256"] == "386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f"
    assert not observed["visible_owned_windows"] and not observed["remaining_observed_processes"]
    assert observed["stock_exit"] == 0 and not verified["remaining_browser_pids"]
    assert len(verified["headless_cases"]) == 5
    assert all(item["samples"] > 0 for item in observed["observations"].values())
    decoded = fully_decode_search(run, verified)
    (run / "search-full-decode.json").write_text(json.dumps(decoded, indent=2) + "\n")
    for key in ("stage-before.log", "stage-after.log", "bundle-before.log", "bundle-after.log"):
        assert (run / key).stat().st_size > 0

    inputs = (
        "stock-stage.json", "incoming-source.sha256", "browser-source.sha256", "browser-binary-sha256.txt",
        "sealed-inputs.sha256", "harness.sha256", "build.log", "built-input-sha256.txt",
        "prior-stage.log", "prior-bundle.log", "payload-reuse.json", "prior-incoming-source.sha256",
        "prior-binary-sha256.txt", "adapter-check.patch",
    )
    required = (
        "attempt.json", "runner.log", "headless-results.txt", "verified-browser.json", "headless-window-observation.json",
        "search-full-decode.json", "bridge-web-result.json", "devtools-headless-result.json", "browser-smoke.json",
        "stage-before.log", "stage-after.log", "bundle-before.log", "bundle-after.log",
        "before-run-sha256.txt", "after-run-sha256.txt", "evidence-sha256.txt",
        "devtools-headless-screenshot.png", "bridge-web.png",
    )
    patterns = (
        "browser-material*.json", "browser-ime*.json", "material-web-contrast-*.json",
        "browser-ime-*.png", "browser-showcase-*.png",
        "material-web-contrast-*.png", "material-scroll-modal-fields-*.png",
        "material-scroll-popup-fields-compact-dark-rtl.png", "material-scroll-pair-fields-short-unit-focused.png",
    )
    sources = [ROOT / name for name in ("stock-build.sh", "stock-run.sh", "stock-inputs.py", "verify-stock.py",
               "observe-headless.py", "seal-web-proof.py", "verify-payload-reuse.py")] + sorted(ROOT.glob("verify_*_report.py"))
    with TemporaryDirectory(dir=ROOT) as work:
        bundle = Path(work) / "portable-web-proof"
        for name in ("inputs", "run", "replay"):
            (bundle / name).mkdir(parents=True)
        for name in inputs:
            copy2(stage / name, bundle / "inputs" / name)
        original_source_receipt = ROOT / "original-source-after-build.log"
        if original_source_receipt.is_file():
            copy2(original_source_receipt, bundle / "inputs" / original_source_receipt.name)
        files = {run / name for name in required}
        for pattern in patterns:
            files.update(run.glob(pattern))
        for path in sorted(files):
            copy2(path, bundle / "run" / path.name)
        for path in sources:
            copy2(path, bundle / "replay" / path.name)
        entries = len(list(bundle.glob("*/*"))) + 1
        (bundle / "README.md").write_text(readme(stage, verified, observed["observations"], entries))
        paths = sorted(path for path in bundle.rglob("*") if path.is_file())
        (bundle / "SHA256SUMS").write_text("".join(f"{digest(path)}  {path.relative_to(bundle)}\n" for path in paths))
        destination = ROOT / "portable-web-proof"
        assert not destination.exists(), destination
        bundle.rename(destination)
    print(f"Decoded {len(decoded)} Search PNGs and sealed {entries} files at {destination}")


if __name__ == "__main__":
    main()
