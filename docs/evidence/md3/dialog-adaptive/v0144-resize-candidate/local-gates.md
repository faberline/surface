# Exact local Surface v0.1.44 gates

- 1287 Rust tests passed. Provenance, formatting, Clippy, native and WebAssembly checks passed. [Workspace log](workspace.log)
- Sixteen offscreen Metal cases ran. [Metal log](gpu.log)
- The macOS headless Chrome run passed five cases and 18129 checks. It ran M263 with the original resize assertion. The verifier decoded all 41 Dialog PNGs. All 138352 Chrome and 15 Gallery window observations found zero visible windows. [Receipts and PNGs](mac)

See [the source and native replay instructions](README.md). The stock
macOS replay uses the exact source export, Rust with an offline Cargo
cache, Google Chrome, `wasm-opt`, Python and macOS Command Line Tools.

```sh
export SURFACE_EXPECTED_SOURCE_SHA256=f69a406b6b86be6a0f24fb951a413b89312a729c5275862446912ba2039be161
batch=$(mktemp -d /tmp/surface-md3-dialog-stock.XXXXXXXX)
cp -R mac/replay-scripts/. "$batch/"
stock=$(mktemp -d "$batch/stock.XXXXXXXX")
bash "$batch/stock-build.sh" "$exact_source" "$(pwd)/source.sha256" "$stock"
python3 -B "$batch/observe-headless.py" "$stock"
```
