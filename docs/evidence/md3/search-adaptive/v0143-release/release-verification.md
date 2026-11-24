# Surface v0.1.43 release verification

Main commit `fec35d355025fa7476fac223baa60628471fd8fb` has the same tree as candidate `fc233e9a592d886f56dbc1397234edeaded2331b`.
The candidate source manifest verified against the released checkout.

The workspace gate used the exact source export with Rust debug symbols and incremental compilation disabled.

## Verification

- 1250 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed.
- Eleven Metal tests and two exact native headless replays passed. Each replay produced eight PNGs. The three transports agreed on every image.
- The Mac headless suite passed five cases and 17295 checks, with 46 Search PNGs checked. Both Linux CI runs passed eight cases and 17672 checks each with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/75f07bedd6abf55c7e665d4add21de985b2eddd8/docs/evidence/md3/search-adaptive/v0143-release)

Live native input methods, screen-reader speech and additional platform settings remain open. The browser tests use real Chrome focus and simulated composition.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45452814888) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45452793148)
