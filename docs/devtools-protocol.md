# Devtools protocol wire types

The crate `surface-devtools-protocol` holds the wire types of the Surface
devtools protocol. It is data and serde only. It depends on `serde` and
`serde_json`, and on no other crate. The executor in `surface-app` and the
native transport in `surface-platform` use it, both behind `devtools`. The design is
in [observability.md](observability.md) and
[decision 0002](decisions/0002-devtools-protocol.md).

## Framing

Newline-delimited JSON. One value per line.

- Request: `{"v":1,"id":7,"method":"Trace.read","params":{}}`
- Reply, ok: `{"id":7,"ok":true,"result":{}}`
- Reply, error: `{"id":7,"ok":false,"error":{"code":"unknown_method","message":"..."}}`
- Event: `{"event":"Trace.record","seq":3,"params":{}}`

`params` is optional on input and defaults to `{}`.

A reply holds `result` when `ok` is true and `error` when `ok` is false.
A line with both, or with neither, is rejected on parse.

Request ids are integers from 0 to 2^53 - 1 (`MAX_ID`), so a JavaScript
client holds them exactly. The crate does not check the cap. Id 0 is a legal
id; it is also the id of the error reply when the request id cannot be read.

## Version

One integer major version, `VERSION = 1`. A request with another `v` gets
`unsupported_version`. Unknown fields in the envelope and in `params` are
ignored. An unknown method gets `unknown_method`.

`parse_request(line)` applies these rules in this order:

1. Not JSON, no integer `id`, no integer `v`, or a wrong envelope shape:
   `invalid_params`. When the id cannot be read, the reply id is 0.
2. `v` is not `VERSION`: `unsupported_version`.
3. The method is not in `METHODS`: `unknown_method`.

`METHODS` lists the methods the crate knows. Later issues add to it.

## Error codes

`unsupported_version`, `unknown_method`, `invalid_params`, `unauthorized`,
`not_ready`, `missing_node`, `disabled`, `outside_modal`, `too_large`,
`timeout`, `action_failed`, `host_timeout`, `host_closed`, `render_failed`.

## Capabilities

`Devtools.capabilities` returns `version`, `domains`, `methods`, `limits`
(`request_bytes` 1 MiB, `response_bytes` 16 MiB, `ring_size` 4096) and
`schemas` (`surface.dom.v1`, `surface.trace.v1`, `surface.paint.v1`).
`Capabilities::current()` describes the crate. A host may narrow it.

## Trace record (`surface.trace.v1`)

