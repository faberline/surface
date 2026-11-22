# Current Page in Material Navigation

The rail and directional drawer expose the current page as
`aria-current="page"` on the Web and `AXARIACurrent` on macOS.
Arrow focus preserves the selected page. Press selects the
destination once. The drawer remembers it when it reopens.

## Verified Results

- Rust workspace passed 1,379 tests with 25 ignored. Formatting, strict
  Clippy, provenance and the WASM consumer checks passed.
- Canonical Chrome 157 passed 24,845 checks. Material ran 176 steps and
  20,738 checks. Browser accessibility ran 67 steps and 4,107 checks.
  The targeted Chrome run passed 748 checks.
- The Apple M1 Max Metal Gallery captured 14 CLI PNGs. Its 13 public
  AppKit queries recorded 12 active Page attributes and 21 absent
  attributes. The rail and drawer each have a byte-identical compact
  MCP capture. The separate retained-control fixture recorded four real
  AppKit notifications. VoiceOver speech remains outside this receipt.

[Rust receipt](rust-summary.log) · [Chrome receipt](chrome-summary.log) ·
[Native audit](native-audit.json) · [Representative captures](captures)

## Restore the Tested Source

Start with the [published Drawer Swipe source](https://github.com/faberline/surface/tree/e3f426a5bdeaf7737e5dc6928dec0bdac1a91447/docs/evidence/md3/drawer-swipe)
replayed from its pinned base and source patch. Pass that checkout to the
recovery script. It makes a fresh copy under `/tmp` and reverses the 19
Drawer Swipe paths. The final line is the recovered source path.

```sh
sh restore-source.sh /absolute/path/to/verified/drawer-swipe/source
```

Recovery was compared byte-for-byte with the original Navigation Current
checkpoint. All 662 source files, symlink strings, and executable masks
match. The script excludes generated target and artifact directories and Git
metadata. It also verifies the 13 pinned product inputs.

[Source receipt](source-receipt.txt) · [Product digests](product.sha256)

## Repeat the Runs

Set `CARGO_TARGET_DIR` to a dedicated output directory under
`/tmp/cargo-target` and `CARGO_BUILD_JOBS=2`. From the recovered source,
run `scripts/check.sh` and `cap cargo xtask web e2e material web-accessibility`.

Extract [the native replay capsule](navigation-current-native-replay.tar.gz)
and run its `build-and-run.sh` with the recovered source path. It includes
the original verifier and auditor, the private QA patch, the previous
comparison PNGs, and the AppKit notification collector source. Use an
Accessibility-authorized macOS terminal.
