# Surface v0.1.46 native command menu proof

The stock Gallery and CLI come from the 590-file release candidate. Its
source manifest SHA-256 is
`92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5`.
The proof checks the source and executable hashes before and after the
replays.

| Gallery viewport | Appearance | Captures |
| --- | --- | --- |
| 960 × 800 dp | Light and LTR | [Open](run/replay-1/wide-light-ltr-opened.png), [disabled focus](run/replay-1/wide-light-ltr-disabled-focused.png), [next command](run/replay-1/wide-light-ltr-find-focused.png) |
| 320 × 480 dp | Dark and RTL | [Open at 200%](run/replay-1/compact-dark-rtl-200-opened.png), [last command after End](run/replay-1/compact-dark-rtl-200-scrolled-end.png), [footer clip at 200%](run/replay-1/compact-dark-rtl-200-status-200.png), [complete result at 100%](run/replay-1/compact-dark-rtl-200-status-100.png) |

The wide menu opens below the trigger. The native key API focuses a disabled
item, ignores Enter and Space, and chooses the next enabled item. A direct
activation request for the disabled item returns `disabled`. Tab,
Shift+Tab and Escape restore the expected focus.

The compact menu opens above the trigger and End reveals its last command.
Enter chooses it. The replay measures the named preview scroll clip, the
status and the trigger at 200%, 100% and 200% again. It checks 8 dp of
clearance and exact pixels across that gap. At 100%, the full status
and its text ink are visible. The trigger stays at the same right
and bottom edges as its width reflows.

Stock native DevTools drives semantic keys and activation. Chrome M269 and
the Gallery Runtime test cover wheel reveal of the full passive status at
200%. The native replay measures the retained status below the clip at that scale.

[Verified native receipt](run/verified-native.json) · [Replay 1](run/replay-1/native-menu-report.json) · [Replay 2](run/replay-2/native-menu-report.json)

## Reproduce

Use macOS, Command Line Tools, Rust, `cap`, Python 3 and a populated offline
Cargo cache. Set `exact_source` to the checkout that matches the manifest.
The build uses a private Cargo target. The Gallery runs with `--headless`
and uses the native offscreen GPU. The test samples only its own process
from the macOS window list.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-md3-menu-v0146-native.XXXXXXXX)
cp "$archive/build-pinned.sh" "$archive/replay-native.sh" "$archive/verify-native.py" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
sh "$batch/build-pinned.sh" "$exact_source" "$archive/source.sha256" \
  92972f3fabec4458fab582356aba1aa21441a0413c28c22a9cb76f95df7ee3d5
PYTHON_BIN=python3 sh "$batch/replay-native.sh" "$batch/run"
```

Each replay produces seven PNGs and one JSON report. It checks the exact same
PNG bytes and metadata through Page.screenshot, CLI and MCP. All ten visible
window samples per replay must be zero.
