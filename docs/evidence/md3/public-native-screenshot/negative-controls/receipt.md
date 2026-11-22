# Native screenshot negative controls

On November 22, 2026, the native Material Gallery on macOS arm64 passed 128 checks across seven steps with fault injection disabled. DN25 exercised authenticated `Page.screenshot` and produced 960×800 PNGs before and after the theme change. DN26 verified that the CLI wrote a private PNG and rejected an overwrite. DN27 verified that MCP returned a PNG image. [Passing transcript](scoped-evidence/baseline.log) · [Light PNG](scoped-evidence/baseline/devtools-native-screenshot.png) · [Dark PNG](scoped-evidence/baseline/devtools-native-screenshot-toggled.png)

| Fault applied independently | Original DN25 assertion | Exit |
| --- | --- | --- |
| Swap red and blue after GPU readback | Pixel (70, 476) expected RGBA `[101,85,143,255]` and returned `[143,85,101,255]` | 1 |
| Return the first default PNG after changing theme through public DevTools | Pixel (1, 1) expected dark RGBA `[20,18,24,255]` and returned the original light RGBA `[253,247,255,255]` | 1 |
| Drop the valid `max_side` request after validation | Requested a 400 px maximum and received a 960 px maximum | 1 |

The audit required each process to stop on its expected DN25 assertion. It matched the stderr markers to the injected faults and cross-checked the stale pixel and ignored size against the passing PNGs. [Audited JSON](scoped-evidence/negative-controls.json) · [Red and blue transcript](scoped-evidence/swapped-red-blue.log) · [Stale image transcript](scoped-evidence/stale-default-image.log) · [Ignored size transcript](scoped-evidence/ignored-max-side.log)

## Source and test scope

The disposable copy was verified against the 19-file source manifest with SHA-256 `fa6ccdfdc5cf1eed654ac3c7951ded91b6672524411d0c94c46bfd77bb8e2825`. The temporary application change affected only `crates/surface-platform/src/screenshot.rs`. All four runs used the same freshly built Gallery, CLI, and scoped runner binaries. [Source manifest](frozen-source.sha256) · [Fault diff](fault.patch.gz) · [Binary hashes](scoped-binaries.sha256) · [Compiler source paths](compiler-origins.json)

The initial full runner stopped at inherited DN5. It expected DOM snapshot version 1 and received version 4. The disposable route kept DN1–4 and DN25–27. The screenshot helpers and assertions remained byte-identical to the frozen runner, with SHA-256 `68219a1f8b2387fb55627f40f8566cf55c92665764f60c1c67f136f310815f05`. Application input ran through the authenticated public protocol. [Initial transcript](baseline.log) · [Route diff](runner-route.patch.gz) · [Helper hash](runner-route-check.log)

## Reproduce

Use a new directory in `/tmp` and a checkout that passes `shasum -a 256 -c frozen-source.sha256`. Build with the repository's capped Cargo command. The build script copies the checkout, applies the disposable fault, and records the app and client binaries. Generate the scoped test route and rebuild only the runner. Record the scoped binary and runner source hashes before running the four cases in a native macOS session. [Copy and build](build-controls.sh) · [Fault generator](inject.py) · [Route generator](scope-runner.py) · [Session runner](run-scoped.sh) · [Audit](audit-controls.py)

```sh
sh "$proof/build-controls.sh" "$frozen_checkout"
cd "$proof/repo"
python3 "$proof/scope-runner.py" > "$proof/runner-route-check.log"
export CARGO_HOME=/tmp/surface-md3-cargo
export CARGO_TARGET_DIR="/tmp/cargo-target/$(basename "$proof" | tr . -)"
export CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
cap cargo clean -p surface-e2e
cap cargo build --locked -p surface-e2e --bin surface-e2e
cp "$CARGO_TARGET_DIR/debug/surface-e2e" "$proof/bin/surface-e2e-scoped"
shasum -a 256 e2e/src/devtools_native.rs e2e/src/main.rs > "$proof/scoped-runner-source.sha256"
shasum -a 256 "$proof/bin/surface-e2e-scoped" "$proof/bin/surface-material-gallery" "$proof/bin/surface-devtools" > "$proof/scoped-binaries.sha256"
sh "$proof/run-scoped.sh"
```
