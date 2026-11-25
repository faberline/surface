"""Pin the full source, the only scoped registry change, and root-built Gallery."""

import argparse
from hashlib import file_digest
import json
from pathlib import Path
import re
from shutil import copy2
import sys

ROOT = Path(__file__).resolve().parent
Q68 = Path("/tmp/surface-md3-text-field-label-color-v0168.XNrxeeOp").resolve()
Q68_SOURCE_SHA = "e7c6c189cd48f31257bf8b7de66da02ca9201d58a566544e741651cec4f1b006"
REGISTRY = "e2e/src/material.rs"
FEATURE = "e2e/src/material_text_field_label_motion.rs"
EXPECTED = "44b422b5f7c1be3fe517153c0867385cf295f9268a0242b765241d4868c72e55"
WEB_BUILD_SOURCE_SHA = "dfc0e717a5a5641c07b63ae3cbf140797fd39cabf3197cd7c93509cd324b0dd0"
PRIOR_SOURCE = Q68 / "prior-source.sha256"
PRIOR_SOURCE_SHA = "4aa53964b34f033b59b5ab723e58deb612d5db8ce1975bd3cf2115e687479271"
ENV_FLAG = "SURFACE_TEXT_FIELD_LABEL_COLOR_FOCUSED"
MODULES = [
    "material_outlined_text_field", "material_filled_text_field",
    "material_text_field_adornments", "material_text_field_counter",
    "material_multiline_text_field", "material_text_field_label_motion",
]
STEPS = [f"M{i}" for i in range(325, 351)]


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
    need(re.findall(r'"(M\d+)"', raw) == [f"M{i}" for i in range(1, 351) if i != 23],
         "original registry did not retain M1–M350")


def validate(source, source_manifest, source_sha):
    need(source_sha == Q68_SOURCE_SHA, "wrong exact development source manifest")
    files = manifest(source_manifest, source_sha)
    verify_files(source, files)
    base = manifest(PRIOR_SOURCE, PRIOR_SOURCE_SHA)
    need(set(base) < files.keys() and len(files) == 685, "wrong full candidate source inventory")
    needs = set(MODULES[:-1])
    for module in needs:
        path = f"e2e/src/{module}.rs"
        need(files[path] == base[path], f"retained old test moved: {path}")
    need(files[REGISTRY] == base[REGISTRY] and files[FEATURE] == EXPECTED, "wrong .68 feature or stock registry")
    for path in ("crates/surface-app/src/color_spring.rs", "crates/surface-app/src/editor_paint.rs",
                 "crates/surface-material/src/outlined_text_field.rs"):
        need(path in files and files[path] != base.get(path), f".68 runtime not in the seal: {path}")
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
    need(changed == pin["registry_delta"] == [REGISTRY] and original[FEATURE] == pin["test_source_sha256"] == EXPECTED
         and pin["steps"] == STEPS, "scoped runner changed more than its one registry or lost steps")
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
    files = manifest(web_manifest, web_sha)
    need(set(files) >= {"index.html", "surface-web.json"} and len(files) == 4
         and set(files) == {p.name for p in web_pack.iterdir()}
         and sum(p.endswith(".js") for p in files) == sum(p.endswith(".wasm") for p in files) == 1,
         "expected four uncompressed original Gallery files")
    verify_files(web_pack, files)
    need(web_pack == Q68 / "web-development", "expected the controller-built .68 Gallery at web-development")
    dest = ROOT / "web" / "material-gallery"
    dest.mkdir(parents=True, exist_ok=False)
    for name in files:
        copy2(web_pack / name, dest / name)
    copy2(web_manifest, ROOT / "web.sha256")
    verify_files(dest, files)
    write_json(ROOT / "gallery-pin.json", {
        "original_gallery": str(web_pack), "original_web_manifest": str(web_manifest),
        "web_manifest_sha256": web_sha, "web_files": files, "product_source_manifest_sha256": source_sha,
        "web_build_source_manifest_sha256": WEB_BUILD_SOURCE_SHA, "web_compatible_test_only_delta": [FEATURE],
    })


def check_gallery(scope):
    pin = json.loads((ROOT / "gallery-pin.json").read_text())
    web = manifest(ROOT / "web.sha256", pin["web_manifest_sha256"])
    origin = Path(pin["original_gallery"])
    need(web == pin["web_files"] and len(web) == 4 and origin == Q68 / "web-development"
         and pin["product_source_manifest_sha256"] == scope["product_source_manifest_sha256"],
         "wrong matching product or four-file Gallery pin")
    build_source = manifest(ROOT / "preflight-r1/original-source.sha256", WEB_BUILD_SOURCE_SHA)
    runner_source = manifest(ROOT / "original-source.sha256", Q68_SOURCE_SHA)
    changed = sorted(name for name in runner_source if build_source.get(name) != runner_source[name])
    need(pin["web_build_source_manifest_sha256"] == WEB_BUILD_SOURCE_SHA
         and runner_source.keys() == build_source.keys()
         and changed == pin["web_compatible_test_only_delta"] == [FEATURE],
         "existing Gallery and R2 runner source differ outside the focused test")
    need(sha(Path(pin["original_web_manifest"])) == pin["web_manifest_sha256"], "controller Gallery manifest changed")
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
        child.add_argument("--source", type=Path, default=Q68 / "source")
        child.add_argument("--source-manifest", type=Path, required=True)
        child.add_argument("--source-sha256", required=True)
        if command == "pin-gallery":
            child.add_argument("--web-pack", type=Path, default=Q68 / "web-development")
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
    need(source == Q68 / "source", "incorrect controller .68 source root")
    arguments = [source, args.source_manifest.resolve(strict=True), args.source_sha256]
    if args.command == "prepare":
        scoped_source(*arguments)
    else:
        gallery(*arguments, args.web_pack.resolve(strict=True),
                args.web_manifest.resolve(strict=True), args.web_sha256)


if __name__ == "__main__":
    main()
