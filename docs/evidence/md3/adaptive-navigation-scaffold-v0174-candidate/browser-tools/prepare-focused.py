"""Bind the private Navigation selector to the v0.1.73 R3 tests and R2 Gallery build."""

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tomllib

from evidence_common import HEX, data, digest, json_file, path_entries, require, source_tree, verify_entries

ROOT = Path(__file__).resolve().parent
Q73 = Path("/tmp/surface-md3-horizontal-nav-v0173.OEjA5TvP")
R2 = Q73 / "development-r2"
R2_SHA = "13716392d80ba252314d2be056d41b7c4812e85347684cca5ee671acd6cf836c"
R2_TREE = "6f2b29fb151e0544d2d5eeb06ed4e3df07a5cd85"
R3 = Q73 / "development-r3"
R3_SHA = "38cdcd510d2808cd3685691ae0656c25ca1017f7c3cbf8d88af42c08370caa27"
R3_TREE = "1bfec92f8370dee480c497034be62f09310986ba"
R3_PATCH = "8238f6e4a80b4d7a6e888605be5a2d79c4eca391f61e67418d2826a912aa917e"
WEB_SHA = "d4d194543bb95dda8505681fee1a05bdfb9321e0ad458a96a5c9a9a6f803cc69"
WEB_DELTA = ["e2e/src/material_horizontal_navigation_bar.rs"]
REGISTRY = "e2e/src/material.rs"
ENV_FLAG = "SURFACE_HORIZONTAL_NAVIGATION_BAR_FOCUSED"
MODULES = ("material_flexible_navigation_bar", "material_horizontal_navigation_bar")
STEPS = [f"M{i}" for i in (*range(270, 274), *range(363, 366))]
FULL = [f"M{i}" for i in range(1, 366) if i != 23]
SCOPE = "q73 R3 focused Compact M270–M273 and Horizontal M363–M365"


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
    branch = (
        f'    if std::env::var_os("{ENV_FLAG}").is_some() {{',
        f"        {MODULES[0]}::run(env, chrome, &url).await?;",
        f"        return {MODULES[1]}::run(env, chrome, &url).await;",
        "    }",
    )
    selector = "pub const STEPS: &[&str] = &[" + ", ".join(f'"{step}"' for step in STEPS) + "];"
    return re.sub(pattern, selector, text, count=1, flags=re.S).replace(serve, serve + "\n" + "\n".join(branch))


def same_inventory(root, files):
    actual = {path.relative_to(root).as_posix() for path in root.rglob("*") if not path.is_dir() or path.is_symlink()}
    require(actual == files.keys(), f"Wrong source inventory at {root}")


def approved_snapshot(snapshot, expected_sha, expected_tree):
    files = manifest(snapshot / "source.sha256", expected_sha)
    root = snapshot / "source"
    require(len(files) == 692 and all(f"e2e/src/{module}.rs" in files for module in MODULES),
            "Wrong exact source inventory or missing Navigation case")
    same_inventory(root, files)
    verify_entries(root, files)
    require(source_tree(root, files) == expected_tree
            and (snapshot / "source.tree").read_text() == expected_tree + "\n"
            and tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"] == "0.1.73",
            "Wrong exact tree or workspace version")
    return files


def approved_source():
    files = approved_snapshot(R3, R3_SHA, R3_TREE)
    require(digest(Q73 / "development-r3.patch") == R3_PATCH, "Wrong reviewed R3 development patch")
    return files


def compatibility_delta():
    producer = approved_snapshot(R2, R2_SHA, R2_TREE)
    tested = approved_source()
    require(producer.keys() == tested.keys(), "R2 and R3 source inventories differ")
    delta = sorted(name for name in producer if producer[name] != tested[name])
    require(delta == WEB_DELTA and all(
        (R2 / "source" / name).stat().st_mode == (R3 / "source" / name).stat().st_mode
        for name in producer
    ), "Unexpected R2–R3 source change")
    return delta


