# Paint-only modal retirement and panel opacity

The [569-file base](base-source.sha256) has SHA-256 `cb2c641d89a0d1676234edf559c59c9ed4eb6302cb5d51b21332e2a505681eea`. The [Runtime patch](modal-retirement-opacity.patch) has SHA-256 `b43431d00faeb86e1f28c3f89f89a9d3c3c8a219dbb101df5ec3001fed341251`. The [570-file result](source.sha256) has SHA-256 `9a1378fc2e976c0a8850d3515a5280c21d5537056ea46180e01baf0a86e2cfc8`. The patch applied byte-for-byte in an independent copy. It changes five Runtime and Core paint files and adds one test module. The renderer and imported Core files retain their input hashes.

## Runtime API

- Use `View::retiring_modal_overlay()` or `View::retiring_modal_with_backdrop(backdrop)` on the same panel when dismissal starts. Keep it in its original authored position with its stable IDs. It keeps its place in the modal paint order. Its controls and backdrop immediately give up focus, input and accessibility. Runtime restores focus to the previous scope. The application removes the retained view when its exit is done.
- Use `View::paint_modal_panel_opacity(move || alpha.get())` on the panel before attaching its live or retiring backdrop. The extended render reads the current paint value without rebuilding the view. The panel, its flow content and its own fixed descendants share one isolated group. Scrims and separate modal layers stay outside that group. Each scrim can use its own color alpha.
- An application can reverse retirement by using the active modal builder again with the same stable panel ID. The panel regains its scope. It does not inherit a press from before dismissal.

The legacy display list and paint export retain their original format. Existing modal and visual overlay callers keep their behavior. Core reports exact panel boundaries during paint ordering. Inside the extended compiler, deferred ripple and focus paint cannot cross the boundaries of an opacity group.

## Verification

The [focused Runtime receipt](runtime-focused-retry.log) records 15 passing tests. The new tests cover nested modal paint order, handback of focus and real Runtime pointer events, accessibility exclusion, reversal, grouped opacity around panel and fixed paint, ungrouped scrims, and raster deferral at a group boundary. The opacity callback also changes a frame without replacing its accessibility identity. The existing modal layer and press ripple suites pass unchanged.

The [broader receipt](runtime-check.log) records 462 Core and Runtime tests with zero ignored. It also records scoped strict Clippy, formatting, native and WebAssembly Gallery checking, and provenance verification. No OS window opened during these checks. The Runtime tests assert emitted commands and input behavior. Their result carries no separate GPU or live accessibility-host claim. A Dialog animation controller and widget wiring belong to the next integration.

To reproduce the focused check, reconstruct the [exact opacity base](../opacity-preflight/), apply the patch and run from the patched source. Set the normal offline Cargo environment, the Command Line Tools SDK, two build jobs, and a unique target under `/tmp/cargo-target`.

```bash
cap cargo test --locked -p surface-app --features devtools --test modal_retirement --test modal_layers --test press_ripple
```

The independent reconstruction verified all 570 output hashes. Run `shasum -a 256 -c SHA256SUMS` from this archive.
