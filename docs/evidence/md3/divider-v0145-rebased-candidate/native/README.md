# Surface v0.1.45 Divider verification on the repaired Dialog base

This verification uses the exact 583-file source with SHA-256
`86f886cc10ab1d1b82e62364caf738824d2ce5295e3fb82533082f155763277e`.
The Divider patch is byte-for-byte identical to the earlier v0.1.45 patch.
The repaired Dialog base tree is `50c3a5646a033b04280f7050748ca92c42154ad5`.

| Viewport | Appearance | Regular | Heavy |
| --- | --- | --- | --- |
| 960 × 800 dp | Light, LTR, 100% text | [1 dp](run/replay-1/wide-light-ltr-regular.png) | [8 dp](run/replay-1/wide-light-ltr-heavy.png) |
| 320 × 800 dp | Dark, RTL, 200% text | [1 dp](run/replay-1/compact-dark-rtl-200-regular.png) | [8 dp](run/replay-1/compact-dark-rtl-200-heavy.png) |

The headless replay checks horizontal, single-inset, symmetric-inset, and
vertical line thickness in real GPU pixels. It checks the Outline Variant
color, the unpainted one-pixel border, and the mirrored logical inset.
Tab skips the decorative dividers. Enter changes the weight and restores it
on a second press. Back restores launcher focus.

The native screenshot API, CLI, and MCP must produce the same PNG and
metadata on every capture. The two independent replays must produce identical
files. Each replay samples the Gallery's visible window count eight times.
All counts must be zero.

[Verified receipt](run/verified-native.json) · [Replay 1](run/replay-1/native-divider-report.json) · [Replay 2](run/replay-2/native-divider-report.json)

## Reproduce

Use macOS, Command Line Tools, Rust, `cap`, Python 3, and a populated offline
Cargo cache. The build script verifies the supplied source before and after
compilation. It uses a private Cargo target and pins both executables. The
replay recomputes each source and binary hash before and after both runs.

```sh
set -eu
shasum -a 256 -c SHA256SUMS
(cd run && shasum -a 256 -c repeat.sha256 && diff -qr replay-1 replay-2)
archive=$(pwd -P)
batch=$(mktemp -d /tmp/surface-divider-rebased-native.XXXXXXXX)
cp "$archive/build-pinned.sh" "$archive/replay-native.sh" "$archive/verify-native.py" "$batch/"
cp -R "$archive/native-scripts" "$batch/"
sh "$batch/build-pinned.sh" "$exact_source" "$archive/source.sha256" \
  86f886cc10ab1d1b82e62364caf738824d2ce5295e3fb82533082f155763277e
PYTHON_BIN=python3 sh "$batch/replay-native.sh" "$batch/run"
python3 -B "$batch/verify-native.py" "$batch" "$batch/run"
```

The native Gallery uses `--headless` during every replay. This receipt covers
native paint and semantic input.
