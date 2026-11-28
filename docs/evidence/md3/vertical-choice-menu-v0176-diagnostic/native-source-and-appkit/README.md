# Surface v0.1.76 menu diagnostic evidence

The [original R3 source proof](https://github.com/faberline/surface/tree/6ad148ea4b6419d5d011873bcb45a720f2f88c34/docs/evidence/md3/vertical-choice-menu-v0176-recovered-source) reconstructs tree `b3e62f07a03b08f1741de8695fe845d3b771f11d` from v0.1.75. The source manifest covers 702 files.

## R3 results

- The five targeted Rust suites passed 14 tests. See [the original log](r3/choice-component-tests.log).
- Two headless native runs made eight PNGs. The Standard light, Vibrant light and 320 dp dark RTL at 200% text captures match across both runs. Page.screenshot, the CLI and MCP returned matching bytes. The observer found zero visible windows across 46 samples. All six Gallery processes exited. See [the receipt](r3/native/native-vertical-choice-menu-report.json) and [offline readback](r3/native-readback.json).
- R3 Chrome was in progress when this diagnostic snapshot was prepared. This directory has no Chrome result.

Use Python 3.11 or later to check the stored native evidence without opening an app.

```sh
proof=$(pwd -P)
shasum -a 256 -c SHA256SUMS
python3 -E -B r3/native-tools/replay-vertical-choice-menu.py \
  --readback "$proof/r3/native" \
  --source-sha b1b3dea6427bb8bdd5debdd2715d3956e8ac4549a03a70880aa7fe12aabc700f \
  --binary-sha bc9b9cf64478abed3d5d0b282fe87cd0930e5ceba8b9c2de12d4aa7fd7d9a1d4 \
  --evidence-sha 1f6ad356d954fe0fba2d3b66bc2d826e96084dfb02ceb6c53032f71291110e43
```

## R4 source

The combined R4 tree is `37e4454bc7d3f63f7111c8264d4e59e451c8ef9e`. The [703-file manifest](r4/source.sha256) matches that tree. The three patches fix outside-click detection on a disabled item, expose the macOS checkmark attribute and update the menu coverage record. The combined R4 tree has yet to run as a full application.

Set `proof` to this evidence directory. Use a fresh temporary R3 checkout and apply the patches in this order.

```sh
test "$(git write-tree)" = "$(cat "$proof/r3/source.tree")"
shasum -a 256 -c "$proof/r3/source.sha256"
(cd "$proof/r4/patches" && shasum -a 256 -c SHA256SUMS)

git apply --index --whitespace=error-all "$proof/r4/patches/disabled-menu.patch"
git apply --index --whitespace=error-all "$proof/r4/patches/macos-menu-mark-v0176.patch"
git apply --index --whitespace=error-all "$proof/r4/patches/v0176-menu-coverage-implementation.patch"
test "$(git write-tree)" = "$(cat "$proof/r4/source.tree")"
shasum -a 256 -c "$proof/r4/source.sha256"
```

## Detached AppKit regression

The baseline fails at the checked menu checkbox attribute selector. The patched test passes. An unattached AppKit view lets the test call the actual selectors on the main thread without opening a window. It checks the role, attribute names and value, numeric AXValue, and enabled or disabled action state. VoiceOver speech and external accessibility client delivery still need separate checks.

The [baseline log](appkit/accesskit-baseline.log) and [patched log](appkit/accesskit.log) record the original runs. To repeat both cases, start from exact R3 and set `proof` to this directory.

```sh
scratch=$(mktemp -d /tmp/surface-appkit-menu-mark.XXXXXXXX)
for lane in baseline patched; do
  mkdir -p "$scratch/$lane/third-party"
  cp -R third-party/accesskit-macos "$scratch/$lane/third-party/"
  cp "$proof/appkit/Cargo.lock" "$scratch/$lane/third-party/accesskit-macos/Cargo.lock"
done
ax_patch="$proof/r4/patches/macos-menu-mark-v0176.patch"
(cd "$scratch/baseline" && git apply \
  --include=third-party/accesskit-macos/Cargo.toml \
  --include=third-party/accesskit-macos/tests/menu_item_mark_char.rs "$ax_patch")
(cd "$scratch/patched" && git apply --exclude=docs/provenance.json "$ax_patch")

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_BUILD_JOBS=2
CARGO_TARGET_DIR="/tmp/cargo-target/surface-${scratch##*/}-baseline" \
  cap cargo test --locked --manifest-path "$scratch/baseline/third-party/accesskit-macos/Cargo.toml" \
  --test menu_item_mark_char
CARGO_TARGET_DIR="/tmp/cargo-target/surface-${scratch##*/}-patched" \
  cap cargo test --locked --manifest-path "$scratch/patched/third-party/accesskit-macos/Cargo.toml" \
  --test menu_item_mark_char
```

The first command must fail at the same selector assertion as the stored baseline. The second must exit with code 0.
