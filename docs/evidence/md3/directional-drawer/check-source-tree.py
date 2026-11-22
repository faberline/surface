"""Check the directional-drawer source paths, file hashes, and executable bits."""

from pathlib import Path
import hashlib
import os
import sys


root = Path(sys.argv[1]).resolve()
skip = {".git", "target", "artifacts"}
entries = {}
for directory, dirs, files in os.walk(root):
    dirs[:] = sorted(name for name in dirs if name not in skip)
    folder = Path(directory)
    for name in files + [d for d in dirs if (folder / d).is_symlink()]:
        if name in skip:
            continue
        path = folder / name
        relative = path.relative_to(root).as_posix()
        data = os.readlink(path).encode() if path.is_symlink() else path.read_bytes()
        mode = "symlink" if path.is_symlink() else f"{path.stat().st_mode & 0o111:o}"
        entries[relative] = (mode, hashlib.sha256(data).hexdigest())
manifest = hashlib.sha256()
for path, (mode, digest) in sorted(entries.items()):
    manifest.update(f"{mode}  {digest}  {path}\n".encode())
actual = manifest.hexdigest()
expected = "01ebc338464b01cdaf9e9b860c065edefb1710740be37c8698dc896b555c0efe"
if len(entries) != 662 or actual != expected:
    raise SystemExit(f"Source tree differs. {len(entries)} files. SHA-256 {actual}.")
print(f"Verified {len(entries)} source files. Tree manifest SHA-256 {actual}.")
