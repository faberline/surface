"""Pin the full source, the only scoped registry change, and root-built Gallery."""

import argparse
from hashlib import file_digest
import json
from pathlib import Path
import re
from shutil import copy2
import sys

ROOT = Path(__file__).resolve().parent
Q69 = Path("/tmp/surface-md3-text-field-placeholder-v0169.bc9QKVhY").resolve()
DEVELOPMENT_SOURCE = Q69 / "development-snapshot-r2/source"
DEVELOPMENT_SHA = "47642376bb5f97f07a542ba6c1fb159e1bdeda1d904cfb4c3dc39096d2c75354"
R1_SOURCE = Q69 / "development-snapshot-r1"
R1_SHA = "15e771de1fc3e14fddc8810e3d3c6388b823b477b124eb3f6d43171320f224e7"
WEB_SHA = "700ba34b6e25aaf7406f0ecbdbae056274a6fba9725e7a3e120a6e099e81cd33"
REGISTRY = "e2e/src/material.rs"
LABEL_FEATURE = "e2e/src/material_text_field_label_motion.rs"
FEATURE = "e2e/src/material_text_field_placeholder.rs"
PRIOR_SOURCE = Q69 / "base.sha256"
PRIOR_SOURCE_SHA = "17aeee497edef781a3da477d8fdb7820c0eaa3494a4b840fcf41133925949df6"
ENV_FLAG = "SURFACE_TEXT_FIELD_PLACEHOLDER_FOCUSED"
MODULES = [
    "material_outlined_text_field", "material_filled_text_field",
    "material_text_field_adornments", "material_text_field_counter",
    "material_multiline_text_field", "material_text_field_label_motion",
    "material_text_field_placeholder",
]
STEPS = [f"M{i}" for i in range(325, 354)]


def need(ok, description):
    if not ok:
        raise ValueError(description)


def sha(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def manifest(path, expected):
    need(sha(path) == expected, f"manifest changed: {path}")
    values = {}
    for line in path.read_text().splitlines():
        digest, name = line.split("  ", 1)
        parts = Path(name)
        need(re.fullmatch(r"[0-9a-f]{64}", digest) and not parts.is_absolute()
             and ".." not in parts.parts and name not in values, "invalid manifest")
        values[name] = digest
    need(bool(values), "empty manifest")
    return values


def verify_files(source, files):
    for name, digest in files.items():
        path = source / name
        need(path.is_file() and not path.is_symlink() and sha(path) == digest, f"input changed: {name}")


def write_json(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, sort_keys=True, indent=2) + "\n")


def registry(text):
    selector, count = re.subn(
        r"pub const STEPS: &\[&str\] = &\[.*?\];",
        "pub const STEPS: &[&str] = &[" + ", ".join(f'"{step}"' for step in STEPS) + "];",
        text, count=1, flags=re.S,
    )
    serve = '    let (_server, url) = env.serve("material-gallery")?;'
    need(count == 1 and selector.count(serve) == 1, "unknown registry format")
    scope = [f'    if std::env::var_os("{ENV_FLAG}").is_some() {{']
    scope.extend(f"        {module}::run(env, chrome, &url).await?;" for module in MODULES[:-1])
    scope.extend([f"        return {MODULES[-1]}::run(env, chrome, &url).await;", "    }"])
    return selector.replace(serve, serve + "\n" + "\n".join(scope))


def stock_registry(text):
    raw = text.split("pub const STEPS:", 1)[1].split("];", 1)[0]
    need(re.findall(r'"(M\d+)"', raw) == [f"M{i}" for i in range(1, 354) if i != 23],
         "original registry did not retain M1–M353")


