# Trace recorder (`surface.trace.v1`)

Item O1 of [observability.md](observability.md). Issue #125.

`Runtime` can record a bounded, ordered trace of what it did for each
frame. The trace is plain Rust data with a JSON-lines export. It is for AI
agents and tests. There is no protocol, transport or host change yet.

## Feature

The recorder is behind the `surface-app` feature `devtools`, off by
default. It adds no dependency.

- Feature off: `Runtime` has no trace field and no hook code.
- Feature on, no recorder attached: each hook is one `Option` branch. It
  does not allocate.
- Recorder attached: the ring is allocated when the recorder is created.
  Writing a record does not allocate, except for the `describe` text and
  error messages.

`crates/surface-app/tests/trace_alloc.rs` proves the last two rules with a
counting global allocator. 100 click and render cycles allocate exactly
`CYCLE_ALLOCS` times, with and without the feature, and with a recorder
attached.

## API

```rust
use surface_app::trace::Recorder;

// The host supplies the clock in milliseconds. surface-app never reads time.
let recorder = Recorder::new(Box::new(move || now_ms()));
runtime.attach_trace(recorder);          // start
let read = runtime.trace().unwrap();     // borrow
for record in read.records_after(Some(last_seq)) { /* ... */ }
let text = read.export_jsonl();          // all held records
let recorder = runtime.detach_trace();   // stop; keeps the records
```

- `Recorder::new` keeps 4096 records (`DEFAULT_CAPACITY`).
  `Recorder::with_capacity` sets another size. When the ring is full, the
  oldest record is dropped.
- `records_after(None)` reads everything held. `records_after(Some(n))`
  reads records with `seq > n`. A cursor older than the ring returns what
  is left.
- `Recorder::record_present(t_ms, dur_ms)` is for the host. It is not
  called yet.

## Records

Each record has `seq`, `frame`, `kind`, `t_ms`, `dur_ms` and `data`.

- `seq` starts at 0 and rises by one per record, also across a wrap.
- `frame` is the frame id. It moves on after the `Paint` record of a
  `render`. Input, message, update and rebuild records belong to the frame
  that the next `render` finishes.
- `t_ms` is the start time and `dur_ms` the duration, from the recorder's
  clock.

| Kind | Written by | `data` |
| --- | --- | --- |
| `Input` | `dispatch`, `key`, `activate` | `event`, `id`, `x`, `y` |
| `Message` | `flush`, before `Application::update` | `ordinal`, `text`, `source` |
| `Update` | `flush`, around `Application::update` | `{}` |
| `Rebuild` | the view rebuild | `nodes` |
| `Layout` | `render`, layout and reveal | `nodes` |
| `Paint` | `render`, motion and display list | `ops` |
| `Present` | the host (later) | `{}` |
| `Error` | an `AppError` from an input call, `flush` or `render` | `message` |

### Input

`event` is the `Event` variant name, `"Key"` for `Runtime::key` and
`"ActivateId"` for `Runtime::activate`. `id` is the stable id the input
names (`ImeFor`, `SetValue`, `ActivateId`), else `null`. `x` and `y` are
set for pointer and scroll events. Typed text is not recorded.

A call nested in another input call writes no `Input` record. For example,
a roving group that activates on arrow keys writes one `Edit` record.

### Message

- `ordinal` counts messages in the frame from 0.
- `text` is `Application::describe(&message)`. The default returns `None`.
  `describe` is called only while a recorder is attached. It adds no trait
  bound on `Application::Message`.
- `source` is the binding that produced the message: `{"id", "handler"}`.
  `handler` is `press`, `input`, `value`, `key` or `dismiss`. It is `null`
  for a message from `Runtime::apply` or posted through `Queue`, including
  posts from inside `update`.

Known limits:

- `Runtime::set_value` and `Runtime::cut_selection` called directly by the
  host write no `Input` record of their own. `cut_selection` goes through
  `dispatch`, so it writes an `Edit` record. A direct `set_value` writes its
  message with no source, and its errors are not recorded.
- During a slider drag the source is the focused slider. For `Cancel` and an
  outside click, the source is the open modal panel.

## JSON lines

`Recorder::export_jsonl` writes one record per line, with a fixed field
order. Strings are escaped by `serde_json`. A non-finite time is `null`.

```json
{"seq":2,"frame":0,"kind":"Message","t_ms":4.0,"dur_ms":0.0,"data":{"ordinal":0,"text":"Bump","source":{"id":"ok","handler":"press"}}}
```

The full pinned fixture is `CLICK_JSONL` in
`crates/surface-app/tests/trace.rs`.

## Not in this item

- The host writing `Present` and host errors (O5).
- Wire types and the protocol crate (O4).
- Reading the trace over a transport (O9), the `log` facade (O16) and
  replay (O15).
