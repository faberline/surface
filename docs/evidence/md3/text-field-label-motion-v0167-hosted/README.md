# Surface v0.1.67 hosted label motion verification

The original pull request and branch push both passed. They checked the
same Git source tree `c218faaf7527ec2872fb1ec1097c87667f3bf1f2`.

| Check | Pull request | Branch push |
| --- | --- | --- |
| Original GitHub run | [46109248208](https://github.com/faberline/surface/actions/runs/46109248208/attempts/1) | [46109213750](https://github.com/faberline/surface/actions/runs/46109213750/attempts/1) |
| Rust and WebAssembly | 1,545 passed, 20 ignored, 3 Wasm checks | 1,545 passed, 20 ignored, 3 Wasm checks |
| Browser and bridge suite | 8 cases, 25,350 checks | 8 cases, 25,350 checks |
| Material | 349 steps, including label motion M347–M350 | 349 steps, including label motion M347–M350 |

Chrome used the SwiftShader software renderer in both Linux runs. A second
reader checked the artifact digests, exact job and checkout identities,
input method receipts and 15 original label motion PNGs per run.

[Measured output and replay](proof/) · [macOS source and test evidence](https://github.com/faberline/surface/tree/d74ad36e0c588a4a43fb3a43f11d1ce5fe7918f4/docs/evidence/md3/text-field-label-motion-v0167-candidate)

Run the packaged readback with Python 3.12 and Pillow from this directory.

```sh
python3 -E -B proof/verify-hosted.py --verify-package proof \
  --package-sha fec6926daa5956aa5551a8ec8d8ef48a4ce289dff66d97edc9b2cef6b5726beb
```
