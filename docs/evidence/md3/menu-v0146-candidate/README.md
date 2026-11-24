# Surface v0.1.46 Menu candidate evidence

This package reconstructs the exact v0.1.46 candidate from the repaired
v0.1.45 Divider source. It records two headless native replays and the
focused Chrome M267–M269 run. Full stock browser and hosted CI acceptance
remain separate gates at this handoff.

| Source | Files | Manifest SHA-256 |
| --- | ---: | --- |
| Repaired v0.1.45 Divider | 583 | `86f886cc10ab1d1b82e62364caf738824d2ce5295e3fb82533082f155763277e` |
| Exact v0.1.46 Menu candidate | 590 | `92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5` |

The [single release patch](surface-v0.1.46.patch) changes 18 files and adds
seven of them. It includes the command menu, disabled-item focus, pointer
scope handling, the Gallery footer, tests, coverage docs and the release
version. The repaired Dialog resize code is already in the base.
[Exact changed paths](source-diff.json).

## Verified runs

- Native macOS uses stock Gallery and CLI. The two independent runs each
  produce seven identical PNGs through Page.screenshot, CLI and MCP.
  All twenty visible-window samples are zero. The compact replay retains
  status across 200%, 100% and 200% text while the trigger stays in place
  with an 8 dp gap. [Native report](native/run/verified-native.json) ·
  [Native instructions](native/README.md).
- Focused Chrome 158 uses the unchanged candidate Gallery and a runner
  selector for M267–M269. All three steps and 128 checks pass on the
  observed Apple Metal WebGPU adapter. The observer records 275 process
  samples, zero visible windows and no remaining owned Chrome process.
  The compact run scrolls the complete 200% status into view by 112 dp
  and keeps 8 dp of clearance to the stationary trigger.
  [Raw browser report](browser/run/browser-material-command-menu.json) ·
  [Raw Chrome log](browser/run/runner.log) ·
  [Compact footer PNG](browser/run/material-command-menu-compact-footer.png).
- The independent browser validator confirms the browser report and PNGs.
  It also rejects a clipped status, a repeated open-menu PNG and a truncated
  PNG in its negative controls. The earlier Linux SwiftShader input in
  those controls tests adapter metadata parsing.
  [Exact validator receipt](browser/validator/exact-focused-check.json) ·
  [Controls](browser/validator/controls-check.json).

The raw browser runner log retains its task-local artifact path so the
validator reproduces its exact recorded hash. The two browser build logs
use relative paths and carry the hashes of the originals. The native
folder and its own manifest are unchanged. Source and binary hash
manifests bind each build to its corresponding replay.

## Verify the archive

Run these commands from the package directory. Use Python 3 with Pillow
for the browser image checks.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
(cd native && shasum -a 256 -c SHA256SUMS)
(cd native/run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
(cd browser/validator && shasum -a 256 -c SHA256SUMS)
browser_receipt=$(mktemp /tmp/surface-menu-browser-receipt.XXXXXXXX)
python3 -B browser/validator/verify_menu_report.py browser/run --platform macos > "$browser_receipt"
cmp "$browser_receipt" browser/validator/exact-focused-check.json
```

## Reconstruct and replay

Set `repaired_divider` to the source directory that matches
`base.sha256`. The reconstruction copies only the 583 verified paths,
applies the single release patch and checks all 590 output paths.

```sh
archive=$(pwd -P)
scratch=$(mktemp -d /tmp/surface-menu-reconstruct.XXXXXXXX)
candidate="$scratch/source"
python3 -B "$archive/reconstruct-source.py" "$repaired_divider" "$candidate"
(cd "$candidate" && shasum -a 256 -c "$archive/candidate.sha256")
```

Use the [native instructions](native/README.md) with this candidate.
For focused Chrome, use macOS, Command Line Tools, Rust with the wasm32
target, `cap`, a populated offline Cargo cache, `wasm-opt`, Python 3 and
Chrome. The focused runner selector only changes `e2e/src/material.rs`.
The Gallery build uses all 590 unmodified candidate files.

```sh
chrome_work=$(mktemp -d /tmp/surface-menu-focused.XXXXXXXX)
cp "$archive/browser/build-gallery.sh" "$archive/browser/build-focused.sh" \
  "$archive/browser/prepare-focused.py" "$archive/browser/run-focused.sh" \
  "$archive/browser/observe-menu.py" "$archive/browser/window-observer.py" "$chrome_work/"
candidate_hash=92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5
bash "$chrome_work/build-gallery.sh" "$candidate" "$archive/candidate.sha256" "$candidate_hash"
python3 -B "$chrome_work/prepare-focused.py" "$candidate" "$archive/candidate.sha256" "$candidate_hash"
cmp "$chrome_work/focused-selector.patch" "$archive/browser/focused-selector.patch"
cmp "$chrome_work/focused-source.sha256" "$archive/browser/focused-source.sha256"
bash "$chrome_work/build-focused.sh"
python3 -B "$chrome_work/observe-menu.py"
```

The observer prints the new `run.*` directory. Pass that directory to
the browser validator with `--platform macos`. Its [instructions](browser/validator/README.md)
also define the separate stock and Linux modes.
