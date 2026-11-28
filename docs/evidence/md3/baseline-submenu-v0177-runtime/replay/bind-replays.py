"""Bind the native and Browser headless inputs after an exact build."""

from hashlib import file_digest
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def save(path, data):
    with path.open("x") as stream:
        stream.write(json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True) + "\n")
    print(digest(path), path)


root = Path(__file__).resolve().parent
tree, source_sha = sys.argv[1:3]
out = Path(sys.argv[3]).resolve()
assert out.is_relative_to(Path("/tmp").resolve())
native_template = Path("/tmp/surface-md3-v0177-native-template.1x75KNxK/pins.example.json")
browser_template = Path("/tmp/surface-md3-v0177-browser-template.RRQnaWCd/config.example.json")
source = root / "surface"
manifest = root / "source.sha256"
outputs = Path(sys.argv[4]).resolve() if len(sys.argv) > 4 else root / "outputs"
assert (outputs / "BUILD-COMPLETE").read_text().splitlines() == [tree, source_sha]
assert digest(manifest) == source_sha
assert subprocess.check_output(["git", "-C", source, "rev-parse", "HEAD^{tree}"], text=True).strip() == tree
entries = manifest.read_text().splitlines()
assert len(entries) == 706
for directory in (outputs, outputs / "native"):
    subprocess.run(["shasum", "-a", "256", "-c", "SHA256SUMS"], cwd=directory, check=True,
                   stdout=subprocess.DEVNULL)

fields = {
    "source": str(source),
    "source_manifest": str(manifest),
    "source_sha256": source_sha,
    "source_tree": tree,
    "source_files": len(entries),
}
native = json.loads(native_template.read_bytes())
native_manifest = outputs / "native/SHA256SUMS"
native.update(fields, binary_manifest=str(native_manifest), binary_sha256=digest(native_manifest))
browser = json.loads(browser_template.read_bytes())
runner = outputs / "stock/debug/surface-e2e"
build_manifest = outputs / "SHA256SUMS"
browser.update(fields, build_root=str(outputs), build_manifest=str(build_manifest),
               build_sha256=digest(build_manifest), runner=str(runner), runner_sha256=digest(runner),
               web_root=str(outputs / "stock/web/release"))
out.mkdir(mode=0o700)
save(out / "native-pins.json", native)
save(out / "browser-config.json", browser)
