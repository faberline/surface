# Surface v0.1.78 choice menu contrast evidence

Source commit 4afa793c9332340de373c7164e200995280572be has tree
0de7289a17faab38a240c387a8534471fb9d8906. The source manifest covers 707 files.

- Rust passed 1,626 tests. WebAssembly passed 3 checks.
- Apple Metal passed 18 offscreen tests and saved 21 original PNGs.
- Native Gallery used standard light, vibrant light and compact dark right-to-left layouts.
  Two headless runs produced 8 matching PNGs through Page.screenshot, CLI and MCP.
  Across 46 samples, no test window was visible. All 6 Gallery processes exited.
- Headless Chrome WebGPU passed 379 Material steps and 31,274 checks.
  Chrome used the actual CSS SelectedItem pair for both menu styles. The reader decoded both
  original PNGs and checked the selected fill and mark ink. Keyboard and accessibility checks
  covered the disabled checked row and palette restoration.

## Replay and offline verification

This collector requires real completed q78 native and Browser runs. Historical and synthetic fixtures are not acceptance evidence. Source and build checks run before collection. The package contains the pinned source manifest and forward patch, source and build receipts, native and Browser receipts, original images, logs, and portable verifiers. It does not contain a second source checkout. The source-bound binaries are not included.

Use CPython 3.12 and Pillow 12.3.0. The portable readback needs only Python and Pillow and does not read the original /tmp lanes. It checks the reviewed package manifest, source manifest and patch seals, build and input pins, native Page.screenshot / CLI / MCP receipts and eight decoded PNGs, Browser 379 ordered steps and guards, inherited q76 and q77 checks, and q78 typed reports and two decoded PNGs. It does not prove VoiceOver speech or live OS settings.

After both real runs pass, review build and config hashes, then run:

```sh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
kit=/tmp/surface-md3-v0178-proof-r2.NY5APirZ
"$py" -E -B "$kit/collect.py" --native-run NATIVE_RUN --browser-run BROWSER_RUN --browser-config BROWSER_CONFIG --config-sha REVIEWED_CONFIG_SHA --build-sha REVIEWED_BUILD_SHA --native-sha REVIEWED_NATIVE_SHA --out NEW_PACKAGE_DIR
"$py" -E -B NEW_PACKAGE_DIR/readback.py --manifest-sha REVIEWED_PACKAGE_MANIFEST_SHA
```

Review the printed package manifest hash separately before readback. The package must be a fresh path under /tmp. Preserve the reviewed hash outside the package.

## Reconstruct source separately

The prior Surface submenu evidence includes a self-contained Git bundle for
commit `6c856eb5cdfef69f8fa10949be5abc2d44d40266`. This recipe reconstructs
the exact current tree and checks the 707-file manifest. The portable
readback verifies the evidence package and leaves source reconstruction to
this command. [Prior source bundle](https://github.com/faberline/surface/tree/6194bdcbc251acecbd3ff3557e7194b989ec2641/docs/evidence/md3/baseline-submenu-v0177-runtime/source)

```sh
package=/absolute/path/to/reviewed/package
lane=$(mktemp -d /tmp/surface-md3-q78-reconstruct.XXXXXXXX)
git clone --single-branch --branch codex/evidence-md3-baseline-submenu-v0177 \
  https://github.com/faberline/surface.git "$lane/evidence"
git -C "$lane/evidence" show \
  6194bdcbc251acecbd3ff3557e7194b989ec2641:docs/evidence/md3/baseline-submenu-v0177-runtime/source/r5-source.bundle \
  > "$lane/r5-source.bundle"
test "$(shasum -a 256 "$lane/r5-source.bundle" | cut -d ' ' -f 1)" = \
  18c4e7216465ec74a46784ba6ebe2f7be2a8f17bb3da18e0a081e9a31cb25645
git clone --branch codex/md3-baseline-submenu "$lane/r5-source.bundle" "$lane/source"
git -C "$lane/source" checkout --detach 6c856eb5cdfef69f8fa10949be5abc2d44d40266
test "$(git -C "$lane/source" rev-parse 'HEAD^{tree}')" = 957b77b0c35e535f9c8191277bfc20673d937c60
git -C "$lane/source" apply --index "$package/source/forward-from-v0177.patch"
test "$(git -C "$lane/source" write-tree)" = 0de7289a17faab38a240c387a8534471fb9d8906
(cd "$lane/source" && shasum -a 256 -c "$package/source/source.sha256")
```

The pinned tree checks the complete tracked inventory. The manifest checks every file. This recipe writes only to the new temporary clone.

The package retains the R1 stock failure and the separate focused M380 pilot as diagnostic history only. Neither is R2 stock acceptance. R2 collection requires fresh source-bound build, workspace, Metal, native and full Browser receipts.
