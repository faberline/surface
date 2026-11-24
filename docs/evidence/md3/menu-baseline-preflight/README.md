# Surface anchored command menu interim evidence

This package records local headless checks for an additive baseline Material 3 anchored command menu. The work still carries version `0.1.45`. This evidence covers the Menu slice. Hosted CI, the complete stock suite, and a Menu release remain open.

## Source

The [portable patch](source/divider-to-menu.patch) applies to the sealed 583-file Divider export. It changes 14 paths, including seven added files. The independent verifier reproduced all 590 files in the Menu manifest. It also reproduced the earlier Web Gallery and native build inputs, and both focused test inputs.

| Input | SHA-256 of its manifest or patch |
| --- | --- |
| [Divider source](source/divider.sha256) | `afad7c8d68a82dd5ee23bfb20bc05de0afd36292ffa906d72b4012e0ea724da3` |
| [Portable Menu patch](source/divider-to-menu.patch) | `3f8b4f93eed647325e59768875f67db2d1399a515a8a8b8af0890c66ea21f0e9` |
| [Final interim Menu source](source/menu.sha256) | `bdb64599289aa03b6097ab9c2e3fdeff9d6f04c0bb967cd8e9eeb60643907316` |
| [Shared Web Gallery and native build source](native/source.sha256) | `5eeeb4ef56134d28edc731f137fd648871e199edb2708bc54accaff38bbc9519` |
| [Focused runner source](browser/focused-source.sha256) | `4b0419c3781a162525f75153752eead87c0881b48fbc4adaf8f497b7579c773a` |
| [Three-step selector patch](browser/focused-selector.patch) | `7871dfa735792d8fbb8fe954e902d0812d1badb8fbb07ae2d2290630dce9078e` |

The final tree and the shared build source differ only in one browser test file. The [test patch](browser/wait-for-projection.patch) waits for the opener's actual browser `aria-expanded` state after a pointer dismissal. It leaves the Gallery and runtime bytes unchanged. The independent verifier also matched the original Gallery WASM, both browser runners, and both native binaries against their recorded hashes. The executable files are stored separately from this evidence.

## Headless Chrome on macOS

Chrome `158.0.8127.37` passed M267–M269, with **117 checks**, zero reported page errors, and 294 process-window samples that found no visible window. No observed Chrome process remained. These three tests ran against the Gallery WebGPU build. The receipt does not record the adapter name.

The wide test checks the named menu and separator, a disabled command that accepts focus but rejects activation, and open-below placement. The next test checks Tab and Shift+Tab handback, outside-click dismissal and protection of the action behind the popup. At 320 × 480 CSS pixels, the dark RTL menu flips above, keeps its content in bounds at 200% text, and scrolls the last command into view on End. The decoder confirmed both PNGs. It matched the wide panel samples `[242,236,244]` and `[236,231,238]` to the recorded receipt.

[Raw browser report](browser/pass/browser-material-command-menu.json) · [Runner log](browser/pass/runner.log) · [Window observations](browser/pass/headless-observation.json) · [Web bundle and runner hashes](browser/runtime.sha256) · [Wide PNG](browser/pass/material-command-menu-wide.png) · [Compact PNG](browser/pass/material-command-menu-compact.png)

The earlier M268 attempt failed on an immediate `aria-expanded` read. The captured state already showed the popup closed and focus returned to the opener. The test then waited for the same browser attribute to become false. Both attempts used the same WASM, and their wide PNGs match byte for byte. An earlier separate diagnostic also confirmed that a real canvas click keeps Surface's disabled-command focus while transferring the browser's active element to the canvas. [Original M268 failure](browser/negative/browser-material-command-menu.json) · [Focus diagnostic](native/dom-diagnostic/dom-focus-diagnostic.json)

## Offscreen native Gallery on macOS

Two independent replays each produced five PNGs. The raw API, CLI, and MCP screenshot bytes and metadata matched. Both output sets were byte-identical and each recorded eight zero-window samples. The 960 × 800 light case verifies open-below layout, the disabled command, keyboard focus, and selection. The 320 × 480 dark RTL case verifies open-above layout, 200% text, and the last command revealed from below the clip. Native tests drive semantic keys through DevTools and use the offscreen Reduce Motion path.

[Native verification](native/native-interim/verified-native.json) · [Raw native report](native/native-interim/replay-1/native-menu-report.json) · [Native binary hashes](native/pins/native-binaries.sha256) · [Disabled focus PNG](native/native-interim/replay-1/wide-light-ltr-disabled-focused.png) · [Compact scrolled PNG](native/native-interim/replay-1/compact-dark-rtl-200-scrolled-end.png)

## Verify and replay

Use `python3` with Pillow for this package's independent PNG decoder. Use an exact export of the Divider base to verify the patch. `verify-evidence.py` uses a temporary copy and leaves the export unchanged.

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verify-evidence.py --divider-source "$divider_source"
```

To rerun the checks on macOS, use Command Line Tools, `cap`, Rust, the offline Cargo cache, and a no-window environment with private loopback access. The Web build also needs the installed WASM toolchain and Google Chrome. All output goes below `/tmp`.

```sh
set -eu
archive=$(pwd -P)
stage=$(mktemp -d /tmp/surface-menu-replay.XXXXXXXX)
cp -R "$divider_source" "$stage/final"
(cd "$stage/final" && shasum -a 256 -c "$archive/source/divider.sha256" && \
  patch -p1 < "$archive/source/divider-to-menu.patch" && shasum -a 256 -c "$archive/source/menu.sha256")
cp -R "$stage/final" "$stage/shared"
(cd "$stage/shared" && patch -R -p1 < "$archive/browser/wait-for-projection.patch" && \
  shasum -a 256 -c "$archive/native/source.sha256")
mkdir "$stage/web-runner" "$stage/native-runner"
cp "$archive/browser/"*.sh "$archive/browser/"*.py "$stage/web-runner/"
cp "$archive/native/"*.sh "$archive/native/verify-native.py" "$stage/native-runner/"
cp -R "$archive/native/native-scripts" "$stage/native-runner/"
bash "$stage/web-runner/build-gallery.sh" "$stage/shared" "$archive/native/source.sha256" \
  5eeeb4ef56134d28edc731f137fd648871e199edb2708bc54accaff38bbc9519
python3 -B "$stage/web-runner/prepare-focused.py" "$stage/final" "$archive/source/menu.sha256" \
  bdb64599289aa03b6097ab9c2e3fdeff9d6f04c0bb967cd8e9eeb60643907316
cmp "$archive/browser/focused-selector.patch" "$stage/web-runner/focused-selector.patch"
bash "$stage/web-runner/build-focused.sh"
python3 -B "$stage/web-runner/observe-menu.py"
sh "$stage/native-runner/build-pinned.sh" "$stage/shared" "$archive/native/source.sha256" \
  5eeeb4ef56134d28edc731f137fd648871e199edb2708bc54accaff38bbc9519
PYTHON_BIN=python3 sh "$stage/native-runner/replay-native.sh" "$stage/native-output"
```

The browser launcher uses `--headless=new` and observes only its own Chrome process tree. The native launcher uses `--headless` and the existing offscreen GPU path. The build scripts use separate capped Rust targets with two jobs. For details and the source reconstruction results, see the [independent verification receipt](verified-interim.json).
