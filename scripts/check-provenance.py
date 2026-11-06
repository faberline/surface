#!/usr/bin/env python3
"""Check the hashes in docs/provenance.json against the files it pins.

Only two sets of files are hashed: the source files imported from Core,
listed in "original_source_sha256", and the pinned Noto CJK files. Other
files are not tracked here. Each imported source file must still hash to
its Core value, unless "modified_since_import" lists it with a reason. A
listed file must differ from Core, so the list cannot go stale. The pinned
fonts must match their hash and size. The checker tests itself on a
temporary tree first.
"""
import hashlib
import json
from pathlib import Path
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = Path("docs/provenance.json")
FONT_DIR = Path("crates/surface-wgpu/assets")


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def required_map(manifest, *keys):
    """Return the non-empty map at keys, or None when it is missing or empty."""
    value = manifest
    for key in keys:
        value = value.get(key) if isinstance(value, dict) else None
    return value if isinstance(value, dict) and value else None


def problems(root, manifest):
    found = []
    core = required_map(manifest, "original_source_sha256")
    fonts = required_map(manifest, "noto_cjk", "files")
    if core is None:
        found.append("original_source_sha256: missing or empty")
        core = {}
    if fonts is None:
        found.append("noto_cjk.files: missing or empty")
        fonts = {}
    modified = manifest.get("modified_since_import", {})
    for path in sorted(set(modified) - set(core)):
        found.append(f"{path}: listed as modified but not imported from Core")
    for path, core_hash in sorted(core.items()):
        file = root / path
        if not file.is_file():
            found.append(f"{path}: imported file is missing")
            continue
        same = sha256(file) == core_hash
        if path in modified:
            reason = modified[path]
            if not isinstance(reason, str) or not reason.strip():
                found.append(f"{path}: modified_since_import needs a reason")
            if same:
                found.append(f"{path}: matches Core again; remove it from modified_since_import")
        elif not same:
            found.append(
                f"{path}: differs from Core; restore it or list it in "
                "modified_since_import with a reason"
            )
    for name, font in sorted(fonts.items()):
        file = root / FONT_DIR / name
        if not file.is_file():
            found.append(f"{FONT_DIR / name}: pinned font file is missing")
        elif file.stat().st_size != font["bytes"] or sha256(file) != font["sha256"]:
            found.append(f"{FONT_DIR / name}: size or hash differs from provenance.json")
    return found


def self_test():
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "kept.rs").write_text("kept\n")
        (root / "changed.rs").write_text("changed\n")
        (root / FONT_DIR).mkdir(parents=True)
        (root / FONT_DIR / "font.otf").write_bytes(b"font")
        core = {
            "kept.rs": sha256(root / "kept.rs"),
            "changed.rs": hashlib.sha256(b"original\n").hexdigest(),
        }
        font = {"font.otf": {"sha256": hashlib.sha256(b"font").hexdigest(), "bytes": 4}}

        listed = {"changed.rs": "local edit"}
        absent = "0" * 64

        def manifest(modified=listed, fonts=font, sources=core, noto=True):
            case = {"original_source_sha256": sources, "modified_since_import": modified}
            if noto:
                case["noto_cjk"] = {"files": fonts}
            return case

        cases = [
            ("clean tree passes", manifest(), 0),
            ("unlisted change fails", manifest({}), 1),
            ("stale listing fails", manifest({**listed, "kept.rs": "none"}), 1),
            ("blank reason fails", manifest({"changed.rs": " "}), 1),
            ("null reason fails", manifest({"changed.rs": None}), 1),
            ("false reason fails", manifest({"changed.rs": False}), 1),
            ("unknown listing fails", manifest({**listed, "other.rs": "typo"}), 1),
            ("missing Core file fails", manifest(sources={**core, "absent.rs": absent}), 1),
            ("empty Core map fails", manifest({}, sources={}), 1),
            ("font size drift fails", manifest(fonts={"font.otf": {**font["font.otf"], "bytes": 5}}), 1),
            ("font hash drift fails", manifest(fonts={"font.otf": {**font["font.otf"], "sha256": absent}}), 1),
            ("missing font fails", manifest(fonts={**font, "absent.otf": {"sha256": absent, "bytes": 4}}), 1),
            ("empty font map fails", manifest(fonts={}), 1),
            ("missing noto_cjk fails", manifest(noto=False), 1),
        ]
        failures = []
        for name, case, expected in cases:
            got = len(problems(root, case))
            if got != expected:
                failures.append(f"self-test {name}: {got} problems, expected {expected}")
        assert not failures, "\n".join(failures)
    return len(cases)


def main():
    cases = self_test()
    manifest = json.loads((ROOT / MANIFEST).read_text())
    found = problems(ROOT, manifest)
    if found:
        print(f"{MANIFEST} does not match the imported files:", file=sys.stderr)
        for line in found:
            print(f"  {line}", file=sys.stderr)
        return 1
    # problems() has already rejected a missing or empty map.
    core = manifest["original_source_sha256"]
    modified = manifest.get("modified_since_import", {})
    fonts = manifest["noto_cjk"]["files"]
    print(
        f"provenance: {len(core) - len(modified)} of {len(core)} Core files unchanged, "
        f"{len(modified)} listed as modified, {len(fonts)} font files match "
        f"({cases} self-test cases passed)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
