# Surface v0.1.43 stock headless Search proof

This gate used the exact 557-file source export for the repaired v0.1.43 candidate. The incoming manifest SHA-256 is `82d29caaad575703758710ec4e068716dee32a7c099f277e21930827d115ad23`. The stock bundle manifest is `fb08e7575e9a0b50ad6931cfa814b5917736a4c531fb444f5642bc5054cd22e1`. It seals four runner or host binaries and three Web bundles. All source and bundle files passed checks before and after the run.

The stock Mac fixture used Chrome/158.0.8127.37 with Apple Metal WebGPU. It ran `smoke`, `ime`, `material`, `bridge-web`, and `devtools-headless`. All five cases passed with 17,295 checks. I1 through I20 ran with 77 IME checks. The Material case ran all 254 scheduled steps through M255. The stock suite omits the M23 failure-dump step.

Both cancellation reports passed. Select All survived a Canvas resize after cancellation. A separate backward selection was restored before repaint. Trusted Right and insertion produced `台文X` while repaint was held and kept it after repaint. Search M248–M255 passed. The verifier decoded all 46 Search PNGs and checked selected caret, palette, and motion pixels independently.

The reported Showcase and Material page errors were zero. CoreGraphics found zero visible windows in 129,419 Chrome and 15 Gallery process observations. The runner also checked that its owned Chrome and Surface processes exited. Native screenshot bytes matched through the API, CLI, and MCP. The run used browser-simulated composition and headless platform reports.

Run `shasum -a 256 -c SHA256SUMS` from this folder to check the 186 published proof files. To replay, reconstruct the [sealed candidate source](../v0143-cancel-fixed-source/). Copy the replay folder and use a fresh private stage. Run the included `replay/stock-build.sh` with the source path, manifest path, and private `stock.*` stage under the replay directory. Then run `replay/observe-headless.py` with that stage. `replay/seal-web-proof.py` requires Pillow for the full Search image decode. The exact build and runner records are in `inputs` and `run`.

Local paths in logs, metadata and replay setup were replaced. The original build seals in `inputs` record the executed inputs. The archive checksum covers these published copies.
