#!/usr/bin/env python3
"""Check the exact provisional q79 source only; starts no build or app."""
import hashlib
import pathlib
import subprocess

root=pathlib.Path('/tmp/surface-md3-v0179-divider.cOzU6oi6/surface')
manifest=root.parent/'source.sha256'
assert hashlib.sha256(manifest.read_bytes()).hexdigest()=='4ff1e8e17f1eb66d230ad43a8f3226bb126c31eea79791bda765f62c90b2b979'
entries={}
for line in manifest.read_text().splitlines():
    h,n=line.split('  ',1); p=pathlib.PurePosixPath(n)
    assert len(h)==64 and not p.is_absolute() and '..' not in p.parts and str(p)==n and n not in entries
    entries[n]=h
actual={p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0]!='.git'}
assert len(entries)==708 and set(entries)==actual
for n,h in entries.items():
    p=root/n
    assert not p.is_symlink() and hashlib.sha256(p.read_bytes()).hexdigest()==h,n
result=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD','HEAD^{tree}'],text=True).splitlines()
assert result==['2c5003c521f518ed80c59035f2b74e86cf97ea73','c2470303ad4a11e356f93f6f66d361d8d78e13d5']
print('708 source files, manifest, HEAD and tree verified; no build or app started')
