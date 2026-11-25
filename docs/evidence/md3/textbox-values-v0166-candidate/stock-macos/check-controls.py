"""Offline checks for the .66 wrapper using the real .65 stock and .66 focused receipts."""

from copy import deepcopy
import json
from pathlib import Path
import re
from runpy import run_path
import subprocess
import sys

from verify_forms_pixels import require
from verify_multiline import REPORT
from verify_textbox_values import FOCUSED, focused_receipt, read, sha, verify, verify_report

ROOT = Path(__file__).resolve().parent
STAGE = Path("/tmp/surface-md3-textbox-values-v0166.l4950l9e")
PRIOR = Path("/tmp/surface-md3-multiline-stock-v0165.C31fd1AG")
BASE_SOURCE = "6a420b56fc090457f9841fc8fda7c1bf0602e27138f586ca3f6b251652b580f2"
BASE_STOCK = "94522e0336f0c2002e5faa8b6dfc7503b9885e3b81984cea3082cbb53b32a1c0"
BASE_HARNESS = "ebc28ad807afa006f250c67c427103587a0a89f7d5bcd65d9843b4ea6971937f"
PATCH_SHA = "a5a09a61580a7ae186d0fc95477d40582bba8799b21773e387bc21c4e38760e1"
OBS = run_path(str(ROOT / "observe-multiline.py"))


def reject(action, note):
    try:
        action()
    except (ValueError, RuntimeError, KeyError) as error:
        require(note in str(error), f"Unexpected rejection: {error}")
        return str(error)
    raise RuntimeError(f"Accepted defective input: {note}")


def main():
    exact = STAGE / "exact-candidate"
    manifest = exact / "source.sha256"
    require(sha(manifest) == OBS["SOURCE_SHA"] and (STAGE / "source-tree").read_text().strip() == OBS["SOURCE_TREE"],
            "Wrong true .66 source pin")
    files = OBS["checksums"](manifest)
    require(len(files) == 681, "Wrong exact source inventory")
    OBS["verify_entries"](exact / "source", files)
    OBS["check_source_version"](exact / "source")
    patch = STAGE / "textbox-values-v0166.patch"
    prior_source = PRIOR / "browser-source.sha256"
    require(sha(patch) == PATCH_SHA and sha(prior_source) == BASE_SOURCE and (STAGE / "exact-apply-checked").is_file(),
            "Exact independent .65-to-.66 patch provenance differs")
    base = OBS["checksums"](prior_source)
    changed = {path for path in files.keys() | base.keys() if files.get(path) != base.get(path)}
    patch_paths = set(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M))
    require(changed == patch_paths == set((STAGE / "owned-textbox-values.paths").read_text().splitlines())
            and len(changed) == 8, "Wrong eight-path patch scope")
    material = (exact / "source/e2e/src/material.rs").read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require([int(item) for item in re.findall(r'"M(\d+)"', material)] ==
            [i for i in range(1, 347) if i != 23], "Final registry lost a Material step")

    focus = focused_receipt(manifest)
    artifacts = FOCUSED / "development-r1/artifacts"
    report = read(artifacts / REPORT)
    metadata = {key: focus[key] for key in ("browser", "adapter")}
    retained, values = verify(artifacts, metadata)
    require(json.loads(json.dumps(retained["pngs"])) == focus["retained_m341_to_m345"]["pngs"]
            and values["inactive_ax"] == focus["inactive_ax"], "Independent focused stock-style adapter differs")
    controls = {}
    defective = deepcopy(report)
    defective.pop("inactive_ax")
    controls["missing_ax"] = reject(lambda: verify_report(defective, metadata), "Missing the actual M346 AX snapshot")
    defective = deepcopy(report)
    defective["inactive_ax"]["fixed"]["value"]["value"] = defective["inactive_ax"]["fixed"]["value"]["value"].rstrip("\n")
    controls["truncated_terminal_line"] = reject(lambda: verify_report(defective, metadata), "fixed has unexpected")
    defective = deepcopy(report)
    for item in defective["inactive_ax"]["single_line"]["properties"]:
        if item["name"] == "multiline":
            item["value"]["value"] = True
    controls["single_line_wrong_mode"] = reject(lambda: verify_report(defective, metadata), "single_line has unexpected")
    defective = deepcopy(report)
    defective["inactive_ax"]["disabled"]["childIds"] = ["invented-duplicate"]
    controls["extra_child"] = reject(lambda: verify_report(defective, metadata), "separate AX value child")
    defective = deepcopy(report)
    defective["inactive_ax"]["active"]["backend_dom_node"] = defective["inactive_ax"]["before"]["backendDOMNodeId"]
    controls["wrong_handoff"] = reject(lambda: verify_report(defective, metadata), "textarea handoff")

    original = PRIOR / "stock-mac-exact-offline.json"
    require(sha(original) == BASE_STOCK and sha(PRIOR / "harness.sha256") == BASE_HARNESS,
            "Prior exact stock fixture changed")
    for name, digest in OBS["checksums"](PRIOR / "harness.sha256").items():
        require(sha(PRIOR / name) == digest, f"Prior harness bytes differ: {name}")
    for name in OBS["checksums"](PRIOR / "harness.sha256"):
        if name.startswith("verify_"):
            require(sha(ROOT / name) == sha(PRIOR / name), f"Inherited PNG verifier changed: {name}")
    result = subprocess.run([sys.executable, "-E", "-B", str(PRIOR / "verify-stock-receipt.py"), str(PRIOR / "run")],
                            capture_output=True, text=True, check=False)
    require(result.returncode == 0, f"Prior original stock verification failed: {result.stderr.strip()}")
    prior = json.loads(result.stdout)
    require(prior == read(original) and prior["total_checks"] == 24689 and prior["cases"]["material"]["steps"] == 344
            and prior["release"]["last_material_step"] == 345, "Prior exact stock replay differs")
    log = (PRIOR / "run/runner.log").read_text()
    require(OBS["verify_runner_log"](log, 345) == (prior["cases"], 24689),
            "New wrapper does not retain the complete prior log coverage")
    controls["missing_new_m346"] = reject(lambda: OBS["verify_runner_log"](log), "Material steps changed")
    print(json.dumps({
        "scope": "offline preflight only", "source": {"files": len(files), "tree": OBS["SOURCE_TREE"],
        "source_sha256": OBS["SOURCE_SHA"], "patch_sha256": PATCH_SHA, "changed_paths": sorted(changed)},
        "prior_actual_stock": {"version": "0.1.65", "checks": 24689, "material_steps": 344,
                               "last_material": 345, "receipt_sha256": BASE_STOCK},
        "actual_focused": focus, "expected_exact_material_steps": 345, "negative_controls": controls,
    }, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