def validate(source, source_manifest, source_sha):
    need(source_sha == DEVELOPMENT_SHA, "not the approved full .69 R2 source manifest")
    files = manifest(source_manifest, source_sha)
    verify_files(source, files)
    base = manifest(PRIOR_SOURCE, PRIOR_SOURCE_SHA)
    need(set(base) < files.keys() and len(files) == 688, "wrong full candidate source inventory")
    needs = set(MODULES[:-2])
    for module in needs:
        path = f"e2e/src/{module}.rs"
        need(files[path] == base[path], f"retained old test moved: {path}")
    previous = manifest(R1_SOURCE / "source.sha256", R1_SHA)
    need(files.keys() == previous.keys()
         and [name for name in files if files[name] != previous[name]] == [LABEL_FEATURE]
         and previous[LABEL_FEATURE] == base[LABEL_FEATURE],
         "the R1 to R2 change must be the earlier label-motion test only")
    old_test = (R1_SOURCE / "source" / LABEL_FEATURE).read_bytes()
    next_test = (source / LABEL_FEATURE).read_bytes()
    barrier = b'    focus(tab, &long, "surface-text-input").await?;\n'
    addition = b'    // Let the first focused paint start the spring at the held time.\n    tab.settle().await?;\n'
    need(old_test.count(barrier) == 1 and next_test == old_test.replace(barrier, barrier + addition),
         "the retained label test change is exactly the awaited above-label frame barrier")
    for path in (REGISTRY, FEATURE, "crates/surface-app/src/placeholder_motion.rs",
                 "crates/surface-app/src/editor_paint.rs", "examples/material-gallery/src/text_field_placeholder_demo.rs",
                 "crates/surface-material/src/outlined_text_field.rs"):
        need(path in files and files[path] != base.get(path), f".69 runtime or browser case not in the seal: {path}")
    stock_registry((source / REGISTRY).read_text())
    return files


def scoped_source(source, source_manifest, source_sha):
    files = validate(source, source_manifest, source_sha)
    dest = ROOT / "source"
    need(not dest.exists() and not (ROOT / "scope-pin.json").exists(), "use a fresh focused source lane")
    for name in files:
        target = dest / name
        target.parent.mkdir(parents=True, exist_ok=True)
        copy2(source / name, target)
    copy2(source_manifest, ROOT / "original-source.sha256")
    verify_files(dest, files)
    old = (dest / REGISTRY).read_text()
    (ROOT / "scope-original.rs").write_text(old)
    (dest / REGISTRY).write_text(registry(old))
    scoped = {**files, REGISTRY: sha(dest / REGISTRY)}
    (ROOT / "focused-source.sha256").write_text(
        "".join(f"{digest}  {name}\n" for name, digest in sorted(scoped.items()))
    )
    write_json(ROOT / "scope-pin.json", {
        "product_source": str(source),
        "product_source_manifest_sha256": source_sha,
        "focused_source_manifest_sha256": sha(ROOT / "focused-source.sha256"),
        "source_count": len(scoped),
        "stock_registry_sha256": files[REGISTRY],
        "focused_registry_sha256": scoped[REGISTRY],
        "test_source_sha256": files[FEATURE],
        "retained_label_source_sha256": files[LABEL_FEATURE],
        "registry_delta": [REGISTRY],
        "steps": STEPS,
    })
    check_scope()


def check_scope():
    pin = json.loads((ROOT / "scope-pin.json").read_text())
    original = manifest(ROOT / "original-source.sha256", pin["product_source_manifest_sha256"])
    scoped = manifest(ROOT / "focused-source.sha256", pin["focused_source_manifest_sha256"])
    verify_files(ROOT / "source", scoped)
    verify_files(Path(pin["product_source"]), original)
    need(original.keys() == scoped.keys() and len(scoped) == pin["source_count"], "source inventory differs")
    changed = sorted(name for name, digest in original.items() if scoped[name] != digest)
    need(changed == pin["registry_delta"] == [REGISTRY] and original[FEATURE] == pin["test_source_sha256"]
         and pin["steps"] == STEPS and original[LABEL_FEATURE] == pin["retained_label_source_sha256"],
         "scoped runner changed more than its one registry or lost steps")
    old = ROOT / "scope-original.rs"
    need(sha(old) == original[REGISTRY] == pin["stock_registry_sha256"]
         and pin["focused_registry_sha256"] == scoped[REGISTRY]
         and (ROOT / "source" / REGISTRY).read_text() == registry(old.read_text()), "scoped registry bytes differ")
    return pin