def scope_data(original, focused_sha, registry_sha):
    return {"scope": SCOPE, "product_source": str((R3 / "source").resolve(strict=True)),
            "product_source_manifest": str((R3 / "source.sha256").resolve(strict=True)),
            "product_source_manifest_sha256": R3_SHA, "product_tree": R3_TREE,
            "development_patch_sha256": R3_PATCH, "source_count": len(original),
            "focused_source_manifest_sha256": focused_sha, "stock_registry_sha256": original[REGISTRY],
            "focused_registry_sha256": registry_sha, "registry_delta": [REGISTRY],
            "modules": list(MODULES), "steps": STEPS}


def make_scope():
    original = approved_source()
    target = ROOT / "source"
    require(not target.exists() and not (ROOT / "scope-pin.json").exists(), "Use a fresh private scope")
    subprocess.run(["/bin/cp", "-cR", str(R3 / "source"), str(target)], check=True)
    shutil.copyfile(R3 / "source.sha256", ROOT / "original-source.sha256")
    selected = target / REGISTRY
    selected.write_text(registry(selected.read_text()))
    focused = {**original, REGISTRY: digest(selected)}
    seal = ROOT / "focused-source.sha256"
    seal.write_text("".join(f"{sha}  {name}\n" for name, sha in sorted(focused.items())))
    json_file(ROOT / "scope-pin.json", scope_data(original, digest(seal), focused[REGISTRY]))
    return check_scope()


def check_scope():
    original = approved_source()
    pin = data(ROOT / "scope-pin.json")
    require(manifest(ROOT / "original-source.sha256", R3_SHA) == original, "Copied original source differs")
    focused = manifest(ROOT / "focused-source.sha256", pin["focused_source_manifest_sha256"])
    target = ROOT / "source"
    same_inventory(target, focused)
    verify_entries(target, focused)
    require(focused.keys() == original.keys()
            and sorted(name for name in original if original[name] != focused[name]) == [REGISTRY]
            and (target / REGISTRY).read_text() == registry((R3 / "source" / REGISTRY).read_text())
            and pin == scope_data(original, digest(ROOT / "focused-source.sha256"), focused[REGISTRY]),
            "Scoped source changed outside its private selector")
    return pin


def web_inputs(pack, manifest_path, sha):
    files = manifest(manifest_path, sha, web=True)
    require(len(files) == 4 and {"index.html", "surface-web.json"} <= files.keys()
            and all(Path(name).parent == Path(".") for name in files)
            and sum(name.endswith(".js") for name in files) == sum(name.endswith(".wasm") for name in files) == 1
            and files.keys() == {file.name for file in pack.iterdir()}, "Expected exactly four sealed Gallery originals")
    verify_entries(pack, files)
    return files


def verify_web_producer(marker):
    require(marker.is_file() and not marker.is_symlink() and marker.read_text() == R2_SHA + "\n",
            "No controller-verified Gallery marker for the exact R2 producer")


def gallery_data(web_sha):
    require(web_sha == WEB_SHA, "Wrong reviewed R2 Gallery asset manifest")
    delta = compatibility_delta()
    stage = Q73.resolve(strict=True)
    verify_web_producer(stage / "web-development-built")
    origin = stage / "web-development"
    manifest_path = stage / "web-development.sha256"
    require(origin.is_dir() and not origin.is_symlink(), "Wrong canonical Gallery location")
    return {"original_gallery": str(origin), "original_web_manifest": str(manifest_path),
            "web_manifest_sha256": web_sha, "web_files": web_inputs(origin, manifest_path, web_sha),
            "product_source_manifest_sha256": R3_SHA, "web_producer_manifest": str(R2 / "source.sha256"),
            "web_producer_manifest_sha256": R2_SHA, "web_producer_tree": R2_TREE,
            "web_compatible_test_only_delta": delta}


def pin_gallery(web_sha):
    check_scope()
    gallery = gallery_data(web_sha)
    dest = ROOT / "web/material-gallery"
    require(not dest.exists() and not dest.is_symlink() and not (ROOT / "gallery-pin.json").exists(),
            "Use a fresh Gallery pin")
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
        files = approved_source()
        result = {"source_count": len(files), "product_tree": R3_TREE}
    else:
        result = check_scope()
        if args.operation == "recheck":
            result.update(check_gallery())
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
