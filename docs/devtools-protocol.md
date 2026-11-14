# Devtools protocol wire types

The crate `surface-devtools-protocol` holds the wire types of the Surface
devtools protocol. It is data and serde only. It depends on `serde` and
`serde_json`, and on no other crate. No other crate uses it yet. The design is
in [observability.md](observability.md) and
[decision 0002](decisions/0002-devtools-protocol.md).

## Framing

Newline-delimited JSON. One value per line.

- Request: `{"v":1,"id":7,"method":"Trace.read","params":{}}`
- Reply, ok: `{"id":7,"ok":true,"result":{}}`
- Reply, error: `{"id":7,"ok":false,"error":{"code":"unknown_method","message":"..."}}`
- Event: `{"event":"Trace.record","seq":3,"params":{}}`

`params` is optional on input and defaults to `{}`.

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
`Layout`, `Paint`, `Present`, `Error`. The trace recorder (#125) exports the
same lines; its fixture must be checked against this one.

## Input mirror

`InputEvent`, `EditKey` and `ImeEvent` mirror the input events of
`surface_app::Event`. They are tagged with `"type"` in snake case. `Text(String)`
and `Ime(ImeEvent)` become struct variants (`{"type":"text","text":"a"}`),
because an internally tagged enum cannot hold a bare string. The crate has no
conversion to `Event`; that needs `surface-app` and comes with the executor.
