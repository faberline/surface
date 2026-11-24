# Surface v0.1.42 release verification

Main commit `eacb33d06269a0b0a75c3c13f4de82de05f19031` has the same tree as candidate `86bf5eb86a5d070245db039b37977198ba1a417e`.
The candidate source manifest verified against the released checkout.

The workspace gate used the exact source export with Rust debug symbols and incremental compilation disabled. Original primary workspace log remained separate.

## Verification

- 1226 Rust tests, provenance, formatting, Clippy and WebAssembly checks passed.
- Eleven Metal tests and two exact native headless replays passed. Each replay produced eight PNGs. The three transports agreed on every image.
- Mac stock Chrome passed five cases and 16123 checks. Both Linux CI runs passed eight cases and 16500 checks each.

[Reproduction and screenshots](https://github.com/faberline/surface/tree/b3712280fda88b756adf78b033b61e997dcdc54c/docs/evidence/md3/scroll-contrast-v0.1.42)

Live OS preference notifications, native input methods, screen-reader speech and Windows forced-color captures remain open. The browser composition tests use trusted Chrome input.

[Linux PR CI](https://github.com/faberline/surface/actions/runs/45435977006) · [Linux branch CI](https://github.com/faberline/surface/actions/runs/45435959502)
