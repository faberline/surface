# Surface v0.1.74 Adaptive Navigation evidence

Exact candidate tree f7a92f77ecc9b37c91d81ace4877cd8038044d5c. Five macOS headless stock suites passed 30,690 checks, including 368 Material steps through M369. The packet retains the full runner log and original window and process receipts. It independently decodes four Adaptive and seven inherited Navigation PNGs from stock, plus four from scoped R4. It verifies six exact native originals, 1,603 Rust tests, three Wasm checks and 18 Apple Metal tests with 21 GPU PNGs.

Native uses three fixed viewport profiles. The browser exercises real viewport changes, stable backward draft selection and independent rail scroll. The preserved R3 browser failure belongs to its old JSON key-order assertion.

The inherited decoders ran on the original stock outputs before export. The packet includes their scripts, selected reports and the complete original stock PNG/JSON hash index.

[Published q73 Horizontal Navigation evidence](https://github.com/faberline/surface/tree/961ab6fe023a4d883e0aa6843854429060a2e402/docs/evidence/md3/horizontal-navigation-bar-v0173-candidate)

Run the following with Python 3.12 and Pillow. Use the reviewed digest of SHA256SUMS.

    python3 -E -B pack-adaptive-nav.py --verify-package . --package-sha REVIEWED_SHA

Offline verification needs no source or build directories. The packet contains no app binaries, browser profiles or development session credentials.
