# Desktop and WASM application bridge

Surface owns the live UI tree and all state changes. The independent
[Jet bridge](https://github.com/faberline/jet-bridge) owns CLI and MCP tools.
Both hosts accept the same version 1 request format. Surface depends only
on `jet-bridge-protocol`, pinned at
`a21f93425f8f27dfc7ab2bea0c34fdebcc71335f`. Its only dependencies are Serde
and JSON. Surface does not import the browser or MCP server.

## Enable the host

The `surface-platform` feature `devtools` is off by default. Enable it in
your app's dependency, or use either example's `devtools` feature.

For a native gallery:

```sh
cap cargo build --locked -p surface-material-gallery --features devtools
SURFACE_BRIDGE_SESSION=/path/to/new-session.json target/debug/surface-material-gallery
jet-bridge --app-session /path/to/new-session.json ui snapshot
jet-bridge --app-session /path/to/new-session.json mcp
```

The parent directory must exist. The host creates the session file once.
It will not overwrite an existing file. The file holds the loopback
endpoint, app PID, title and random token. On Unix its mode is 0600.
The host removes its own descriptor on normal window shutdown. A forced
process exit can leave a stale descriptor. Keep the file private.

Requests enter the window event loop. No background thread mutates UI
state. The bridge returns the result after that request is handled.
An action timeout does not prove that the action did not run. The client
does not retry submitted actions automatically.

For a browser gallery:

```sh
cargo xtask web build material-gallery --features devtools --out target/web/devtools/material-gallery
cargo xtask web serve material-gallery --dir target/web/devtools/material-gallery
jet-bridge --root /path/to/browser-session bb launch http://127.0.0.1:8080
jet-bridge --root /path/to/browser-session ui snapshot
jet-bridge --root /path/to/browser-session mcp --ui
```

The WASM host installs `window.__jet_bridge` after GPU initialization.
It has `version` and an asynchronous `request` method. It does not
overwrite an existing adapter. Jet's legacy `window.__jet_debug` API
remains separate. HTML pages still use the normal browser tools.

## Operations

The UI MCP mode exposes seven tools:

| Tool | Result or action |
| --- | --- |
| `ui_capabilities` | App name, protocol and supported operations |
| `ui_snapshot` | Current DOM, matching optional layout, focus and editor state |
| `ui_query` | Exact stable ID, Surface role and accessible name filters |
| `ui_activate` | Normal runtime control activation |
| `ui_fill` | Replace a writable field through `Application::update` |
| `ui_set_value` | Normal slider snapping, clamping and update |
| `ui_key` | Navigation, selection, editing and modal keys |

The `ui` CLI command exposes the same operations. For example:

```sh
jet-bridge --app-session /path/to/new-session.json ui query --role TextInput
jet-bridge --app-session /path/to/new-session.json ui activate open-dialog
jet-bridge --app-session /path/to/new-session.json ui fill dialog-name '中文橋接'
jet-bridge --app-session /path/to/new-session.json ui key tab
jet-bridge --app-session /path/to/new-session.json ui activate dialog-save
```

Role names use the exact Surface enum spelling, such as `TextInput`.
Queries can return background nodes, with `in_scope: false` during a
modal. Actions reject those nodes. Missing or removed IDs do not select
another node by index. Disabled and read-only controls return explicit
errors. Rejected requests preserve focus and composition.

Fill changes focus without firing a field's optional press action.
It rejects active composition. Commit or cancel composition first.
Escape cancels composition before it closes a modal. Selection positions
are UTF-8 byte offsets on grapheme boundaries.

Snapshots use [the DOM export contract](dom-export.md). The platform-neutral
runtime can return absent layout after a change. The running host updates
layout before a query. The renderer and editing code remain shared.

The imported MCP contract is `2025-06-18`. The UI mode has seven tools;
the default browser mode retains its 27 tools.

## Runtime verification

`cargo xtask web e2e` builds the native and WASM gallery with
`devtools`, the compact example and jet-bridge at the revision pinned in
`Cargo.toml`. It then runs the bridge suites with the other browser suites:

```sh
cap cargo xtask web e2e bridge-native bridge-native-compact bridge-web
```

`SURFACE_CHROME` selects Chrome; otherwise a system install is used. The
web test serves `target/web/devtools/material-gallery`. `--bridge-web`
selects another built directory and `--jet-bridge` another binary.

The test creates its own window, browser, server and sessions. It checks
actual controlled Chinese input, saved state, slider snapping, disabled
controls, removed nodes, modal scope, Tab loops, restored focus and shared
CLI/MCP state. The native test checks authentication, file permissions and
normal descriptor cleanup. The web test checks WebGPU readiness, PNG
capture and actual browser exit. The suites live in
[`e2e/src/bridge.rs`](../e2e/src/bridge.rs).

`bridge-native-compact` runs the `compact` example, a small native
viewport, as an explicit focus-scrolling regression.
It requires the slider to start clipped. Tab must reach it and its new
geometry must be visible on both hosts. The fixture uses the same gallery
application and changes only its native viewport size.

Local macOS native and Chrome 157 WASM journeys pass. Linux and Windows
native runtime journeys remain unrun. Semantic Chinese fill is not a
test of OS IME candidate windows. Physical native Zhuyin composition
remains unverified. The Material catalog is still incomplete.
