"""Pin a selector-only M325–M356 runner and the exact controller Gallery by reference."""

import argparse
import json
from pathlib import Path
import re
from runpy import run_path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent
PRIOR = Path("/tmp/surface-md3-field-placeholder-focused-v0169-r2.8AArzw7l")
Q70 = Path("/tmp/surface-md3-field-boundary-v0170.X35mBqoS")
R1 = Q70 / "development-snapshot-r1"
R1_SHA = "f0b9bf4f43c9ec853fd2ec327028dabfe8215846ef0beec6222f3ac7fecb684c"
R1_TREE = "a3e9e2962861a71825fc7c12ee3580561d813806"
WEB_SHA = "cd449385e300bf6a78226070f0a8941534520ab461abf3ddaa764b9e5f47304a"
Q69 = Path("/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY/exact-candidate/source.sha256")
Q69_SHA = "1b1379f04bbb534b6a25e857185de34160b367bfa60ef9a36a1fce1d0a8d3d55"
E2E_ONLY = Path("/tmp/surface-md3-field-boundary-e2e.ay4lQf1c/source.sha256")
E2E_ONLY_SHA = "8bdeeb9f959163974957834884da08fc992d34c2981cef0f0a0cedc788fe8e1a"
PREP_FILE_SHA = "5ca47abbfcbc216e629652bfa68065946a1916fc39e8b0578da3c45e8c184ba3"
REGISTRY = "e2e/src/material.rs"
ENV_FLAG = "SURFACE_TEXT_FIELD_BOUNDARY_FOCUSED"
MODULES = (
    "material_outlined_text_field", "material_filled_text_field",
    "material_text_field_adornments", "material_text_field_counter",
    "material_multiline_text_field", "material_text_field_label_motion",
    "material_text_field_placeholder", "material_text_field_boundary_motion",
)
STEPS = [f"M{i}" for i in range(325, 357)]
FULL = [f"M{i}" for i in range(1, 357) if i != 23]

from hashlib import file_digest


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


if sha(PRIOR / "prepare-focused.py") != PREP_FILE_SHA:
    raise ValueError("prior sealed input parser changed")
OLD = run_path(str(PRIOR / "prepare-focused.py"))
need, manifest, verify_files, write_json = (
    OLD[k] for k in ("need", "manifest", "verify_files", "write_json")
)


def read(path):
    return json.loads(path.read_text())


def registry(text):
    expected = r"pub const STEPS: &\[&str\] = &\[(.*?)\];"
    source_steps, = re.findall(expected, text, flags=re.S)
    need(re.findall(r'"(M\d+)"', source_steps) == FULL, "wrong original full Material registry")
    serve = '    let (_server, url) = env.serve("material-gallery")?;'
    need(text.count(serve) == 1, "unknown Material server entry")
    branch = [f'    if std::env::var_os("{ENV_FLAG}").is_some() {{']
    for module in MODULES[:-1]:
        branch.append(f"        {module}::run(env, chrome, &url).await?;")
    branch.extend([f"        return {MODULES[-1]}::run(env, chrome, &url).await;", "    }"])
    selector = "pub const STEPS: &[&str] = &[" + ", ".join(f'"{step}"' for step in STEPS) + "];"
    return re.sub(expected, selector, text, count=1, flags=re.S).replace(serve, serve + "\n" + "\n".join(branch))


def same_inventory(path, files):
    actual = {p.relative_to(path).as_posix() for p in path.rglob("*") if not p.is_dir() or p.is_symlink()}
    need(actual == files.keys(), f"missing or extra source paths in {path}")


