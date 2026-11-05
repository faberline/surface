# Export the current DOM

Call `Runtime::export_dom()` to get a JSON string. The call reads current
state. It does not run `update`, trigger a render, or write a file.
Desktop and WASM use the same method.

For a running desktop window or WASM app, the optional
[application bridge](application-bridge.md) exposes this data through
Jet bridge's `ui snapshot` CLI command and `ui_snapshot` MCP tool.

```rust,ignore
let json = runtime.export_dom()?;
std::fs::write("surface-dom.json", json)?;
```

`runtime.snapshot()` returns the typed `surface_app::DomExport` value.
For semantic nodes and styles alone, use `runtime.dom().snapshot()`.
That existing `surface::DomSnapshot` type also supports Serde, Rust's
data serialization library.

The browser examples expose this method through their WASM module:

```js
const snapshot = JSON.parse(window.gallery.export_dom());
console.log(snapshot);
// The small showcase uses window.showcase.export_dom().
```

To download it in the gallery's browser console:

```js
const url = URL.createObjectURL(new Blob([window.gallery.export_dom()], {
  type: 'application/json'
}));
const link = document.createElement('a');
link.href = url;
link.download = 'surface-dom.json';
link.click();
setTimeout(() => URL.revokeObjectURL(url), 1000);
```

## Version 1

| Field | Contents |
| --- | --- |
| `version` | Export schema version, currently `1` |
| `dom` | Semantic nodes, IDs, parent/child/sibling links, text, control states and typed styles |
| `layout` | Viewport, node bounds, clipping rectangles and paint order, or `null` when layout needs a render |
| `interaction` | Current focused, hovered and pressed stable IDs |
| `text_inputs` | Committed value, display value, UTF-8 selection positions and composition state per field |

The array index identifies a node within this capture. Stable IDs identify
controls across state updates. Bounds use logical pixels. `layout` is
`null` before the first frame and after a state, editing or scroll change
invalidates geometry. Render once to capture matching bounds.

An active Chinese composition has its temporary text in `display_value`.
The DOM's field text and `value` still contain the committed application
value. This preserves `UI = f(state)` while exposing the current editor.

Repeated reads of unchanged state produce the same JSON. The capture has
no callbacks, GPU objects or platform handles. Loading JSON into `DomExport`
reads inspection data; it does not restore a running application.

## Style transitions

A style may carry an optional `transition`. It is omitted when `None`, so
the export of a style that does not use motion is byte-identical to before.
When present it has two fields:

| Field | Contents |
| --- | --- |
| `duration_ms` | Transition length in milliseconds. Finite and not negative. |
| `easing` | `"Linear"`, or `{"CubicBezier": [x1, y1, x2, y2]}` like CSS `cubic-bezier()`. `x1` and `x2` are in `[0, 1]`. |

An invalid transition makes layout fail with `InvalidStyle` for that node.
An older document without the field still loads.

The transition declares how a node's `background`, `foreground`,
`border_color`, `state_layer` and corners (`corners`, or `radius` when
unset) follow a change. It is a declaration, not a state. The export
always holds the target styles. It never holds a value that is part-way
through a transition, so the JSON taken mid-transition equals the JSON
taken after the transition ends.

Animate only nodes with a stable ID. A node without one is tracked by
position, so inserted or reordered siblings can inherit another node's
track.

When no node in the tree has a transition, motion keeps no state. A
transition added in the same update as a value change does not animate that
first change if no other node had a transition. Later changes animate.

Corners are blended as four values. They come from `corners`, or from
`radius` when `corners` is unset. The export keeps the target corners. See
[Style corners](#style-corners).

Presentation is runtime-only. The runtime blends values on a display copy
of the DOM that is used to build the display list. The semantic DOM, the
export and the application bridge never see that copy. Layout also uses
target styles. A saved export cannot restore a running transition.

## Style corners

A node style has `radius`, the legacy uniform corner radius. It may also
have `corners`, a per-corner shape. `corners` is omitted when it is not
set, so the export of an unchanged style is the same as before.

```json
"corners": { "top_left": 4.0, "top_right": 4.0, "bottom_right": 0.0, "bottom_left": 0.0 }
```

The fields are always in this order: `top_left`, `top_right`,
`bottom_right`, `bottom_left`. Values are logical pixels. When `corners`
is present it replaces `radius` for painting. `Style::corner_radii()`
returns the effective corners either way.

`surface::Corners::FULL_RADIUS` is `f32::MAX`. It means "fully rounded".
It is finite, so it reads back from JSON unchanged. Painting resolves each
corner into `0` through half of the shorter side of the node bounds. A
fully rounded node therefore becomes a circle or a pill at any size.
Layout rejects negative or non-finite corners.

Display lists use `PaintOp::Quad` and `PaintOp::Stroke` when the resolved
corners are equal. Otherwise they use `PaintOp::Shape` and
`PaintOp::ShapeStroke`. A node that sets only `radius` is painted as before.
A focused node that sets `corners` paints one inside `Stroke` or
`ShapeStroke` that follows its corners. A focused legacy node still paints
four square edge `Quad`s, as before.

## Text style fields

Each node's `style` always has `font_size`. Four more text fields appear
only when they differ from the default, so exports of plain text are
byte-for-byte what they were before text styles existed. Version 1 does not
change. Older JSON without these fields still loads as regular text.

| Field | Default (omitted) | Meaning |
| --- | --- | --- |
| `font_weight` | `400` | CSS-style weight, 1 to 1000 |
| `line_height` | none | Absolute line height in logical pixels. Without it the height is `font_size * 1.5` |
| `letter_spacing` | `0` | Extra advance per glyph in logical pixels. Carried, not yet rendered |
| `font_family` | `"Default"` | `"Default"` or `{"Named": "Inter"}` |

The export records what a node asks for. It does not say what a renderer
can draw. Letter spacing is not rendered yet. The bundled renderer has only
regular faces, so a weight above 400 draws as regular with the `"Default"`
family. A named family selects the face that carries the name, at the
declared weight nearest to the request. For example, `{"Named": "Tuffy"}`
draws with the bundled Tuffy face, which `"Default"` does not use for
regular text. A name that no face carries draws like `"Default"`. See
[Material typography](material.md#typography).

Example for a Material Headline Large node:

```json
{ "font_size": 32.0, "line_height": 40.0 }
```

Layout rejects a weight outside 1 to 1000, a line height that is not finite
and positive, and a letter spacing that is not finite.

A display list from `Runtime::render` still uses `Text` and `EditableText`
for size-only styles. Any other style paints as `StyledText`, which carries
the full `TextStyle` beside the same rectangle, clip, color and scroll data.
`StyledText` is a new last variant of `PaintOp`. Consumers that match every
variant must handle it.
