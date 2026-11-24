# Surface headless E2E default

The test runner starts five headless suites when no suite name is given.
Explicit `all` keeps the full eight-suite list. Hosted CI uses that list
on a virtual display.

The macOS default run passed 18,673 checks across all five suites. The
guards observed the primary Chrome process, a detached bridge Chrome
process, and the native Gallery. All ran headless. The observer recorded
no visible windows and no remaining test processes. The windowed
Compact launcher was never invoked.

- [Runtime counts and verified receipts](observed-run/verified-default.json)
- [Window observer results](observed-run/headless-window-observation.json)
- [Commands and offline verification](observed-run/README.md)
- [Native Gallery PNG](observed-run/artifacts/devtools-headless-screenshot.png)
- [Workspace checks](workspace.log)

## Source

The parent commit of this evidence is the exact Surface Menu candidate
`1f150200e46fde7035811760613ba4fbec79bb9b`. Decompress
[the default-runner patch](headless-default.patch.gz) with `gzip -dc` and
apply it to that commit. Its uncompressed SHA-256 is
`c5aaa1107fd8ed314ccc09ceda5bdb089c48ed1c915fe4b95115361569a9e706`.
Only the test runner, its tests, the CI invocation, and documentation
are changed. The 16 re-used app and browser inputs matched their original
Menu build hashes before and after the run.

Run `cap sh scripts/check.sh` in the patched source. The workspace gate
passed 1,310 Rust tests, along with provenance, formatting, Clippy, and
WebAssembly compilation. The parser tests cover the no-suite default and
explicit windowed opt-in.

For the full runtime test, build the runner and the app inputs from that
source. The observer in this package checks the default invocation with
paths only and uses CoreGraphics to sample window owners. See the exact
replay commands in the observation package.