def approved_source(source, source_manifest, source_sha):
    need(source.parent == source_manifest.parent and source.name == "source",
         "use a source and its adjacent manifest")
    files = manifest(source_manifest, source_sha)
    same_inventory(source, files)
    verify_files(source, files)
    base = manifest(Q69, Q69_SHA)
    need(len(files) == 689 and base.keys() < files.keys()
         and files.keys() - base.keys() == {"e2e/src/material_text_field_boundary_motion.rs"},
         "wrong .70 source inventory")
    for module in (*MODULES[:5], MODULES[6]):
        preserved = f"e2e/src/{module}.rs"
        need(files[preserved] == base[preserved], f"retained case differs: {preserved}")
    if source_sha == R1_SHA:
        proposal = manifest(E2E_ONLY, E2E_ONLY_SHA)
        for name in (REGISTRY, f"e2e/src/{MODULES[5]}.rs", f"e2e/src/{MODULES[-1]}.rs"):
            need(files[name] == proposal[name], f"original R1 expected E2E bytes differ: {name}")
    for name in ("Cargo.toml", "crates/surface-app/src/editor_paint.rs",
                 "crates/surface-material/src/outlined_text_field.rs"):
        need(files[name] != base[name], f"missing .70 product path: {name}")
    registry((source / REGISTRY).read_text())
    return files


def make_scope(source, source_manifest, source_sha, tree):
    need(re.fullmatch(r"[a-f0-9]{40}", tree), "a controller-pinned product tree is required")
    if source_sha == R1_SHA:
        need(source == (R1 / "source").resolve() and tree == R1_TREE, "wrong known R1 source or tree")
    original = approved_source(source, source_manifest, source_sha)
    dest = ROOT / "source"
    need(not dest.exists() and not (ROOT / "scope-pin.json").exists(), "use a new private scope")
    # A private APFS clone avoids a second physical copy of product inputs.
    subprocess.run(["/bin/cp", "-cR", str(source), str(dest)], check=True)
    shutil.copyfile(source_manifest, ROOT / "original-source.sha256")
    copy = dest / REGISTRY
    copy.write_text(registry(copy.read_text()))
    scoped = {**original, REGISTRY: sha(copy)}
    (ROOT / "focused-source.sha256").write_text(
        "".join(f"{digest}  {name}\n" for name, digest in sorted(scoped.items()))
    )
    write_json(ROOT / "scope-pin.json", {
        "product_source": str(source),
        "product_source_manifest": str(source_manifest),
        "product_source_manifest_sha256": source_sha,
        "product_tree": tree,
        "source_count": len(scoped),
        "focused_source_manifest_sha256": sha(ROOT / "focused-source.sha256"),
        "stock_registry_sha256": original[REGISTRY],
        "focused_registry_sha256": scoped[REGISTRY],
        "registry_delta": [REGISTRY],
        "steps": STEPS,
    })
    return check_scope()


def check_scope():
    pin = read(ROOT / "scope-pin.json")
    original_path = Path(pin["product_source_manifest"])
    parent = Path(pin["product_source"])
    parent_hash = pin["product_source_manifest_sha256"]
    original = approved_source(parent, original_path, parent_hash)
    need(sha(ROOT / "original-source.sha256") == parent_hash, "copied original source manifest differs")
    scoped = manifest(ROOT / "focused-source.sha256", pin["focused_source_manifest_sha256"])
    dest = ROOT / "source"
    same_inventory(dest, scoped)
    verify_files(dest, scoped)
    need(len(scoped) == pin["source_count"] == len(original) and scoped.keys() == original.keys()
         and sorted(name for name in original if original[name] != scoped[name]) == [REGISTRY]
         and pin["registry_delta"] == [REGISTRY] and pin["steps"] == STEPS
         and scoped[REGISTRY] == pin["focused_registry_sha256"]
         and original[REGISTRY] == pin["stock_registry_sha256"]
         and (dest / REGISTRY).read_text() == registry((parent / REGISTRY).read_text()),
         "scoped source changed more than its one temporary registry")
    return pin


def web_inputs(pack, path, web_sha):
    raw = manifest(path, web_sha)
    files = {name.removeprefix("./"): digest for name, digest in raw.items()}
    need(len(raw) == len(files) == 4, "duplicate or missing Gallery asset paths")
    need(len(files) == 4 and {"index.html", "surface-web.json"} <= files.keys()
         and sum(name.endswith(".js") for name in files) == sum(name.endswith(".wasm") for name in files) == 1
         and files.keys() == {file.name for file in pack.iterdir()}, "expected four canonical Gallery originals")
    verify_files(pack, files)
    return files


