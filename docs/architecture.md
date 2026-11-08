# Architecture

## State owns the UI

The contract is `UI = f(state)`. An application implements `Application`.
Its immutable `view` produces values, not platform handles. An input event
resolves to a stable control ID, produces a typed message, and calls
`update`. The runtime rebuilds the view after the update. Components are
ordinary Rust functions that compose views.

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
the full CSS stacking-context model.

The display list contains ordered rounded quads, borders and text. It has no wgpu
types. The backend preserves order when mixing text and rectangles.
There is no incremental GPU damage optimization yet; each dirty frame
paints a full display list.

## Platform boundary

`surface-platform` owns winit windows, a browser canvas, GPU initialization,
resize, scale changes, and device surface recovery. It translates native
and web events into the same `surface_app::Event` values.

`surface-wgpu` owns the device, queue, shaders, font database, glyph atlas,
and text renderers. It supports native and WebGPU adapters through wgpu.
Its default font database contains bundled Tuffy and Noto Sans CJK TC. Call
`TextSystem::add_font` before the first render for other script coverage.
The editor and renderer share shaped caret and selection geometry.
See [text input](text-input.md) for the tested input paths and remaining gates.

## Upstream references

- [Taffy 0.7.7](https://docs.rs/taffy/0.7.7/taffy/) supplies layout and caching.
- [wgpu 24](https://docs.rs/wgpu/24.0.5/wgpu/) supplies native GPU and WebGPU.
- [glyphon 0.8](https://docs.rs/glyphon/0.8.0/glyphon/) supplies GPU text.
- [Material color roles](https://m3.material.io/styles/color/roles) guide the
  initial light/dark palette.
