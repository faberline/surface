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
