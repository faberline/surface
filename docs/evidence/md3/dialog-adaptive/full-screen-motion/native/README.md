# Full-screen Dialog on native Surface

The stock Gallery ran on Apple Metal without a window. The source fingerprint is `bb0e450ddb5c7a6ebaf7d592cf6c3f54e97f8db51ba118e5dc74d743ac9eba6a`. This is the exact pre-release source used for these captures. The archive includes its source manifests and binary fingerprints. The debug executables stay outside this archive.

Both independent runs captured 14 PNGs. Page.screenshot, the CLI and MCP returned identical PNG bytes and metadata. The 14 PNGs and the JSON report match across runs. Each run sampled 22 times for visible windows. Every sample was zero.

| Viewport | Appearance | Verified result |
| --- | --- | --- |
| 320 × 400 | Dark RTL, 200% text | Full-screen editor and clear saved status |
| 480 × 640 | Light LTR, 100% text | Full-screen editor |
| 960 × 800 | Light LTR, 100% text | Centered Basic Dialog fallback |
| 320 × 240 | Dark RTL, 200% text | Tab scrolls the settings 68 dp and leaves the FAB fixed |

The journey verifies where focus goes when the dialog opens and closes. It checks that Continue keeps the draft, Save persists it, and Discard keeps the earlier saved value. The stock headless host enables Reduce Motion. This receipt covers the start and end states. Timing and physical input have their own checks.

## Saved text and FAB clearance

- The [initial compact capture](negative/initial/compact-dark-rtl-200-fab.png) shows the FAB covering part of the Alert label.
- The first clearance change removed the duplicate launchers. Its [LTR reference](negative/first-clearance/ltr.png) and [RTL capture](negative/first-clearance/rtl.png) have the same saved-text layout. The RTL FAB covers 60 foreground pixels at x33–72 and y320–324. [Comparison receipt](negative/first-clearance/comparison.json)
- The [corrected compact capture](native/replay-1/compact-dark-rtl-200-fab.png) shows the full saved status. Its bounds end at y300. The preview clip ends at y312 and the FAB starts at y320. The [short viewport capture](native/replay-1/short-dark-rtl-200-fab-after-tab.png) shows the revealed setting with the same 8 dp clearance.

## Check the archive

Run these from this directory. The original repeat manifest retains its relative paths. The logs replace machine paths with placeholders.

```sh
shasum -a 256 -c SHA256SUMS
(cd native && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
PYTHONDONTWRITEBYTECODE=1 python3 scripts/verify-ink-comparison.py
```

## Reproduce the final native run

Start from an exact source tree that matches `inputs/final/source.sha256`. The build requires a Mac with Metal, the Command Line Tools SDK, Rust, `cap`, Python 3, and an offline Cargo cache. Set `SURFACE_QA_CARGO_HOME` if the cache is outside the default `/tmp/surface-md3-cargo`. Run the scripts from a new scratch directory.

```sh
archive=$(pwd)
work=$(mktemp -d /tmp/surface-dialog-native.XXXXXXXX)
cp "$archive/scripts/build-pinned.sh" "$archive/scripts/replay-native.sh" "$work/"
cp -R "$archive/scripts/native-scripts" "$work/"
sh "$work/build-pinned.sh" "$exact_source" "$archive/inputs/final/source.sha256" \
    bb0e450ddb5c7a6ebaf7d592cf6c3f54e97f8db51ba118e5dc74d743ac9eba6a
PYTHON_BIN=python3 sh "$work/replay-native.sh" "$work/run"
```

Set `exact_source` to the final source directory before running the commands. The scripts check source and binary hashes before and after building or replaying. A build in a different location can produce different executable bytes. It records its own binary fingerprints.

To reconstruct the final source from the exact Basic Motion checkpoint, copy that checkpoint and run this from the copy. Set `archive` to the absolute path to this evidence directory first. Every stage was reconstructed and checked for this archive. [Reconstruction receipt](native/reconstruction.log)

```sh
shasum -a 256 -c "$archive/inputs/basic-motion/source.sha256" || exit 1
for stage in core-transform runtime-transform initial first-clearance final; do
    for delta in "$archive/inputs/$stage/"*.patch; do
        patch --batch -p1 -i "$delta" || exit 1
    done
    shasum -a 256 -c "$archive/inputs/$stage/source.sha256" || exit 1
done
```

The two raw reports are in `native/replay-1/` and `native/replay-2/`. [Input fingerprints and evidence index](archive.json)
