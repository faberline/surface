# Material components

This is a working component checkpoint. The full Material catalog remains
the target in [the completion contract](completion-contract.md).
[The coverage file](material-coverage.json) records all 36 families.
No family is marked complete by this checkpoint.

## Themes and state

`Theme::from_seed(Color::rgb(103, 80, 164), false)` generates a light
scheme. Pass `true` for dark mode. `Theme::light()` and `Theme::dark()`
cache that seed's schemes. The generator uses HCT, Material's hue, chroma
and tone color model, through `material-colors` 0.4.2. `Theme::colors`
exposes the generated color roles. The older fields remain available.

Buttons, choices and tabs use hover and press state layers. A state layer
paints a translucent color over the component. Rounded borders use the
same GPU pipeline as filled shapes. Typography, elevation, motion and
ripple tokens still need implementation.

All selected values belong to the application. A component receives the
current value and returns messages through `update`. Rendering a view
does not change application state.

## Current API

| API | Current behavior |
| --- | --- |
| `button_with` | Filled, tonal, outlined and text; disabled state; 48-pixel touch area |
| `checkbox_with` | Unchecked, checked and mixed; disabled state |
| `radio` / `radio_group` | Controlled choice; arrows select and skip disabled choices |
| `switch` | Controlled on/off choice and disabled state |
| `segmented_button` / `segmented_group` | Controlled choice; arrows move focus, Enter/Space selects |
| `tab` / `tabs` | Controlled selection, selected underline and manual keyboard activation |
| `chip` | Assist, suggestion, filter and input; input removal has its own action |
| `slider` | Controlled finite range, pointer drag, arrows, Home/End and Page Up/Down |
| `dialog` | Viewport scrim, adaptive panel, focus trap, Escape/outside dismiss, opener restore |
| `menu_at` / `menu_item` | Viewport popup, vertical keyboard navigation, disabled skip and dismiss |
| `divider` | One-pixel semantic separator |
| `card` / `outlined_card` | Filled and outlined containers |
| `text_field` | Controlled Unicode editing, selection, clipboard and composition |

The short `button`, `tonal_button`, `disabled_button` and `checkbox`
methods call the same implementations. Stable IDs preserve focus after
an update. Nested modal panels keep their own focus restoration points.
A click that dismisses a modal panel does not activate a background control.

Tab can reach controls outside a scroll viewport. A new focus target is
revealed from its inner scroll container to its outer container. A viewport
resize also reveals focus. Fixed overlays do not move the background.
Repeated frames preserve a deliberate manual scroll when focus and viewport
have not changed. A target larger than its viewport is aligned at its start.

## Gallery and evidence

Run `cap cargo run --locked -p surface-material-gallery` for a native window.
Run `./scripts/build-gallery.sh` and serve `examples/material-gallery/web`
for the browser. The same `Gallery` state and view drive both.

`scripts/browser-material.mjs` runs the real WASM/WebGPU gallery. It checks
selections, keyboard groups, slider drag, both themes, chip removal,
Chinese dialog editing, focus trapping/restoration, menus and responsive
modal bounds. It also reads the live DOM export. The native smoke check
presents two frames. It does not prove every native interaction or variant.

Open gaps include the remaining families and variants, exact Material
shapes and typography, elevation, animation, touch refinements and platform
accessibility. See [verification](verification.md).
