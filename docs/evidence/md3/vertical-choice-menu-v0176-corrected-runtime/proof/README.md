# Surface v0.1.76 menu runtime evidence

The [R3 source recovery](https://github.com/faberline/surface/tree/6ad148ea4b6419d5d011873bcb45a720f2f88c34/docs/evidence/md3/vertical-choice-menu-v0176-recovered-source) supplies the starting point. The R4 product is tree `37e4454bc7d3f63f7111c8264d4e59e451c8ef9e`. R4 passed 1,615 workspace tests, three Wasm checks and 18 Metal tests. The Metal run retained 21 PNGs. R4 native headless made eight original PNGs from two repetitions. Page.screenshot, the CLI and MCP produced matching bytes. The native observer found zero windows across 46 samples. All six Gallery processes exited.

R4 Chrome reached M375 after passing M373 and M374. M375's left-corner probe hit the RTL scroll thumb. The original PNG and the pixel crop show the thumb color on the left, the dark group color at the mirrored right corner, and the selected face color just inside. The archive keeps this failed run and its exact source and build pins as a diagnostic.

R5 tree `b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9` changes only `e2e/src/material_choice_menu.rs`. It mirrors the probes to the logical leading edge. All 702 other source files match R4. The R5 Chrome run uses a newly built test runner with the verified R4 Web files.

## Preflight and collect

Use Python 3.11 or later with Pillow on the original host. Both commands only read run outputs and write the new archive. Use a fresh output folder under `/tmp`.

```sh
python3 -E -B collect.py --preflight
python3 -E -B collect.py --collect /tmp/surface-v0176-r5-runtime
```

The collector requires a real terminal R5 stock result with all 374 steps, three menu reports and nine original menu PNGs. M373 must show that a disabled pointer click leaves the menu open with the same focus and values. M375 must show the dark group color and selected color in the mirrored samples and in the original PNG. It must also show successful scroll, toggle and Escape focus return. Both browser runs have separate process and window receipts. The failed R4 run cannot satisfy the R5 requirement.

## Read back the archive

Use the SHA-256 printed by collection.

```sh
python3 -E -B /path/to/archive/collect.py --verify /path/to/archive --sha "$reviewed_manifest_sha256"
```

The reader resolves physical paths before verifying the native originals. The archive holds the logs and hashes and omits executables, build caches, browser profiles and live sessions.

| Folder | Evidence |
| --- | --- |
| `tools/`, `build/` | R4 source patches, build logs, toolchain and producer pins |
| `rust/`, `native/` | R4 Rust, Wasm, Metal and native headless originals |
| `r4-browser-failure/` | Original failed receipt, matching compact PNG and a lossless diagnostic crop |
| `r5/`, `r5-tools/` | Test-only patch, old and new test source, R5 source and runner pins, and browser oracle |
| `browser/` | Actual R5 stock result, guard ledger, typed reports and original PNGs |

The crop uses device pixels `(104,496)` through `(583,521)` from the unmodified compact PNG. The old probe at `(114,506)` is `[255,217,227]`. The mirrored corner at `(574,506)` is `[99,59,72]`. The inner point at `(564,510)` is `[239,184,200]`.

## Reproduce source and checks

Reconstruct exact R3 from the source recovery link. In a fresh scratch checkout set `proof` to this archive. Apply the product patches and verify the R4 tree and files.

```sh
test "$(git write-tree)" = b3e62f07a03b08f1741de8695fe845d3b771f11d
shasum -a 256 -c "$proof/tools/base.sha256"
for name in disabled-menu.patch macos-menu-mark-v0176.patch v0176-menu-coverage-implementation.patch; do
  git apply --index --whitespace=error-all "$proof/tools/patches/$name"
done
test "$(git write-tree)" = "$(cat "$proof/tools/source.tree")"
shasum -a 256 -c "$proof/tools/source.sha256"
```

Use the [R4 build and replay recipe](tools/README.md) for the product. In a fresh temporary R4 checkout apply `r5/rtl-corner-probe.patch` and check the tree and every file against `r5/source.tree` and `r5/source.sha256`. Use the [R5 runner recipe](r5-tools/README.md) to build the runner and replay Chrome. Set tool paths and regenerate pins for a new location.

The archive covers local headless runs. Hosted Linux and actual VoiceOver speech need separate evidence.
