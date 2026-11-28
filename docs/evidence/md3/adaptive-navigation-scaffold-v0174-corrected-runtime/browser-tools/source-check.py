#!/usr/bin/env python3
import hashlib
import pathlib
import sys

root = pathlib.Path('/tmp/surface-md3-v0174-corrected.gP1uy0FW/source')
manifest = root.parent / 'source.sha256'
expected = '53fb1fd46ab0b360186238bc3a946862c16047646e4578fc25dff9d308811f8f'
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
if len(entries) != 696:
    sys.exit('wrong file count')
actual = {p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0] != '.git'}
if actual != entries.keys():
    sys.exit(f'file inventory mismatch: extra={sorted(actual - entries.keys())}, missing={sorted(entries.keys() - actual)}')
for name, digest in entries.items():
    p = root / name
    if p.is_symlink() or hashlib.sha256(p.read_bytes()).hexdigest() != digest:
        sys.exit('file mismatch: ' + name)
if (root.parent / 'source.tree').read_text().strip() != '2532449d43fdce54448a5535cb6fcd5cb5ff6a79':
    sys.exit('tree record mismatch')
print('source manifest and 696 files verified; recorded tree 2532449d43fdce54448a5535cb6fcd5cb5ff6a79')
