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
| `button_group` | Standard and Connected press groups with one row, five sizes, round or square shapes, and controlled selection |
| `split_button` | Independent leading action and trailing disclosure with five sizes, four appearances, and an optional command menu |
| `fab` | Small, regular and large; surface, primary, secondary and tertiary containers; disabled state; no elevation or ripple yet |
| `expressive_fab` | Square icon action in three sizes with six palettes, lowered elevation, an exterior focus ring, and a clipped press ripple |
| `extended_fab` | One named action with an optional icon, three sizes, six palettes, and scroll-driven expansion |
| `fab_menu` | Native floating actions or an anchored Web menu with one stable FAB, caller-owned open state, and focus return |
| `date_picker` | One date in a modal calendar or text field, with year browsing, validation, and adaptive full-screen layout |
| `docked_date_picker` | An editable date field and a separate calendar button, with month and year lists and an anchored panel |
| `bottom_sheet_layout` / `bottom_sheet_with_motion` | Standard and modal bottom sheets with peek and expanded heights, a fixed accessible handle, contained scrolling and optional vertical drag |
| `app_bar` / `app_bar_search_view` | Small, center-aligned, flexible and search app bars with caller-controlled slots, scroll behavior and a modal Search View |
| `toolbar` / `toolbar_button` | Docked and horizontal or vertical floating action groups with Standard and Vibrant palettes, arrow-key focus and optional adjacent FAB |
| `carousel` | Multi-browse, two free-scroll layouts, two Hero alignments and full-screen paging with adaptive card sizes and logical focus navigation |
| `plain_tooltip` / `rich_tooltip` | Anchored descriptive text or a rich popup with optional title, caret and action |
| `navigation_rail` | Classic 80 dp rail with a 56 × 32 dp indicator, controlled selection, arrow focus and named destinations. The vector item builder supports 24 dp icons and scaled labels. |
| `navigation_drawer` / `modal_navigation_drawer` | Standard and modal drawer; 360-pixel panel; 56-pixel destinations; selected and disabled destination; full-width active indicator; modal scrim, focus trap and Escape/outside dismiss; light and dark; text-glyph icons, no badge, section headline or ripple |
| `navigation_bar` / `navigation_bar_item` | Three to five destinations; selected and disabled item; 64x32 active indicator; arrows move focus; light and dark; text-glyph icons, no badge, ripple or hide-on-scroll |
| `progress-indicators` | Linear determinate and indeterminate; light and dark; phase set by the application; no circular variant, no rotation or motion yet |
| `loading_indicator` | Default and contained; light and dark; phase set by the application; shape cycle by corner radius, no rotation or polygon morph yet |
| `icon_button` | Standard, filled, tonal and outlined; optional toggle selection; disabled state; 48-pixel touch area; no elevation or ripple yet |
| `checkbox_with` | Unchecked, checked and mixed; disabled state |
| `radio` / `radio_group` | Controlled choice; arrows select and skip disabled choices; ring uses primary when selected, on-surface-variant when unselected, outline when disabled |
| `switch` | Controlled on/off choice and disabled state |
| `segmented_button` / `segmented_group` | Controlled choice; arrows move focus, Enter/Space selects |
| `segment` | Position-aware segment: round outer ends, square inner edges, check mark when selected; single or multi select owned by the application |
| `tab` / `tabs` | Controlled selection, selected underline and manual keyboard activation |
| `chip` | Assist, suggestion, filter and input; input removal has its own action |
| `slider` | Controlled finite range, pointer drag, arrows, Home/End and Page Up/Down |
| `dialog` | Viewport scrim, adaptive panel, focus trap, Escape/outside dismiss, opener restore |
| `dialog_with` | Basic dialog with optional icon, headline, supporting text, actions; container `surface_container_high`, scrim from `colors.scrim`; no full-screen variant, shadow or motion yet |
| `menu_at` / `menu_item` | Viewport popup, vertical keyboard navigation, disabled skip and dismiss |
| `menu_item_with` | Menu item with optional leading glyph, trailing text, selected and disabled state; no submenus or checkmark yet |
| `divider` | One-pixel semantic separator |
| `divider_with` / `vertical_divider` | Full, inset and middle-inset horizontal dividers and a vertical divider; 1 dp, `outline_variant`; no list or sheet integration yet |
| `card` / `outlined_card` | Filled and outlined containers |
| `card_with` / `action_card` | Elevated, filled and outlined kinds with M3 container tokens; action card is a button with focus, press and disabled state; elevated has no shadow yet; no ripple, drag or slots |
| `material_card` / `material_action_card` | Elevated, filled and outlined cards with actual shadows, structured content, whole-card selection or independently reachable actions |
| `text_field` | Controlled Unicode editing, selection, clipboard and composition |
| `search_bar` | Pill search bar with leading glyph, controlled text input and a clear button while the query is not empty; no search view, suggestions or placeholder yet |
| `snackbar` / `snackbar_with` | Nonmodal status, optional text action and optional Close icon. Measured inline and stacked layouts use the same action IDs in both directions. |
| `badge` / `badged_anchor` | Passive dot, short label or count. An attached badge follows the logical top trailing edge of its anchor. |

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