`TraceRecord` has the fields `seq`, `frame`, `kind`, `t_ms`, `dur_ms`, `data`,
in this order. `kind` is one of `Input`, `Message`, `Update`, `Rebuild`,
`Layout`, `Paint`, `Present`, `Error`. `t_ms` and `dur_ms` are finite numbers. The recorder must never emit NaN or
infinity: `serde_json` writes them as `null`, and the read side rejects
`null` for these fields, so one bad value would make the whole line
unreadable. The same rule holds in the recorder (#125). The trace recorder (#125) exports the
same lines; its fixture must be checked against this one.

## Input mirror

`InputEvent`, `EditKey` and `ImeEvent` mirror the input events of
`surface_app::Event`. They are tagged with `"type"` in snake case. `Text(String)`
and `Ime(ImeEvent)` become struct variants (`{"type":"text","text":"a"}`),
because an internally tagged enum cannot hold a bare string. The crate has no
conversion to `Event`; that needs `surface-app` and comes with the executor.

## Native transport

A native app built with the `devtools` feature serves the protocol on a
loopback socket when `SURFACE_DEVTOOLS_SESSION=<path>` is set (#131). The
code is `crates/surface-platform/src/devtools/native_protocol.rs`. It is
native only; there is no web transport yet. Without the feature nothing
listens and `surface-platform` does not depend on this crate.

- The listener binds `127.0.0.1:0` only. Each start makes a new 32-byte
  random token.
- The session file is created with `create_new` and mode 0600. An existing
  file is never overwritten: the app fails to start and the file is kept.
  On a normal exit the app removes the file if it still holds its own token
  and pid. It holds one JSON object:
  `{"protocol":"surface-devtools","version":1,"endpoint":"127.0.0.1:<port>","token":"<64 hex>","pid":<pid>,"application":"<title>"}`.
- The jet bridge (`SURFACE_BRIDGE_SESSION`) is separate. Both may be set at
  once; each has its own listener, token and session file.
- A connection carries many requests. Each request is one line of the
  envelope above with one more field, `"token"`. A wrong or missing token
  gets `unauthorized` and the connection closes. The token is checked before
  anything else runs.
- Requests on one connection run in order. Each runs on the window event
  loop. The host runs its own methods and forwards other requests to
  `Runtime::execute_devtools`. The host draws first, so queries
  see current geometry, and draws again after an `Action.*` method. Before
  the renderer exists the reply is `not_ready`. The host waits at most 5 s
  for the event loop, then replies `host_timeout`.
- Request cap 1 MiB per line. A longer line gets `too_large` with id 0, then
  the connection closes, because the stream cannot be resynced.
- Response cap 16 MiB per line. A larger reply is replaced by `too_large`
  with the request id, and the connection stays usable.
- Not JSON or no integer id: `invalid_params` (id 0 when the id cannot be
  read). Version and method checks are the executor's.
- A connection with no complete line for 60 s is closed. At most 8
  connections are open at once.

Example with a shell (`jq` reads the session file):

```sh
token=$(jq -r .token "$SURFACE_DEVTOOLS_SESSION")
port=$(jq -r .endpoint "$SURFACE_DEVTOOLS_SESSION" | cut -d: -f2)
printf '{"v":1,"id":1,"method":"Devtools.capabilities","token":"%s"}\n' "$token" | nc 127.0.0.1 "$port"
```

The e2e suite `devtools-native` (`cargo xtask web e2e devtools-native`)
starts the native gallery with both session files and checks this over raw
TCP.

## Native screenshots

`Page.screenshot` returns the drawing commands from the frame the native
host just presented, rendered as PNG. This keeps the capture at the same
logical layout as the window. Check `Devtools.capabilities` for support.

```json
{"v":1,"id":8,"method":"Page.screenshot","params":{"max_side":1200},"token":"<session token>"}
```

`scale` is output pixels per logical unit. It defaults to the window
scale. `max_side` limits the longer edge. It defaults to 4096 and accepts
1 through 8192. The result contains `format` (`"png"`), `width`,
`height`, the effective `scale`, and `data_base64`.

A window without a drawable frame returns `not_ready`. A failed render
or PNG encoding returns `render_failed`. If the PNG would exceed the
response cap, the reply uses `too_large` and suggests a smaller
`max_side`. The application returns the PNG in the reply. It writes no
screenshot file.

## Client: `surface-devtools`

The crate `surface-devtools` (`crates/surface-devtools`, #132) is a CLI and
an MCP server in one binary. It uses the protocol crate and small
dependencies for JSON, base64 and file output. It speaks the native
transport above.

```sh
cap cargo build --locked -p surface-devtools
export SURFACE_DEVTOOLS_SESSION=/path/to/session.json   # or --session <file>
surface-devtools capabilities
surface-devtools dom snapshot
surface-devtools dom query --role Button --name Save
surface-devtools action activate <id>
surface-devtools action fill <id> <text>
surface-devtools action set-value <id> <number>
surface-devtools action key tab [--extend] [--by-word]
surface-devtools page screenshot --max-side 1200 --out surface.png
surface-devtools mcp
```

- A screenshot command with `--out` writes a private PNG to a new
  file and returns the metadata. If the file exists, the command fails.
  Omit `--out` to print the JSON result with `data_base64`.
- Output is the JSON `result` on stdout. Exit codes: 0 ok; 1 protocol error,
  with the error code on stderr; 2 usage error; 3 session, app or
  connection failure.
- Stale session: a pid that is not running, or a refused connection, gives
  a message that names the session file and says the app is not running.
  The client never sends a request again. A request that may have reached
  the app and got no reply is reported, not retried.
- The token goes only to a loopback address from the session file. Any
  other endpoint is refused before a connection is made.
- `mcp` serves MCP over stdio (hand-written JSON-RPC: `initialize`,
  `tools/list`, `tools/call`, one JSON message per line). There is one tool
  per method in `Devtools.capabilities` of the connected app, named in snake
  case (`DOM.snapshot` is `dom_snapshot`, `Action.setValue` is
  `action_set_value`). Descriptions come from the capabilities. A tool call
  is forwarded only when its method is listed there. A protocol error is a
  tool result with `isError: true`.
- The MCP tool `page_screenshot` accepts optional `scale` and `max_side`.
  It returns an `image/png` content block followed by a text block with
  the image metadata.
- The command layer uses a small `Connection` trait. Native TCP is the only
  implementation; a web transport is out of scope until decision 0001 D-B
  lifts.

`METHODS` and `parse_request` in the protocol crate do not yet list
`DOM.snapshot`, `DOM.query` and `Action.*`, which the executor serves. The
client does not use that list: it works from `Devtools.capabilities` at run
time.

The e2e suite `devtools-native` drives the app through this binary and
checks raw, CLI and MCP screenshots. `cargo xtask web e2e` builds it first.
