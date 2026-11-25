"""Bind the private q72 R8 selector to the exact R4 Gallery producer."""

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

from evidence_common import HEX, data, digest, json_file, path_entries, require, source_tree, verify_entries

ROOT = Path(__file__).resolve().parent
Q72 = Path("/tmp/surface-md3-field-pair-v0172.SEhFjvMD")
R8 = Q72 / "development-r8"
R8_SHA = "4a4ad2522f1c080fd870932077619d3c85e2171598e620e93dfbc2065944ea99"
R8_TREE = "6af2ed5be7232a482d09e5a924dec6ab85536651"
R8_PATCH = "10029cedbdb5396eb5712590d87b271489422d74fbfe9c712905083c33317886"
R7 = Q72 / "development-r7"
R7_SHA = "6d8d03a89b2e2a95b8c725efe04774badcc5e922b471d51bb5ed8ebdbca40135"
R7_TREE = "a353a2d2cff5f03489c718e92166beed79acabd7"
R7_PATCH = "15867f015d17b34e960a5f35f4666eda9d88032640f73d895134a4296a5cc66b"
R6 = Q72 / "development-r6"
R6_SHA = "cbaa196fa23a3e3ed9c3507c60e78bee7a5ab02aa737171b993cfbcf1c449f01"
R6_TREE = "ad845b3a8db2d315566df3075a0d6a456166be29"
R6_PATCH = "4929086bb0be195e006c81a4a44bde8280c3ad37518e99369dfe0626a22a9045"
R5 = Q72 / "development-r5"
R5_SHA = "80ae80c08efe8025bd1765154fa32343934cc4dcde8902a42a52cd2ce2ad5683"
R5_TREE = "debd8209e02bf63a4a9263c1c855ae1a3e1d39e6"
R5_PATCH = "29101edf50ad238d2e39c36b93c8a6a565507e393df1cfec7fb04529771f3e6e"
R4 = Q72 / "development-r4"
R4_SHA = "5c342f40941e6f5350411c7787faed8651356f8f739bd64f059bdb65622e8649"
R4_TREE = "166eeafa32115770dcf9097a63de31aa7d3201a3"
R4_PATCH = "45a5963f4556dfccc7b17ac860fba2bdee945e96a7105874de4670d2e835493e"
R3 = Q72 / "development-r3"
R3_SHA = "613944d9cd0c940b53b73b39ffe70d55c5a3deec13d37f10bb6c4afda1e9c10b"
R3_TREE = "98f202e8fbd1fe25d86a5c9add5acbe96b17329e"
R3_PATCH = "a56bbdb6e5100990c375f46c80d9038fa9716344d3a71fe7148373fc63616dd0"
R2 = Q72 / "development-r2"
R2_SHA = "5f61e5b748a70861fae0686215a79ba8c030bbeab06220175abe4526ab3d1233"
R2_TREE = "f56be19b1c093a24f648eef8e5e52d166ad5e7ab"
BASE = Path("/tmp/surface-md3-field-contrast-v0171.xE0ruhQh/exact-candidate/source.sha256")
BASE_SHA = "65c1bf0fd49f018b575cf069af016aa46229cff9dc23d521942f0ba0b6465ee2"
REGISTRY = "e2e/src/material.rs"
ENV_FLAG = "SURFACE_TEXT_FIELD_SYSTEM_PAIR_FOCUSED"
MODULES = ("material_web_contrast", "material_text_field_disabled_contrast", "material_text_field_system_pair")
WITNESS = "e2e/src/material_web_contrast.rs"
NEW = "e2e/src/material_text_field_system_pair.rs"
WEB_DELTA_R6 = [NEW, "e2e/src/material_web_contrast_selection.rs"]
WEB_DELTA_R7 = [NEW, WITNESS, "e2e/src/material_web_contrast_selection.rs"]
ALTERED = [REGISTRY, WITNESS]
R4_CHANGED = ["docs/material.md", "examples/material-gallery/src/lib.rs",
              "examples/material-gallery/src/text_field_placeholder_demo.rs"]
STEPS = [f"M{i}" for i in (*range(241, 248), *range(357, 363))]
FULL = [f"M{i}" for i in range(1, 363) if i != 23]


def manifest(path, expected, *, web=False):
    require(HEX.fullmatch(expected or "") and path.is_file() and not path.is_symlink() and digest(path) == expected,
            "Wrong reviewed manifest: " + path.name)
    return path_entries(path, allow_dot_prefix=web)


def registry(text):
    pattern = r"pub const STEPS: &\[&str\] = &\[(.*?)\];"
    full = re.findall(pattern, text, flags=re.S)
    require(len(full) == 1 and re.findall(r'"(M\d+)"', full[0]) == FULL, "Wrong ordered full Material registry")
    serve = '    let (_server, url) = env.serve("material-gallery")?;'
    require(text.count(serve) == 1 and ENV_FLAG not in text
            and all(text.count(f"mod {module};") == 1 for module in MODULES), "Unknown Material registry entry or module")
    branch = [f'    if std::env::var_os("{ENV_FLAG}").is_some() {{']
    branch.extend(f"        {module}::run(env, chrome, &url).await?;" for module in MODULES[:-1])
    branch.extend([f"        return {MODULES[-1]}::run(env, chrome, &url).await;", "    }"])
    selector = "pub const STEPS: &[&str] = &[" + ", ".join(f'"{step}"' for step in STEPS) + "];"
    return re.sub(pattern, selector, text, count=1, flags=re.S).replace(serve, serve + "\n" + "\n".join(branch))


