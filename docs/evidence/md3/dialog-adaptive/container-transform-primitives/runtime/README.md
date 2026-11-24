# Modal panel transform in Surface Runtime

The [574-file Core/WGPU input](base-source.sha256) has SHA-256 `2273a9fad52bfbe7167d5e206ae87fe62791ac892861581553ca60718d0a68ef`. The [Runtime patch](modal-transform-runtime.patch) has SHA-256 `235de31951eb74042adc88c774d2cef14c6d7add71da9a40bc985e60b00283f2`. The [574-file result](source.sha256) has SHA-256 `d7c52221f56b056227dc2911336ce2c70344dce2ab36d5362c7f5bc30bef90ab`. The patch applied byte-for-byte in an independent copy. The six modified paths cover Runtime, Core compilation, tests, and the original Core file provenance note. The renderer, Material and Gallery source retain their input hashes.

## Caller contract

Attach `View::paint_modal_panel_transform(move || transform.get())` to the explicitly layered panel before adding a backdrop. The callback returns `Option<surface::PaintTransform>`. The extended render reads it each frame and applies it to the complete panel range, including its fixed descendants. The backdrop and independent nested layers retain their own paint. A panel that also uses `paint_modal_panel_opacity` gets one transform group with the two opacities multiplied.

A missing or invalid transform leaves the panel geometry unchanged. Independent opacity still applies. When the source equals the destination, the radius is zero, and both source and clip cover the viewport, the panel uses ordinary geometry paint. That path still applies any effective opacity. A smaller source can crop fixed descendants even when it equals the destination. That case keeps the transform group.

Transform paint leaves layout, hit testing, keyboard focus and accessibility unchanged. Retiring panels keep their paint position as they move, while input and focus immediately return to the lower scope. Legacy rendering keeps its existing output and never invokes the transform painter. The Core opacity range API is still available.

## Verification

The [focused receipt](runtime-transform-focused.log) records 17 passing tests. The transform tests compare complete display lists at identity and `None`, verify invalid fallback and an intentional same-size crop, then check the flat outer and inner transform groups with their expected alpha products. The tests verify unchanged layout and accessibility target identity, separate scrims, and removal of the retiring panel from input and accessibility. They also confirm a held press from the old scope is rejected and a new press reaches the lower scope.

The [broader command receipt](runtime-transform-check.log) records 465 passing Core and Runtime tests in 74 groups with zero ignored. It also records formatting, strict Clippy, Core and Runtime tests, Gallery native and WebAssembly compile checks, and provenance verification. This lane never opens a window. The Runtime tests verify generated paint commands and scope behavior. The underlying renderer has a separate Metal proof. The Material controller and live Full-screen Dialog motions belong to downstream integration.

```bash
cap cargo test --locked -p surface-app --features devtools --test modal_retirement --test modal_layers --test press_ripple
```

Use the configured Command Line Tools SDK, offline Cargo, two build jobs and a unique target under `/tmp/cargo-target`.
