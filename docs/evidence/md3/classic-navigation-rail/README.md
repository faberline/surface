# Classic Material Navigation Rail checkpoint

This checkpoint adds the classic 80 dp rail with 24 dp vector icons and separate visible and accessible names. The app owns the current destination. The rail moves focus with arrows and skips disabled destinations. Activating a destination changes the page. The 200% Chinese labels fit a single 32 dp line at 320 dp viewport width.

## Restore the exact product source

Restore the Directional Drawer source with the published procedure. Use the named revision and verify its full source tree hash. The procedure below reverses the Directional Drawer delta and then the Snackbar delta. It verifies the exact Navigation Rail file list, file hashes, symlink targets and executable bits. The original 12-path Navigation Rail forward delta is included as `navigation-rail.patch.gz`.

- [Pinned Directional Drawer recovery](https://github.com/faberline/surface/tree/2725afd021230fd73df20582098a2587d2149a20/docs/evidence/md3/directional-drawer)
- Source tree SHA-256 `d1c6adecf9e4397259a34bfd844f4a0b3bf18ecf2fc9470d8dfc1e5b1b974f78` for 657 source files.
- Original forward delta SHA-256 `e92f6c52a7a409fa748bca3631c6f7c572b24966a881b8488f6f895d49e05cb3` before compression. Its predecessor is Search Feedback. Use `restore-source.sh` to start from the published Directional Drawer checkpoint.

```sh
shasum -a 256 -c evidence.sha256
sh restore-source.sh /path/to/verified-directional-drawer-source
```

The historical test receipts are summarized in `test-receipts.json`. This recovery run verified the source and selected captures. It preserved the original scope of the runtime checks.

## Retained results

- Rust workspace verification recorded 1,373 passed, zero failed and 25 ignored. The receipt lists the suites and browser fixture revisions.
- Chrome 157 WebGPU used Apple Metal 3. The two focused cases passed 430 checks. The final full run passed 24,357 checks after rebuilding the WASM platform library.
- Native Gallery on Apple M1 Max Metal captured 28 CLI PNGs. The audit checked 84 label boxes, 84 icon samples and 20 public AppKit accessibility results. The retained 320 × 640 dark RTL 200% selected PNG matched the MCP capture byte for byte.

The native run used a private test overlay for its receipts and input. The source recovery uses the product tree. The AppKit checks verified focus, names, disabled controls and press actions. Public selected state and live screen-reader speech were outside the verified scope. The packet includes representative original PNGs and omits working sessions, binaries and broad generated output.
