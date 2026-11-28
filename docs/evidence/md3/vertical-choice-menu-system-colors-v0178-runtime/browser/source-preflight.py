#!/usr/bin/env python3
"""Check the current q78 R2 source only; starts no build or app."""
import hashlib
import pathlib
import subprocess

root=pathlib.Path('/tmp/surface-md3-v0178-menu-system-colors.hlLwvQTS/surface')
manifest=root.parent/'source.sha256'
assert hashlib.sha256(manifest.read_bytes()).hexdigest()=='e58979727fbe6a9adc0b8ba64f9141e11bb7a32e570063f328d2449a8d034a4e'
entries={}
for line in manifest.read_text().splitlines():
    h,n=line.split('  ',1); p=pathlib.PurePosixPath(n)
    assert len(h)==64 and not p.is_absolute() and '..' not in p.parts and str(p)==n and n not in entries
    entries[n]=h
actual={p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0]!='.git'}
assert len(entries)==707 and set(entries)==actual
for n,h in entries.items():
    p=root/n
    assert not p.is_symlink() and hashlib.sha256(p.read_bytes()).hexdigest()==h,n
result=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD','HEAD^{tree}'],text=True).splitlines()
assert result==['4afa793c9332340de373c7164e200995280572be','0de7289a17faab38a240c387a8534471fb9d8906']
print('707 source files, manifest, HEAD and tree verified; no build or app started')
