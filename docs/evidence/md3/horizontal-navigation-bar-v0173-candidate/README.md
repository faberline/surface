# Surface v0.1.73 Horizontal Navigation evidence

Exact candidate tree 5d5ab69f57bdb3f31a8db95ae78f9f332789578b. Five macOS headless stock suites passed 30,490 checks, including 364 Material steps through M365. The packet retains the full runner log and original window and process receipts. It independently decodes seven Navigation PNGs from stock and seven from scoped R3. It also decodes four exact native originals and verifies 18 Apple Metal tests and 21 GPU PNGs.

The inherited decoders ran on the original stock outputs before export. Their scripts, raw report hashes and the published q72 evidence identify the inherited coverage. This packet retains all Navigation PNGs and selected inherited reports. The complete original log and artifact index preserve the rest of the stock scope.

[Published q72 Field-pair evidence](https://github.com/faberline/surface/tree/75835231436a1cbc4d0cacf41a1401c5020e1e39/docs/evidence/md3/text-field-system-pair-v0172-candidate)

Run the following with Python 3.12 and Pillow. Use the reviewed digest of SHA256SUMS.

    python3 -E -B pack-horizontal-nav.py --verify-package . --package-sha REVIEWED_SHA

Offline verification needs no source or build directories. The packet contains no app binaries, browser profiles or development session credentials.
