# Surface v0.1.46 release verification

Main commit `9a134085706a73b89fe26554d070a8a1516fe9c4` has the same tree as candidate `1f150200e46fde7035811760613ba4fbec79bb9b`.
The candidate source manifest verified against the released checkout.

The workspace gate used the exact source export with Rust debug symbols and incremental compilation disabled.

## Verification

- 1308 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed.
- Sixteen Metal tests and two exact native headless replays passed. Each replay produced seven PNGs. The three transports agreed on every image.
- The Mac headless suite passed five cases and 18673 checks, with three independently decoded Menu PNGs. Both Linux CI runs passed eight cases and 19050 checks each with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/db7aa22d94fa93833780d53675b8a233f490b9ce/docs/evidence/md3/menu-v0146-reverified-candidate)

Live native input methods, screen-reader speech and additional platform settings remain open. The browser tests use real Chrome focus and simulated composition.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45485680192) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45485655807)
