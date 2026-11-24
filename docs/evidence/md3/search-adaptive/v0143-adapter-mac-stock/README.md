# Surface v0.1.43 stock headless Search proof

This gate used the exact 557-file source export for the repaired v0.1.43 candidate. The incoming manifest SHA-256 is `386aec95d1ad1b3fe7dfb08393eb678055eebd895cd8b1a83ae445677987fd9f`. The stock bundle manifest is `9b278d6f0ff1d0e72b6f09f87bb2d3cae50f4431a65a04658044e5c0123267c7`. The exact runner was rebuilt. Three unchanged companion binaries and the three unchanged Web products were reused byte for byte. `inputs/payload-reuse.json` and the two prior manifests prove the only changed source file was the runner test. All source and bundle files passed checks before and after the run.

The stock Mac fixture used Chrome/158.0.8127.37 with Apple Metal WebGPU. It ran `smoke`, `ime`, `material`, `bridge-web`, and `devtools-headless`. All five cases passed with 17,295 checks. I1 through I20 ran with 77 IME checks. The Material case ran all 254 scheduled steps through M255. The stock suite omits the M23 failure-dump step.

Both cancellation reports passed. Select All survived a Canvas resize after cancellation. A separate backward selection was restored before repaint. Trusted Right and insertion produced `台文X` while repaint was held and kept it after repaint. Search M248–M255 passed. The verifier decoded all 46 Search PNGs and checked selected caret, palette, and motion pixels independently.

The reported Showcase and Material page errors were zero. CoreGraphics found zero visible windows in 128,879 Chrome and 6 Gallery process observations. The runner also checked that its owned Chrome and Surface processes exited. Native screenshot bytes matched through the API, CLI, and MCP. The run used browser-simulated composition and headless platform reports.

Run `shasum -a 256 -c SHA256SUMS` from this folder to check the 193 published proof files. To replay, use the source and manifest under the same checksum. Run the included `replay/stock-build.sh` with the source path, manifest path, private `stock.*` stage under the replay directory, and prior sealed stock stage as the fourth argument. Copy `inputs/adapter-check.patch` into the replay directory before running the build. Then run `replay/observe-headless.py` with that stage. `replay/seal-web-proof.py` requires Pillow for the full Search image decode. The exact build and runner records are in `inputs` and `run`.

Original pinned binaries and full scratch run are under `/private/tmp/surface-md3-v0143-adapter-mac-stock.bEcN38Qs/stock.ue0iqC1e`.
