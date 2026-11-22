# Surface v0.1.8 app-bar verification

This batch starts from Surface v0.1.7
(`40dd3535b3797c7372ed5d9231eb8a44228f6fe6`). From a matching source
checkout, set `evidence_dir` to this folder and check the source manifests.

```sh
shasum -a 256 -c "$evidence_dir/native/native-source.sha256"
shasum -a 256 -c "$evidence_dir/web/browser-source.sha256"
cap sh scripts/check.sh
```

## Native headless Metal

The 960 × 800 replay checks nominal heights and placement of five
app-bar variants, with and without a subtitle where applicable. It
verifies the collapsed flexible bar, its raised surface, true centering,
a named search trigger, editable modal search, query results, empty state,
Clear and Escape. Modal and contextual close restore focus.

The dark 320 × 580 profile checks RTL, 200% Chinese text and action
targets. It compares PNG bytes and metadata from `Page.screenshot`, CLI
and MCP. The two profiles run with Reduce Motion and no windows.

[Native report](native/native-app-bar-report.json) ·
[Center aligned](native/light-center-64.png) ·
[Dark compact large](native/dark-rtl-200-large.png) ·
[Filtered search](native/dark-rtl-200-search-filtered.png)

Use a Cargo target under `/tmp/cargo-target` and a command environment
with access to Metal.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
scratch_dir=$(mktemp -d /tmp/surface-app-bar-replay.XXXXXXXX)
cap cargo build --locked -p surface-material-gallery -p surface-devtools \
    --features surface-material-gallery/devtools --bins
source_digest=$(shasum -a 256 "$evidence_dir/native/native-source.sha256" | cut -d ' ' -f 1)
PYTHONDONTWRITEBYTECODE=1 python3 "$evidence_dir/native/verify-app-bar-native.py" \
    --source "$source_digest" --gallery "$CARGO_TARGET_DIR/debug/surface-material-gallery" \
    --cli "$CARGO_TARGET_DIR/debug/surface-devtools" --out "$scratch_dir/native-results"
```

## Headless Chrome WebGPU

Chrome 157 on Apple Metal 3 passed the full Material suite with 61 steps
and 2,719 checks. M56–M62 account for 405 checks.

M56–M62 verify actual WebGPU paint, visual centering with unequal action
counts, keyboard and RTL focus, and the rest and scrolled surfaces.
Real wheel events exercise Pinned, EnterAlways and ExitUntilCollapsed.
The tests compare the wheel delta left for the content against actual
row displacement and the DOM offset. Browser input drives the Search
View and both contextual actions. A 320 dp dark RTL profile with 200%
Chinese text checks large headings and scrollable settings.

[Browser report](web/browser-material-app-bar.json) ·
[Compact dark large](web/material-app-bar-compact-dark-large.png) ·
[Collapsed after wheel](web/material-app-bar-enter-always-collapsed.png)

```sh
cap cargo build --locked -p surface-e2e --bin surface-e2e
cap cargo xtask web build material-gallery --release --no-compress
cap "$CARGO_TARGET_DIR/debug/surface-e2e" \
    --chrome "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    --web-root "$CARGO_TARGET_DIR/web/release" \
    --artifacts "$scratch_dir/web-results" material
```

Full hide and reveal for regular bars, kinetic scrolling and settling,
and Search View transition animation remain open. Physical touch and
live VoiceOver speech still need platform tests.
