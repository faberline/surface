# Multiline Chrome development proof

The E2E patch adds M341–M345 to the normal Material case. Its only product paths are the E2E registration and test module. The registry now requires 344 Material steps through M345. The development runner selects just these five.

The isolated Web build starts from v0.1.64, source manifest SHA-256 `fe7349b293de3078ca44fd6b1fa1885a5fa156ba559503c795cd9d6daab5e4a3`. It applies the wheel base SHA-256 `526b97d2f32c0a3db76f72a92f781a42cf82e406fa21051dbaeb2441a2c16c85` and the Gallery patch SHA-256 `d566af1780c5b7580ed2cfbe41d0e92cc408470ff4fe2f4f4fe144eaada533b5`. The integrated run uses the four separate Web pack bytes recorded in `web-integrated.sha256`. Its Wasm contains the additional Page Up/Down and touch routing.

| Run | Scope | Result |
| --- | --- | --- |
| `development-r1` | Initial wheel-only test | Red at M343. The commanded 48 units moved the body 48 device pixels, which was 24 logical pixels at DPR 2. The second event did not reach the inner boundary. |
| `development-r2` | Scaled wheel movement | Red at the typed event-log comparison. Inner movement, outer residual and reverse paint checks passed. Chrome exposed logical deltas even though CDP received device units. |
| `development-r3` | Corrected wheel-only test | M341–M344, 184 checks, eight original PNGs, no page errors. |
| `integrated-r1` | Integrated touch and Page keys | M341–M345, 247 checks, twelve original PNGs, no page errors. |

Both passing Chrome runs used Apple Metal 3. The wheel proof shows the 48 dp inner movement, exactly 64 dp of parent residual and complete reversal. The integrated touch proof shows a 56 dp parent residual during a real captured touch preview and exact original body pixels after the browser's trusted capture loss. PageDown, Shift+PageUp and PageUp reached the actual textarea and moved its Rust and browser selections. Chrome exposed the visible disabled and read-only proxies, the read-only textarea's value and associated help, and the disabled field was skipped in Tab order. This checks Chrome accessibility metadata and CDP composition, not live screen reader speech or OS IME.

The existing headless guard observed 56 Chrome samples for the integrated run. It found no visible owned windows or surviving owned processes. The verification script independently decodes the original PNGs and compares the wheel and touch body regions. It also checks the focused step list, raw browser and observer reports, and actual Chrome adapter.

## Reproduce

Use the repository Rust build instructions, the exact dependency patches, an isolated Cargo output directory and two jobs. Build the wheel-only Web release from its source copy before applying the final E2E patch. The integrated Web pack is separate and must match `web-integrated.sha256`.

```sh
cap cargo run --locked --offline -p xtask -- web build material-gallery --out "$PWD/../web-development/material-gallery"
cap cargo fmt --all --check
cap cargo test --locked --offline -p surface-e2e --bin surface-e2e material::tests
```

Place the focused fixture scripts one directory above `source`. Run the following for the integrated scope from the fixture directory with the same isolated Cargo environment.

```sh
python3 -B build-integrated.py --out "$PWD/focused-replay"
python3 -B run-integrated.py --runner focused-replay/surface-e2e --runner-sha256 "$(cat focused-replay/runner.sha256)" --web-root web-integrated --out integrated-replay
python3 -B verify-development.py integrated-replay
```

The run script requires headless Chrome, the pinned existing observer and a fresh output directory under /tmp. The read-back script uses Pillow. For the wheel-only scope, apply `multiline-e2e-wheel-only.patch` in place of the final E2E patch, use `build-focused.py` and `run-focused.py`, and pass `--wheel-only` to the read-back script. The complete stock suites and published source remain separate acceptance steps.