def same_inventory(root, files):
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if not path.is_dir() or path.is_symlink()}
    require(actual == files.keys(), f"Wrong source inventory at {root}")


def test_delta(web, tested):
    added = sorted(tested.keys() - web.keys())
    removed = sorted(web.keys() - tested.keys())
    changed = sorted(name for name in web.keys() & tested.keys() if web[name] != tested[name])
    require(added == [NEW] and removed == [] and changed == ALTERED,
            "R2 Web producer differs outside the reviewed one-added, two-changed E2E paths")
    return {"added": added, "changed": changed, "removed": removed}


def approved_source():
    base = manifest(BASE, BASE_SHA)
    web = manifest(R2 / "source.sha256", R2_SHA)
    r3 = manifest(R3 / "source.sha256", R3_SHA)
    r4 = manifest(R4 / "source.sha256", R4_SHA)
    r5 = manifest(R5 / "source.sha256", R5_SHA)
    r6 = manifest(R6 / "source.sha256", R6_SHA)
    r7 = manifest(R7 / "source.sha256", R7_SHA)
    tested = manifest(R8 / "source.sha256", R8_SHA)
    require(len(base) == len(web) == 690 and web.keys() == base.keys()
            and len(r3) == len(r4) == len(r5) == len(r6) == len(r7) == len(tested) == 691,
            "Wrong exact q71 or q72 source inventory")
    previous = test_delta(web, r3)
    require(r4.keys() == r3.keys() and sorted(name for name in r3 if r3[name] != r4[name]) == R4_CHANGED,
            "R4 differs outside its reviewed Gallery correction and documentation")
    test_delta_r5(r4, r5)
    test_delta_r6(r4, r6)
    test_delta_r7(r4, r7)
    test_delta_r7(r4, tested)
    require(tested.keys() == r7.keys() and sorted(name for name in r7 if r7[name] != tested[name]) == [NEW],
            "R8 differs from R7 outside the reviewed viewport setup test")
    for stage, files, tree in ((R2, web, R2_TREE), (R3, r3, R3_TREE), (R4, r4, R4_TREE), (R5, r5, R5_TREE),
                                (R6, r6, R6_TREE), (R7, r7, R7_TREE), (R8, tested, R8_TREE)):
        root = stage / "source"
        same_inventory(root, files)
        verify_entries(root, files)
        require(source_tree(root, files) == tree and (stage / "source.tree").read_text() == tree + "\n"
                and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == "0.1.72",
                f"Wrong exact q72 source tree or version at {stage.name}")
    require(digest(Q72 / "development-r3.patch") == R3_PATCH and digest(Q72 / "development-r4.patch") == R4_PATCH
            and digest(Q72 / "development-r5.patch") == R5_PATCH and digest(Q72 / "development-r6.patch") == R6_PATCH
            and digest(Q72 / "development-r7.patch") == R7_PATCH and digest(Q72 / "development-r8.patch") == R8_PATCH,
            "Wrong reviewed q72 development patch")
    return tested, {"r2_to_r3": previous, "r3_to_r4": {"added": [], "removed": [], "changed": R4_CHANGED},
                    "r4_to_r5": {"added": [], "removed": [], "changed": [NEW]},
                    "r4_to_r6": {"added": [], "removed": [], "changed": WEB_DELTA_R6},
                    "r4_to_r7": {"added": [], "removed": [], "changed": WEB_DELTA_R7},
                    "r4_to_r8": {"added": [], "removed": [], "changed": WEB_DELTA_R7},
                    "r7_to_r8": {"added": [], "removed": [], "changed": [NEW]}}


def test_delta_r5(producer, tested):
    require(producer.keys() == tested.keys(), "R4 and R5 source inventories differ")
    changed = sorted(name for name in producer if producer[name] != tested[name])
    require(changed == [NEW], "R5 differs outside its one reviewed E2E file")
    return changed


def test_delta_r6(producer, tested):
    require(producer.keys() == tested.keys(), "R4 and R6 source inventories differ")
    changed = sorted(name for name in producer if producer[name] != tested[name])
    require(changed == WEB_DELTA_R6, "R6 differs outside its two reviewed E2E files")
    return changed


def test_delta_r7(producer, tested):
    require(producer.keys() == tested.keys(), "R4 and R7 source inventories differ")
    changed = sorted(name for name in producer if producer[name] != tested[name])
    require(changed == WEB_DELTA_R7, "R7 differs outside its three reviewed E2E files")
    return changed


