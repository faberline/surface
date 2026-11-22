# Surface v0.1.2 Snackbar verification

This record covers the standalone Material 3 Snackbar. It includes the
text-only notice, one text action, the close icon, and an action row that
wraps below the message in narrow layouts. The Gallery uses the same
Surface runtime on native macOS and in Chrome WebGPU.

## Source pins

- [Product manifest](product-source.sha256) lists 281 exact source files.
  Its SHA-256 is
  `d693740b8635b0db4cf4a5f3cb96b421116defc6fc3ae603857a39ac91c6e507`.
  The native run-4 report uses this value as its source ID.
- [Browser source manifest](browser/browser-product-source.sha256) lists
  267 exact source files, including the browser driver and fixtures.
  Its SHA-256 is
  `9174fa09fe12ed2a859954fa04e70edf403bd5631e6ccf62d2f3744150c5d66d`.
- The implementation ports the passive message and separate actions from
  the earlier Snackbar work. That earlier workspace contained unrelated
  component changes and its receipts apply only to its own source tree.
  The clean candidate starts with Surface v0.1.1 at
  `d0963e6fde96b1f331a1f29e1c6236dbf0120593`, applies the native screenshot
  source at `36ea9b655a12f71c0620bd6500ec7aa8487d8e2b`, and adds the
  Snackbar, its queue, native and browser status projection, and the
  Gallery fixtures. The manifests pin the resulting code before the
  evidence was packaged.
- PR #175 later adjusted only the independent DevTools test driver,
  `e2e/src/devtools_native.rs`. The final browser replay ran with that
  adjustment. The [release source manifest](release-source.sha256)
  covers all 281 paths. Its SHA-256 is
  `85da5697e5e87dbd013c3ff36c90c9270462498c775f9d781c5d34f27edddfdb`.
  The native report keeps its original manifest ID. The production
  files used by the native Gallery match the release manifest.
  [Screenshot PR](https://github.com/faberline/surface/pull/175)

## Native macOS

Run 4 passed the light and dark 960 × 800 profiles and the dark
320 × 480 right-to-left profile with 200% text. It checked painted glyphs,
the bottom inset, the 600 dp maximum width, separate controls with 48 dp
hit areas, and compact CJK wrapping. Each Undo and Close sent one message.
Showing the notice kept background focus. Action or Close restored the
opener. The text-only notice expired after about 4.000 seconds without
input. [Exact native report](native/run-4/report.json)

AppKit exposed the notice as one passive status, with separate named
actions. The observer received one message-only announcement request with
priority 50 for the sampled show in each profile. The text-only status was
read before its timer expired. The native driver sent Tab and Enter
through the Surface DevTools protocol. Physical keyboard and touch input
and live VoiceOver speech remain unverified. Full animated exit remains
open. [AppKit and replay source](native/replay/README.md)

- [Light](native/run-4/light-wide/visible.png),
  [dark](native/run-4/dark-wide/visible.png),
  [compact](native/run-4/dark-rtl-200-compact/visible.png),
  [text only](native/run-4/light-wide/text-only.png)
- Raw `Page.screenshot`, CLI file output, and the MCP image block
  returned the same compact PNG. The retained files and hashes can be
  compared directly. [PNG parity receipt](native/png-parity.json)

## Chrome WebGPU

Chrome 157.0.8103.79 passed 30 steps and 429 checks in the full Material
suite. Steps M28–M31 cover timed disappearance, queue progression through
Enter and pointer release, dark RTL at 320 dp and 200% CJK text, and the
accessible status and actions. The recorded WebGPU adapter was Apple
Metal 3 with `isFallbackAdapter: false`. The browser receipt records DOM
announcement updates and accessibility tree state. [Test log](browser/browser-exact.log) · [Browser receipt](browser/browser-material-snackbar.json)

- [Light action](browser/material-snackbar-light-action.png)
- [Dark RTL at 200%](browser/material-snackbar-dark-rtl-compact-200.png)

## Replay

From an exact candidate checkout, set `evidence_dir` to this evidence
directory. Check the source before building.

```sh
shasum -a 256 -c "$evidence_dir/release-source.sha256"
shasum -a 256 -c "$evidence_dir/browser/browser-product-source.sha256"
```

The desktop replay uses the stock native host. Follow the
[build and run commands](native/replay/README.md). The small profile
launcher changes only the logical window size. It stays in the replay
copy of the source.

For the browser case, use Chrome with WebGPU and the repository's
WebAssembly tools. Run these commands from the exact checkout. Set
`browser_output` to a new temporary directory.

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-browser-replay"
cap cargo xtask web build material-gallery --release --no-compress
cap cargo build --locked -p surface-e2e
"$CARGO_TARGET_DIR/debug/surface-e2e" material \
  --web-root "$CARGO_TARGET_DIR/web/release" --artifacts "$browser_output"
```

From this evidence directory, verify the saved evidence bytes.

```sh
shasum -a 256 -c evidence.sha256
```

## Workspace and Metal verification

The workspace passed strict Clippy, provenance, formatting, 672 Rust
tests and all three WASM checks. Six were ignored in the routine
Rust run. Five separate offscreen GPU tests passed on Apple M1 Max Metal.
[Workspace log](workspace-check.log) · [Metal log](metal-check.log)
