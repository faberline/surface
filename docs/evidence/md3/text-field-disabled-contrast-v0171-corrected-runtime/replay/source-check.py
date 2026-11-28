#!/usr/bin/env python3
import hashlib
import pathlib
import sys

root = pathlib.Path('/tmp/surface-md3-v0171-corrected.BWoiL6mk/source')
manifest = root.parent / 'source.sha256'
expected = '7ff2289093941bab73f8f718492d800b8a9c2bd33c3a03aae0f2396b6d03a65e'
raw = manifest.read_bytes()
if hashlib.sha256(raw).hexdigest() != expected:
    sys.exit('manifest digest mismatch')
entries = {}
for line in raw.decode().splitlines():
    digest, name = line.split('  ', 1)
    p = pathlib.PurePosixPath(name)
    if p.is_absolute() or '..' in p.parts or name in entries:
        sys.exit('invalid manifest path: ' + name)
    entries[name.removeprefix('./')] = digest
if len(entries) != 690:
    sys.exit('wrong file count')
actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0] != '.git'}
if actual != entries.keys():
    sys.exit(f'file inventory mismatch: extra={sorted(actual - entries.keys())}, missing={sorted(entries.keys() - actual)}')
for name, digest in entries.items():
    p = root / name
    if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest() != digest:
        sys.exit('file mismatch: ' + name)
if (root.parent / 'source.tree').read_text().strip() != 'e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc':
    sys.exit('tree record mismatch')
print('source manifest and 690 files verified; recorded tree e1c63aa7f5ec80309c9b6fe50d8c50aed1c25afc')
