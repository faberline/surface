# Web build

Surface builds, packages and serves its browser apps itself. It needs no
Jet, Trunk or Node build step. The tasks live in [`xtask`](../xtask) and
run through the `cargo xtask` alias in [`.cargo/config.toml`](../.cargo/config.toml).

```sh
cargo xtask web build showcase           # release package
cargo xtask web build showcase --dev     # dev package, no wasm-opt
cargo xtask web dev material-gallery     # watch, rebuild, reload
cargo xtask web serve showcase           # serve the release package
cargo xtask web apps                     # list the apps
cargo xtask web e2e                      # build inputs, run browser and bridge tests
```

`web e2e` is described in [verification](verification.md#re-run-the-browser-tests).

## Requirements

- rustup's Rust with `wasm32-unknown-unknown`. Put `~/.cargo/bin` before
  other Rust installs, such as Homebrew's, in `PATH`. The build checks the
  target first and names the fix when it is missing.
- The `wasm-bindgen` CLI at the exact `wasm-bindgen` version in
  `Cargo.lock` (now 0.2.129):
  `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
  A different version fails the build. It is never a warning, because the
  generated JS glue only matches its own crate version.
- `wasm-opt` from Binaryen for release builds (`brew install binaryen`).
  A release build fails when it is missing. Use `--dev` for a build
  without it. `SURFACE_WASM_BINDGEN` and `SURFACE_WASM_OPT` select other
  binaries.

## Release build

`cargo xtask web build <app>` (or `--release`) does this:

1. `cargo build --locked --target wasm32-unknown-unknown --lib` with the
   `wasm-release` profile: `opt-level = "z"`, fat LTO,
   `codegen-units = 1`, `panic = "abort"`. Native `release` is unchanged.
2. `wasm-bindgen --target web --no-typescript`.
3. `wasm-opt -Oz` with the WASM features that current Rust enables by
   default. The exact arguments are recorded in the manifest.
4. Content-hashed names: `<lib>-<16 hex>.js` and `<lib>_bg-<16 hex>.wasm`.
   The JS glue is rewritten to load the hashed WASM. The build fails if
   the glue does not reference the WASM exactly once.
5. `index.html` from the app's `web/index.html` template. The template
   must contain `%SURFACE_JS%` exactly once, as the module import URL. The
   build adds `modulepreload` for the module and a `preload` for the WASM.
6. `.br` (quality 11) and `.gz` (level 9) sidecars for every file.
   `--no-compress` skips them.
7. `surface-web.json`: app, profile, features, rustc, wasm-bindgen and
   wasm-opt versions and arguments, the entry, JS and WASM names, and the
   sha256, size, sidecar sizes and `immutable` flag of each file.

Output goes to `target/web/<release|dev>/<app>/`. `--out` selects another
directory. Every step writes to a staging directory. The output is
replaced only when all steps succeed, so a failed build leaves the
previous output in place.

## Dev loop

`cargo xtask web dev <app>` builds with the dev profile and serves it at
`http://127.0.0.1:8080/` (`--host`, `--port`). It polls the workspace
manifests, `crates/` and the app directory. After a change it rebuilds.
After a good rebuild the page reloads. After a failed rebuild the server
keeps the last good build and the page shows the error at the bottom. The
reload client uses server-sent events on `/__surface/events`. It is only
added to pages served by `web dev`, never to built files.

## Serving

`cargo xtask web serve <app>` serves a built package. It uses the
headers a production static host should copy:

| File | Headers |
| --- | --- |
| Hashed JS and WASM | `Cache-Control: public, max-age=31536000, immutable` |
| `index.html` and other files | `Cache-Control: no-cache` and an sha256 `ETag` |
| `.wasm` | `Content-Type: application/wasm` |
| Any listed file | `Content-Encoding: br` or `gzip` from the sidecar, chosen by `Accept-Encoding`, with `Vary: Accept-Encoding` |

`--isolate` adds `Cross-Origin-Opener-Policy: same-origin`,
`Cross-Origin-Embedder-Policy: require-corp` and
`Cross-Origin-Resource-Policy: same-origin`. Surface does not need them
today. They are for apps that use `SharedArrayBuffer` or WASM threads.
The server refuses `..`, hidden paths and direct sidecar requests. It is a
local tool, not a production server.

Browser tests serve `target/web/release/<app>`. `SURFACE_WEB_PROFILE=dev`
selects a `--dev` build. `SURFACE_WEB_DIR` selects any built directory.

## Why `cargo xtask`

| Option | For | Against |
| --- | --- | --- |
| Trunk | Watch, reload, hashing and `wasm-opt` built in. | Another tool to install and pin. It fetches its own wasm-bindgen and wasm-opt, so a version mismatch does not fail clearly. No Surface manifest or precompressed sidecars. Build rules live in `data-trunk` HTML attributes, not tested Rust. |
| `cargo xtask` | Nothing extra to install. Locked and tested with the repository. Clippy and `cargo test` cover it. | Only works inside this repository. |
| A `surface-cli` crate | Usable by apps outside this repository. | No outside app needs it yet. It would add a published binary and a stable CLI contract too early. |

Surface uses `cargo xtask`. The build, manifest, serve and dev code are
plain modules, so they can move into a `surface-cli` crate when an outside
app needs them.

## Sizes

Measured on 2026-11-06 with rustc 1.96.1, wasm-bindgen 0.2.129 and
wasm-opt 133, from each `surface-web.json`. The old path is the debug
`wasm-bindgen` output of the removed `scripts/build-web.sh` and
`scripts/build-gallery.sh`. Sizes are in bytes.

| App | Build | WASM raw | WASM br | WASM gzip | JS raw | JS br |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| showcase | old debug | 26,504,183 | 12,998,622 | 16,282,632 | about 110 KB | — |
| showcase | `web build --release` | 18,576,407 | 11,819,038 | 14,501,667 | 94,263 | 13,668 |
| showcase | change | −29.9% | −9.1% | −10.9% | | |
| material-gallery | old debug | 26,640,691 | 13,018,187 | 16,313,745 | about 110 KB | — |
| material-gallery | `web build --release` | 18,606,878 | 11,828,791 | 14,512,506 | 93,414 | 13,493 |
| material-gallery | change | −30.2% | −9.1% | −11.0% | | |

The embedded font is 16,435,884 bytes, about 88% of each release WASM.
It is already compressed CFF data, so brotli and gzip save little on it.
The code itself shrinks much more than these totals show. Only a smaller
font changes the download much.

## Limits

A successful build does not prove browser or GPU behavior. Run the browser
tests for that. Most of the WASM is the embedded Noto Sans CJK TC font
(about 16 MB). Subsetting it is tracked in #59. Browsers without WebGPU
show an error; a WebGL2 fallback is tracked in #60.
