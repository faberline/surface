#!/usr/bin/env python3
"""Check the hashes in docs/provenance.json against the imported files.

Each file imported from Core must still hash to its Core value, unless
"modified_since_import" lists it with a reason. A listed file must differ
from Core, so the list cannot go stale. The pinned fonts must match their
hash and size. The checker tests itself on a temporary tree first.
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


def problems(root, manifest):
    found = []
    core = manifest["original_source_sha256"]
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
            if not str(modified[path]).strip():
                found.append(f"{path}: modified_since_import needs a reason")
            if same:
                found.append(f"{path}: matches Core again; remove it from modified_since_import")
        elif not same:
            found.append(
                f"{path}: differs from Core; restore it or list it in "
                "modified_since_import with a reason"
            )
    for name, font in sorted(manifest.get("noto_cjk", {}).get("files", {}).items()):
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

        def manifest(modified, fonts=font):
            return {
                "original_source_sha256": core,
                "modified_since_import": modified,
                "noto_cjk": {"files": fonts},
            }

        listed = {"changed.rs": "local edit"}
        cases = [
            ("clean tree passes", manifest(listed), 0),
            ("unlisted change fails", manifest({}), 1),
            ("stale listing fails", manifest({**listed, "kept.rs": "none"}), 1),
            ("empty reason fails", manifest({"changed.rs": " "}), 1),
            ("unknown listing fails", manifest({**listed, "other.rs": "typo"}), 1),
            ("font drift fails", manifest(listed, {"font.otf": {**font["font.otf"], "bytes": 5}}), 1),
        ]
        for name, case, expected in cases:
            got = len(problems(root, case))
            assert got == expected, f"self-test {name}: {got} problems, expected {expected}"
        (root / "kept.rs").unlink()
        assert len(problems(root, manifest(listed))) == 1, "self-test missing file is not caught"
    return len(cases) + 1


def main():
    cases = self_test()
    manifest = json.loads((ROOT / MANIFEST).read_text())
    found = problems(ROOT, manifest)
    if found:
        print(f"{MANIFEST} does not match the imported files:", file=sys.stderr)
        for line in found:
            print(f"  {line}", file=sys.stderr)
        return 1
    core = manifest["original_source_sha256"]
    modified = manifest.get("modified_since_import", {})
    fonts = manifest.get("noto_cjk", {}).get("files", {})
    print(
        f"provenance: {len(core) - len(modified)} of {len(core)} Core files unchanged, "
        f"{len(modified)} listed as modified, {len(fonts)} font files match "
        f"({cases} self-test cases passed)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
