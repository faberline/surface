# Surface v0.1.45 release verification

Main commit `455d76eccdf64e7579b544ddea929bb5005e8632` has the same tree as candidate `c24162d07ef23de0a646bbf7a923ecd16cd88244`.
The candidate source manifest verified against the released checkout.

The workspace gate used the exact source export with Rust debug symbols and incremental compilation disabled.

## Verification

- 1289 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed.
- Sixteen Metal tests and two exact native headless replays passed. Each replay produced four PNGs. The three transports agreed on every image.
- The Mac headless suite passed five cases and 18545 checks, with four independently decoded Divider PNGs. Both Linux CI runs passed eight cases and 18922 checks each with Chrome SwiftShader.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/189be1cefe88f9094843b681d0b84dcdd1565cc8/docs/evidence/md3/divider-v0145-final-candidate)

Live native input methods, screen-reader speech and additional platform settings remain open. The browser tests use real Chrome focus and simulated composition.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45479269629) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45479253160)
