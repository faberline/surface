# Surface v0.1.44 release verification

Main commit `9e2e89d77e584f66888cdfb20c0bb1257dc55d9c` has the same tree as candidate `9b8e02b8668eba8da49c1fdcdf1832dee05865fb`.
The candidate source manifest verified against the released checkout.

The workspace gate used the exact source export with Rust debug symbols and incremental compilation disabled.

## Verification

- 1287 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed.
- Sixteen Metal tests and two exact native headless replays passed. Each replay produced fourteen PNGs. The three transports agreed on every image.
- The Mac headless suite passed five cases and 18129 checks, with 41 Dialog PNGs checked. Both Linux CI runs passed eight cases and 18506 checks each with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/839fec3cdaa8c79dc68ef831541ec46fb8cf9dde/docs/evidence/md3/dialog-adaptive/v0144-resize-candidate)

Live native input methods, screen-reader speech and additional platform settings remain open. The browser tests use real Chrome focus and simulated composition.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45467288705) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45467274408)
