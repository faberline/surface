#!/usr/bin/env python3
import hashlib
import pathlib
import sys

root = pathlib.Path('/tmp/surface-md3-v0169-hint-regression.dAa3WXAs/source')
manifest = root.parent / 'source.sha256'
expected = '3eed0f67f0440d4cc37ba48843857e45ad165188f174ebaf2dacf62fd19c426f'
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
if len(entries) != 688:
    sys.exit('wrong file count')
actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if p.is_file() or p.is_symlink()}
if actual != entries.keys():
    sys.exit(f'file inventory mismatch: extra={sorted(actual - entries.keys())}, missing={sorted(entries.keys() - actual)}')
for name, digest in entries.items():
    p = root / name
    if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest() != digest:
        sys.exit('file mismatch: ' + name)
if (root.parent / 'source.tree').read_text().strip() != '5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb':
    sys.exit('tree record mismatch')
print('source manifest and 688 files verified; recorded tree 5db91b856a4a7fe8d4e8f66a0c7d8f69df1376eb')
