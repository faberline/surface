# Repeat the recovered Surface v0.1.75 checks on macOS

Use this addendum beside the sealed local proof. It adds the standalone
Chrome guard bundle. The local proof includes the native replay tools and
the offline package reader.

[Exact source and patch recovery](https://github.com/faberline/surface/tree/4b7a41753fc00b996b3a1b70885e272876cb77ca/docs/evidence/md3/vertical-command-menu-v0175-recovered-source)

## Verified original run

The recovered source contains 698 files. Its Git tree is
1a8f21971d18d2782882fc8e8e57424bdb9cf301. The workspace passed 1,606 Rust
tests and three Wasm checks. The workspace skipped 20 ignored tests. The
separate Metal runs passed 16 offscreen tests and two text-selection tests
and retained 21 original PNGs.

The native replay passed two repeats. Each of its eight original PNGs
matched across Page.screenshot, the CLI and MCP. The guard observed 46
samples with zero visible owned windows and reaped six Gallery processes.
The native evidence manifest SHA-256 is
be6da7fa25432c6d102809c07b1f6070ab35878366cc0d88cd252a3099cccc19.
See native-readback.json for the independent local replay of that receipt.

Chrome WebGPU passed all 371 Material steps and 30,557 checks.
There were 4,692 window samples, zero visible windows and zero surviving
processes. The verified archive includes six original menu PNGs.

## Prerequisites and fresh outputs

Use the build tools listed in toolchain.txt. Install the Rust
wasm32-unknown-unknown target for that toolchain. Use macOS Command Line
Tools, Chrome, Python 3.12, and Pillow. Use the repository's cap wrapper
with two Cargo jobs. Keep any Git dependency credentials in your normal
Cargo setup.

Follow the source recovery link. Set source to the fresh reconstructed
source directory and source_manifest to its archived source.sha256.
Set proof to the final local proof directory. Use absolute paths for
those three values. Run this block from the addendum directory. Repeat the
setup with a new output directory for each independent rerun.

~~~
set -eu
addon=$(pwd -P)
: "${source:?set the exact recovered source directory}"
: "${source_manifest:?set the archived source.sha256 path}"
: "${proof:?set the sealed local proof directory}"
fresh=$(mktemp -d /tmp/surface-v0175-replay.XXXXXXXX)
fresh=$(CDPATH= cd -- "$fresh" && pwd -P)
source_sha=4ab84a94bbc6923a313a66e6be5f68d43d03834e18d8f962bc8f95413b439b87
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:/opt/homebrew/bin:$PATH"
py="${SURFACE_REPLAY_PYTHON:-python3}"
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=$(xcrun --sdk macosx --show-sdk-path)
export RUSTUP_TOOLCHAIN=1.96.1
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-$(basename "$fresh")"
export CARGO_BUILD_BUILD_DIR="$CARGO_TARGET_DIR"
sha256() { shasum -a 256 "$1" | awk '{print $1}'; }
test "$(sha256 "$source_manifest")" = "$source_sha"
(cd "$addon/browser-tools" && shasum -a 256 --status -c SHA256SUMS)
(cd "$proof/native-tools" && shasum -a 256 --status -c SHA256SUMS)
snapshot="$fresh/source"
"$py" -E -B - "$source" "$snapshot" "$source_manifest" <<'PY'
from pathlib import Path
from shutil import copy2
import sys

origin, target, manifest = map(Path, sys.argv[1:])
for entry in manifest.read_text().splitlines():
    name = entry.split("  ", 1)[1]
    file = target / name
    file.parent.mkdir(parents=True, exist_ok=True)
    copy2(origin / name, file)
PY
source="$snapshot"
cd "$source"
shasum -a 256 --status -c "$source_manifest"
~~~

## Workspace, build and Metal

Run these steps in order. Build new outputs for this run and record their
digests. The original binary hashes describe the archived build. Local
paths and toolchains can affect a fresh executable's hash.

~~~
sh scripts/check.sh > "$fresh/workspace-v0175-recovered.log" 2>&1
mkdir "$fresh/web" "$fresh/binaries" "$fresh/gpu"
cap cargo xtask web build material-gallery --release --no-compress \
    --out "$fresh/web/material-gallery" > "$fresh/build-v0175-recovered.log" 2>&1
cap cargo build --locked --release -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools >> "$fresh/build-v0175-recovered.log" 2>&1
cap cargo build --locked -p surface-e2e --bin surface-e2e > "$fresh/stock-runner-build.log" 2>&1
cp -p "$CARGO_TARGET_DIR/release/surface-material-gallery" \
    "$CARGO_TARGET_DIR/release/surface-devtools" "$fresh/binaries/"
(cd "$fresh/binaries" && shasum -a 256 surface-material-gallery surface-devtools > SHA256SUMS)
(cd "$fresh/web/material-gallery" && shasum -a 256 index.html surface-web.json ./*.js ./*.wasm > ../SHA256SUMS)
runner="$CARGO_TARGET_DIR/debug/surface-e2e"
shasum -a 256 "$runner" > "$fresh/stock-runner.sha256"
shasum -a 256 --status -c "$source_manifest"

SURFACE_GPU_EVIDENCE="$fresh/gpu/gpu-offscreen.png" \
    sh scripts/check-gpu.sh > "$fresh/gpu-primary.log" 2>&1
cap cargo test --locked -p surface-wgpu --test text_selection_gpu -- --ignored --nocapture \
    > "$fresh/gpu-supplemental.log" 2>&1
(cd "$fresh/gpu" && shasum -a 256 ./*.png > SHA256SUMS)
shasum -a 256 --status -c "$source_manifest"
~~~

## Guarded Chrome and native runs

Run one graphical test lane at a time. The Chrome guard starts a headless
browser in a new private profile. It checks every Material step in the
unmodified registry. The native guard uses its own headless process for
each repeat and profile.

~~~
chrome_gate() {
    "$py" -E -B "$addon/browser-tools/run-stock-material.py" "$1" \
        --source "$source" --source-manifest "$source_manifest" \
        --runner "$runner" --runner-sha256 "$(sha256 "$runner")" \
        --web-root "$fresh/web" --web-manifest "$fresh/web/SHA256SUMS" \
        --web-sha256 "$(sha256 "$fresh/web/SHA256SUMS")" --out "$fresh/guarded-stock-material"
}
chrome_gate --check-inputs
chrome_gate --run
chrome_gate --verify

binary_manifest="$fresh/binaries/SHA256SUMS"
native_gate() {
    "$py" -E -B "$proof/native-tools/replay-vertical-menu.py" "$@" \
        --source "$source" --source-manifest "$source_manifest" --source-sha "$source_sha" \
        --binary-manifest "$binary_manifest" --binary-sha "$(sha256 "$binary_manifest")"
}
native_gate --check-inputs
native_gate --run --out "$fresh/guarded-native"
"$py" -E -B "$proof/native-tools/replay-vertical-menu.py" --readback "$fresh/guarded-native" \
    --source-sha "$source_sha" --binary-sha "$(sha256 "$binary_manifest")" \
    --evidence-sha "$(sha256 "$fresh/guarded-native/SHA256SUMS")"
~~~

## Browser tool files

Run run-stock-material.py from browser-tools, with the adjacent
evidence_common.py and observe-focused-original.py. The stock runner imports
the first helper. It verifies the second helper's digest before starting the
guard. The supplied browser-tools/SHA256SUMS covers all three files and the
original guard instructions. The local proof reader resolves shared helpers
from its own directory. This addendum provides the reviewed browser helper
beside the standalone runner.
