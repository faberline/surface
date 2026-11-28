"""Recheck exact .67 stock and successful .68 R2 source and pixels offline."""

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

from verify_label_color import FOCUSED, PREFIX, STAGE, blend, focused_receipt, ink_triplet, read, require, sha, verify, verify_frame

ROOT = Path(__file__).resolve().parent
PRIOR = Path("/tmp/surface-md3-field-label-motion-stock-v0167.ITHgCb8e")
BASE_SOURCE = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
BASE_STOCK = "e60dd517932766d4e28569649063e89caf7aac6535441e085726f650a9267038"
BASE_HARNESS = "9e541b1d047e880186941c1e927a10133fa3f2d0fc8e226f2053e3c49abe3ed9"
PATCH_SHA = "05814fa4065bf7b65ce1b85023ae79623fafc0abb4870555368b71153c3463b9"
OBS = run_path(str(ROOT / "observe-multiline.py"))
LAST_MATERIAL = OBS["LAST_MATERIAL"]


def reject(action, reason):
    try:
        action()
    except (ValueError, RuntimeError, KeyError) as error:
        require(reason in str(error), f"unexpected rejection: {error}")
        return str(error)
    raise RuntimeError(f"accepted defective input: {reason}")


def checked_source():
    candidate = STAGE / "exact-candidate"
    manifest = candidate / "source.sha256"
    require(sha(manifest) == OBS["SOURCE_SHA"] and (STAGE / "source-tree").read_text().strip() == OBS["SOURCE_TREE"],
            "wrong exact v0.1.68 source")
    files = OBS["checksums"](manifest)
    require(len(files) == OBS["SOURCE_FILES"], "wrong exact source inventory")
    OBS["verify_entries"](candidate / "source", files)
    OBS["verify_entries"](STAGE / "independent-apply-source", files)
    OBS["check_source_version"](candidate / "source")
    patch = STAGE / "text-field-label-color-v0168.patch"
    prior_source = PRIOR / "browser-source.sha256"
    require(sha(patch) == PATCH_SHA and sha(prior_source) == BASE_SOURCE and (STAGE / "exact-apply-checked").is_file(),
            "independently applied .67-to-.68 patch provenance differs")
    base = OBS["checksums"](prior_source)
    changed = {name for name in files.keys() | base.keys() if files.get(name) != base.get(name)}
    patch_paths = set(re.findall(r"^diff --git a/(\S+) b/\S+$", patch.read_text(), re.M))
    require(changed == patch_paths and len(changed) == 10, "wrong exact ten-path patch")
    registry = candidate / "source/e2e/src/material.rs"
    stages = registry.read_text().split("pub const STEPS:", 1)[1].split("];", 1)[0]
    require([int(i) for i in re.findall(r'"M(\d+)"', stages)] == [i for i in range(1, 351) if i != 23],
            "the stock source lost an ordered Material step")
    return manifest, {"files": len(files), "tree": OBS["SOURCE_TREE"], "source_sha256": OBS["SOURCE_SHA"],
                      "patch_sha256": PATCH_SHA, "changed_paths": sorted(changed)}


def prior_stock(controls):
    original = PRIOR / "stock-mac-exact-offline.json"
    require(sha(original) == BASE_STOCK and sha(PRIOR / "harness.sha256") == BASE_HARNESS,
            "prior stock fixture differs")
    for name, digest in OBS["checksums"](PRIOR / "harness.sha256").items():
        require(sha(PRIOR / name) == digest, f"prior harness file differs: {name}")
        if name == "verify-focused-original.py" or (name.startswith("verify_") and name != "verify_label_motion.py"):
            require(sha(ROOT / name) == digest, f"inherited validator differs: {name}")
    result = subprocess.run([sys.executable, "-E", "-B", str(PRIOR / "verify-stock-receipt.py"), str(PRIOR / "run")],
                            capture_output=True, check=False, text=True)
    require(result.returncode == 0, f".67 original stock rejected: {result.stderr.strip()}")
    prior = json.loads(result.stdout)
    require(prior == read(original) and prior["total_checks"] == 24973 and prior["cases"]["material"]["steps"] == 349
            and prior["release"]["version"] == "0.1.67", "original .67 stock replay differs")
    run = PRIOR / "run"
    require(OBS["verify_runner_log"]((run / "runner.log").read_text()) == (prior["cases"], 24973),
            "new observer changed the inherited suite or ordered ledger checks")
    previous = run / "artifacts"
    metadata = {key: prior[key] for key in ("browser", "adapter")}
    controls["old_stock_missing_actual_color_coverage"] = reject(lambda: verify(previous, metadata),
                                                               "missing the typed .68 color test")
    old = read(previous / f"{PREFIX}.json")["outlined"]
    idle, primary, _ = ink_triplet(old)
    expected = blend(idle, primary, 44)
    typed = {**old["frames"][3], "label_rgb": list(expected)}
    controls["old_stock_pixels_reject_fake_midpoint"] = reject(
        lambda: verify_frame(previous, old, typed, "outlined-middle", 44, "outlined", True, expected),
        "reported label missing")
    return {"version": "0.1.67", "checks": 24973, "material_steps": 349, "receipt_sha256": BASE_STOCK}


def fresh_controls(focus, controls):
    out = FOCUSED / "development-r2"
    scoped = (out / "runner.log").read_text()
    controls["scoped_is_not_full_stock"] = reject(lambda: OBS["verify_runner_log"](scoped),
                                                  "exactly the ordered five")
    originals = out / "artifacts"
    metadata = {key: focus[key] for key in ("browser", "adapter")}
    result = verify(originals, metadata)
    require({name: file["sha256"] for name, file in result["pngs"].items()} == focus["captures"],
            "stock adapter does not match the actual R2 originals")
    with TemporaryDirectory(prefix="surface-label-color-stock-controls-", dir="/tmp") as temp:
        assets = Path(temp) / "artifacts"
        copytree(originals, assets, ignore=lambda _directory, names:
                 [name for name in names if not name.startswith(PREFIX)])
        typed = assets / f"{PREFIX}.json"
        old = read(typed)
        damaged = deepcopy(old)
        damaged["label_color"]["outlined"]["ax"]["error"]["value"] = "wrong"
        typed.write_text(json.dumps(damaged))
        controls["active_ax_lost_draft"] = reject(lambda: verify(assets, metadata), "fixed AX value")
        typed.write_bytes((originals / typed.name).read_bytes())
        missing = assets / f"{PREFIX}-rtl-above-label-middle.png"
        missing.rename(missing.with_suffix(".offline-removed"))
        controls["dark_wrap_missing_midpoint_png"] = reject(lambda: verify(assets, metadata), "no original")
        missing.with_suffix(".offline-removed").rename(missing)
        mid = assets / f"{PREFIX}-outlined-color-error-mid.png"
        x, y = old["label_color"]["outlined"]["probe_points"][0]
        with Image.open(mid) as png:
            altered = png.convert("RGB")
        altered.putpixel((int(x * 2), int((y + 44) * 2)), (165, 54, 61))
        altered.save(mid, format="PNG")
        controls["one_wrong_momentum_probe"] = reject(lambda: verify(assets, metadata),
                                                        "actual original ink does not match the golden color")


def main():
    manifest, source = checked_source()
    focus = focused_receipt(manifest)
    controls = {}
    prior = prior_stock(controls)
    fresh_controls(focus, controls)
    print(json.dumps({
        "scope": "offline preflight only", "source": source, "prior_actual_stock": prior,
        "actual_focused": focus, "expected_exact_material_steps": 349, "negative_controls": controls,
    }, ensure_ascii=False, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
