#!/usr/bin/env python3
import hashlib
import pathlib
import sys

root = pathlib.Path('/tmp/surface-md3-v0176-forward.pjxmpm8l/strict-replay')
manifest = root.parent / 'source.sha256'
expected = '84bb4b9da77641b89a990c703bf59789f7f14657cfe97b60592cb3bbd9cfdb02'
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
if len(entries) != 703:
    sys.exit('wrong file count')
actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0] != '.git'}
if actual != entries.keys():
    sys.exit(f'file inventory mismatch: extra={sorted(actual - entries.keys())}, missing={sorted(entries.keys() - actual)}')
for name, digest in entries.items():
    p = root / name
    if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest() != digest:
        sys.exit('file mismatch: ' + name)
print('source manifest and 703 files verified; recorded tree b61630995f470772785605a944e3515a45b6d553')
