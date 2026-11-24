# Surface v0.1.45 native Divider weight verification

The exact 583-file source built the stock Gallery and DevTools CLI. Both independent headless replays produced the same four PNGs and JSON. Page.screenshot, CLI and MCP returned identical PNG bytes and metadata for each capture. Each replay made eight visible-window checks. Every count was zero. No test process or private session remained.

| Gallery viewport | Appearance | Regular | Heavy |
| --- | --- | --- | --- |
| 960 × 800 dp | Light, LTR, 100% text | [1 dp](run/replay-1/wide-light-ltr-regular.png) | [8 dp](run/replay-1/wide-light-ltr-heavy.png) |
| 320 × 800 dp | Dark, RTL, 200% text | [1 dp](run/replay-1/compact-dark-rtl-200-regular.png) | [8 dp](run/replay-1/compact-dark-rtl-200-heavy.png) |

Each profile changes weights in the same Gallery process. Tab follows the controls in order. Enter on the focused weight button applies Heavy, and Enter again restores the Regular geometry. Tab skips all four decorative dividers. Back restores the launcher focus.

The decoder checks all four physical line shapes in each PNG at native 1× scale. The complete interior has the exact Outline Variant color across the measured 1 or 8 pixels of thickness. The two antialiased end rows of each vertical line paint across its full width. The entire one-pixel border beyond each line keeps the exact page background. The 16 dp leading inset mirrors in RTL. Both middle insets remain 16 dp.

[Independent native receipt](run/verified-native.json) · [Raw run 1](run/replay-1/native-divider-report.json) · [Raw run 2](run/replay-2/native-divider-report.json)

Google's Android implementation specifies 1 dp Regular and 8 dp Heavy with the same color role. The published design export records the placement and inset rules. Surface applies the same thickness choice to its vertical builder. [Google Android Divider guide](https://github.com/material-components/material-components-android/blob/cc2390f1a2b314c4f89114f51ad81fb642e491eb/docs/components/Divider.md#adding-heavy-divider) · [Google Divider design export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/843ffaa9-e73e-45a7-bff7-f198cac0a7c8.json)

## Verify and replay

Use macOS, Command Line Tools, Rust, `cap`, Python 3 and a populated offline Cargo cache. Set `exact_source` to the source export that matches the manifest. The source fingerprint is `afad7c8d68a82dd5ee23bfb20bc05de0afd36292ffa906d72b4012e0ea724da3`. The build and tests used a private Cargo target. The pinned executables remain separate. The archived build log replaces private paths.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-heavy-divider-native.XXXXXXXX)
cp "$archive/build-pinned.sh" "$archive/replay-native.sh" "$archive/verify-native.py" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
sh "$batch/build-pinned.sh" "$exact_source" "$archive/source.sha256" \
  afad7c8d68a82dd5ee23bfb20bc05de0afd36292ffa906d72b4012e0ea724da3
PYTHON_BIN=python3 sh "$batch/replay-native.sh" "$batch/run"
python3 -B "$batch/verify-native.py" "$batch" "$batch/run"
```

Set `SURFACE_QA_CARGO_HOME` if the cache is outside its default path. The native Gallery opens with `--headless` for every session. These tests cover native Runtime and paint. Browser and live screen-reader behavior have separate evidence.
