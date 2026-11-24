# Adaptive Search verification

Surface v0.1.43 candidate `fc233e9a592d886f56dbc1397234edeaded2331b` has Git tree `35f17666034d0f39367dc3b139e06a927fca0455`. The source file manifest has 557 entries. Verify it from the candidate checkout with `shasum -a 256 --status -c /path/to/candidate-source.sha256`.

- `cap sh scripts/check.sh` passed 1,250 Rust tests and the provenance, formatting, Clippy and WebAssembly checks on the same source.
- The offscreen Apple M1 Max Metal run passed 11 tests. [Metal records and PNG](gpu)
- Two native headless replays produced the same eight PNGs. Page.screenshot, the CLI and MCP agreed byte for byte. The runner recorded zero visible windows in 24 samples. [Native replay, source hashes and PNGs](https://github.com/faberline/surface/tree/52794069552f7ba18dbae5cf3679665c52c3ab68/docs/evidence/md3/search-adaptive/v0143-adapter-native)
- Stock macOS headless Chrome passed five cases and 17,295 checks with the physical Apple Metal adapter. It decoded 46 Search screenshots. [Browser replay, source hashes and screenshots](https://github.com/faberline/surface/tree/caca7c5e97a34e90ad44d427ec6bbee0e4a1bb97/docs/evidence/md3/search-adaptive/v0143-adapter-mac-stock)
- [Linux PR CI](https://github.com/faberline/surface/actions/runs/45452814888) and [Linux branch CI](https://github.com/faberline/surface/actions/runs/45452793148) use Chrome SwiftShader. Both Linux runs passed eight cases and 17672 checks. [Exact hosted results](hosted).

The first hosted run exposed a test that expected a hardware adapter. The follow-up changes only that assertion and still requires actual WebGPU adapter data and the same Canvas checks. [Initial hosted result](https://github.com/faberline/surface/tree/52794069552f7ba18dbae5cf3679665c52c3ab68/docs/evidence/md3/search-adaptive/v0143-hosted-first-attempt) and [test-only patch](adapter-check.patch).

The browser tests exercise keyboard routing and simulated composition. Native screen-reader speech and live system input methods remain separate.
