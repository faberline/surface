"""Verify the real v0.1.66 Chrome AX values and retain the v0.1.65 PNG checks."""

from hashlib import file_digest
import json
from pathlib import Path
from runpy import run_path
import subprocess
import sys
from tempfile import TemporaryDirectory

from verify_forms_pixels import require
from verify_multiline import PREFIX, REPORT, verify as verify_multiline

ROOT = Path(__file__).resolve().parent
FOCUSED = Path("/tmp/surface-md3-textbox-values-e2e.ohBgx2tj")
FOCUSED_TOOLS = "f2d531957883dd30514ed4553e16c846b593a92bec703a0ef4ee6115e2c4f51e"
FOCUSED_RECEIPT = "62ee6008c8a701e3c58911277a86f6e9cd8ad9e8b968d4773b78d3dfd1e304ce"
FOCUSED_TEST = "9131d95a83f8544f8c6c448427fff763771d940474771e675ab4e1321fd865f3"
FOCUSED_SOURCE = "0776a6eb9e1d89f390889bf602762b44316dca2e056c0acf7e29f063dcfb7ee7"
FOCUSED_RUNNER = "3f6ffe6bb0b809c0894b2d3be113c9777fb733a3b510bd9089bf1ed8b3e585cf"
FOCUSED_WEB = "d37470c46d9b81fed27eab062b137633039bcbbf0712afee3d7047dd9b86a797"
FOCUSED_VERIFIER = "9b9c29a90214137cf7b3367e8c9512de5455d084eaa0dcd96a76b416dcce471b"
PRIOR_VERIFIER = "edbff9925ebfc4a2c3b4561a1cd86557ea1686b64f8cea315228028976b73ee2"
EXACT_SOURCE = "d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783"
EXPECTED_STEPS = [f"M{i}" for i in range(341, 347)]


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def read(path):
    return json.loads(path.read_text())


def manifest(path):
    return {name: digest for digest, name in
            (row.split("  ", 1) for row in path.read_text().splitlines())}


def verify_report(report, metadata):
    require(report["browser"] == metadata["browser"] and report["adapter"] == metadata["adapter"],
            "Textbox AX metadata does not match the stock Chrome and GPU")
    require(report.get("page_errors") == [] and not report.get("failure") and "inactive_ax" in report,
            "Missing the actual M346 AX snapshot or browser success")
    require(sha(ROOT / "verify-focused-original.py") == FOCUSED_VERIFIER, "Wrong retained focused AX verifier")
    snapshot = report["inactive_ax"]
    verifier = run_path(str(ROOT / "verify-focused-original.py"))
    values = verifier["verify_inactive"](snapshot)
    # These are actual nodes in the serialized Chrome AX report. The E2E
    # separately checks uniqueness among all named textboxes in the full tree.
    require(all(snapshot[name].get("childIds") == [] for name in values),
            "An inactive textbox has a separate AX value child")
    active = snapshot["active"]
    require(type(active["backend_dom_node"]) is int and active["backend_dom_node"] > 0,
            "Missing actual active textarea backend identity")
    require(values["fixed"]["value"].endswith("\n") and values["readonly"]["value"].endswith("\n")
            and not values["disabled"]["value"].endswith("\n")
            and values["single_line"]["multiline"] is False,
            "Inactive value lost the terminal line, disabled state or single-line distinction")
    return {"inactive_ax": values, "active_handoff": active,
            "no_ax_children": {name: True for name in values}}


def verify(artifacts, metadata):
    path = artifacts / REPORT
    report = read(path)
    values = verify_report(report, metadata)
    require(sha(ROOT / "verify_multiline.py") == PRIOR_VERIFIER, "Retained v0.1.65 pixel verifier differs")
    previous = dict(report)
    del previous["inactive_ax"]
    # Keep the old exact report schema and original pixels. Only the
    # local compatibility projection omits the newly added AX key.
    with TemporaryDirectory(prefix="surface-textbox-values-retained-", dir="/tmp") as temporary:
        root = Path(temporary)
        (root / REPORT).write_text(json.dumps(previous, ensure_ascii=False))
        for path_png in artifacts.glob(f"{PREFIX}-*.png"):
            (root / path_png.name).symlink_to(path_png.resolve(strict=True))
        retained = verify_multiline(root, metadata)
    result = sha(path)
    require(len(retained["pngs"]) == 12, "Retained multiline run lost a PNG")
    return {**retained, "report_sha256": result}, {**values, "report_sha256": result}


def focused_receipt(exact_manifest, canonical=None):
    run = FOCUSED / "development-r1"
    verifier = FOCUSED / "verify-focused.py"
    summary = FOCUSED / "verified-focused.json"
    require(sha(FOCUSED / "SHA256SUMS") == FOCUSED_TOOLS
            and sha(verifier) == FOCUSED_VERIFIER and sha(summary) == FOCUSED_RECEIPT,
            "The original focused harness or its positive receipt changed")
    for name, checksum in manifest(FOCUSED / "SHA256SUMS").items():
        require(sha(FOCUSED / name) == checksum, f"The original focused input changed: {name}")
    require(sha(exact_manifest) == EXACT_SOURCE, "Wrong exact v0.1.66 source for focused test binding")
    focused = FOCUSED / "focused-source.sha256"
    exact = manifest(exact_manifest)
    relative = "e2e/src/material_multiline_text_field.rs"
    require(sha(focused) == FOCUSED_SOURCE and exact[relative] == manifest(focused)[relative]
            == sha(FOCUSED / "source" / relative) == FOCUSED_TEST,
            "The focused and exact stock M346 source differ")
    output = subprocess.run([sys.executable, "-E", "-B", str(verifier), str(run)],
                            capture_output=True, check=False, text=True)
    require(output.returncode == 0, f"Original focused verifier rejected its captured artifacts: {output.stderr.strip()}")
    observed = json.loads(output.stdout)
    require(observed == read(summary) and observed["checks"] == 308 and observed["steps"] == EXPECTED_STEPS
            and len(observed["retained_m341_to_m345"]["pngs"]) == 12 and len(observed["inactive_ax"]) == 6
            and observed["runner_sha256"] == FOCUSED_RUNNER and observed["web_manifest_sha256"] == FOCUSED_WEB
            and observed["window_samples"] == 65 and observed["visible_windows"] == observed["survivors"] == 0,
            "Focused result no longer matches 308 checks, 12 PNGs and six AX values")
    report_path = run / "artifacts" / REPORT
    values = verify_report(read(report_path), {"browser": observed["browser"], "adapter": observed["adapter"]})
    require(observed["inactive_ax"] == values["inactive_ax"], "The retained M346 evidence differs")
    assets = read(run / "invocation.json")["web_files"]
    require(len(assets) == 4 and assets == manifest(FOCUSED / "web.sha256"),
            "The focused Gallery pack has the wrong file scope")
    if canonical is not None:
        present = {path.name: sha(path) for path in canonical.iterdir() if path.is_file()}
        require(present == assets, "The final canonical Gallery files differ from the focused run")
    return {**observed, "scope": "focused M341–M346 development, not stock",
            "exact_source_sha256": EXACT_SOURCE, "same_m346_test_sha256": FOCUSED_TEST,
            "focused_receipt_sha256": FOCUSED_RECEIPT, "gallery_files": assets,
            "canonical_gallery_match": canonical is not None}