def pin_gallery(pack, path, web_sha, producer_path, producer_sha):
    scope = check_scope()
    if scope["product_source_manifest_sha256"] == R1_SHA and producer_sha == R1_SHA:
        need(pack == (Q70 / "web-development").resolve() and web_sha == WEB_SHA,
             "wrong exact R1 Gallery build")
    files = web_inputs(pack, path, web_sha)
    tested = manifest(ROOT / "original-source.sha256", scope["product_source_manifest_sha256"])
    producer = manifest(producer_path, producer_sha)
    need(producer.keys() == tested.keys(), "different Web source inventory")
    delta = sorted(name for name in tested if tested[name] != producer[name])
    need(all(name.startswith("e2e/") for name in delta), "the Web producer differs in product bytes")
    dest = ROOT / "web/material-gallery"
    need(not dest.exists() and not dest.is_symlink() and not (ROOT / "gallery-pin.json").exists(),
         "don't replace the pinned Gallery")
    dest.parent.mkdir()
    dest.symlink_to(pack, target_is_directory=True)
    write_json(ROOT / "gallery-pin.json", {
        "original_gallery": str(pack),
        "original_web_manifest": str(path),
        "web_manifest_sha256": web_sha,
        "web_files": files,
        "product_source_manifest_sha256": scope["product_source_manifest_sha256"],
        "web_producer_manifest": str(producer_path),
        "web_producer_manifest_sha256": producer_sha,
        "web_compatible_test_only_delta": delta,
    })
    return check_gallery(scope)


def check_gallery(scope):
    pin = read(ROOT / "gallery-pin.json")
    dest = ROOT / "web/material-gallery"
    origin = Path(pin["original_gallery"])
    current = manifest(ROOT / "original-source.sha256", scope["product_source_manifest_sha256"])
    producer = manifest(Path(pin["web_producer_manifest"]), pin["web_producer_manifest_sha256"])
    delta = sorted(name for name in current if name not in producer or current[name] != producer[name])
    need(dest.is_symlink() and dest.resolve(strict=True) == origin
         and web_inputs(origin, Path(pin["original_web_manifest"]), pin["web_manifest_sha256"]) == pin["web_files"]
         and pin["product_source_manifest_sha256"] == scope["product_source_manifest_sha256"]
         and current.keys() == producer.keys() and pin["web_compatible_test_only_delta"] == delta
         and all(name.startswith("e2e/") for name in delta), "the pinned Gallery origin or bytes changed")
    if scope["product_source_manifest_sha256"] == R1_SHA and pin["web_producer_manifest_sha256"] == R1_SHA:
        need(origin == (Q70 / "web-development").resolve() and pin["web_manifest_sha256"] == WEB_SHA,
             "wrong exact R1 Gallery")
    return pin


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="operation", required=True)
    prep = subs.add_parser("prepare")
    prep.add_argument("--source", type=Path, default=R1 / "source")
    prep.add_argument("--source-manifest", type=Path, default=R1 / "source.sha256")
    prep.add_argument("--source-sha256", default=R1_SHA)
    prep.add_argument("--tree", default=R1_TREE)
    pin = subs.add_parser("pin-gallery")
    pin.add_argument("--web-pack", type=Path, default=Q70 / "web-development")
    pin.add_argument("--web-manifest", type=Path, default=Q70 / "web-development.sha256")
    pin.add_argument("--web-sha256", default=WEB_SHA)
    pin.add_argument("--web-source-manifest", type=Path, default=R1 / "source.sha256")
    pin.add_argument("--web-source-sha256", default=R1_SHA)
    subs.add_parser("recheck-source")
    subs.add_parser("recheck")
    opts = parser.parse_args()
    if opts.operation == "prepare":
        result = make_scope(opts.source.resolve(strict=True), opts.source_manifest.resolve(strict=True),
                            opts.source_sha256, opts.tree)
    elif opts.operation == "pin-gallery":
        result = pin_gallery(opts.web_pack.resolve(strict=True), opts.web_manifest.resolve(strict=True),
                             opts.web_sha256, opts.web_source_manifest.resolve(strict=True), opts.web_source_sha256)
    else:
        result = check_scope()
        if opts.operation == "recheck":
            result.update(check_gallery(result))
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
