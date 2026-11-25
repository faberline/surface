# Surface v0.1.68 text field label color candidate proof

Tree 0db939d3f663fdb5b9fa3dd334382937693b409e has 685 source paths and applies the reviewed ten-path patch
on the v0.1.67 tree c218faaf7527ec2872fb1ec1097c87667f3bf1f2. The original patch and Rust log bytes
are stored in gzip files.

- The exact workspace passed 1,575 Rust tests and three Wasm checks.
  The actual Metal gate ran 18 tests with 21 original GPU PNGs.
- The exact native replay returned 14 PNGs from two headless repetitions
  with 24 window samples and zero visible windows. All endpoint PNGs
  match the v0.1.67 and scoped R2 originals. Page.screenshot, CLI and MCP matched.
- Scoped R2 ran 1,798 checks from M325 through M350. It keeps 34
  original PNGs, its exact typed receipt, and 314 headless samples
  with zero visible windows or surviving processes.
- Exact stock ran 349 Material steps and 25,276 checks
  in five macOS suites. It reproduced the 34 motion and color captures, keeps
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
python3 -E -B pack-label-color.py --verify-package . --package-sha "$reviewed_sha"
~~~

The archive contains original verified screenshots and recorded binary
pins. The independent command starts no app. The stock bundle, raw
scoped and native binaries, and build caches are build inputs rather
than archive contents. The predecessor archive remains at its
[reviewed v0.1.67 commit](https://github.com/faberline/surface/tree/d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4/docs/evidence/md3/text-field-label-motion-v0167-candidate).
