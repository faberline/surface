# Surface Material 3 Checkbox v0.1.49 candidate

This candidate adds an opt-in Material 3 Checkbox. It supports
unchecked, checked and mixed values with enabled, disabled and error
states. The caller owns the selection. A mixed parent can request
selecting its children. An invalid checkbox can point to visible help.
The existing Checkbox builders remain available.

The checkbox uses an 18 dp box, a 40 dp state layer and a 50 dp space
for the focus ring. The checkbox and its wrapping label share a target.
The Gallery uses a mixed group, a locked item, and a checkbox with
error help. It supports dark mode, RTL and enlarged text.
[Material 3 Checkbox specs](https://m3.material.io/components/checkbox/specs)

## Exact source

The candidate has 602 files. The SHA-256 manifest digest is
`3a9aeda30a8ca3520afd04d9d4db8c26f3a900dc5aae20fad14e4d366d2544fe`.
The [Checkbox patch](inputs/surface-v0.1.49.patch.gz) has uncompressed
SHA-256
`ba8e791fec06bca5dacc60177c45931127e3d3bc8afe0b6abcc0d25d5f98433c`.
Use `gzip -dc` to decompress the patch. It applies to Switch v0.1.48
with the sealed source digest
`a4ec5bc9d9c0da42fe004f6f7d6877f59edc08e5503ae15c3bbefb6c73f4d086`.
[Switch candidate and material source](https://github.com/faberline/surface/tree/3255b8d6a79eccf8afdc30d7603b6abc59aa8baf/docs/evidence/md3/switch-v0148-candidate)

Verify [the Checkbox source manifest](inputs/source.sha256) from the
patched checkout with `shasum -a 256 --status -c`.

## Native replay

The light 960 × 800 profile and the dark 320 × 720 RTL profile ran
offscreen. The latter uses 200% text. Both replays verified the same
mixed group, Tab order, disabled rejection, visible invalid help,
and removal of the help after confirmation. The API, CLI and MCP
returned matching PNG bytes. Corresponding PNGs matched across the
two replays. All twelve window observations were zero.
[Light PNG](native/exact-1/wide-light.png) · [Dark RTL PNG](native/exact-1/compact-dark-rtl-200.png) · [First receipt](native/exact-1/native-gallery-report.json) · [Second receipt](native/exact-2/native-gallery-report.json)

Use the [offline verifier](native/scripts/verify-native-replays.py)
with Python and Pillow to independently decode both pairs.

To rerun on macOS, set the temporary `CARGO_TARGET_DIR` for your
checkout and use a Metal adapter. Set `proof_dir` to a new directory
within `/tmp` and `scripts` to this evidence package's `native/scripts`.

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
  --features surface-material-gallery/devtools --bins
cap python3 -B "$scripts/replay-checkbox.py" \
  --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
  --out "$proof_dir/native"
```

## Workspace and browser checks

The workspace gate passed 1,335 Rust tests, formatting, Clippy,
provenance and WebAssembly checks. All 16 offscreen Metal tests passed.
[Workspace log](workspace-exact.log) · [GPU log](gpu/gpu-exact.log)

The five headless macOS suites passed 19,827 checks, including 280
Material steps. Chrome used Apple Metal. The tests exercise real browser
accessibility, parent and child requests, keyboard and pointer input,
touch cancellation, and the dark RTL 200% error and resolved states.
There were zero visible-window observations and no surviving test
processes. The monitor followed the bridge Chrome after it detached.
[Runner log](stock-mac/runner.log) · [Window check](stock-mac/headless-window-observation.json) · [Independent receipt](stock-mac/verified-checkbox.json)

[Keyboard focus](stock-mac/artifacts/material-checkbox-wide-focused.png) · [Held](stock-mac/artifacts/material-checkbox-wide-held.png) · [Dark RTL invalid](stock-mac/artifacts/material-checkbox-compact-invalid.png) · [Dark RTL resolved](stock-mac/artifacts/material-checkbox-compact-resolved.png)

Build the patched checkout and run the explicit headless suites.

```sh
cap cargo xtask web e2e smoke ime material bridge-web devtools-headless \
  --artifacts /tmp/surface-checkbox-browser-evidence
```

Use the [browser verifier](stock-mac/verify_checkbox_report.py) with
`--profile stock-macos`, the runner log, artifacts and exact source manifest.
The complete local archive has 740 files and manifest digest
`dde599ea405b7f1e7333adac701bee5ce38e17fc0e0357e5fca48e24caec903b`.

## Sources

The November 19 official Google page and its DSDB export were pinned.
The page digest is
`59210ef5042b772c56a945e76303a0098fb2e6ee9d58117a500b092514d97c00`.
The token digest is
`f421060d2925e9b9c7722fd623c796570d213517ba4d5dfe7e923aad38993300`.
[Google page](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/c0332583-8417-4b80-a0a1-e283781d31c6.json) · [Google tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.31f16b44fe03b0c0.json) · [WAI Checkbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/checkbox/)
