# Observability

Goal: make a running Surface app as observable as a web page under browser
DevTools. AI agents and automated tests come first. The interface is a CLI
and an MCP server. A human inspector UI is out of scope for now.

The decision is in [0002-devtools-protocol](decisions/0002-devtools-protocol.md).
Items are named O1 to O18. They are not GitHub issue numbers.
Line numbers are from `origin/main` at `aff5844`.

## Current state

### What exists

- `DomExport` v1, schema `surface.dom.v1`
  (`crates/surface-app/src/snapshot.rs:10-68`).
- `DisplayList` is Serde (`crates/surface/src/paint.rs:157`).
- `ComputedDom::hit_test` and `changed_since`
  (`crates/surface/src/layout.rs:103-127`).
- Seven jet-bridge operations in `Runtime::execute_bridge`
  (`crates/surface-app/src/bridge.rs:14`), pinned at jet-bridge rev
  `a21f934`. Native transport: loopback TCP, 32-byte token, session file
  with mode 0600, created once (`crates/surface-platform/src/devtools/native.rs:42-66`).
  Web: a frozen `window.__jet_bridge` (`devtools/web.rs:49-58`).
- Feature `devtools` on `surface-platform`, off by default.

### What is missing

- A trace of input, messages, updates, rebuilds, layout, paint and present.
- Frame timing.
- Any `log` or `tracing` dependency.
- A native screenshot. Offscreen readback exists only in tests
  (`crates/surface-wgpu/tests/it.rs:336-374`).
- A map from paint ops to nodes. `PaintOp` carries no node id
  (`crates/surface/src/paint.rs:18-60`).
- Pointer, scroll, drag and IME injection.
- Waiting (idle, node, text).
- Record and replay.
- Limits in the native transport: one request per connection, a fixed 5 s
  host timeout, a 1 MiB request cap, no response cap
  (`devtools/native.rs:72-90`, `:112`). A push stream does not fit it as is.
- `UserEvent::Bridge` and `devtools::Reply` are typed on
  `jet_bridge_protocol` (`surface-platform/src/lib.rs:143-150`,
  `devtools.rs:11-33`).

### Compared with browser DevTools

| Browser DevTools | Surface today | Planned |
| --- | --- | --- |
| Elements (DOM, CSS) | `surface.dom.v1`, `query` | `DOM.*` and presented style (O6, O11) |
| Console | none | `Log.read` (O9, O16) |
| Performance / Tracing | none | `Trace.*` (O1, O5, O9) |
| Screenshot | tests only | `Page.screenshot` (O2, O10) |
| Input emulation | `activate`, `fill`, `set_value`, `key` | `Input.*` (O12) |
| Accessibility tree | none | after #53 and #54 (O18) |
| Overlay (inspector UI) | none | deferred |
| No CDP twin | none | `Wait.*`, `Paint.displayList`, `Layout.*` |

## Design

### Where things live

| Piece | Crate |
| --- | --- |
| Wire types: envelope, errors, capabilities, trace records, input mirror | New `surface-devtools-protocol`. Depends on `serde` and `serde_json` only. Does not depend on `surface`. |
| Trace recorder and command executor | `surface-app`, feature `devtools` (`src/trace.rs`, `src/devtools.rs`). The host passes the clock as `f64` ms, like `set_time`. No `Instant`. |
| Present and GPU spans, transports, PNG encoding | `surface-platform`, feature `devtools`. `png` becomes an optional dependency. |
| Offscreen capture | `surface-wgpu`: `Renderer::capture(&DisplayList, w, h, scale)` returns RGBA8. The test readback code moves into the library. No `png` in the library. |
| Paint attribution | `surface`: `DisplayList::compile_attributed` returns the list plus a parallel owner list of node ids. `PaintOp` and its JSON stay unchanged. |
| CLI and MCP client | New binary crate `surface-devtools`. Not xtask, because external apps need it. |

