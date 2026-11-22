# Native Surface screenshot verification

Source commit `36ea9b655a12f71c0620bd6500ec7aa8487d8e2b`.
Source tree `4a86a2265106643fefd7ea6bcf1d58bb66837ffb`.
[Pull request #175](https://github.com/faberline/surface/pull/175)

The raw request, CLI and MCP use the production native DevTools path
with a real Gallery window. The test host was a Mac with an Apple M1
Max GPU and Metal.

- The workspace passed formatting, provenance, strict Clippy, 657
  Rust tests and all three WebAssembly checks. Six tests were ignored
  by the routine workspace run.
- Five real Metal offscreen GPU tests passed in their separate run.
- Native DevTools passed all 17 steps and 192 checks. The PNG size
  matches the logical viewport at the reported scale. The tests check
  the channel order, a fresh frame after a theme change and a 400 px
  limit. Raw, CLI JSON, CLI file and MCP PNGs had identical bytes on
  the same frame. The CLI kept an existing path intact.
- The native, compact and Web Jet bridge tests also passed. The four
  suites passed 471 checks. The Chrome run used real WebGPU.
- The feature-off check passed. The normal dependency tree then had
  no PNG encoder or Surface DevTools protocol dependency.

## Replay

Clone Surface into a disposable directory under `/tmp`. Check out
the exact source commit above. On a Mac with a real Metal adapter run:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS=2
export CARGO_TARGET_DIR="/tmp/cargo-target/surface-${PWD##*/}"
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web e2e devtools-native bridge-native bridge-native-compact bridge-web
cap cargo check --locked -p surface-platform --no-default-features
cap cargo tree --locked -p surface-platform --no-default-features -e normal
shasum -a 256 -c evidence.sha256
```

The checksum command runs in this evidence directory. The fixtures
send ordinary Surface protocol requests. The initial sandboxed tests
returned no Metal adapter or window session. The recorded runtime
passes used the native desktop and GPU.

## Native PNGs

| Frame | Evidence |
| --- | --- |
| Stock Gallery in light theme | [Native PNG](devtools-native-screenshot.png) |
| Same Gallery after the theme changed | [Native PNG](devtools-native-screenshot-toggled.png) |
| Scaled frame through the two clients | [CLI PNG](devtools-native-screenshot-cli.png) · [MCP PNG](devtools-native-screenshot-mcp.png) |
| Offscreen renderer on Metal | [PNG](offscreen-metal.png) |

[Workspace](workspace-check.log) · [Metal](metal-check.log) ·
[Transport](transport.log) · [Native receipt](devtools-native-result.json)

The same native test caught three injected faults in a disposable
checkout. It rejected swapped color channels, a stale theme and an
ignored size limit. With the faults disabled, the same binary passed
all 192 checks. [Fault controls and replay](negative-controls/README.md)
