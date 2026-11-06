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
same GPU pipeline as filled shapes. Elevation and ripple tokens
still need implementation. Motion and typography are partly done: see
[Motion](#motion) and [Typography](#typography).

## Shape tokens

`surface_material::shape` holds the Material 3 shape tokens from androidx
`ShapeTokens.kt`, as `surface::Corners` constants. Corner order is top-left,
top-right, bottom-right, bottom-left.

| Token | Corners (dp) |
| --- | --- |
| `NONE` | 0 |
| `EXTRA_SMALL` / `EXTRA_SMALL_TOP` | 4 / top 4 |
| `SMALL` | 8 |
| `MEDIUM` | 12 |
| `LARGE` / `LARGE_TOP` / `LARGE_START` / `LARGE_END` | 16 / top 16 / start 16 / end 16 |
| `LARGE_INCREASED` | 20 |
| `EXTRA_LARGE` / `EXTRA_LARGE_TOP` | 28 / top 28 |
| `EXTRA_LARGE_INCREASED` | 32 |
| `EXTRA_EXTRA_LARGE` | 48 |
| `FULL` | fully rounded |

Components use these tokens through `Style::corners`.

| Component part | Shape |
| --- | --- |
| Button container, segmented button, switch track and thumb | `FULL` |
| Radio ring and dot, slider tracks and handle | `FULL` |
| Chip | `SMALL` |
| Card, outlined card | `MEDIUM` |
| Dialog panel | `EXTRA_LARGE` |
| Menu | `EXTRA_SMALL` |
| Text field | `EXTRA_SMALL_TOP`, so the bottom edge is square |
| Checkbox box | 2 dp on every corner |
| Selected tab indicator | 3 dp on every corner |

Only shapes changed. Card went from radius 16 to 12 to match Material 3.
Sizes and colors are the same. Several parts changed from their old fixed
radius to `FULL`. The renderer clamps `FULL` to half of the shorter side,
so these parts are circles or pills at any size.

Known gaps in shape support:

- A rounded parent does not clip its children. Descendants keep the
  rectangular parent clip, so a child can paint over a rounded corner.
- Text is not clipped by rounded corners.
- `LARGE_START` and `LARGE_END` assume left-to-right layout. They are not
  mirrored for right-to-left text.
- The shape tokens are a foundation. They do not make any component family
  complete. Per-state shape changes (for example a pressed button) are not
  implemented.

## Typography

`surface_material::typography` holds the Material 3 type scale as 30
`surface::TextStyle` constants. There are 15 base roles: Display, Headline,
Title, Body and Label, each in Large, Medium and Small. Each has an
`_EMPHASIZED` variant. The values come from `TypeScaleTokens.kt` in
androidx-main. Surface follows androidx-main where it differs from older
published M3 tables. Three letter spacings differ:

| Role | Older M3 tables | androidx-main (Surface) |
| --- | --- | --- |
| `DISPLAY_LARGE` | -0.25 | -0.2 |
| `BODY_MEDIUM` | 0.25 | 0.2 |
| `TITLE_MEDIUM` | 0.15 | 0.2 |

| Role | Size / line height | Weight |
| --- | --- | --- |
| `DISPLAY_LARGE` / `MEDIUM` / `SMALL` | 57/64, 45/52, 36/44 | Regular |
| `HEADLINE_LARGE` / `MEDIUM` / `SMALL` | 32/40, 28/36, 24/32 | Regular |
| `TITLE_LARGE` | 22/28 | Regular |
| `TITLE_MEDIUM` / `SMALL` | 16/24, 14/20 | Medium |
| `BODY_LARGE` / `MEDIUM` / `SMALL` | 16/24, 14/20, 12/16 | Regular |
| `LABEL_LARGE` / `MEDIUM` / `SMALL` | 14/20, 12/16, 11/16 | Medium |

Emphasized roles keep the size and line height. They raise the weight to
Medium (Display, Headline, Title Large, Body) or Bold (Title Medium and
Small, Label).

Apply a role with `Style::with_text_style(typography::BODY_LARGE)`. The
`Style` builders `font_weight`, `line_height`, `letter_spacing` and
`font_family` set each field alone. Layout measures a node with its full
text style, so a role's line height sets the node height.

The theme uses these roles. `heading` uses Headline Large. `text` and
`text_field` use Body Large. A size 14 label, such as a button label, uses
Label Large. A size 16 label uses Body Large. A dialog title uses Headline
Small. The checkbox mark stays a plain 12 pixel glyph.

Known limits:

- Letter spacing is stored and exported but not rendered. glyphon 0.8 and
  cosmic-text 0.12 cannot apply it.
- Only regular faces are bundled: Noto Sans CJK TC and Tuffy. Medium and
  Bold roles with the default family render with the same regular face as
  Regular roles. They get a heavier look only after `TextSystem::add_font`
  registers a face named Tuffy that declares that weight.
- Tuffy declares weight 500, but it is a separate typeface from Noto, not
  Noto's Medium. cosmic-text 0.12.1 lets a family match a face only at the
  exact weight that the face declares. If the default family honored that
  number, every Medium and Bold role would switch typeface and change width.
  So the bundled faces count as regular for the default family. This is a
  deliberate choice. It is recorded here because it means the emphasized
  roles are not visually distinct yet.
- `FontFamily::Named` selects the face that carries the name, at the
  declared weight nearest to the request (the lighter one on a tie).
  `Named("Tuffy")` therefore draws with Tuffy, which the default family
  does not use for regular text. A name that no face carries shapes like
  the default family. The text system loads no system fonts, so only
  bundled and added faces can be named. A face of another family added
  with `add_font` is reached by name or as glyph fallback. It does not
  become a weight of the default family.
- Brand and Plain are both the default family, and there is no per-locale
  scale.
- The tests prove layout, paint and `TextSystem` behavior, including that a
  named family changes measured widths. They do not prove how the text looks
  in a browser or on a GPU.

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
methods call the same implementations. Button and chip containers paint
the focus ring in primary. The ring follows the control shape. The input
chip remove button keeps an `on_container` ring. Stable IDs preserve
focus after an update. Nested modal panels keep their own focus
restoration points.
A click that dismisses a modal panel does not activate a background control.

Tab can reach controls outside a scroll viewport. A new focus target is
revealed from its inner scroll container to its outer container. A viewport
resize also reveals focus. Fixed overlays do not move the background.
Repeated frames preserve a deliberate manual scroll when focus and viewport
have not changed. A target larger than its viewport is aligned at its start.

## Motion

`surface_material::motion` holds the Material 3 motion tokens, copied from
androidx `MotionTokens.kt`.

- `motion::duration` has the 16 duration constants in milliseconds:
  `SHORT1` to `SHORT4` (50 to 200), `MEDIUM1` to `MEDIUM4` (250 to 400),
  `LONG1` to `LONG4` (450 to 600) and `EXTRA_LONG1` to `EXTRA_LONG4`
  (700 to 1000).
- `motion::easing` has the cubic Bezier curves `EMPHASIZED`,
  `EMPHASIZED_DECELERATE`, `EMPHASIZED_ACCELERATE`, `STANDARD`,
  `STANDARD_DECELERATE`, `STANDARD_ACCELERATE`, `LEGACY`,
  `LEGACY_DECELERATE` and `LEGACY_ACCELERATE`, plus `LINEAR`.

The core `surface` crate owns the mechanism. A style with a `transition`
makes the runtime blend color and corner changes over time. The host
gives the runtime a clock, and a reduced-motion switch turns blending off.
See [DOM export](dom-export.md#style-transitions) for the data shape.

`motion::STATE_CHANGE` is 200 ms (`duration::SHORT4`) with the `STANDARD`
easing. It is a Surface choice. It is not an official Material mapping.
These components use it, and nothing else does:

- the filled button's visual container;
- the checkbox box;
- the radio outer ring and its dot;
- the switch track and thumb;
- the segmented button container;
- the chip container;
- the tab active indicator.

The tab indicator exists only while its tab is selected. It is added
and removed, so it appears at once. Its transition has no visible effect.

Only `background`, `foreground`, `border_color`, `state_layer` and the
corners are blended. A color is blended per channel, alpha included, in
0 to 255 sRGB space. Changing a target part-way through a transition
restarts from the value being shown. Layout always uses the target style.

Corners are blended as four values. They come from `corners`, or from
`radius` when `corners` is unset. A run starts only when that unresolved
value changes. A resize of a `FULL` shape starts no run. The start and end
values are clamped to half the short side of the node. The end value is
resolved against the node's current size on every frame, so `FULL` blends
toward half the short side and never toward `f32::MAX`. Each shown corner
stays finite and within that range, even with an easing that overshoots.
While a run is active the display copy carries the shown corners. The
exported DOM keeps the target corners.

Known motion gaps:

- Springs are not implemented.
- Layout and position changes (size, offset, scroll) do not animate.
- Hover and press state-layer opacity does not animate. Paint picks that
  opacity from the interaction state, so it jumps. Only a change to the
  `state_layer` color style is blended.
- Enter and exit animations are missing. A node that first appears, or a
  dialog or menu that opens, shows its final style at once. A removed node
  disappears at once.
- Native hosts have no OS reduced-motion query. A native app can set
  `SURFACE_REDUCED_MOTION=1`. Only the browser follows the OS setting.
- The Material emphasized path easing is a path curve. Surface uses the
  androidx cubic approximation of it.
- `STATE_CHANGE` is not mapped to each component's official Material
  duration and easing. Ripple and shape morphing are not implemented.
- Other tokens and components do not use a transition yet.

Motion does not complete the Material catalog. The coverage file keeps
the motion foundation at `partial`.

## Gallery and evidence

Run `cap cargo run --locked -p surface-material-gallery` for a native window.
Run `cargo xtask web dev material-gallery` for the browser. See
[web build](web-build.md). The same `Gallery` state and view drive both.

`scripts/browser-material.mjs` runs the real WASM/WebGPU gallery. It checks
selections, keyboard groups, slider drag, both themes, chip removal,
Chinese dialog editing, focus trapping/restoration, menus and responsive
modal bounds. It also reads the live DOM export. The native smoke check
presents two frames. It does not prove every native interaction or variant.

Open gaps include the remaining families and variants, rounded
clipping and RTL mirroring for shapes, heavier font faces and rendered
letter spacing, elevation, finished animation, touch refinements and
platform accessibility. See [verification](verification.md).
