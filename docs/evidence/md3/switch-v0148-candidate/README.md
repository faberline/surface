# Surface Material 3 Switch v0.1.48 candidate

The candidate adds an opt-in Material 3 Switch. It supports no icon, an
on-state icon, and icons in both states. The checked value stays in the
application. The builder provides one named target for the label and
track. It supports keyboard, pointer, touch, RTL, disabled states,
independent focus, and reduced motion. The earlier Switch API remains
available.

The component uses a 52 × 32 dp track, 16/24/28 dp handle sizes, a 40 dp
state layer, and a 3 dp focus stroke with 2 dp clearance. The label
wraps when the text grows. All five tracks align to the logical trailing
edge in the Gallery. [Material 3 Switch specs](https://m3.material.io/components/switch/specs)

## Exact source

The sealed Switch source has 598 files. Its SHA-256 manifest digest is
`a4ec5bc9d9c0da42fe004f6f7d6877f59edc08e5503ae15c3bbefb6c73f4d086`.
The exact Switch-only patch applies to the sealed Navigation v0.1.47
source, manifest digest
`1b4c1cc1acc3719c5064fd678f91cf99a98c0dc8d1c2204b8d77eb08e732ac76`.
[Navigation candidate evidence](https://github.com/faberline/surface/tree/f9c78507c6ad0945e3faee658dd0057900d78c5b/docs/evidence/md3/flexible-navigation-bar-v0147-candidate)

Apply [the Switch patch](inputs/surface-v0.1.48.patch.gz) after
decompressing it with `gzip -dc`. The uncompressed digest is
`ef1b073a8cf1ed41b6d634ad5345a76fbb2f659c0fe61c5adec0ce8e33b90954`.
Check [the file manifest](inputs/source-final.sha256) from the source root
with `shasum -a 256 --status -c`.

## macOS native and Rust checks

The workspace gate passed 1,326 Rust tests, along with formatting, Clippy,
provenance, and WebAssembly checks. Sixteen GPU tests passed on Metal.
[Workspace log](workspace-final.log) · [GPU log](gpu/gpu-final.log)

The Gallery ran in headless mode. Two replays produced the same light
960 × 800 and dark RTL 320 × 640 PNGs. The compact profile used 200% text.
Each individual screenshot matched across `Page.screenshot`, the CLI,
and MCP. The checks include checked states, keyboard focus, denied
activation of disabled rows, exact track and handle dimensions, and
the wrapped Chinese label. All twelve window observations were zero.
[Wide light](native/final-1/wide-light.png) · [Dark RTL](native/final-1/compact-dark-rtl-200.png) · [First receipt](native/final-1/native-gallery-report.json) · [Second receipt](native/final-2/native-gallery-report.json)

Run [the offline verifier](native/scripts/verify-native-replay.py) with
Python and Pillow to decode both pairs of PNGs and match their receipts.

For a fresh native replay, set `CARGO_TARGET_DIR` to a private temporary
Cargo target for the tested checkout. Use macOS with a Metal adapter.

```sh
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
  --features surface-material-gallery/devtools --bins
proof_dir=$(mktemp -d /tmp/surface-switch-proof.XXXXXXXX)
cap python3 -B /path/to/evidence/native/scripts/replay-switch.py \
  --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
  --cli "$CARGO_TARGET_DIR/debug/surface-devtools" \
  --out "$proof_dir/native"
```

The checkout's `scripts/check.sh` and `scripts/check-gpu.sh` repeat the
workspace and GPU gates.

## Browser focus regression

The first stock Chrome run selected the switch and the runtime retained
its focus. The test waited for the accessibility proxy to own browser
focus after a physical click. Surface keeps browser focus on the canvas
for that input path. The corrected test checks both the focused runtime
switch and browser canvas. The separate keyboard case still checks the
projected switch and Chrome's accessibility focus.
[Original failing run and exact receipts](initial-browser-run/NEGATIVE-EVIDENCE.md)

## macOS Chrome WebGPU

All five headless suites passed, with 19,486 checks and 276 Material
steps. Chrome 158 used Apple Metal. The guards observed both Chrome
processes and the native Gallery, including the bridge after it left
its parent process. The run had zero observed windows and no surviving
test processes.
[Runner log](stock-mac/runner.log) · [Window check](stock-mac/headless-window-observation.json)

The independent verifier checks actual Chrome accessibility states,
the 28 dp held handle, cancellation, RTL positions and pixels in all
four captures. It also checks the wrapped label against the hit row.
[Verified receipt](stock-mac/portable-subset-verified.json) · [Raw report](stock-mac/artifacts/browser-material-switch.json)

[Wide](stock-mac/artifacts/material-switch-wide-settled.png) · [Hover](stock-mac/artifacts/material-switch-hovered.png) · [Held](stock-mac/artifacts/material-switch-held.png) · [Dark RTL](stock-mac/artifacts/material-switch-compact-dark-rtl-200.png)

Rebuild the exact source and run the stock suite with named headless
cases. Save the original test log and artifacts in a temporary directory.

```sh
cap cargo xtask web e2e smoke ime material bridge-web devtools-headless \
  --artifacts /tmp/surface-switch-browser-evidence
```

Use the [browser verifier](stock-mac/verify_switch_report.py) with
`--profile stock-macos`, the original runner log and artifact directory,
and the source manifest above. The complete local 740-file archive
has manifest digest
`02db8c94a3ec0b3158828705d8275f67d289b6da2c11cdd9646decb2610faaa8`.

## Upstream sources

The measurements and color mappings use the official November 19 Google
export. The page hash is
`5d181e144b00817da10999a0bdfe8817a067368ab19c1f7b5abcb493c21c10dd`.
The token table hash is
`86ac6daf573746e3798ccefbb7e1bc1343f0666ca60ce76f572fb021389e6d47`.
[Google page](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/a4ad43f1-5c39-44b2-ae15-a7110bdf4fa6.json) · [Google token table](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.33b1b2925d9ff561.json) · [WAI Switch pattern](https://www.w3.org/WAI/ARIA/apg/patterns/switch/)

The remaining Switch work includes drag to select, live screen-reader
speech, and high-contrast platform settings.
