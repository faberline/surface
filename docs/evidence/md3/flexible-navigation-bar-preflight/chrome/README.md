# Flexible Navigation Bar headless browser evidence

M270–M273 passed 458 checks on Chrome 158.0.8127.37. WebGPU reported Apple Metal 3 and a non-fallback adapter. The observer sampled 376 Chrome processes and found zero onscreen windows. No Chrome process from the test remained after it exited.

| Step | Checked result |
| --- | --- |
| M270 | Three, four, and five equal cells, 64 dp bar, 56 by 32 dp pill, light and dark WebGPU pixels |
| M271 | Chrome navigation region and disabled semantics, DOM current-page marker, Tab and Shift+Tab without selection |
| M272 | Enter, Space, and real browser touch each activate once. A pointer click on the disabled destination does not activate. |
| M273 | Dark RTL at 320 dp, 200% text, complete two-line label, real wheel scroll with fixed bar and focus, Back restores launcher focus |

The PNGs use device scale 2. The bar occupies the bottom 112 dp in the compact fixture. The last content row moved up 40 dp when the test scrolled. The bar pixels remained byte-identical. The independent check found both lines of the selected label and the visible focus ring. The test requests Reduce Motion to freeze endpoint colors. Its scope is this four-step case on macOS Chrome. The full browser matrix, native runtime, and live VoiceOver are separate checks.

## Check the archived evidence

From this directory, run:

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verify-browser.py run | cmp run/verified-browser.json -
```

The verifier uses Python 3 and Pillow. The log, report, observation, PNGs, and runtime checks in `run/` are the original files.

## Reproduce the focused run

Use a clean copy of the integrated navigation source with the pinned page header and the new stock steps. `flexible-navigation-bar-e2e.patch` registers and implements M270–M273. It applies before the release version bump. `source.sha256` seals the 594-path test candidate. Its version metadata is still 0.1.46. Apply the patch only if the source does not yet contain the new cases.

The original Gallery build predates the last assertions in the e2e fixture. `gallery-build-input.patch` changes only that fixture in a separate source copy. Gallery does not compile it. All Gallery and dependency files match between the two seals. `focused-selector.patch` changes the runner entry point only, to select M270–M273 from the stock test.

On macOS with the normal Surface build tools, Chrome, and cached Cargo dependencies, set `stock` to the absolute path of the source verified against `source.sha256`. Run these commands from the archived evidence directory:

```sh
proof=$(pwd -P)
(cd "$stock" && shasum -a 256 -c "$proof/source.sha256")
build=$(mktemp -d /tmp/surface-flex-nav-proof.XXXXXXXX)
mkdir "$build/gallery-input"
cp -R "$stock/." "$build/gallery-input"
(cd "$build/gallery-input" && patch -p1 -i "$proof/gallery-build-input.patch")
cp build-gallery.sh build-focused.sh prepare-focused.py run-focused.sh observe-menu.py window-observer.py verify-browser.py "$build/"
gallery_sha=$(shasum -a 256 gallery-source.sha256 | cut -d ' ' -f 1)
bash "$build/build-gallery.sh" "$build/gallery-input" "$proof/gallery-source.sha256" "$gallery_sha"
cp source.sha256 "$build/source.sha256"
stock_sha=$(shasum -a 256 source.sha256 | cut -d ' ' -f 1)
python3 -B "$build/prepare-focused.py" "$stock" "$proof/source.sha256" "$stock_sha"
cmp focused-source.sha256 "$build/focused-source.sha256"
cmp focused-selector.patch "$build/focused-selector.patch"
bash "$build/build-focused.sh"
python3 -B "$build/observe-menu.py"
```

Scripts use `cap`, two Cargo jobs, the Command Line Tools SDK, offline Cargo, a private target under `/tmp/cargo-target`, no incremental compilation, and no debug symbols. `SURFACE_CARGO_HOME` and `SURFACE_WASM_OPT` can override tool locations. Use `verify-browser.py` on the new `run.*` folder to check the replay. Browser builds and hardware can change exact PNG and binary hashes. The manifests in this archive record the exact original inputs and binaries.

The selected source predates cleanup of one unused test helper. The stock runner compile check finished with that warning. See `stock-e2e-check.log`. The release candidate removes the helper and runs the full stock matrix separately.