## Snackbar

Place `Theme::snackbar` near the bottom of the app, above bottom
navigation. Supply localized labels for the action and the optional Close
icon. Both controls send their own messages. `SnackbarOptions` can set
right-to-left layout, put the action on a separate line, or restore focus
to a caller-chosen control when a focused Snackbar action goes away.

`SnackbarQueue` is optional state for showing one message at a time.
Pass the same monotonic clock to enqueue, act, dismiss and check expiry.
Short messages stay for four seconds. Long messages stay for ten seconds.
Messages with an action stay available until an explicit action or
dismissal. The app owns the clock, placement, timeout adjustments and
action result. Use each returned `SnackbarId` for both its view ID and
its interaction messages.

The visual reference is [Material 3 Snackbar](https://m3.material.io/components/snackbar/overview).
The Snackbar sizes, inverse colors, typography and action layout follow the
[AndroidX Snackbar](https://android.googlesource.com/platform/frameworks/support/+/3a4444a5b801bb899ca38bb982b83be53c169090/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Snackbar.kt).
The [AndroidX host](https://android.googlesource.com/platform/frameworks/support/+/3a4444a5b801bb899ca38bb982b83be53c169090/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/SnackbarHost.kt)
defines the timing reference. The Gallery adds a short entrance animation.
It settles immediately with Reduce Motion.

## Badges

`Theme::badge` makes a standalone badge. The dot is 6 dp. A label uses
Label Small typography and a 16 dp minimum height. The colors use Error
and On Error, and the container uses fully rounded corners. Large text
can grow the label. Use a localized name to describe the status.

`Theme::badged_anchor` attaches a dot, short label or count to a
control. It can also keep the badge hidden. The parent passes the current
value and direction. Use the anchor's accessible name to give the full
localized status, such as “Inbox, 1,200 unread messages”. Count badges
display `999+` above 999. The anchor keeps its focus, dimensions and
actions.

`Theme::badged_anchor_with_clearance` fits the measured badge into
the visible clip. `BadgeBoundary::Owner` limits any axis where the badge
fits the owner. The badge stops painting while its owner is fully
outside the visible clip. For a navigation bar destination, use
`navigation_bar_item_with_badge` or its clearance variant. The destination
accepts a vector or glyph icon and a name separate from its visible label.
Navigation labels and counts overlap the icon by up to 4 dp.
At larger text sizes, these destinations reserve room above the icon.
Use the same builder with `BadgeContent::Hidden` for the other
destinations in that bar to align their icons.

The variant and size references are the [Material 3 badge catalog](https://m3.material.io/components/badges/overview)
and the [AndroidX badge tokens](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/BadgeTokens.kt).
The [pinned AndroidX placement](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Badge.kt#104)
sets the overlap with the top and trailing edges.

## Navigation drawers

Build destinations with `Theme::navigation_drawer_item_with_icon`. Give
each one a stable ID, a localized label and a vector icon. Use
`ChoiceState` to mark the current page and disabled destinations. The
current page stays under application control. Set the same
`LayoutDirection` on the drawer and its destinations.

`Theme::standard_navigation_drawer_layout` places the drawer at logical
start and leaves the rest of the space for the main content. Choose
`StandardNavigationDrawerMode::Permanent` for an always-visible drawer.
For `Dismissible`, keep the open flag in application state and pass the
toggle's stable ID as `restore_focus_to`. Mark the toggle with
`View::disclosure_expanded`. Closing the drawer restores focus to the
toggle when a removed destination held it. Main content stays active
while the drawer is open. Use `standard_navigation_drawer_sheet` for
layouts where the application allocates the drawer's column.

`Theme::modal_navigation_drawer_with_direction` places a dialog above the
page. Keep a button to open it. Remove the dialog when you receive its
dismiss message from Close, Escape or the scrim. The drawer keeps
keyboard focus inside the dialog and restores the opener's focus when it
closes. For either drawer, Up and Down move among enabled destinations.
The destination list scrolls to reveal focus.

To allow horizontal swipes, keep a `NavigationDrawerSwipe` in application
state and mount `Theme::swipeable_modal_navigation_drawer` after the page
content. Feed it the logical viewport through `set_viewport`, and the
drag messages through `apply`. In `Application::animation_frame`, pass
the frame clock and Reduce Motion flag to `advance`. Return its
`AnimationUpdate` and apply any new open flag it returns. Call `reset`
on explicit open, close or direction changes. The swipe uses the
24 dp logical start edge when closed. The static modal builder leaves
that edge free for the host.

In Material Gallery, choose 浮層, then 導覽抽屜. The preview offers permanent,
dismissible and modal drawers, right-to-left placement, dark mode and
200% text. The 360 dp panel cap, 56 dp destination, 24 dp icon, logical
corners and color roles follow the [AndroidX drawer tokens](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/NavigationDrawerTokens.kt).
The swipe uses the 50% position threshold, 400 dp/s velocity threshold
and 256 ms duration from the pinned [AndroidX drawer implementation](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/NavigationDrawer.kt).
Reduce Motion settles immediately. The older drawer builders remain
available.

## Progress indicators

Use `Theme::linear_progress_indicator` or `Theme::circular_progress_indicator`
to show the progress of an operation. Give `ProgressIndicatorOptions` a
stable ID and a localized operation name. The indicator tells assistive
technology the value in `ProgressState::Determinate`. Use
`ProgressState::Indeterminate` until an actual value is known. Both
states are read-only and stay out of the Tab order.

Choose `ProgressShape::Flat` or `ProgressShape::Wavy`. The linear builder
takes the available width and a `LayoutDirection`. Its nominal width is
240 dp. Flat bars are 4 dp high and wavy bars are 10 dp high. The fill
and the round stop marker follow the logical direction. Circular
indicators are 40 dp or 48 dp in diameter. Their determinate fill starts
at twelve o'clock and moves clockwise.

The application owns the value and clock. Pass elapsed milliseconds
from a monotonic timer, and keep requesting frames while an unknown
progress or wavy indicator is visible. Pass the user's Reduce Motion
preference. The unknown state keeps a visible static frame under that
preference. The wavy shape uses sampled vector contours on both
renderers. The existing `Theme::linear_progress` builder remains available.

In Material Gallery, choose 動作, then 進度指示器. Change the progress mode,
value, direction, motion, theme and text size. The sample uses the
[Material 3 progress indicator guidelines](https://m3.material.io/components/progress-indicators/overview),
the [pinned flat AndroidX implementation](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/ProgressIndicator.kt)
and the [pinned wavy implementation](https://android.googlesource.com/platform/frameworks/support/+/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WavyProgressIndicator.kt).

## Date picker

Use `Theme::date_picker` to mount a single-date modal while the app
holds a `DatePickerSession`. The app supplies its committed date,
local today, inclusive bounds, and an optional rule for unavailable
dates. Open the session in Calendar or Input mode. Forward the picker
events to `DatePickerSession::apply`. Save the date only when it returns
`DatePickerEffect::Commit`. Drop the session on Commit or Dismiss.

Supply the localized names, date order, format hint and first weekday
in `DatePickerLabels` and `DatePickerLocale`. Year, month, and day use
the Gregorian calendar. Input accepts ASCII and full-width digits.
Use `observe_input_focus` in the frame input observer to check and
format the field after focus leaves it. Surface gives the first Enter
to the active IME composition. A separate Enter can submit the date.

Adaptive layout uses a full-screen panel below 600 dp. Close and Save
stay in reach when the body scrolls. A narrow grid can scroll
horizontally to preserve 48 dp date targets. Focus starts at the
selected day or an available fallback. Day and week keys move focus.
Page Up and Page Down browse months. Hold Shift on a page key to browse
years. Enter or Space on a day updates the draft. Escape dismisses the
picker and returns focus.

In Material Gallery, choose 浮層, then 日期選擇器. The example can open
either mode and switch locale, text scale, theme, and direction.

[Material 3 date picker](https://m3.material.io/components/date-pickers/specs) ·
[Published date picker tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.01c5310367774eb8.json) ·
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

### Docked date picker

Keep a `DockedDatePickerState` for the field. Pass the current viewport
and the latest layout bounds of `{id}-field` to
`Theme::docked_date_picker`. Read those bounds from `AnimationFrame`.
Update the view when the field moves. The panel fits below or above
the field and can scroll inside a short viewport.

The field and calendar button have separate Tab stops. The button
opens the calendar. Month and year lists change the month on display.
Choosing a day submits it. Escape closes a month or year list first.
Dismiss the calendar to restore focus to its button.

Forward every picker event to `DockedDatePickerState::apply`. Feed
the field's focus to `observe_input_focus` from the frame input
observer. Save the date on `DatePickerEffect::Commit` from either
method. The field keeps incomplete text until the user finishes
editing. Enter and ordinary field exit validate and format the text.
During IME composition, Enter belongs to the input method.

In Material Gallery, choose 浮層, then 錨定日期選擇器. Move the field to
the lower edge to try the placement above it.

[Docked date picker accessibility](https://m3.material.io/components/date-pickers/accessibility) ·
[Published docked tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.01c5310367774eb8.json)

## FAB menus

Use `Theme::fab_menu` with a stable ID, the viewport, the trigger size,
localized names, and the app's open state. Place the returned view at
the original FAB location. The view reserves that square while the menu
is open. Supply two to six related actions, each with a stable ID and an
application message. The toggle, dismissal, and command messages let the
application update the open state.

Choose `NativeFloating` for the floating action pills. The square FAB
changes into a round close button at the original top trailing corner.
The close button is the first focus target. Tab moves from it through
the enabled items in visual order. The list can scroll within a short
viewport. It starts with the nearest action visible. Escape, outside
input, the close button, and a chosen action can dismiss the menu.

Choose `WebAnchored` for the conventional menu. The same square FAB
reports its popup and expanded state. Focus starts at the first enabled
item. ArrowDown without modifiers opens it. Arrow keys move between
items. Tab and Shift+Tab dismiss the menu and continue from its opener.

Keep one `FabMenuMotion` per menu. Pass the current open state and item
IDs in visual order to `advance` from the detailed frame input callback.
The retained state measures the original anchor. Floating items enter
nearest to the close button first. Reduce Motion uses the settled
geometry. Use `fab_menu_edge_margin` for the 16 or 24 dp placement margin.

In Material Gallery, choose 動作, then 浮動動作選單. The scene selects the
presentation for the platform. It also lets you switch presentations
and place the Web anchor near the top or bottom.
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

[Official FAB Menu specs](https://m3.material.io/components/fab-menu/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.1a076cec8bc202c2.json) ·
[FAB Menu accessibility](https://m3.material.io/components/fab-menu/accessibility) ·
[Pinned Compose FAB Menu](https://github.com/androidx/androidx/blob/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/FloatingActionButtonMenu.kt)

## Expressive icon floating action buttons

Use `Theme::expressive_fab` with a stable ID, a localized action name,
and a `VectorIcon`. Regular, Medium and Large use 56, 80 and 96 dp faces.
Each size supports the three direct and three tonal palettes. Choose
standard or lowered elevation. The icon stays decorative. The button
receives Tab focus and accepts keyboard, pointer and assistive activation.

Keep one `ExpressiveFabMotion` per button. Opt into frame input and call
`advance` from `Application::animation_frame_with_press_detail`. Pass the
accepted press, hover target, and current visibility. The observation carries the
original press point through a hold, release or cancellation. The ripple
starts at that point and stays inside the rounded face. Reduce Motion uses
the static press layer.

Place the FAB in the page or its navigation area and leave space for the
outside focus ring. Keep it mounted for visibility animation. When it hides
or becomes disabled, the fallback ID lets focus move to another action.
Use the existing plain tooltip with the same localized label for a visual
name on hover or focus. Add a longer description only when it gives
information beyond that name.

The Gallery places the FAB at the logical trailing corner while the notes
scroll. From 動作, choose 圖示浮動按鈕. The headless Gallery can replay the
scene and capture PNGs.
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

[Official FAB specs](https://m3.material.io/components/floating-action-button/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.41587918e51cca98.json) ·
[FAB accessibility](https://m3.material.io/components/floating-action-button/accessibility)

## Extended floating action buttons

Use `Theme::extended_fab` for the main action of a page. Choose Small,
Medium, or Large. Choose Tonal Primary, Tonal Secondary, Tonal Tertiary,
Primary, Secondary, or Tertiary. Pass the visible action label, an optional
icon, and the available logical width. Choose standard or lowered elevation.
The button keeps its stable ID and
accessible name while it changes size. A text-only action keeps its label
when the application requests collapse.

The application controls expansion, visibility, and placement. Anchor the
view to the page edge or the center of a bottom slot and leave room for its
focus ring. Supply a stable fallback control for focus when the action is
disabled or hidden. On a narrow page, pass the actual space available so
large labels can wrap. Surface mirrors the icon and label for RTL.

Keep one `ExtendedFabMotion` per action. Opt into frame input and call
`advance` with the animation frame, input, accepted press, effective
expanded state, and visible state. For a text-only action, pass true as
the effective expanded state. Keep the component mounted during
visibility changes. Reduce Motion uses the current endpoints.

To follow scroll, store `ExtendedFabScroll` in the application state.
Call its `apply` in the scroll region's vertical observer. It uses body
travel, allows the body to scroll, and updates expansion after 40 dp of
downward travel or the first real reverse travel.

In Material Gallery, choose 動作, then 延伸浮動按鈕. Scroll the notes to
collapse or extend the action. Run the same scene in headless mode to
capture PNGs without opening a window.
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

The size-specific minimum widths follow the pinned Expressive styles.
The general Material placement drawing gives an 80 dp minimum.
[Material 3 Extended FAB specs](https://m3.material.io/components/extended-fab/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.36500f77b86d20a5.json) ·
[Pinned Expressive sizes](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/floatingactionbutton/res/values/styles.xml#L282-L330)

## Split buttons

Use `Theme::split_button` with a stable ID, the available logical width
and the layout direction. Choose ExtraSmall, Small, Medium, Large or
ExtraLarge and use the Elevated, Filled, Tonal or Outlined appearance.
Supply text, an icon with a spoken name, or both for the leading action.
The trailing button has its own name and opens related options. Each
enabled action has a Tab stop. The faces grow to the same height when
large text wraps.

The application controls open state and each button's availability.
Use `SplitButtonMenu` for a built-in command menu or supply a custom
disclosure. The command menu aligns to the trailing face in the
layout's logical direction. It can move above the row and scroll when
the viewport needs more room. Enter or Space opens it, Arrow keys move
focus, and Escape returns focus to the trigger. Tab closes the menu
and continues keyboard traversal in the chosen direction.

Keep one `SplitButtonMotion` per mounted button. Opt into frame input
and pass the frame, pointer input, accepted press and open state to
`advance`. The trailing chevron turns toward the split as it opens.
Reduce Motion selects the endpoint immediately.

In Material Gallery, choose 動作, then 分割按鈕. The example can place
the control near the bottom to show how its menu flips. Run the scene
in headless mode to replay it and capture PNGs without opening a window.
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

[Material 3 split button specs](https://m3.material.io/components/split-button/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.624e8f2da4007fc8.json) ·
[Menu button keyboard and semantics](https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/)

## Button groups

Use `Theme::button_group` with stable item IDs and the available logical
width. Standard groups fit their content and can mix commands and
toggles. Connected groups fill their allocated width with a common
appearance and use toggles. Both types stay on one row and support
ExtraSmall, Small, Medium, Large and ExtraLarge. Text and icon-only
buttons use the size tokens and at least a 48 dp touch target.
Connected faces share a height when large text needs more room.

The application owns selected IDs and updates its state on an item
message. Choose command, single or multiple selection. A required
selection keeps at least one currently selected item. Tab enters on the
first enabled item. Arrow keys move focus and mirror in RTL. Enter and
Space activate the focused item.

Keep one `ButtonGroupMotion` per mounted group. Opt into frame input and
call `advance` from `Application::animation_frame_with_press`. Pass the
current pressed stable ID from the runtime. A Standard press aims to grow
the pressed face by 15% of its resting width using spare width from the
adjacent buttons. The spring can pass that target before settling if
the neighbors have more room. A Connected press changes its shape.
Reduce Motion keeps Standard widths stable and lets the interaction
state change immediately.

Supply `ButtonGroupOverflow` to open a trailing command menu when the
row runs out of width. Toggles stay in the row. A constrained RTL row
starts at the right edge and can scroll horizontally. Keep the popup
open flag in the application and handle command activation and dismissal
there. On resize, close the popup when its trigger leaves the layout.
Focus can follow the same command back into the row.
The overflow trigger reports a menu popup. Tab closes the menu and
continues from the trigger.

The extra-small resting inner corner follows the 8 dp value from the
published token export. The guideline text lists 4 dp. In Material
Gallery, choose 動作, then 按鈕群組.

[Material 3 button group specs](https://m3.material.io/components/button-groups/specs) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.082d7fd6e058b011.json) ·
[Accessibility](https://m3.material.io/components/button-groups/accessibility)

## Cards

Choose Elevated, Filled or Outlined in `MaterialCardOptions`. Give the
card a stable ID, a localized name and a layout direction. `CardContent`
holds optional media, title, subtitle, supporting text and body views.
The card grows with its content. Actions wrap when the available width
gets smaller. All three kinds use 12 dp corners. Outlined cards use a
1 dp border. Elevated cards paint both Material shadow layers.

Use `Theme::material_card` for a named group with separate actions.
Tab reaches each enabled action. Use `Theme::material_action_card` for a
single press or controlled toggle. The card exposes one named button.
The application owns the toggle value. Enter and Space invoke the card.
Use a localized name that gives users the information needed to act.
Set `restore_focus_to` for a control that can take focus if the card
or an action becomes unavailable.

An actionable card in `CardInteraction::Rest` uses live hover, press
and focus feedback. A caller can set `CardInteraction::Dragged` during
captured drag. The Gallery moves a card between two slots. The Move
button performs the same operation with a click or the keyboard.
Cancelling a drag restores the original position.

In Material Gallery, choose 色彩, then 卡片. Switch among all three
variants and try the whole-card action, selection and reorder.

[Material 3 card guidelines](https://m3.material.io/components/cards/guidelines) ·
[Official Compose card guide](https://developer.android.com/develop/ui/compose/components/card)

## Carousels

Choose a `CarouselKind` from MultiBrowse, Uncontained,
UncontainedMultiAspectRatio, HeroStart, HeroCenter or FullScreen.
Set the allocated width, height, scroll axis and layout direction in
`CarouselOptions`. Give `CarouselLayout::new` the same options and the
item aspect ratios in display order. Mixed ratios range from 9:16 to
16:9. Use the vertical axis for a portrait full-screen carousel.

Keep one `CarouselState` in the application. Pass the layout, state and
items to `Theme::carousel`. Each item has a stable ID, passive content,
a localized action name and an activation message. Include the item’s
position and count in its name. Forward `CarouselInput` to `state.apply`.
Call `state.advance` from `Application::animation_frame` with the host
clock and Reduce Motion preference. When the viewport or motion
preference changes, rebuild `CarouselLayout` with that value.

The contained layouts snap after wheel or captured drag input. The
uncontained layouts scroll freely. Arrow keys follow the scroll axis
and mirror horizontal movement in RTL. Home, End and Page Up or Down
reveal the target before giving it focus. Enter and Space invoke the
focused item. Offscreen and undersized cards leave the focus order.
Horizontal carousels inside a scrolling page let the page keep vertical
wheel input. Offer a separate action that opens the complete vertical
list. The Gallery list returns focus to the control that opened it.

In Material Gallery, choose 動作, then 輪播. Use the layout chips to
switch among all six arrangements. Art inside the cards has a rectangular
clip. Rounded descendant clips and photo rendering remain open.

[Material 3 carousel specs](https://m3.material.io/components/carousel/specs) ·
[Android carousel guide](https://developer.android.com/develop/ui/compose/components/carousel)

## Tooltips

Use `Theme::plain_tooltip` for descriptive text. Give the anchor
`View::described_by` with the tooltip's stable ID. The web proxy and
AccessKit use that description while the tooltip is visible. Focus
stays on the anchor. Use `Theme::rich_tooltip` with an action view to
open a named nonmodal dialog. Put that view after the trigger in
document order so Tab can reach its action. The caller owns the
action and dismissal.

Supply anchor and viewport bounds in logical pixels. On the first
frame, omit `TooltipOptions::measured_size`. Read the tooltip's
measured bounds from the next layout and supply the size on rebuild.
The first layout measures without paint or input. Placement starts
above by default, flips when needed, and clamps to the viewport.
Invalidate the measurement when content, text scale or viewport changes.

`TooltipState` controls one active anchor. Opt into
`Application::observes_frame_input` and forward pointer, focus and
touch state from `animation_frame_with_input`. Include the visible
popup and the gap next to its anchor in the hover region. Hover and
long press wait 500 ms in Surface. Focus and explicit rich activation
open immediately. The input hook suppresses a long press's original
activation. Escape dismisses transient content after IME and modal
handling. The caller can use `cancel_transient` to dismiss even when
the hovered anchor did not take focus.

In Material Gallery, choose 浮層, then 提示泡泡. Try both anchors at the
top and bottom, with and without the caret.

[Material 3 Tooltips](https://m3.material.io/components/tooltips/specs) ·
[Pinned Compose layout](https://android.googlesource.com/platform/frameworks/support/+/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Tooltip.kt) ·
[W3C hover and focus guidance](https://www.w3.org/WAI/WCAG22/Understanding/content-on-hover-or-focus.html)

## Toolbars

Use `Theme::toolbar` with `ToolbarOptions` and `ToolbarContent`.
Place a Docked toolbar at the top or bottom of your layout. Give a
FloatingHorizontal or FloatingVertical toolbar at least 16 dp of
viewport clearance. Supply its leading, core and trailing actions in
logical order. A floating toolbar can pair with a separate FAB.

Use `Theme::toolbar_button` for the Standard or Vibrant palette,
including disabled and controlled toggle states. Each button has a
named 48 dp target and a fixed 24 dp icon. The toolbar exposes a named
action group. Tab enters the group. The axis arrows move focus without
activating actions. Horizontal arrows follow the mirrored layout in
RTL. An overflow button can open a caller-controlled menu.

The application owns Expanded, Collapsed and Hidden. A collapsed
floating toolbar displays its core actions or its paired FAB. A focused
control can supply its own fallback when it disappears. The toolbar
tries the page fallback if that target is unavailable.

Attach `View::on_vertical_scroll_with_focus` to the content scroller.
Use `ToolbarScroll::apply` with Pinned, HideOnScroll or CollapseOnScroll
and a flag that keeps the toolbar visible when appropriate. Pass the
flag while one of the toolbar's controls has focus or the user opts to
keep the toolbar visible. The policy waits for 40 dp of actual downward
movement. Upward movement reveals it. The content receives the full
wheel delta.

In Material Gallery, choose 動作, then 工具列 to try both palettes,
keyboard focus, overflow, a paired FAB and automatic visibility.

[Material 3 Toolbars](https://m3.material.io/components/toolbars/overview) ·
[MDC 1.14.0 Toolbar guide](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/docs/components/DockedFloatingToolbars.md) ·
[Pinned Compose Toolbar layout](https://android.googlesource.com/platform/frameworks/support/+/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/FloatingToolbar.kt)

## App bars

Build a top app bar with `Theme::app_bar`, `AppBarOptions` and
`AppBarContent`. Choose Small, CenterAligned, MediumFlexible,
LargeFlexible or Search. Supply the leading control and actions as views.
`Theme::app_bar_icon_button` provides a named 48 dp target with a fixed
24 dp icon. The bar mirrors start and end in RTL. Flexible bars keep one
title and stable action IDs as they collapse. CenterAligned centers the
title between two equally wide lanes.

Attach `View::on_vertical_scroll` to the clipped content with a stable ID.
Pass the wheel gesture to `AppBarScroll::apply` with the selected
`AppBarScrollBehavior` and `options.collapse_distance()`. Store the
returned bar state in the application message. Pass its collapse
fraction and scrolled state into `AppBarOptions`. Small, CenterAligned and
Search use a pinned height. Flexible bars offer three behaviors. Pinned
keeps its height. EnterAlways expands as soon as scroll reverses. ExitUntilCollapsed expands
after content returns to its start. The runtime applies any unconsumed
wheel delta to the same content.

Use `app_bar_search_entry_in_direction` for the Search slot. Its action
opens a caller-controlled `app_bar_search_view` with a query field and
result views. The field takes initial focus. Escape or a result action
can close the view and restore focus to the trigger. A contextual app bar
uses the Small layout with a selection title and actions supplied by the
application.

In Material Gallery, choose 動作, then 頂部應用列. Try the search and
selection actions and scroll the content with each scroll setting.

[Material 3 App Bars](https://m3.material.io/components/app-bars/overview) ·
[MDC 1.14.0 app bars](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/docs/components/TopAppBar.md) ·
[Pinned Compose layout and scroll](https://android.googlesource.com/platform/frameworks/support/+/ffbddbd1295b99a57b395a5dcfb86af31f451492/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/AppBar.kt)

## Bottom sheets

Use `Theme::bottom_sheet_layout` to anchor supplementary content to the
bottom edge. Supply a `BottomSheetOptions`, a `BottomSheetState` and
`BottomSheetContent`. The caller owns the state and both action messages.
Use `Closed`, `Peek` or `Expanded`. Supply equal heights to open directly
to Expanded. Fit both heights to the available layout.

A standard sheet keeps the uncovered page interactive. A modal sheet
dims the page, limits focus to the dialog, and closes on an outside
press. Both have a keyboard-operated handle and a Close button. Escape
closes a standard sheet from inside the sheet. Escape closes the active
modal first. Focus returns to the opener when the sheet closes. The
header stays fixed while the heading and body scroll below it.

Use `Theme::bottom_sheet_with_motion` to enable vertical drag on the
handle. Give `BottomSheetMotion::new` the same heights and presentation.
Call `set_state` when the application changes state. Forward handle
events to `apply`. Call `advance` on the frame clock and commit its
returned state in the same update. Upward drag expands. Downward drag
collapses a standard sheet or can dismiss a modal. The motion uses
Material durations and honors Reduce Motion. Content scrolling has its
own target below the handle.

In Material Gallery, choose 浮層, then 底部面板. Try both presentations,
open directly to Expanded, and turn on large text in a narrow viewport.
The original `bottom_sheet` and `modal_bottom_sheet` builders remain
available.

[Material 3 bottom sheets](https://m3.material.io/components/bottom-sheets/overview) ·
[MDC 1.14.0 behavior](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/docs/components/BottomSheet.md) ·
[MDC 1.14.0 styles](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/bottomsheet/res/values/styles.xml)

## Side sheets

Use `Theme::side_sheet` to present supplemental content beside the main
view. Keep the open flag and both views in the app. Give the sheet a
stable ID, a localized heading, a Close label and a focus return target.
Send the app's dismissal message when a user closes the sheet.

Choose a presentation and geometry.

| Presentation | Behavior |
| --- | --- |
| `StandardOverlay` | Covers one part of the page. The uncovered main content stays active. |
| `StandardCoplanar` | Reserves space for the sheet and reflows the main view. |
| `Modal` | Places a scrim behind the dialog, blocks the main view and keeps focus in the dialog. |

All three presentations support `Docked` and `Detached` styles. The
panel is up to 256 dp wide. Detached panels leave 16 dp of space at the
outer edges. The edge is relative to the writing direction. `End` is
right in LTR and left in RTL. Choose `Start` for the other edge.

The content scrolls below the fixed heading. Escape closes a standard
sheet from inside the sheet. An active modal takes priority for Escape.
Modal sheets also close on an outside click. When the sheet closes, the
runtime returns focus to the opener or the standard sheet's supplied
focus target.

To enable animated opening and horizontal drag dismissal, use
`Theme::side_sheet_with_motion` and keep one `SideSheetMotion` per
sheet. Call `set_open` on a programmatic state change. Pass horizontal
drag events and their measured travel distance to `apply`. Call
`advance` with the app frame time and Reduce Motion preference and
apply any returned open state. The app still owns its state.

In Material Gallery, choose 浮層, then 側邊面板. The sample switches from
coplanar to modal below 600 dp. Apps can choose their own adaptive
presentation. The variants follow [Material 3 side sheets](https://m3.material.io/components/side-sheets/overview)
and the [MDC Android 1.14.0 implementation](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/docs/components/SideSheet.md).
The size, color and elevation come from the [pinned tokens](https://github.com/material-components/material-components-android/blob/66c334b7946dabf33adfe1a2b7cad6bcaa4ea3ad/lib/java/com/google/android/material/sidesheet/res/values/tokens.xml).

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
- the slider active fill and thumb;
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

- Spring adoption remains limited across the catalog.
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
- `STATE_CHANGE` uses a shared duration and easing. A general radial
  ripple and shape morphing for the catalog remain open.
- Other tokens and components do not use a transition yet.

Motion does not complete the Material catalog. The coverage file keeps
the motion foundation at `partial`.

## Gallery and evidence

Use the [headless native Gallery](devtools-protocol.md#headless-native-gallery)
for tests and PNG capture without a window.
Run `cap cargo run --locked -p surface-material-gallery` for a native window.
Run `cargo xtask web dev material-gallery` for the browser. See
[web build](web-build.md). The same `Gallery` state and view drive both.

The `material` suite of `cargo xtask web e2e`
([`e2e/src/material.rs`](../e2e/src/material.rs)) runs the real
WASM/WebGPU gallery. It checks
selections, keyboard groups, slider drag, both themes, chip removal,
Chinese dialog editing, focus trapping/restoration, menus and responsive
modal bounds. It also reads the live DOM export. The native smoke check
presents two frames. It does not prove every native interaction or variant.

Open gaps include the remaining families and variants, rounded
clipping and RTL mirroring for shapes, heavier font faces and rendered
letter spacing, elevation, finished animation, touch refinements and
platform accessibility. See [verification](verification.md).
### Wide navigation rail

Place `Theme::wide_navigation_rail` and the page in a row.
Give the rail and its destinations the same
`WideNavigationRailMode`. Use stable destination IDs and give
each button a localized name. The expanded layout uses the
logical start for its icon. Let the page fill the remaining width.

For animation, keep a `WideNavigationRailMotion` in the app.
Call `advance` from `Application::animation_frame` and return
its `AnimationUpdate`. Pass `frame()` to
`Theme::animated_wide_navigation_rail`. Pass `frame().rail` to
`Theme::animated_wide_navigation_rail_item_with_icon`.
The icon and indicator move together. The label keeps its
compact wrap and fades through zero before moving beside the icon.
Use the same destination IDs when the layout switches to a bottom
navigation bar.

### Modal wide navigation rail

Mount the rail after the page content. An optional collapsed rail
stays in the page. Use distinct IDs for its destinations and the
active modal. A stable opener ID lets focus return after dismissal.
Close, Escape or a scrim tap sends the dismiss message.

Use `WideNavigationRailMotionKind::Modal` for animation. The
`Retain` variant grows over the collapsed rail. The `Hide`
variant slides from the logical start edge. Use
`Theme::swipeable_modal_wide_navigation_rail` to add swipe
dismissal to the standalone variant. Store a
`WideNavigationRailSwipe` in the app. Pass the viewport to
`set_viewport` and forward the gesture and measured panel width
to `apply`.

The panel keeps its width, named scope and current page during
dragging. The scrim fades with visible width. A slow release
uses half the panel width. A release of at least 125 dp per second
uses its direction. Call `advance` from the animation frame.
When it returns a dismissal, close the app state and call
`WideNavigationRailMotion::snap(false)`. Reduce Motion settles
the endpoint immediately. Escape during a drag cancels it.

Predictive back and direct vertical touch scrolling remain open.
[Material navigation rail](https://m3.material.io/components/navigation-rail/overview)
· [AndroidX modal anchors](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/WideNavigationRail.kt#L1249-L1324)
· [AndroidX release threshold](https://github.com/androidx/androidx/blob/268d841a45644cadf438fc335c793869728449ec/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/gestures/AnchoredDraggable.kt#L1535-L1541)
