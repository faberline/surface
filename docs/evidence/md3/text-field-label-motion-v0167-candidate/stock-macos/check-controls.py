"""Read-only v0.1.67 preflight against original scoped and prior stock receipts."""

from copy import deepcopy
import json
from pathlib import Path
import re
from runpy import run_path
from shutil import copytree
import subprocess
import sys
from tempfile import TemporaryDirectory

from PIL import Image

from verify_forms_pixels import require
from verify_label_motion import FOCUSED, PREFIX, focused_receipt, read, sha, verify

ROOT = Path(__file__).resolve().parent
STAGE = Path("/tmp/surface-md3-text-field-label-motion-v0167.Y7f9HqIi")
PRIOR = Path("/tmp/surface-md3-textbox-values-stock-v0166.z81UZBVd")
BASE_SOURCE = "d4b0c5b7a1ae5daf1f7a9084e765b30b24f8cae6ecbda881efb9468daf730783"
BASE_STOCK = "47d397e905a928079411fcf4af821b095a7e76c6e08ae56d4f5cc81483fbeb52"
BASE_HARNESS = "d1d73190c1bb5f08225369777b5a8a20530449ff98ebab40ffc44744df8f4ea9"
PATCH_SHA = "d3b38aac44fad4cb071fcf2734e1edc935f741af2e6e5d710da92b2b8b40c6ba"
OBS = run_path(str(ROOT / "observe-multiline.py"))


def reject(action, note):
    try:
        action()
    except (ValueError, RuntimeError, KeyError) as error:
        require(note in str(error), f"Unexpected rejection: {error}")
        return str(error)
    raise RuntimeError(f"Accepted defective input: {note}")


def checked_source():
    exact = STAGE / "exact-candidate"
    manifest = exact / "source.sha256"
    require(sha(manifest) == OBS["SOURCE_SHA"] and (STAGE / "source-tree").read_text().strip() == OBS["SOURCE_TREE"],
            "Wrong exact v0.1.67 source pin")
    files = OBS["checksums"](manifest)
    require(len(files) == OBS["SOURCE_FILES"], "Wrong exact source inventory")
    OBS["verify_entries"](exact / "source", files)
    OBS["verify_entries"](STAGE / "independent-apply-source", files)
    OBS["check_source_version"](exact / "source")
    patch = STAGE / "text-field-label-motion-v0167.patch"
    prior_source = PRIOR / "browser-source.sha256"
    require(sha(patch) == PATCH_SHA and sha(prior_source) == BASE_SOURCE and (STAGE / "exact-apply-checked").is_file(),
            "Independent .66-to-.67 patch provenance differs")
    base = OBS["checksums"](prior_source)
    changed = {path for path in files.keys() | base.keys() if files.get(path) != base.get(path)}
    patch_paths = set(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M))
    require(changed == patch_paths and len(changed) == 17, "Wrong exact patch path scope")
    material = (exact / "source/e2e/src/material.rs").read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require([int(item) for item in re.findall(r'"M(\d+)"', material)] ==
            [i for i in range(1, 351) if i != 23], "Final registry lost a Material step")
    return manifest, {
        "files": len(files), "tree": OBS["SOURCE_TREE"], "source_sha256": OBS["SOURCE_SHA"],
        "patch_sha256": PATCH_SHA, "changed_paths": sorted(changed),
    }


def prior_stock(controls):
    original = PRIOR / "stock-mac-exact-offline.json"
    require(sha(original) == BASE_STOCK and sha(PRIOR / "harness.sha256") == BASE_HARNESS,
            "Prior exact stock fixture changed")
    for name, digest in OBS["checksums"](PRIOR / "harness.sha256").items():
        require(sha(PRIOR / name) == digest, f"Prior harness bytes differ: {name}")
        if name.startswith("verify_") or name == "verify-focused-original.py":
            require(sha(ROOT / name) == digest, f"Inherited validator changed: {name}")
    result = subprocess.run([sys.executable, "-E", "-B", str(PRIOR / "verify-stock-receipt.py"), str(PRIOR / "run")],
                            capture_output=True, check=False, text=True)
    require(result.returncode == 0, f"Original stock verification failed: {result.stderr.strip()}")
    prior = json.loads(result.stdout)
    require(prior == read(original) and prior["total_checks"] == 24750 and prior["cases"]["material"]["steps"] == 345
            and prior["release"]["last_material_step"] == 346, "Prior exact stock replay differs")
    log = (PRIOR / "run/runner.log").read_text()
    require(OBS["verify_runner_log"](log, 346) == (prior["cases"], 24750),
            "New reader does not retain complete prior coverage")
    controls["prior_stock_is_not_v0167"] = reject(lambda: OBS["verify_runner_log"](log), "Material steps changed")
    return {"version": "0.1.66", "checks": 24750, "material_steps": 345,
            "last_material": 346, "receipt_sha256": BASE_STOCK}


