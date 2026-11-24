# v0.1.42 composition cancellation repair preflight

Both controlled Chrome cases pass on the repaired exact Showcase. The test keeps redraw pending through the real composition and the trusted key action. It then compares textarea state with Surface state.

- A collapsed cancellation keeps Select All at browser 0–2 and Rust 0–6 through a real redraw. The following text replaces the whole value.
- A backward selection recovers the original value 台文 and browser range 1–2 in backward order. Rust retains bytes 6–3. Trusted Right and the next input produce 台文X before any redraw. The value remains 台文X after repaint.

The version with only matching browser values passes the first case and loses a character in the second. Earlier v0.1.41 and v0.1.42 bundles also reproduce the second case. The repair restores Runtime's value and directed selection only for an active same-owner empty composition end. Nonempty commit keeps its browser-value acknowledgement.

The updated stock Showcase runner passes I1–I20 with 77 checks in headless Chrome. The focused smoke case passed 34 checks. The bundle and runner are pinned in the [replay record](replays/README.md). This is preflight evidence for a draft update. Full release gates remain.

Apply [the final source patch](final-source.patch) to `ab8615c8495d99a652f4fac38d4db37f08c82c7f`. Verify [the 549-file source manifest](final-source.sha256) from that source root. The Showcase Web build uses [this binary manifest](showcase-web.sha256). Use `shasum -a 256 -c SHA256SUMS` to verify this folder.

Local output paths are removed from the replay logs. The nested checksum file verifies the published copies. The original manifest records the unedited receipts.