Trace hooks sit in `Runtime` (`crates/surface-app/src/lib.rs`: `flush`,
`rebuild`, `render`, `dispatch`), not in `Host`. `Host::input` is not the
only input path. Web text input calls `Runtime::dispatch` directly
(`crates/surface-platform/src/web_input.rs:28`). Cut and reduced-motion
also bypass it (`keyboard.rs:114`, `motion.rs:65`).

### Compat adapter

`Runtime::execute_bridge` becomes a thin map from the seven jet commands to
the new executor. `crates/surface-app/tests/bridge.rs` and the suites
`bridge-native`, `bridge-native-compact` and `bridge-web` stay unchanged.
They are the adapter's acceptance test. Both session files coexist:
`SURFACE_BRIDGE_SESSION` (jet) and `SURFACE_DEVTOOLS_SESSION` (new).
The adapter stays until migration step 23 is decided.

### Protocol shape

- Framing: newline-delimited JSON.
  - Request: `{"v":1,"id":7,"method":"Trace.read","params":{}}`
  - Reply: `{"id":7,"ok":true,"result":{}}` or
    `{"id":7,"ok":false,"error":{"code":"","message":""}}`
  - Event: `{"event":"Trace.record","seq":n,"params":{}}`
- One integer major version. `Devtools.capabilities` returns domains,
  methods, limits and schema names (`surface.dom.v1`, `surface.trace.v1`,
  `surface.paint.v1`). Params accept unknown fields. An unknown method
  returns `unknown_method`.
- Streaming starts as cursor polling: `Trace.read {after_seq, limit}`.
  Push subscription comes later (O17).
- Transport is native first. It reuses the loopback listener, the token and
  the 0600 session file pattern.
- Security: feature off by default. Native also needs an environment
  variable. Binds 127.0.0.1 only. The token is checked on every connection.
- Caps: request 1 MiB. Response 16 MiB, then `too_large` with paging. Ring
  4096 records by default. A maximum screenshot size.
- Client: `surface-devtools <domain> <op>` and `surface-devtools mcp` over
  stdio.

### Domains

1. **Trace.** Records: `Input`, `Message`, `Update`, `Rebuild`, `Layout`,
   `Paint`, `Present`, `Error`. Each has a frame id, a sequence number, a
   start time and a duration. Fixed-capacity ring. Export as JSON lines.
   Message logging uses the default method
   `Application::describe(&self, &Self::Message) -> Option<String>`,
   returning `None` by default. A message record also carries its ordinal
   and its source binding (stable id and handler kind).
2. **Screenshot.** `Renderer::capture` on a second texture of the surface
   format, then PNG. `Renderer::new` binds the pipeline to one texture
   format (`crates/surface-wgpu/src/lib.rs:136`). The host prefers a non-sRGB
   format (`surface-platform/src/lib.rs:104-112`). Capture must use the same
   format and swizzle BGRA.
3. **Paint.** Ops plus owner stable ids as `surface.paint.v1`.
4. **Layout.** Text line boxes from `TextMeasurer::text_layout_styled`. The
   box model waits for migration step 11.
5. **Input.** A Serde mirror of `Event` in the protocol crate. Requests
   convert to `Event` and go through `Runtime::dispatch`.
6. **Wait.** `idle` means the queue is empty, `needs_frame()` is false,
   layout is present and no redraw is pending. Also `node` and `text`. Each
   request has a timeout.
7. **Log.** The `log` facade. Records go into the ring.
8. **Replay.** Trace input and time are replayed through `set_time` and a
   shared `FixedMetrics` measurer.
9. **Accessibility.** A view of the semantic tree, after #53 and #54.

### Overhead rules

- Feature off: no fields in `Runtime`, no hook code, no protocol crate in
  the dependency graph. `scripts/check.sh` uses `--all-features`, so this
  needs its own step: a no-feature `cargo check -p surface-platform` and an
  assertion that `cargo tree` has no `surface-devtools-protocol`.
- Feature on, nothing attached: one `Option` branch per hook. No
  allocation.
- Recorder attached: a preallocated ring. Strings come only from
  `describe` and errors.