def scope_data(original, source_delta, focused_sha, registry_sha):
    return {"product_source": str((R8 / "source").resolve(strict=True)),
            "product_source_manifest": str((R8 / "source.sha256").resolve(strict=True)),
            "product_source_manifest_sha256": R8_SHA, "product_tree": R8_TREE,
            "development_patch_sha256": R8_PATCH, "source_count": len(original),
            "focused_source_manifest_sha256": focused_sha, "stock_registry_sha256": original[REGISTRY],
            "focused_registry_sha256": registry_sha, "registry_delta": [REGISTRY], "steps": STEPS,
            "development_history": source_delta}


def make_scope():
    original, source_delta = approved_source()
    target = ROOT / "source"
    require(not target.exists() and not (ROOT / "scope-pin.json").exists(), "Use a fresh private scope")
    subprocess.run(["/bin/cp", "-cR", str(R8 / "source"), str(target)], check=True)
    shutil.copyfile(R8 / "source.sha256", ROOT / "original-source.sha256")
    selected = target / REGISTRY
    selected.write_text(registry(selected.read_text()))
    focused = {**original, REGISTRY: digest(selected)}
    seal = ROOT / "focused-source.sha256"
    seal.write_text("".join(f"{sha}  {name}\n" for name, sha in sorted(focused.items())))
    json_file(ROOT / "scope-pin.json", scope_data(original, source_delta, digest(seal), focused[REGISTRY]))
    return check_scope()


def check_scope():
    original, source_delta = approved_source()
    pin = data(ROOT / "scope-pin.json")
    require(manifest(ROOT / "original-source.sha256", R8_SHA) == original, "Copied original source differs")
    focused = manifest(ROOT / "focused-source.sha256", pin["focused_source_manifest_sha256"])
    target = ROOT / "source"
    same_inventory(target, focused)
    verify_entries(target, focused)
    require(focused.keys() == original.keys()
            and sorted(name for name in original if original[name] != focused[name]) == [REGISTRY]
            and (target / REGISTRY).read_text() == registry((R8 / "source" / REGISTRY).read_text())
            and pin == scope_data(original, source_delta, digest(ROOT / "focused-source.sha256"), focused[REGISTRY]),
            "Scoped source changed more than its one private registry")
    return pin


def web_inputs(pack, manifest_path, sha):
    files = manifest(manifest_path, sha, web=True)
    require(len(files) == 4 and {"index.html", "surface-web.json"} <= files.keys()
            and all(Path(name).parent == Path(".") for name in files)
            and sum(name.endswith(".js") for name in files) == sum(name.endswith(".wasm") for name in files) == 1
            and files.keys() == {file.name for file in pack.iterdir()}, "Expected exactly four canonical Gallery originals")
    verify_entries(pack, files)
    return files


def gallery_data(web_sha):
    origin = (Q72 / "web-development").resolve(strict=True)
    digest_file = (Q72 / "web-development.sha256").resolve(strict=True)
    require(origin == (Q72.resolve() / "web-development") and digest_file == (Q72.resolve() / "web-development.sha256"),
            "Wrong canonical Gallery path")
    tested = manifest(ROOT / "original-source.sha256", R8_SHA)
    producer_file = (R4 / "source.sha256").resolve(strict=True)
    producer = manifest(producer_file, R4_SHA)
    delta = test_delta_r7(producer, tested)
    return {"original_gallery": str(origin), "original_web_manifest": str(digest_file),
            "web_manifest_sha256": web_sha, "web_files": web_inputs(origin, digest_file, web_sha),
            "product_source_manifest_sha256": R8_SHA, "web_producer_manifest": str(producer_file),
            "web_producer_manifest_sha256": R4_SHA, "web_producer_tree": R4_TREE,
            "web_compatible_test_only_delta": delta}


def pin_gallery(web_sha):
    check_scope()
    gallery = gallery_data(web_sha)
    dest = ROOT / "web/material-gallery"
    require(not dest.exists() and not dest.is_symlink() and not (ROOT / "gallery-pin.json").exists(),
            "Do not replace a pinned Gallery")
    dest.parent.mkdir()
    dest.symlink_to(gallery["original_gallery"], target_is_directory=True)
    json_file(ROOT / "gallery-pin.json", gallery)
    return check_gallery()


def check_gallery():
    saved = data(ROOT / "gallery-pin.json")
    dest = ROOT / "web/material-gallery"
    require(saved == gallery_data(saved["web_manifest_sha256"]) and dest.is_symlink()
            and dest.resolve(strict=True) == Path(saved["original_gallery"]), "Pinned Gallery origin or bytes changed")
    return saved


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    operations = parser.add_subparsers(dest="operation", required=True)
    operations.add_parser("prepare")
    operations.add_parser("recheck-source")
    operations.add_parser("recheck")
    operations.add_parser("verify-product-only")
    pin = operations.add_parser("pin-gallery")
    pin.add_argument("--web-sha256", required=True)
    args = parser.parse_args()
    if args.operation == "prepare":
        result = make_scope()
    elif args.operation == "pin-gallery":
        result = pin_gallery(args.web_sha256)
    elif args.operation == "verify-product-only":
        files, delta = approved_source()
        result = {"source_count": len(files), "product_tree": R8_TREE, "development_history": delta}
    else:
        result = check_scope()
        if args.operation == "recheck":
            result.update(check_gallery())
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
