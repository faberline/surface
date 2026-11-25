# Surface v0.1.67 text field label motion candidate proof

Tree c218faaf7527ec2872fb1ec1097c87667f3bf1f2 has 684 source paths and applies the reviewed 17-path patch
on the v0.1.66 tree 3de0c143c43d35911593cfc95d3ea5931016c308. The original patch and Rust log bytes
are stored in gzip files.

- The exact workspace passed 1,570 Rust tests and three Wasm checks.
  The actual Metal gate ran 18 tests with 21 original GPU PNGs.
- The exact native replay returned 14 PNGs from two headless repetitions
  with 24 window samples and zero visible windows. All endpoint PNGs
  match the v0.1.66 originals. Page.screenshot, CLI and MCP matched.
- Three scoped Chromium trials ran 1,495 checks each. All 15 original
  label motion PNGs have the same bytes in all three runs. The first
  two remain negative cleanup controls. The third recorded 256
  headless samples with zero visible windows or surviving processes.
- Exact stock ran 349 Material steps and 24,973 checks
  in five macOS suites. It reproduced the 15 motion captures, keeps
  the six inactive textbox value cases and all 20 IME steps, and shows
  zero windows and survivors.

The headless traces cover real Chromium accessibility structure and
synthetic composition, alongside real WebGPU rendered pixels. Native
captures here verify reduced-motion endpoints. Live speech and OS
input composition require separate proof.

## Offline replay

Use Python 3.12 with Pillow and the reviewed manifest digest.

~~~sh
shasum -a 256 -c SHA256SUMS
python3 -E -B pack-label-motion.py --verify-package . --package-sha "$reviewed_sha"
~~~

The archive contains original verified screenshots and recorded binary
pins. The independent command starts no app. The stock bundle, raw
scoped and native binaries, and build caches are build inputs rather
than archive contents. The predecessor archive remains at its
[reviewed v0.1.66 commit](https://github.com/faberline/surface/tree/2ce969df988909133af945ae7c3c0534c68351d8/docs/evidence/md3/textbox-values-v0166-candidate).
