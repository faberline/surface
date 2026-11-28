# Surface v0.1.79 Divider verification

Source commit 2c5003c521f518ed80c59035f2b74e86cf97ea73 has tree
c2470303ad4a11e356f93f6f66d361d8d78e13d5. The manifest covers 708 files.
A new checkout recovered that tree from the two earlier GitHub archives and
the forward patch here.

The Gallery places a decorative divider inside one named List. It also uses
that List in a modal side sheet. Regular dividers use 1 dp. The Gallery shows
8 dp Heavy with 16 dp of space on each side of the break. The test checks
narrow right-to-left layouts and text at 200 percent.

- Rust passed 1,627 tests. WebAssembly passed three checks.
- Apple Metal passed 18 offscreen tests. The archive keeps 21 original PNGs.
- Two native headless replays produced four original PNGs. Page.screenshot,
  CLI and MCP returned the same bytes for each capture. All four Gallery
  processes exited. None of the 12 window samples showed a test window.
- Headless Chrome WebGPU passed 381 Material steps and 31,752 checks. It
  checked keyboard focus and return, forced colors and palette restoration.
  The reader decoded eight original Divider PNGs. It checked line placement,
  ink and the side-sheet heading in both themes.
- The Rust Runtime test checks direct pointer events at the separator.
  The native Gallery replay checks semantic keyboard events and shared
  accessibility structure.

[Material sources and variant review](source/divider-spec-review.md)

## Replay and offline verification

This collector requires fresh successful R2 workspace, Metal, native and full Browser receipts. The focused R2 pilot and R1 dark-heading screenshot are diagnostic history only. The archive contains source pins and patch, logs, original receipts and representative PNGs. It does not duplicate the source checkout or retain Chrome profiles.

Python 3.12 and Pillow 12.3.0 are required for portable readback. It verifies the external manifest seal, original run and process receipts, source and build pins, native PNGs and transports, all 381 ordered Material steps, the inherited Browser checks, and q79's typed and independently decoded divider and heading pixels. It does not prove macOS AccessKit, VoiceOver speech, host appearance changes or physical hardware input.

After successful R2 runs, review each config and manifest SHA, then run:

```sh
py=/Users/chrischeng/.pyenv/versions/3.12.11/bin/python3
kit=/tmp/surface-md3-v0179-proof-r2.R2Bv5HFB
"$py" -E -B "$kit/collect.py" --native-run NATIVE_RUN --native-pins NATIVE_PINS --native-pins-sha REVIEWED_NATIVE_PINS_SHA --native-sha REVIEWED_NATIVE_MANIFEST_SHA --browser-run BROWSER_RUN --browser-config BROWSER_CONFIG --browser-config-sha REVIEWED_BROWSER_CONFIG_SHA --build-sha REVIEWED_BUILD_MANIFEST_SHA --out NEW_PACKAGE_DIR
"$py" -E -B NEW_PACKAGE_DIR/readback.py --manifest-sha REVIEWED_PACKAGE_SHA
```

The output path must be new under /tmp. Review and save the printed package SHA separately. Portable readback does not reconstruct the source.

## Reconstruct source separately

The q77 bundle and q78 patch are published at [q77 source evidence](https://github.com/faberline/surface/tree/6194bdcbc251acecbd3ff3557e7194b989ec2641/docs/evidence/md3/baseline-submenu-v0177-runtime/source) and [q78 source evidence](https://github.com/faberline/surface/tree/ed5d7b606a3ce62d62a3a57aec69ea5109fdbe73/docs/evidence/md3/vertical-choice-menu-system-colors-v0178-runtime/source). The following recipe uses only a fresh temporary clone:

```sh
package=/absolute/path/to/reviewed/package
lane=$(mktemp -d /tmp/surface-md3-q79-reconstruct.XXXXXXXX)
git clone https://github.com/faberline/surface.git "$lane/evidence"
git -C "$lane/evidence" show 6194bdcbc251acecbd3ff3557e7194b989ec2641:docs/evidence/md3/baseline-submenu-v0177-runtime/source/r5-source.bundle > "$lane/r5-source.bundle"
test "$(shasum -a 256 "$lane/r5-source.bundle" | cut -d ' ' -f 1)" = 18c4e7216465ec74a46784ba6ebe2f7be2a8f17bb3da18e0a081e9a31cb25645
git -C "$lane/evidence" show ed5d7b606a3ce62d62a3a57aec69ea5109fdbe73:docs/evidence/md3/vertical-choice-menu-system-colors-v0178-runtime/source/forward-from-v0177.patch > "$lane/q78.patch"
test "$(shasum -a 256 "$lane/q78.patch" | cut -d ' ' -f 1)" = 7381765d31afc25cb5e71d7aa5812ff5ac8c5d038774173bf5f9c6917879ff75
git clone --branch codex/md3-baseline-submenu "$lane/r5-source.bundle" "$lane/source"
git -C "$lane/source" checkout --detach 6c856eb5cdfef69f8fa10949be5abc2d44d40266
git -C "$lane/source" apply --index "$lane/q78.patch"
test "$(git -C "$lane/source" write-tree)" = 0de7289a17faab38a240c387a8534471fb9d8906
git -C "$lane/source" apply --index "$package/source/forward-from-v0178-r2.patch"
test "$(git -C "$lane/source" write-tree)" = c2470303ad4a11e356f93f6f66d361d8d78e13d5
(cd "$lane/source" && shasum -a 256 -c "$package/source/source.sha256")
```
