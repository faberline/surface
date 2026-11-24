# Surface v0.1.46 Menu native successor replay

The 590-file Menu successor uses source manifest SHA-256
`bda81b7c07f063d65aced4a4d29c3aefbee33ca40275fd2a34489b0930326019`.
The only source change from the first exact v0.1.46 Menu proof is the
Divider browser test at `e2e/src/material_divider.rs`. The replay uses the
native Gallery and CLI from the sealed stock build. It verifies the entire
stock binary bundle, then copies and checks the two native pins. It checks
source and native binary hashes before and after the replay. No additional
Cargo build is needed.

| Gallery viewport | Appearance | Captures |
| --- | --- | --- |
| 960 × 800 dp | Light, LTR | [Open](run/replay-1/wide-light-ltr-opened.png), [disabled focus](run/replay-1/wide-light-ltr-disabled-focused.png), [next command](run/replay-1/wide-light-ltr-find-focused.png) |
| 320 × 480 dp | Dark, RTL | [Open at 200%](run/replay-1/compact-dark-rtl-200-opened.png), [last command after End](run/replay-1/compact-dark-rtl-200-scrolled-end.png), [footer at 200%](run/replay-1/compact-dark-rtl-200-status-200.png), [complete result at 100%](run/replay-1/compact-dark-rtl-200-status-100.png) |

The wide menu opens below the trigger. The native key API focuses a
disabled command, blocks Enter and Space on that command, and runs the next
enabled action. Direct activation of the disabled command returns
`disabled`. Tab, Shift+Tab and Escape return focus to the expected target.

The compact menu opens above the trigger. End reveals the last command and
Enter runs it. The replay checks the 8 dp clearance in the layout and PNG
after changing text size from 200% to 100% and back. The full status and its
text appear at 100%. This replay uses semantic native keys and activation.
It does not send a mouse wheel event into the Gallery.

Each replay compares Page.screenshot, CLI and MCP at every capture. The
replays must produce the same seven PNGs and JSON bytes. The CoreGraphics
window list must show zero onscreen windows owned by the tested Gallery in
each of ten samples per replay.

## Reproduce

Use macOS, Python 3.10 or later, the exact source and the matching sealed
stock stage. Set `stock` and `exact_source` to their absolute paths.

```sh
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-menu-fringe-proof.XXXXXXXX)
cp "$archive/pin-from-stock.sh" "$archive/replay-native.sh" "$archive/verify-native.py" \
  "$archive/source.sha256" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
ln -s "$exact_source" "$batch/source"
sh "$batch/pin-from-stock.sh" "$stock" "$exact_source" \
  bda81b7c07f063d65aced4a4d29c3aefbee33ca40275fd2a34489b0930326019
PYTHON_BIN=python3 sh "$batch/replay-native.sh" "$batch/run"
```

[Verified native result](run/verified-native.json) · [Original replay 1 report](run/replay-1/native-menu-report.json) · [Original replay 2 report](run/replay-2/native-menu-report.json) · [Comparison with the prior Menu proof](prior-capture-comparison.json)