- Proof: a counting global allocator in its own test binary,
  `crates/surface-app/tests/trace_alloc.rs`. 100 dispatch and render cycles
  with no recorder allocate exactly as often as the baseline. Negative
  control: a test-only allocating record must fail the test.

### Verification

- Every item: `sh scripts/check.sh`. It includes the wasm32 checks and
  `consumer_contract`.
- O2 and O10: also `scripts/check-gpu.sh` on a real macOS adapter, with the
  PNG saved. A skipped GPU test is never evidence.
- O7 to O10, O13, O17: `cargo xtask web e2e` suites, and the three bridge
  suites stay green.
- Headless only: O1, O3, O4, O6, O11, O12, O15.

## Items

"Host" means the item edits `crates/surface-platform/src/lib.rs`. Host items
run one at a time.

| # | Title | Scope | Depends on | Protocol | Host |
| --- | --- | --- | --- | --- | --- |
| O1 | Trace recorder | Ring, records, injected clock, JSONL, `describe` | none | no | |
| O2 | `Renderer::capture` | Offscreen readback, format swizzle | none | no | |
| O3 | Paint attribution | `compile_attributed`, `Runtime::export_paint` | O1 | no | |
| O4 | Protocol crate | Envelope, errors, capabilities, records | none | n/a | |
| O5 | Host trace to file | Present span, errors, `SURFACE_TRACE=path` | O1 | no | yes |
| O6 | Executor and jet adapter | `execute_devtools`; `bridge.rs` maps onto it | O4 | yes | |
| O7 | Native transport | Generic `Reply` and `UserEvent`; new session file | O5, O6 | yes | yes |
| O8 | CLI and MCP client | `surface-devtools` binary; e2e suite | O7 | yes | |
| O9 | Trace and Log over protocol | `Trace.start/stop/read` with cursor | O5, O7 | yes | |
| O10 | `Page.screenshot` (native) | Native capture and PNG | O2, O7 | yes | yes |
| O11 | Paint, Layout and DOM extras | Display list, text line boxes, bindings, scopes, presented style | O3, O6 | yes | |
| O12 | Input domain | Pointer, scroll, drag, text, IME | O6 | yes | |
| O13 | Wait domain | idle, node, text; per-request timeout | O7 | yes | yes |
| O14 | Headless driver journey | In-process `Driver`: selectors, actions, waits, evidence bundle | O1, O2, O3, O12 | no | |
| O15 | Replay | Record and replay with `FixedMetrics` | O1, O12 | no | |
| O16 | Log facade | Adopt `log`; records into the ring | O1 | no | |
| O17 | Push subscribe | Persistent connection | O9 | yes | yes |
| O18 | Web transport, web screenshot, accessibility tree | `window.__surface_devtools`, canvas capture, a11y view | D-B lifted; #53, #54 | yes | yes |

Each of O1 to O8 gets a full spec: scope, files, acceptance table, one
negative test, and what is not done. O9 to O18 are tracked and split later.

### Parallel groups

- O1, O2 and O4 can start together. They touch disjoint files.
- O3 follows O1. O6 needs only O4.
- O11, O12, O15 and O16 never touch `Host`.
- O1, O2, O3 and O5 need no protocol.
- Host queue: O5, then O7, then O10, then O13, then O17.

### Relation to #68

Issue #68 is re-split onto O2, O6, O12, O13 and O14:

- Selectors: O6.
- Typed actions: O12. Waits: O13.
- Evidence format: O1, O3 and `DomExport`.
- Offscreen screenshot: O2.
- Headless journey: O14.

## Risks

1. `Host` contention. Five items edit `surface-platform/src/lib.rs`.
2. Protocol drift between the adapter and the new executor. The unchanged
   bridge suites are the guard.
3. Trace hooks inside `Runtime` must not change behavior or goldens. The
   injected clock keeps tests deterministic.
4. Security. A loopback token gives full input control. The feature is off
   by default and must never be on in a release build by accident.
