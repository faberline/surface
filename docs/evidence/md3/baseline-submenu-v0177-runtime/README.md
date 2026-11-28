# Surface v0.1.77 submenu R5 evidence

Source commit 6c856eb5cdfef69f8fa10949be5abc2d44d40266 has tree 957b77b0c35e535f9c8191277bfc20673d937c60 and 706 files. Source manifest SHA-256: 11b2891b47a85681ac0dd5b56274ad98cf9a412abdb6dfac110baf2253b567c3. The base commit is f5beae439e76f141395932156c9dd84d1b037e96, with tree b61630995f470772785605a944e3515a45b6d553. Forward patch SHA-256: da28567bb5f820abb1b3f1b8a51b3efb7a0c41fc505447da24f5540a7640d780. R3-to-R5 test-only patch SHA-256: f93bd9d9b16d2afbcbae95e82c56d6e878f98279f6c9cab6f2201b7b3c9db721.

The R5 workspace, Metal, build, native and full Browser receipts are included. The package keeps a short R3 failure record with its guard and clean process exit. R3 passed M376 and timed out at M377. This is negative history, not acceptance evidence.

The separate R4 focused pilot passed M376-M379 with 164 checks. It records Canvas focus after pointer actions and outside dismissal. Its two original PNGs pass independent geometry and color readback. This pilot is diagnostic evidence, not the R5 stock-suite result.

## Portable readback

The reader was verified with CPython 3.12.11 at /Users/chrischeng/.pyenv/versions/3.12.11/bin/python3 and Pillow 12.3.0. It uses the Python standard library and Pillow. The required Pillow pin is in readback-requirements.txt.

Set PYTHON to an interpreter that has Pillow installed. For a configurable virtual environment:

~~~sh
PYTHON=${PYTHON:-python3}
READBACK_VENV=${READBACK_VENV:-.venv}
"$PYTHON" -m venv "$READBACK_VENV"
"$READBACK_VENV/bin/python" -m pip install -r readback-requirements.txt
"$READBACK_VENV/bin/python" -E -B portable-readback.py
~~~

The verification used this interpreter:

~~~sh
/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3 -E -B portable-readback.py
~~~

## Rebuild from the pinned base

The archive contains a self-contained Git bundle with complete history, plus the source patch and manifest. Bundle size: 19,448,533 bytes. Bundle SHA-256: 18c4e7216465ec74a46784ba6ebe2f7be2a8f17bb3da18e0a081e9a31cb25645. It contains the exact R5 commit and the pinned base commit.

~~~sh
PKG=$(pwd -P)
LANE=$(mktemp -d /tmp/surface-md3-v0177-r5-rebuild.XXXXXXXX)
git bundle verify "$PKG/source/r5-source.bundle"
git clone --branch codex/md3-baseline-submenu "$PKG/source/r5-source.bundle" "$LANE/surface"
test "$(git -C "$LANE/surface" rev-parse HEAD)" = 6c856eb5cdfef69f8fa10949be5abc2d44d40266
test "$(git -C "$LANE/surface" rev-parse HEAD^{tree})" = 957b77b0c35e535f9c8191277bfc20673d937c60
(cd "$LANE/surface" && shasum -a 256 -c "$PKG/source/source.sha256")
git -C "$LANE/surface" reset --hard f5beae439e76f141395932156c9dd84d1b037e96
git -C "$LANE/surface" apply --index "$PKG/source/forward-from-v0176.patch"
test "$(git -C "$LANE/surface" write-tree)" = 957b77b0c35e535f9c8191277bfc20673d937c60
(cd "$LANE/surface" && shasum -a 256 -c "$PKG/source/source.sha256")
git -C "$LANE/surface" -c user.name="Local Evidence Rebuild" -c user.email="local@example.invalid" commit -m "Reconstruct pinned R5 source"
test "$(git -C "$LANE/surface" rev-parse HEAD^{tree})" = 957b77b0c35e535f9c8191277bfc20673d937c60
cp "$PKG/replay/build.sh" "$PKG/source/source.sha256" "$PKG/source/target.tree" "$LANE/"
mv "$LANE/target.tree" "$LANE/source.tree"
sh "$LANE/build.sh" --build 957b77b0c35e535f9c8191277bfc20673d937c60 11b2891b47a85681ac0dd5b56274ad98cf9a412abdb6dfac110baf2253b567c3 "$LANE/outputs"
~~~

The bundle clone contains all 706 files at the exact R5 commit and tree. The verified commands and results are in source-reconstruction.log. Resetting only the fresh clone to the bundled base and applying the full patch with --index reproduces the same tree and file manifest. The local commit makes the reconstruction clean for the build script. The source bundle and reconstruction instructions are inside this archive, so source recovery does not use GitHub. The archived build script is replay/build.sh, SHA-256 20840fa071acc6fd5c88e27ddb2fcd9687f5cb79ffd585272fd502bd92ddac6c. It keeps these original-machine paths: CARGO_HOME=/tmp/surface-md3-recovery-cargo.n0G16w5v, CARGO_TARGET_DIR=/tmp/cargo-target/surface-surface-md3-v0177-submenus.P7WLBuVn-surface, and DEVELOPER_DIR=/Library/Developer/CommandLineTools. On another Mac, edit a copy of that script to use local valid paths before building. The recorded build output and hashes remain pinned to the original R5 run. The archived native and Browser configs also point to original R5 paths. A new build path needs newly reviewed pin files and config hashes before runtime replay.
