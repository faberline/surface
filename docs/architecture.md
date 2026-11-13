# Architecture

## State owns the UI

The contract is `UI = f(state)`. An application implements `Application`.
Its immutable `view` produces values, not platform handles. An input event
resolves to a stable control ID, produces a typed message, and calls
`update`. The runtime rebuilds the view after the update. Components are
ordinary Rust functions that compose views.

## Out-of-band updates and wake

Input is not the only source of change. Timers, workers and async results
also change state. Two paths cover them.

- **Same thread.** `Runtime::apply(message)` runs `update` and rebuilds.
  `Runtime::queue().post(message)` queues a message. A post made inside
  `update` runs after that `update` returns, in order. It never re-enters
  `update`. Equal messages do not coalesce; each post runs once.
- **Any thread.** `surface_platform::run_with_wake(options, |wake| ...)`
  hands the app a `WakeHandle`. It is `Clone + Send + Sync` and works on
  native and on the web, so
  `let f: Arc<dyn Fn() + Send + Sync> = Arc::new(move || wake.wake());`
  fits a worker callback. The handle carries no message. A wake calls
  `Application::wake(&mut self)` on the event loop thread; poll your worker
  there and drain everything that is ready. Many calls coalesce into one
  pending event, one `wake` call and one redraw. A call made while `wake`
  runs schedules one more, so the last call is never lost.

## Window close

On native desktop a close request no longer exits the loop at once. The host
calls `Runtime::request_close`, which calls `Application::close_requested`
once. It returns `CloseDecision::Allow` (the default, so old apps behave as
before) or `CloseDecision::Cancel`. After a cancel the window and the app
state stay, and later events run. The app can close later, for example after
a "discard and close" choice, with `Queue::close()` from `update` or `wake`.
That command skips the hook, and from inside `close_requested` it takes
effect at once. The host checks it in `about_to_wait`, so every path that
runs `update` (input, wake, bridge, cut) is covered, and then ends the event
loop. A close request that arrives after the loop began to exit is ignored. The loop returns normally from
`run_with_wake`, so the app state is dropped by `Drop` and
`std::process::exit` is not used. The host drops the GPU and the window
first, then the app state, so cleanup in `Drop` does not run behind a frozen
window. A caller of `run_runtime` that keeps its own `Rc<RefCell<Runtime>>`
clone delays the app drop until that clone is gone. The web host has no window close and does
not use this path.

## App key handler

`View::on_key(|press| ...)` lets an app see a key press in the focused
node, for example a chat field (#103). A `KeyPress` holds a Surface `Key`
and `Modifiers` (`shift`, `control`, `alt` for Option, `meta` for Command).
The hosts map winit and DOM keys to these types inside `surface-platform`
(`key_route.rs`); no platform type leaves it.

- **Order.** The handler runs first, before the field's own key handling
  (newline, Activate, Escape, caret moves, Tab, shortcuts). It returns
  `Some(message)` to consume the press: the field does not see it and
  `message` runs. `None` passes the press to the field unchanged.
- **Menu case.** A slash menu that is open while the field keeps focus is
  app state. The field's handler reads it and consumes ArrowUp, ArrowDown,
  Enter and Escape; the field value, the caret and a `modal` panel do not see
  them. When the menu is closed, the handler passes these keys.
- **Composition.** The handler is not called during an IME composition.
  The composition owns the key, and Enter commits the candidate; it never
  becomes "send". The runtime checks its own editor state, and the host
  adds its platform state: the native IME snapshot, the DOM composition
  flags, and `keyCode` 229, which Safari sends for the Enter that commits
  after `compositionend`.
- **Defaults.** `KeyPress::text_action()` maps Enter with no modifier to
  `TextAction::Submit` and Shift+Enter to `TextAction::NewLine`. Apps may
  use it or read the raw key.
- **No handler.** The host path is the same as before. Enter inserts a
  newline in a multiline field and activates elsewhere.

The decisions are pure functions (`route_key`, the key maps and
`field_action`) shared by both hosts and unit-tested without a window.
Real-device IME evidence (macOS Zhuyin and Cangjie) is tracked in #48.

`NodeId` is an arena index for the current tree. It is not durable identity.
Use a stable ID for an interactive node. Duplicate IDs return `DomError`.
Focus and pending pointer presses store stable IDs. Disabling a container
also disables its descendants. Text input is controlled by application
state. It must return the current value on each view call.

## MicroDOM and geometry

`MicroDom` stores node records in one `Vec`. Parent, first-child,
last-child, and next-sibling links are typed indices. Stable IDs and
role/name selectors have deterministic indexes. Snapshots contain semantic
data and styles, with no closures or platform handles. Schema version 1 is
separate from the pre-existing `SurfaceSnapshot` schema. `MicroDom::from_snapshot` rebuilds a tree from a snapshot and returns `DomError` for a bad version, a duplicate ID or broken links.

`Runtime::snapshot()` adds current interaction and editor state, with
matching layout when available. `Runtime::export_dom()` serializes it as
versioned JSON. See [DOM export](dom-export.md).

`LayoutEngine` retains a Taffy tree. With unchanged topology it updates only
changed layout inputs and text contexts. Taffy caches the measurements.
A topology change rebuilds that layout tree. This is not a keyed Virtual
DOM diff. Text measurement and paint use the same font system and width.
Coordinates use logical pixels. Only the host converts them to device
pixels.

`ComputedDom` holds absolute bounds and ancestor clips. Scroll offsets
shift descendants. Stable sibling sorting by `z_index` defines paint order.
Fixed layers use viewport coordinates and escape ancestor clips and scroll.
They paint after flow content, ordered by `z_index`.
Hit testing walks that order backwards and checks bounds, clipping, and
enabled control roles. This supports the current layout model; it is not
the full CSS stacking-context model. `ComputedDom::changed_since` lists the nodes whose bounds or clip differ from an earlier frame.

The display list contains ordered rounded quads, borders and text. It has no wgpu
types. The backend preserves order when mixing text and rectangles.
There is no incremental GPU damage optimization yet; each dirty frame
paints a full display list. `DisplayList::damage` returns the visible rectangles of the ops that differ from an earlier list.

## Platform boundary

`surface-platform` owns winit windows, a browser canvas, GPU initialization,
resize, scale changes, and device surface recovery. It translates native
and web events into the same `surface_app::Event` values.

`surface-wgpu` owns the device, queue, shaders, font database, glyph atlas,
and text renderers. It supports native and WebGPU adapters through wgpu.
Its default font database contains bundled Tuffy and Noto Sans CJK TC. The
browser build bundles a Big5-common subset of the CJK font; see
`docs/web-font.md`. Call `TextSystem::add_font` before the first render for
other script coverage.
The editor and renderer share shaped caret and selection geometry.
See [text input](text-input.md) for the tested input paths and remaining gates.

## Upstream references

- [Taffy 0.7.7](https://docs.rs/taffy/0.7.7/taffy/) supplies layout and caching.
- [wgpu 24](https://docs.rs/wgpu/24.0.5/wgpu/) supplies native GPU and WebGPU.
- [glyphon 0.8](https://docs.rs/glyphon/0.8.0/glyphon/) supplies GPU text.
- [Material color roles](https://m3.material.io/styles/color/roles) guide the
  initial light/dark palette.

Product positioning is a proposal: see [decision 0001](decisions/0001-positioning.md).
