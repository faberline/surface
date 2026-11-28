# Surface q83 adaptive navigation Badge runtime evidence

Source commit 758b0af489e659fb871bdf2b238340c4f1ff479c, tree af4ac05e6bbffb92fbad3d89d9c5da3d58675ab5, manifest SHA-256 41ed9aef1a8e6e809407edd675d0629a5c266e818c1168fcc8940123e0cc0d34; 717 files.

Rust passed 1633 tests; 20 ignored. Metal passed 18 GPU tests (16 offscreen and 2 text-selection). Native replay verified 12 captures, 22 zero-window checks, and 10 reaped processes.

Headless Chrome passed 389 ordered Material steps and 32700 checks. The verifier read back four original PNGs, responsive navigation geometry, accessible destination names, badge pixels, the guarded Chrome launch, zero visible owned windows, and process cleanup.

The build and binary manifests are retained as original hash receipts. Compiled executables and Web binaries are omitted. The exact source bundle is included and checked by a detached local checkout.

This evidence covers headless rendering and accessibility metadata. It does not claim VoiceOver speech or physical-display behavior.