def gallery(source, source_manifest, source_sha, web_pack, web_manifest, web_sha):
    scope = check_scope()
    validate(source, source_manifest, source_sha)
    need(scope["product_source"] == str(source) and scope["product_source_manifest_sha256"] == source_sha,
         "Gallery and scoped runner must bind the same complete product source")
    need(web_sha == WEB_SHA, "the R1-built Web bytes are the only approved R2 assets")
    files = manifest(web_manifest, web_sha)
    need(set(files) >= {"index.html", "surface-web.json"} and len(files) == 4
         and set(files) == {p.name for p in web_pack.iterdir()}
         and sum(p.endswith(".js") for p in files) == sum(p.endswith(".wasm") for p in files) == 1,
         "expected four uncompressed original Gallery files")
    verify_files(web_pack, files)
    need(web_pack == Q69 / "web-development", "expected the controller-built .69 Gallery at web-development")
    dest = ROOT / "web" / "material-gallery"
    dest.mkdir(parents=True, exist_ok=False)
    for name in files:
        copy2(web_pack / name, dest / name)
    copy2(web_manifest, ROOT / "web.sha256")
    verify_files(dest, files)
    write_json(ROOT / "gallery-pin.json", {
        "original_gallery": str(web_pack), "original_web_manifest": str(web_manifest),
        "web_manifest_sha256": web_sha, "web_files": files, "product_source_manifest_sha256": source_sha,
        "web_build_source_manifest_sha256": R1_SHA, "web_compatible_test_only_delta": [LABEL_FEATURE],
    })


def check_gallery(scope):
    pin = json.loads((ROOT / "gallery-pin.json").read_text())
    web = manifest(ROOT / "web.sha256", pin["web_manifest_sha256"])
    origin = Path(pin["original_gallery"])
    need(web == pin["web_files"] and len(web) == 4 and origin == Q69 / "web-development"
         and pin["product_source_manifest_sha256"] == scope["product_source_manifest_sha256"],
         "wrong matching product or four-file Gallery pin")
    before = manifest(R1_SOURCE / "source.sha256", R1_SHA)
    now = manifest(ROOT / "original-source.sha256", DEVELOPMENT_SHA)
    need(before.keys() == now.keys()
         and pin["web_build_source_manifest_sha256"] == R1_SHA
         and [name for name in now if now[name] != before[name]] == pin["web_compatible_test_only_delta"] == [LABEL_FEATURE],
         "R1 Web origin and R2 runner differ beyond the approved test-only barrier")
    need(sha(Path(pin["original_web_manifest"])) == pin["web_manifest_sha256"] == WEB_SHA,
         "controller Gallery manifest changed")
    dest = ROOT / "web/material-gallery"
    verify_files(dest, web)
    verify_files(origin, web)
    need(set(web) == {p.name for p in dest.iterdir()} == {p.name for p in origin.iterdir()}, "Gallery inventory changed")
    return pin


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for command in ("prepare", "pin-gallery"):
        child = sub.add_parser(command)
        child.add_argument("--source", type=Path, default=DEVELOPMENT_SOURCE)
        child.add_argument("--source-manifest", type=Path, required=True)
        child.add_argument("--source-sha256", required=True)
        if command == "pin-gallery":
            child.add_argument("--web-pack", type=Path, default=Q69 / "web-development")
            child.add_argument("--web-manifest", type=Path, required=True)
            child.add_argument("--web-sha256", required=True)
    sub.add_parser("recheck-source")
    sub.add_parser("recheck")
    args = parser.parse_args()
    if args.command in {"recheck-source", "recheck"}:
        pin = check_scope()
        if args.command == "recheck":
            pin = {**pin, **check_gallery(pin)}
        print(json.dumps(pin, ensure_ascii=False, indent=2))
        return
    source = args.source.resolve(strict=True)
    need(source == DEVELOPMENT_SOURCE, "incorrect controller .69 source root")
    arguments = [source, args.source_manifest.resolve(strict=True), args.source_sha256]
    if args.command == "prepare":
        scoped_source(*arguments)
    else:
        gallery(*arguments, args.web_pack.resolve(strict=True),
                args.web_manifest.resolve(strict=True), args.web_sha256)


if __name__ == "__main__":
    main()