def negative_controls(focus, controls):
    scoped = run_path(str(ROOT / "verify-label-motion-focused-original.py"))
    for capture in ("development-r1", "development-r2"):
        controls[f"guard_negative_{capture}"] = reject(
            lambda: scoped["verify_run"](FOCUSED / capture, False), "did not naturally exit")
    run = FOCUSED / "development-r3"
    controls["scoped_is_not_stock"] = reject(
        lambda: OBS["verify_runner_log"]((run / "runner.log").read_text()), "exactly the ordered five")
    metadata = {key: focus[key] for key in ("browser", "adapter")}
    with TemporaryDirectory(prefix="surface-label-motion-controls-", dir="/tmp") as temporary:
        artifacts = Path(temporary) / "artifacts"
        originals = run / "artifacts"
        copytree(originals, artifacts, ignore=lambda _directory, names:
                 [name for name in names if not name.startswith(PREFIX)])
        raw = artifacts / f"{PREFIX}.json"
        exact = read(raw)
        wrong = deepcopy(exact)
        wrong["outlined"]["active"]["description"] = "Wrong label"
        raw.write_text(json.dumps(wrong))
        controls["wrong_active_ax"] = reject(lambda: verify(artifacts, metadata), "textbox name, description or focus")
        raw.write_bytes((originals / raw.name).read_bytes())
        controls["wrong_adapter"] = reject(lambda: verify(artifacts, {**metadata, "adapter": {}}),
                                            "Chrome or WebGPU adapter differs")

        missing = artifacts / f"{PREFIX}-rtl-middle.png"
        missing.rename(missing.with_suffix(".old"))
        controls["missing_png"] = reject(lambda: verify(artifacts, metadata), "no original")
        missing.with_suffix(".old").rename(missing)

        early = artifacts / f"{PREFIX}-outlined-early.png"
        x = int((exact["outlined"]["floated"][0] - 2) * 2)
        y = int((exact["outlined"]["face"][1] + 44.25) * 2)
        with Image.open(early) as source:
            changed = source.convert("RGB")
        changed.putpixel((x, y), tuple(exact["outlined"]["palette"]["primary"][:3]))
        changed.save(early, format="PNG")
        controls["tampered_partial_notch_pixel"] = reject(lambda: verify(artifacts, metadata), "original notch/sides differ")

    guard, marker = OBS["guard_verdict"], OBS["JET_MARKER"]
    temp = Path("/tmp").resolve() / "headless-updater-control"
    minimal = [f"--user-data-dir={temp / 'chrome'}", "--headless=new"]
    flag, = OBS["CHROME_FLAGS"]
    for role, env in (("chrome-primary", {}), ("chrome-bridge", {marker: "1"})):
        require(guard("chrome", [*minimal, flag], env, temp)[:2] == (role, True), "safe guarded launch rejected")
        for unsafe in (minimal, [*minimal, flag, flag], [*minimal, "--", flag]):
            require(guard("chrome", unsafe, env, temp)[:2] == (role, False), "invalid launch accepted")
    controls["invalid_chrome_launch_option"] = "Both Chrome guards reject absent, duplicate and post-delimiter flags"


def main():
    manifest, source = checked_source()
    focus = focused_receipt(manifest)
    controls = {}
    negative_controls(focus, controls)
    prior = prior_stock(controls)
    print(json.dumps({
        "scope": "offline preflight only", "source": source, "prior_actual_stock": prior,
        "actual_focused": focus, "expected_exact_material_steps": 349, "negative_controls": controls,
    }, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
