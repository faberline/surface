# Context menu

An app reacts to a right click with `View::on_context_menu` and opens a
menu itself. Surface has no menu API for this: use `Theme::menu_at`.

## The pattern

```rust
View::new(row).on_context_menu(|p| Msg::OpenMenu(p.x, p.y))
```

- The handler gets a `PointerPoint { x, y }` in logical pixels, window
  coordinates. It returns the message to post.
- The runtime hit-tests the point and walks up from the hit node to the
  nearest ancestor that has a handler.
- `update` stores the point. `view` renders `theme.menu_at(.., (x, y),
  items, Msg::CloseMenu)` while it is set. `menu_at` is modal, so a press
  outside it and Escape dismiss it.
- A secondary press does not set pressed, hovered or focus, does not move a
  text cursor and does not start a drag. It cannot activate a control.
- With a menu open, a secondary press outside it dismisses the menu and
  posts no context message. Inside it, nothing happens.
- `menu_at` does not flip at the window edge. A menu near the edge can
  overflow (issue 109).

## Which button is secondary (native hosts)

`crates/surface-platform/src/pointer_route.rs` decides, in a pure function
that runs without a window:

| Input | Result |
| --- | --- |
| Right press | `Event::SecondaryDown`; the release is dropped |
| Left press with Control held, on macOS | `Event::SecondaryDown`; the release is dropped, even if Control was released first |
| Left press with Control held, other systems | plain `PointerDown` |
| Plain left press and release | `PointerDown`, `PointerUp` |
| Middle and other buttons | dropped |

Touch is unchanged. There is no long press.

## Web

The web host sends no secondary press and adds no `contextmenu` listener
(decision 0001 D-B). A right click on the web build shows the browser menu.
