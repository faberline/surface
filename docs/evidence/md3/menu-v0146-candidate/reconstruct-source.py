"""Apply the single release patch to a verified repaired Divider source tree."""

from hashlib import sha256
from pathlib import Path
from shutil import copy2
import subprocess
import sys


package = Path(__file__).resolve().parent
base, output = (Path(path).resolve() for path in sys.argv[1:3])


def read_manifest(filename, expected_sha, expected_count):
    content = (package / filename).read_bytes()
    assert sha256(content).hexdigest() == expected_sha, filename
    entries = dict((path, checksum) for checksum, path in
                   (line.split("  ", 1) for line in content.decode().splitlines()))
    assert len(entries) == expected_count, filename
    return entries


def verify(root, entries):
    for name, checksum in entries.items():
        expected = root / name
        assert expected.is_file() and not expected.is_symlink(), name
        assert sha256(expected.read_bytes()).hexdigest() == checksum, name


before = read_manifest(
    "base.sha256", "86f886cc10ab1d1b82e62364caf738824d2ce5295e3fb82533082f155763277e", 583,
)
after = read_manifest(
    "candidate.sha256", "92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5", 590,
)
patch = package / "surface-v0.1.46.patch"
assert sha256(patch.read_bytes()).hexdigest() == "5cff7383ba368218d374b2d620a8fae8618c1f06fb7a1cba1a0a5dce73542337"
verify(base, before)
output.mkdir()
for name in before:
    dest = output / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    copy2(base / name, dest)
subprocess.run(["patch", "-s", "-p1", "-i", str(patch)], cwd=output, check=True)
actual = {file.relative_to(output).as_posix() for file in output.rglob("*") if file.is_file()}
assert actual == set(after), sorted(actual ^ set(after))
verify(output, after)
print(f"Verified {len(before)} base files and all {len(after)} patched files.")
