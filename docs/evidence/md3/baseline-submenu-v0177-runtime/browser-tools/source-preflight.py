#!/usr/bin/env python3
"""Check the current q77 source only; starts no build or app."""
import hashlib
import pathlib
import subprocess

root=pathlib.Path('/tmp/surface-md3-v0177-submenus.P7WLBuVn/surface')
manifest=root.parent/'source.sha256'
assert hashlib.sha256(manifest.read_bytes()).hexdigest()=='11b2891b47a85681ac0dd5b56274ad98cf9a412abdb6dfac110baf2253b567c3'
entries={}
for line in manifest.read_text().splitlines():
    h,n=line.split('  ',1); p=pathlib.PurePosixPath(n)
    assert len(h)==64 and not p.is_absolute() and '..' not in p.parts and str(p)==n and n not in entries
    entries[n]=h
actual={p.relative_to(root).as_posix() for p in root.rglob('*') if (p.is_file() or p.is_symlink()) and p.relative_to(root).parts[0]!='.git'}
assert len(entries)==706 and set(entries)==actual
for n,h in entries.items():
    p=root/n
    assert not p.is_symlink() and hashlib.sha256(p.read_bytes()).hexdigest()==h,n
result=subprocess.check_output(['git','-C',str(root),'rev-parse','HEAD','HEAD^{tree}'],text=True).splitlines()
assert result==['6c856eb5cdfef69f8fa10949be5abc2d44d40266','957b77b0c35e535f9c8191277bfc20673d937c60']
print('706 source files, manifest, HEAD and tree verified; no build or app started')
