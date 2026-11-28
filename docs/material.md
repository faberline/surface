# Material components

This is a working component checkpoint. The full Material catalog remains
the target in [the completion contract](completion-contract.md).
[The coverage file](material-coverage.json) records the 37 published families.
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
- Per-state shape changes cover a subset of the catalog. See the family
  entries in the [coverage file](material-coverage.json).

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
| `baseline_segmented_button` | Two to five equal-width outlined choices with single or multiple selection and a shared border |
| `button_group` | Standard and Connected press groups with one row, five sizes, round or square shapes, and controlled selection |
| `split_button` | Independent leading action and trailing disclosure with five sizes, four appearances, and an optional command menu |
| `command_menu` | Baseline M3 commands anchored to a caller's button, with grouped rows, keyboard focus, and scrolling |
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
| `list` | Baseline Standard, Expressive Standard and Expressive Segmented with passive rows, one action per row or one controlled selection |
| `multi_select_list` | The same three list styles with independent, app-owned selection for each row |
| `multi_action_list` | A primary action and separate supplementary commands or toggles with a shared keyboard order |
| `expandable_list` | Inline disclosure rows that reveal a nested list and move following rows in the same scroll flow |
| `reorderable_list` | Stable row order, a pointer grip, and separate Move Up and Move Down buttons |
| `media_list` | Leading images and video posters with an optional row command or app-owned checkbox |
| `plain_tooltip` / `rich_tooltip` | Anchored descriptive text or a rich popup with optional title, caret and action |
| `navigation_rail` | Classic 80 dp rail with a 56 × 32 dp indicator, controlled selection, arrow focus and named destinations. The vector item builder supports 24 dp icons and scaled labels. |
| `navigation_drawer` / `modal_navigation_drawer` | Standard and modal drawer; 360-pixel panel; 56-pixel destinations; selected and disabled destination; full-width active indicator; modal scrim, focus trap and Escape/outside dismiss; light and dark; text-glyph icons, no badge, section headline or ripple |
| `navigation_bar` / `navigation_bar_item` | Baseline 80 dp bar with a 64 × 32 dp active indicator and arrow-key focus. Destination builders also support vector icons and badges. |
| `flexible_navigation_bar` | Compact Expressive bar with three to five equal destinations, Tab focus and labels that grow with text |
| `progress-indicators` | Linear determinate and indeterminate; light and dark; phase set by the application; no circular variant, no rotation or motion yet |
| `loading_indicator_with_motion` | Default and Contained Expressive indicators with a seven-shape morph, clockwise spin, Reduce Motion, and named indeterminate progress |
| `icon_button_with_options` | Standalone vector commands and controlled toggles in Standard, Filled, Tonal, and Outlined styles. Five sizes, three widths, and two resting shapes. |
| `icon_button` | Glyph-based Standard, Filled, Tonal, and Outlined buttons. Supports the original 40 dp face and 48 dp target. |
| `checkbox_with` | Unchecked, checked and mixed; disabled state |
| `checkbox_with_options` | Controlled Material 3 checkbox with three states, disabled and error, a stable label and RTL |
| `radio` / `radio_group` | Controlled choice; arrows select and skip disabled choices; ring uses primary when selected, on-surface-variant when unselected, outline when disabled |
| `radio_with_options` / `radio_group_with_options` | Named Material 3 radio group with checked-first Tab entry, mirrored horizontal keys, retained focus and wrapping labels |
| `switch` | Controlled on/off choice and disabled state |
| `switch_with_options` | Controlled Material 3 switch with three icon styles, RTL, press motion and a wrapping label |
| `segmented_button` / `segmented_group` | Controlled choice; arrows move focus, Enter/Space selects |
| `scroll_field` | Inline finite numeric values with three sizes, a snap-to-center wheel, direct entry and validation |
| `scroll_choice_field` | Inline finite text choices with stable IDs, qualified labels and direct-entry completion |
| `segment` | Position-aware segment: round outer ends, square inner edges, check mark when selected; single or multi select owned by the application |
| `tab` / `tabs` | Controlled selection, selected underline and manual keyboard activation |
| `tab_bar` / `tab_panel` | Primary and Secondary bars, fixed and scrollable layouts, controlled selection, a measured indicator and named panels |
| `chip` | Assist, suggestion, filter and input; input removal has its own action |
| `chip_with_options` | Flat and elevated Assist, Filter and Suggestion, plus Input with one or two named actions |
| `chip_in_grid` / `chip_grid` | Measured rows and one Tab entry, with arrows between the individual actions |
| `button_with_options` | Material 3 Expressive commands and caller-controlled toggles with localized labels and optional vector icons. |
| `standard_slider` | Horizontal Standard slider. Five track sizes, continuous values or discrete stops, mirrored RTL input, localized value text, and an independent focus outline. |
| `slider` | Original controlled range and fixed track. Supports pointer drag, arrows, Home/End and Page Up/Down. |
| `dialog` | Viewport scrim, adaptive panel, focus trap, Escape/outside dismiss, opener restore |
| `dialog_with` | Static Basic dialog with optional icon, headline, supporting text and actions |
| `basic_dialog` / `basic_dialog_with_motion` | Measured Basic or alert dialog with a scrolling body, fixed actions, focus management and optional fade |
| `full_screen_dialog` / `full_screen_dialog_with_motion` | Full viewport dialog with a fixed Close and Save header, scrolling body and optional transform from a floating action button |
| `menu_at` / `menu_item` | Viewport popup, vertical keyboard navigation, disabled skip and dismiss |
| `menu_item_with` | Menu item with optional leading glyph, trailing text, selected and disabled state; no submenus or checkmark yet |
| `divider` | One-pixel semantic separator |
| `decorative_divider` / `decorative_divider_with_weight` | Full, leading-inset, middle-inset and vertical Material lines. Regular is 1 dp. Heavy is 8 dp. Both use `outline_variant`. |
| `divider_with` / `vertical_divider` | Earlier named semantic separators. |
| `card` / `outlined_card` | Filled and outlined containers |
| `card_with` / `action_card` | Elevated, filled and outlined kinds with M3 container tokens; action card is a button with focus, press and disabled state; elevated has no shadow yet; no ripple, drag or slots |
| `material_card` / `material_action_card` | Elevated, filled and outlined cards with actual shadows, structured content, whole-card selection or independently reachable actions |
| `text_field` | Controlled Unicode editing, selection, clipboard and composition |
| `outlined_text_field` | One single-line editor with a floating label, measured border opening, hint and validation text |
| `filled_text_field` | One single-line editor with a filled container, inside floating label and bottom indicator |
| `text_field_with_adornments` | Either single-line appearance with optional visual affixes and a separate named action |
| `text_field_with_counter` | Either single-line appearance with a count and a soft limit |
| `multiline_text_field` | Either appearance with wrapped lines, automatic height or a fixed number of visible rows |
| `search_bar` | Inline pill with leading glyph, controlled text input and Clear action |
| `search_overlay` / `search_overlay_with_motion` | Contained or Divided Search with adaptive Docked or Full-screen layout, named editor and suggestions, and optional expand and collapse motion |
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

## Checkbox

Use `checkbox_with_options(options, &motion, on_change)` for the Material 3
checkbox. A checkbox has three values. Unchecked draws an outline, checked
draws a check, and mixed draws a dash. Keep the current `CheckValue` in
the application. Space, Enter, tap and accessible press request one new
value. A mixed parent requests checked. Set the children's values in the
application and derive the parent's value from the children.

Keep one `CheckboxMotion` per stable ID. Opt in to
`Application::observes_frame_input` and call `CheckboxMotion::advance`
from `animation_frame_with_press_detail`. Apply its next-frame and
rebuild flags. The mark keeps its geometry at large text sizes. Match
each label to the current text scale and direction.

When the application marks the checkbox invalid, set `error` and pass
the stable ID of visible help in `error_description`. The checkbox
exposes the invalid state and its description. The disabled state
stays readable and rejects activation.

Open `checkbox-demo-show` in the Gallery to try one mixed parent,
its child controls, a disabled row and a check that has error help.
[Material 3 Checkbox specifications](https://m3.material.io/components/checkbox/specs)

## Radio button

Use `radio_with_options(options, &motion, message)` for a selectable
row and `radio_group_with_options(group, rows)` to name its group.
The application owns the selection and can leave all choices unchecked.
The row exposes one radio control for its label and painted circle.
Selecting the current option keeps the value and sends no new request.

The default Tab stop is the checked enabled option. With no checked
option, Tab enters at the first enabled row and Shift+Tab enters at the
last. The four arrow keys skip disabled rows, wrap at the ends, and
request selection of the next enabled row. In RTL, left and right
follow the visual direction. Up and down preserve authored order.
When a focused option disappears or becomes disabled, the group
moves focus to an eligible option and leaves the value to the caller.

Pick a vertical or horizontal group. Labels wrap at the current text
scale and the circle stays at logical start. Keep one `RadioMotion`
per stable ID. Pass input and press details to `RadioMotion::advance`
from `animation_frame_with_press_detail` when the application opts
in to `observes_frame_input`. Apply the returned rebuild and frame flags.
Reduce Motion settles the dot to its target.

Open `radio-demo-show` in the Gallery. Use the clear command to check
entry in each direction when no option is selected.
[Material 3 Radio button specification](https://m3.material.io/components/radio-button/specs)

## Switch

Use `switch_with_options(options, &motion, on_change)` for a Material 3
switch. Keep one `SwitchMotion` per stable ID. The application owns the
checked value and receives the requested next value in `on_change`.
A tap, Enter, Space or accessible press sends one request. Disabled
switches expose their state and reject activation.

Choose `SwitchIcons::None`, `SwitchIcons::Selected` or `SwitchIcons::Both`
for the handle. The visual track stays 52 × 32 dp. The handle is 16 dp
when off without an icon, 24 dp when on or showing an icon, and 28 dp
while held. The label wraps to the space beside the track. The whole
row has one target at least 48 dp high. Use the layout direction and
text scale from the current application settings.

Opt in to `Application::observes_frame_input`. In
`animation_frame_with_press_detail`, pass the current options, frame,
input, press and hover state to `SwitchMotion::advance`. Apply its
rebuild and next-frame flags as you would for other retained motion.
Reduce Motion goes directly to the current endpoint. The earlier
`Theme::switch` API stays available.

The Gallery's `switch-demo-show` opens a preview with all three icon
styles, disabled examples, dark mode, RTL, and 200% text.
[Material 3 Switch specifications](https://m3.material.io/components/switch/specs)

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

## Flexible navigation bar

Use `Theme::flexible_navigation_bar` for the compact Expressive layout.
Pass the exact available width, stable ID, localized landmark name and
direction in `FlexibleNavigationBarOptions`. Supply three to five
`NavigationRailItem` values with a stable ID, label, vector icon and
selection message. You can supply a separate accessible name when the
visible label needs more context.

The bar starts at 64 dp. It uses the Surface Container color and Level 2
elevation. Icons are 24 dp. The selected indicator is 56 × 32 dp. Each
label uses Label Medium and wraps within its equal-width destination. The
bar grows when the labels wrap. The selected label also requests a heavier
weight from the host's fonts.

Place the bar at the bottom of the layout. Give the page above it its own
scroll area so text scaling can grow the bar. The builder returns `None`
at widths of 600 dp or more, when a destination would be narrower than
56 dp, or when the count falls outside the supported range. Use another
navigation layout in those cases.

Tab moves through enabled destinations in reading order. Enter, Space
and pointer input send the destination's message. The application updates
the selection and page. The chosen destination exposes the current page.

Open **彈性導覽列** in the Gallery's **色彩** tab. Try three, four and five
destinations with large text and right-to-left layout.
[Material 3 Navigation Bar specifications](https://m3.material.io/components/navigation-bar/specs) ·
[Accessibility](https://m3.material.io/components/navigation-bar/accessibility)

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

## Expressive loading indicator

Retain one `LoadingIndicatorMotion` while an operation is in progress.
In `Application::animation_frame`, call `motion.advance(&frame, active)`.
Return its `AnimationUpdate`, or combine it with the other updates.
Mount an indicator while the operation is active.

```rust
theme.loading_indicator_with_motion(
    LoadingIndicatorOptions { id: "load-results", name: "Loading results",
        kind: LoadingKind::Contained, size: 48. },
    &motion,
)
```

Use a stable ID and a localized name for the task. Each variant exposes one
read-only progress indicator without a numeric value or a Tab stop.
Default paints the active shape in Primary. Contained uses a
Primary Container circle with an On Primary Container shape. The default
square is 48 dp with a 38 dp active box. Sizes from 24 to 240 dp keep
that ratio. The legacy `Theme::loading_indicator` accepts a manual phase.

The seven shapes follow the pinned AndroidX sequence. A spring moves
between shapes every 650 ms. A separate spin turns once in 4,666 ms.
Surface eases the end of the spring to the next shape by 450 ms so the
rotation stays continuous. Each animation frame updates the paint and
keeps the same accessibility node. With Reduce Motion or without a
clock, the indicator shows its first pose and stops requesting frames.

In Material Gallery, choose 動作, then 載入指示器. Start or stop the task.
Switch size, theme, direction, and text scale. The headless native
Gallery captures its stationary pose.

[Material 3 loading indicator specs](https://m3.material.io/components/loading-indicator/specs) ·
[Pinned AndroidX motion and shapes](https://android.googlesource.com/platform/frameworks/support/+/428942a35fedec39ca7253a473a2ec3be06f614c/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/LoadingIndicator.kt#380) ·
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

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

To add swipe paging, use `Theme::swipeable_date_picker` and keep a
`DatePickerSwipe` with the open session. Send picker input to
`swipe.apply` and handle its effects as above. Call `swipe.observe`
from the input frame hook. Pass the picker ID, session, locale, date
constraints and the queue from `Application::attach_queue`.

A horizontal swipe previews the adjoining month and keeps the draft
date. The direction follows the locale. A narrow grid pans to reveal
the clipped columns first. Start a new swipe from that edge to change
months. A short or canceled swipe restores the current month. Reduce
Motion removes the preview and applies a completed swipe immediately.

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

When a calendar control has focus, Shift+M moves focus to Month and
Shift+Y moves focus to Year. Press Enter or Space to open the list.
The shortcuts leave the date editor and input method in control of
their own keys.

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

### Date range picker

Keep a `DateRangePickerSession` while the dialog is open. Give
`Theme::date_range_picker` the viewport, local today, bounds, locale,
labels and a stable ID. Open the session in Calendar or Input mode.
Forward its input to `session.apply`. Store the inclusive
`CivilDateRange` it returns on `DateRangeEffect::Commit`. Drop the
session on Commit or Dismiss.

Each endpoint must be available under `DateRangeConstraints`. A range
can include a day that is unavailable as an endpoint. Supply
`allowed_span` if the app needs to reject spans that contain specific
days. The calendar starts a new draft after the user selects a complete
range. An earlier second day moves the start. The same day can be both
endpoints. Save commits the draft.

The full-screen calendar keeps Close, Save and the mode switch above
the month feed. Wheel and drag browse the months. Arrow and page keys
move the day focus across months. At most five months are mounted.
The endpoints have two selected states in the accessibility grid. The
range summary stays in the dialog while the user browses. In short
windows the summary scrolls with the content to leave room for dates
and input fields. The month controls stay in reach. Assistive
technology can still read updates to the range summary.

Input mode has separate start and end fields. Call `session.observe_frame`
from the app's input frame hook. Use its return value to request layout
when needed. Call `session.flush_scroll_requests` with the picker ID and
the queue from `Application::attach_queue`. The same frame hook validates
a field when focus leaves it. Enter finishes editing that field. Save
submits both fields. During composition, the input method handles Enter.

In Material Gallery, choose 浮層, then 日期範圍選擇器. Use the same
scene for the [headless Gallery](devtools-protocol.md#headless-native-gallery).
[Material 3 date pickers](https://m3.material.io/components/date-pickers/specs)

## Time pickers

Use `Theme::time_input` for the Expressive Input variant. Choose
`TimeInputPresentation::Inline` for a form. Choose `Modal` for a
dialog with Save and Cancel. Supply the viewport, stable ID, labels,
direction and locale. The caller stores a `TimeInputSession` with the
open editor. Use a new session when the clock format or display digits
change.

The app stores a `TimeOfDay` between 00:00 and 23:59. It represents an
hour and minute. The input supports 12-hour and 24-hour formats. In
12-hour mode, use the AM and PM buttons to change the draft period.
In 24-hour mode, the fields use 00 through 23 and the period group is
absent. The locale can supply display digits and period labels. The
editors accept ASCII and full-width digits as well as the configured
digits.
Use a font that covers the supplied labels and digits. The bundled
fonts cover the Chinese and Japanese examples in Material Gallery.
Other scripts may need an added font.

Forward input events to `session.apply`. Call `observe_input_focus`
from the app's input frame hook to validate the prior field when focus
leaves it. Enter finishes the focused field. The input method handles
Enter during composition. Each field has its own description for its
hint or error. Read `validation().candidate()` for an inline preview.
Store the time only on `TimeInputEffect::Commit`. Drop the modal
session on Commit or Dismiss. An inline form sends Confirm through its
own submit control.
In a short dialog, the body scrolls and the action buttons stay in
view. The dialog keeps its accessible name when the visible heading
is omitted to make room for the fields and their hints.

In Material Gallery, choose 浮層, then 時間輸入. This scene lets you change
format, locale, direction, theme and text scale. Use it in the
[headless Gallery](devtools-protocol.md#headless-native-gallery).
[Material 3 time picker specification](https://m3.material.io/components/time-pickers/specs)

Use `Theme::time_dial` for the modal Expressive Dial variant and its
Input switch. Set the initial mode when you create a `DialState` with
one `TimeInputSession`. Pass the same viewport and text scale to the
view and to `time_dial_layout`. Send `DialEvent::SetDialFits` when the
viewport or text scale changes. A short window opens or switches to
Input with the draft intact. It stays in Input when space returns
until the user chooses the Dial button.
Supply a localized `TimeDialLabels::value_name` for each dial option.
It receives the field, display digits, and number of values on that dial.

Pass all `DialEvent` values to `DialState::apply`. Forward editor focus
changes to `DialState::observe_input_focus` while the dialog is open.
A held pointer changes the hand preview. Release accepts the choice
for that field. Choosing an hour advances to minute selection.
Press Enter on an hour or minute field to focus the first clock mark.
This starts at hour 12 in 12-hour format, hour 00 in 24-hour format,
or minute 00. The draft keeps its value until you choose a mark.
On the face, use the arrows to step through exact values or Enter to
accept the focused mark.
Use Save to commit. Cancel, Escape and the scrim discard the draft
and return focus to the opener.

For animated feedback, keep a `DialHandMotion` with the open draft and
render it through `Theme::time_dial_with_motion`. Pass all dial events,
including viewport changes, to `DialHandMotion::apply` with the same
draft. Advance the hand in the app animation frame with the current
time and Reduce Motion setting. Apply the returned rebuild and frame
request flags. The Standard Fast Spatial spring moves the displayed
hand along the shortest arc. Focus, the draft, and checked values
update immediately. A held pointer and Reduce Motion move the hand
directly to its value.

In Material Gallery, choose 浮層, then 時間錶盤. The scene uses the same
draft in both modes. It supports portrait and landscape layouts with
the full 256 dp clock face and a text fallback for smaller windows.

Use `Theme::time_scroll` for the Expressive Scroll variant. It can sit
in a form or open in a modal dialog. Keep a `TimeScrollState` with one
`TimeInputSession`. Pass the same viewport, format, text scale and
presentation to `time_scroll_layout` and the view. Give inline forms
their allocated picker width. Send `TimeScrollEvent::SetScrollFits`
when the space or text scale changes. The view switches to Input when
space runs out. The user switches back when the wheels fit again.

Forward events to `state.apply(event, reduced_motion)`. Store the
time only on Commit. Pass focused wheel and editor IDs to
`observe_scroll_focus` and `observe_input_focus` as hour or minute.
Call `state.advance(frame.now_ms, frame.reduced_motion)` in the
animation frame. Use its flags to rebuild and request another frame.

Tap a neighboring value, drag, or use a mouse wheel to move one field.
The draft and accessible value update after release or a quiet wheel
interval. Use Up or Down to choose an adjacent value. Left and Right
move between fields in visual order. Home and End reach the bounds.
The keyboard button switches to direct text input and keeps the draft.
Enter on a focused wheel confirms the time. In Input, Enter finishes
the editor and the input method handles Enter during composition.
Use a separate Save action in an inline form. The modal has Save and
Cancel and returns focus to its opener on close.

The published example shows a 100 × 120 dp field. Surface reserves
three 48 dp rows to meet the touch target rule and grows the rows for
larger text. Each field has one adjustable accessibility target with
a localized accepted value. The Web projection uses a spinbutton.
The macOS adapter exposes an incrementor. The neighboring numerals
are paint inside the same field.

In Material Gallery, choose 浮層, then 捲動選時. The scene includes
both placements, format and scale changes, and the Input switch.
[Material 3 time picker accessibility](https://m3.material.io/components/time-pickers/accessibility)

Use `Theme::time_input_baseline` or `Theme::time_dial_baseline` to
choose the Baseline modal style. Pass the effective text scale to
the Baseline Input builder. Use `time_dial_baseline_layout` for the
Baseline Dial fit check. The Baseline Dial shares its draft with the
Baseline Input view. A short viewport uses Input and retains the
draft. The mode button returns to the Dial when it fits.

Call `observe_baseline_input_focus` on the retained `TimeInputSession`
or `DialState` from the app's frame input hook. Rebuild when the method
returns true so the editor fill and outline follow focus. Use the
existing event and commit handling with either Baseline builder.

The Baseline modal uses the High surface color, Level 3 shadows and
24 dp padding at normal sizes. The portrait 24-hour Dial uses the
wider 114 dp fields. The AM and PM controls retain their Baseline
painted track while each reserves a separate 48 dp touch target.
Long captions keep their full accessible names. The Dial contains
their paint within the period and action button bounds.
The Input view can stack its fields and scroll above the measured
action row when larger text needs more space.

In Material Gallery, choose 浮層, then Baseline 時間輸入 or Baseline 錶盤.
[Baseline time picker specification](https://m3.material.io/components/time-pickers/specs)

## Scroll fields

Use `Theme::scroll_field` for a finite sequence of numeric values. Create
a `ScrollFieldRange` with distinct inclusive bounds and a positive
step. The last value must align with the step. Retain a
`ScrollFieldState` for each field and give it an initial value in the
range. Pass each `ScrollFieldEvent` to `state.apply`. Save the new value
when the method returns `Some(value)`. Call `state.sync_value` to apply
an external change.

Set a localized label and value text in `ScrollFieldOptions`. Give the
options the available width, height and effective text scale. Use
`scroll_field_layout` to check fit when those values change. Report the
result to `state.update_fit` and retire the old gesture geometry with
`state.invalidate_layout`. The direct editor stays open across resize
and preserves active composition. It accepts ASCII and full-width
digits. Use `with_digits` to
display a custom set of digits and accept those digits on input.

Use Up and Down to choose the adjacent value. Surface puts the larger
value above so Up agrees with the native Increment action. Tap a
neighbor or drag the wheel to preview a value. A drag commits on
release. Each neighbor has a 48 dp tap area, including the region
beyond the cropped text. The host page still receives wheel scrolling.

Press Enter or Space on the wheel to open the editor. Enter accepts a
valid value. Escape cancels the draft. The editor gives the first Enter
to active composition. An invalid value keeps focus in the editor and
exposes the error and its description to assistive technology.

Small, Medium and Large have minimum widths of 100, 120 and 186 dp.
Pass 120 dp for a compact face or 200 dp for a regular face. The face
grows for larger text. The editor takes over when the wheel and its
targets cannot fit. Medium follows the 120 dp minimum in the published
tokens and measurement diagram. The guidelines prose gives 144 dp.

Call `state.advance(frame.now_ms, frame.reduced_motion)` from
`Application::animation_frame`. Return its frame request while the
Expressive Fast Spatial spring settles. Reduce Motion snaps to the
accepted value.

In Material Gallery, choose 選取, then 捲動欄位. 四位數 shows the editor
fallback in a narrow window with 大字 enabled.

### Text choices

Use `Theme::scroll_choice_field` for an ordered list of text choices.
Give each `ScrollChoiceItem` a stable ID and a localized label. Add a
qualifier and a distinct input alias when two items have the same label.
`ScrollChoiceFieldOptions::choice_text` formats the label and qualifier
for both the wheel and its accessible value.

Create a `ScrollChoiceState` with at least two items and a valid initial
ID. Send field events to `state.apply` and save the ID it returns.
`state.sync_choice` applies an external choice. `state.replace_items`
accepts the new order and the caller's chosen ID in one operation. Both
keep the open editor text until the user accepts or cancels the draft.
Catalog replacement retires pointer capture from the old list.

The direct editor offers a suggestion when a prefix identifies one
choice. It keeps the typed text and waits for Enter. An exact label
or alias takes precedence. If several items match, the editor shows
an error and keeps focus. Escape cancels. The default comparison trims
surrounding whitespace and preserves case. Use `with_match_key` for
locale-specific matching rules.

Call `scroll_choice_field_layout` and `state.update_fit` when space,
text scale or the catalog changes. The editor keeps active composition
when the view becomes wider. A screen reader can query the qualified
label and adjust the choice by its position in the current list.

In Material Gallery, choose 選取, then 文字捲動欄位. The two 台北 entries
have different qualifiers. Type 桃 to suggest 桃園 and press Enter
to select it. 倒序 and 改名 show how the field keeps the selected ID.

### Looping

Call `with_looping()` on a numeric or text-choice state to join the
first and last rows. Up, Down, tap targets, swipes, and native
Increment and Decrement can cross the join in either direction.
A held drag can cross more than once. It changes the accepted value
only when released. Cancel returns the wheel to its accepted value.
`Home` and `End` select the first and last entries. Direct input and
absolute accessibility actions still use the authored bounds.
Catalog replacement keeps looping enabled and retires the old gesture.

In Material Gallery, choose 選取, then 循環數字欄位 or 循環文字欄位.
The text example uses two choices so both neighbors show the same
label. The upper and lower targets still move in their own direction.

### Quantity and unit pairs

Use `Theme::scroll_field_pair` to group a numeric quantity and a text
unit. Set the group label and give each child its own label, stable ID,
state and event handler. The quantity and the unit can change on their
own. The application decides how to use the selected quantity and unit.

Set the available width, available height and direction on
`ScrollFieldPairOptions`. The pair applies these values to both child
options. Use `scroll_field_pair_layout` when the space, text scale or
unit list changes. Pass `quantity_fits` and `unit_fits` to each state's
`update_fit`. Retire old pointer geometry with `invalidate_layout`.
Advance each state with the application frame clock.

The fields sit side by side when they fit and stack in less width. The
quantity comes first in keyboard order. Right-to-left layouts place it
on the right in a row. Put the pair in a scroll container when the
stack can exceed the view height. Each field keeps its own focus,
text draft, validation and editor fallback.

Use 5 dp of scroll viewport padding to give the outer focus ring room
when Tab reveals a field at the viewport edge.

In Material Gallery, choose 選取, then 數量與單位. The example keeps
the number as entered when the unit changes.

### Anchored popup

Keep fields inline when there is room. Use `Theme::scroll_field_popup`
to put an individual field or a pair in a named modeless dialog. The
caller opens and closes it. Pass the visible opener bounds and the
available viewport in the same coordinate space. Give the dialog,
body, Close button and opener distinct stable IDs.

Call `scroll_field_popup_layout` before building its fields. It places
the dialog below the opener when it fits and flips above when needed.
It can reduce the content height when neither side fits the preferred
size. Use `body_width` and `body_height` for the field allocation.
The body scrolls inside the dialog. The Close button stays visible.
A hidden opener or insufficient room yields no popup.

Keep the opener's disclosure state in sync. Place the popup after the
opener in the authored focus order. Keyboard opening can move focus
directly to the first field. Outside controls remain available. Close
returns focus to the opener when focus was inside. Escape gives active
composition and the current editor first use. Apply Cancel to any
remaining draft when the caller closes the popup.

In Material Gallery, choose 選取, then 浮層捲動欄位. Use 靠近底部 to
see the popup flip above the opener. The 外部 button stays available.

### Modal dialog

Use a modal when the inline fields or anchored popup would make the
screen too crowded. Plan it with `scroll_field_modal_layout`. Set the
minimum body height to the height of the tallest compact input target.
`ScrollFieldSize::compact_editor_height` gives that height at the app's text scale.
Use the planned body width and height to build the fields.

`Theme::scroll_field_modal` places the fields inside a named dialog with a
scrolling body. Cancel and Save stay visible. Tab and Shift+Tab stay
inside the dialog. Closing restores focus to the opener when it is
available. A very short viewport can yield no usable placement. Keep
pending edits in caller state so they can return when the space returns.

Open a fresh draft from the saved values. Each field updates that draft
independently. Save can validate both fields, reveal a visible summary
when a value fails, or commit both values and close. Cancel discards
the draft. Composition gets Escape before the dialog can use it.

When focus moves to an editor, Surface reveals its exterior outline.
When space permits, Surface also reveals the error text. A clipped
error can still describe the focused editor on Web and through AccessKit.

In Material Gallery, choose 選取, then 對話框捲動欄位. Try a valid
pair and an invalid pair. Use Escape or Cancel to discard changes.

### Display contrast

`surface-platform` reports the host preference through
`Application::display_contrast_changed`. Store the preference in the app's
appearance state. Return `true` when the view needs a rebuild. Keep the
control IDs and editor values stable to preserve focus and drafts. A
custom host can pass a preference to `Runtime::set_display_contrast`.
`Runtime::display_contrast` reads the accepted preference.

- `DisplayContrast::Normal` is the initial preference. Use the app's
  usual theme, such as `Theme::from_seed(seed, dark)`.
- `DisplayContrast::Increased` signals a request for higher contrast.
  `Theme::from_seed_high_contrast(seed, dark)` uses the same seed and
  light or dark choice.
- `DisplayContrast::Forced(colors)` carries the system Canvas, Button,
  and Highlight color pairs. Each pair keeps its background, foreground,
  and alpha values. Pass the Canvas and Button pairs to
  `Theme::from_forced_colors(canvas, button, dark)` as tuples with the
  background first.

The Web host gives `forced-colors: active` priority over
`prefers-contrast: more`. The macOS host reads Increase Contrast to
report `Normal` or `Increased`.
[W3C media features](https://www.w3.org/TR/mediaqueries-5/#mf-user-preferences)

The wheel, popup and modal draw a border when their surface matches the
surrounding color and they have no border. Disabled Scroll Fields keep
their disabled state and dimmed text. During forced colors, the extended
renderer uses `Highlight` for the selection background and `HighlightText`
for selected text and the selected part of the composition underline.
The rest of the underline uses the ordinary text color. The underline
marks pending text from the input method editor (IME).

Headless Chrome checks M241–M247 cover emulated contrast, popup and wheel
paint, disabled actions, invalid drafts, and selection with synthetic
composition. The macOS Metal replays read the startup setting and use a
test control to switch between Normal and Increased. They check paint
and focus for an invalid popup draft in light and dark themes.
Live operating system notifications, live input methods, VoiceOver
speech, and Windows forced-color captures remain open.
[Contrast verification](https://github.com/faberline/surface/tree/codex/evidence-md3-scroll-contrast-v0142/docs/evidence/md3/scroll-contrast-v0.1.42)

[W3C Spinbutton pattern](https://www.w3.org/WAI/ARIA/apg/patterns/spinbutton/)

[Material 3 Scroll Fields specs](https://m3.material.io/components/scroll-fields/specs) ·
[Accessibility](https://m3.material.io/components/scroll-fields/accessibility) ·
[Published tokens](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.42f95ef205743188.json) ·
[Headless native Gallery](devtools-protocol.md#headless-native-gallery)

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

## Sliders

Use `Theme::standard_slider` to build a horizontal Standard slider.
Set a stable ID, a localized name, the bounds, and the current value.
The caller applies each requested value. Use Continuous to keep the
pointer position as a value and set the keyboard step. Use Discrete
to divide the range into equal intervals, including both endpoints.

Use the same direction for the slider and its surrounding layout.
In RTL, the logical minimum is on the right. Left and Right follow
the physical rail. Up and Down change the logical value. Home and
End reach the exact bounds. Page Up and Page Down use larger changes.

Set `engaged` from the current focus or accepted press in the frame
hook. The handle narrows from 4 dp to 2 dp while engaged. Its hit
area keeps the same size. If you supply a localized value label,
reserve at least 53 dp above the handle and more for enlarged text.
The label also supplies the human-readable accessibility value.

Use `Theme::centered_slider` for one value that moves around a neutral
point. Supply a finite neutral value inside the bounds. For a discrete
scale, choose a reachable stop. The active track runs between the thumb
and neutral. At neutral only the inactive portions remain.

Use `Theme::vertical_standard_slider` with `VerticalStandardSliderPlacement`
for a vertical control. `height` sets the full hit target height in dp and
must be at least 48. The minimum stays at the bottom in both directions.
Up and Right increase the value. Down and Left decrease it.
Reserve trailing space with `label_room` to show the upright value label
beside the control. This space includes the outline and gap. It stays
outside the slider's hit target. The value text remains available to
accessibility when the label has too little space to paint.

Use `Theme::vertical_centered_slider` for a vertical scale with a
neutral value. It uses the same placement and label rules as vertical
Standard. The label stays upright. The active track fills from the
neutral point to the current value. Choose an existing
stop for a discrete neutral value.

Use `Theme::standard_slider_with_inset_icon` or
`Theme::vertical_standard_slider_with_inset_icon` to add a
decorative `SliderInsetIcon`. Provide a `VectorIcon` and an optional
`at_minimum` glyph. Medium and Large use a 24 dp icon. ExtraLarge
uses 32 dp. The icon uses the active track when it fits and the
inactive track when it is the only one that fits. A short track
can omit the icon. The icon stays upright in both orientations
and keeps the slider name, hit target, and keyboard behavior.

Use `Theme::range_slider` with `RangeSliderOptions` for a horizontal
interval. Give the pair and both ends distinct localized names. Store
the lower and upper values together and apply both values from each
`RangeSliderChange`. The change identifies the end that moved.

Both ends use the same scale. They can meet, and each keeps its own Tab
stop. The order stays lower then upper in either layout direction.
The other value sets each end's accessible limit. A track press chooses
the nearer end. When the handles overlap, a horizontal drag picks the end
that can move in that direction. A canceled gesture keeps changes that
the caller has already accepted.

Set the engaged end from the current focus or accepted press. Each
handle has its own hit area. Allow at least 53 dp above the handle
for its value label and more space for large or wrapped text.

[Material 3 sliders](https://m3.material.io/components/sliders/specs) ·
[WAI slider pattern](https://www.w3.org/WAI/ARIA/apg/patterns/slider/) ·
[WAI vertical slider example](https://www.w3.org/WAI/ARIA/apg/patterns/slider/examples/slider-temperature/) ·
[WAI multi-thumb sliders](https://www.w3.org/WAI/ARIA/apg/patterns/slider-multithumb/)

## Common buttons

Use `Theme::button_with_options` with `ButtonOptions::new`. Give each
action a stable ID, a localized label, and the width it can use. The
builder supports Elevated, Filled, Tonal, Outlined, and Text commands.
Set `checked` for a toggle in one of the first four appearances. Apply
the next checked state when the application receives the button message.
Keep the action name stable as the checked state changes.

Set `size` to ExtraSmall, Small, Medium, Large, or ExtraLarge. The
minimum face heights are 32, 40, 56, 96, and 136 dp. Every target is at
least 48 dp high and wide. The measured label can wrap and make the
face taller at larger text scales. Leave 5 dp around the painted face for
its focus outline.

Use `ButtonIcon::Leading` or `ButtonIcon::Trailing` for one decorative
vector icon. Set `direction` to match the container so the icon follows
the logical side. Set `text_scale` from the current app text size.
The button exposes one action to accessibility and uses the visible
label unless you supply a different accessible name.

Round and Square set the resting shape. An on toggle uses the other
shape. During a pointer hold, the surface follows the pressed shape
and clips the ripple to its face. Reduce Motion presents the state
without moving through the spring.

[Material 3 buttons](https://m3.material.io/components/buttons/specs) ·
[WAI button pattern](https://www.w3.org/WAI/ARIA/apg/patterns/button/)

## Icon buttons

Use `Theme::icon_button_with_options` for a standalone vector button.
Set a unique ID and a localized action name. The command takes one
`VectorIcon`. A toggle takes two icons and the current checked value.
Keep the action name stable across states. Handle the message to update
the checked value.

The five sizes each offer Narrow, Default, and Wide faces. Icons keep
their token size when text grows. Each hit axis is at least 48 dp. Leave
space around the button for the focus outline. Its 3 dp stroke keeps a
2 dp gap from the face.

Round and Square specify the resting shape. The checked toggle uses the
other shape. Accepted pointer presses animate the face and focus outline
together with the Expressive Fast Spatial spring. The ripple stays in the
painted face. Reduce Motion uses the exact pressed and resting endpoints.
Reserve the `-face`, `-icon`, and `-focus` child IDs.

Use an existing plain tooltip for the visible action hint. Pass its
stable ID as the description when it adds useful information.

[Material 3 Icon buttons](https://m3.material.io/components/icon-buttons/specs) ·
[WAI Button pattern](https://www.w3.org/WAI/ARIA/apg/patterns/button/)

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

## Outlined text fields

Use `Theme::outlined_text_field` with `OutlinedTextFieldOptions::new`.
Pass a stable ID, a visible label and the current value. The field uses
the width of its parent. It keeps the editor ID while the label floats
and the border opens around the label. A long label can wrap above the
closed outline when it needs more room.

Set the layout direction and text scale to match the containing view.
An empty field can show a placeholder in its value area. Supporting text has its
own line and is linked to the accessible editor. An error replaces that
line and marks the enabled field invalid. The Web and macOS accessibility
paths expose new errors as assertive updates. A read-only field allows
selection and copying. A disabled field ignores input.

Use `outlined_text_field_with_submit` to handle an unmodified Enter.
Input method composition still owns Enter while it is active. Keep the
committed value in the application and apply the input callback there.

In Material Gallery, choose 動作 and 外框文字欄位. The example shows Chinese
labels, validation, read-only and disabled fields. It has light and dark
themes, both directions and a setting for 200% text.

[Material 3 text field specifications](https://m3.material.io/components/text-fields/specs)

## Filled text fields

Use `Theme::filled_text_field` with `TextFieldOptions::new`. The application
owns the committed value, label and optional error. The editor uses the
whole container as its input area. It places the text below the floating
label and keeps the same caret and selection geometry as the label moves.
An empty focused field can show a hint in the text lane.

The top corners follow the Material Extra Small shape. The bottom
indicator is 1 dp at rest and 3 dp when focused. Text scale can grow the
container. Long labels can wrap above it when they need more room.
Supporting text, errors, read-only and disabled behavior work as they do
for outlined fields. Use `filled_text_field_with_submit` for Enter.

In Material Gallery, choose 動作 and 填色文字欄位.

[Material 3 text field specifications](https://m3.material.io/components/text-fields/specs)

### Text field adornments

Use `Theme::text_field_with_adornments` for either appearance. Pass the
ordinary `TextFieldOptions` and `TextFieldAdornments`. Each option can be
omitted. Supply localized spoken text for a prefix or suffix. The
application receives the input value by itself and handles Enter through
the optional submit callback.

The trailing button has its own name, focus and 48 dp target. Use its
message for a separate action. Set the button's disabled flag if the
action should be unavailable on a read-only field. A disabled field also
disables the button. A pointer press leaves an active composition in the
editor. Tab reaches the field before the button in either direction.

If space is tight, affixes wrap below the field in readable text. The
decorative icon can disappear, and the trailing button can move below
the field with the same focus identity.

In Material Gallery, choose 動作 and 文字欄位配件 to try both appearances.

### Character counter

Use `Theme::text_field_with_counter` with `TextFieldCounterOptions`.
Set the grapheme limit, the localized error and the spoken count format.
An accented letter, a single emoji or a joined emoji counts as one
character when it forms one grapheme. The counter reads the app's
committed value. It excludes active IME text, prefixes and suffixes.

The counter appears at the end of the supporting row. In narrow layouts
it moves below the help or error. It remains passive as the count changes.
When the value exceeds the limit, the field retains the full text and
shows the localized limit error and an error-colored count. An explicit
validation error takes precedence. The count uses its normal color while
the value is within the limit. The app decides when to submit the value.

In Material Gallery, choose 動作 and 文字欄位字數 to try both appearances.

### Multiline text fields

Use `Theme::multiline_text_field` for a multiline Outlined or Filled
field. Choose `MultilineTextFieldSize::Grow` to start with one visible row
and change the height as the text wraps. Choose `FixedRows(NonZeroU16)`
to keep the number of visible rows constant and scroll longer text
inside the field.

Keep the ID stable across appearance, width and value changes. Pass
the authored value through `TextFieldOptions`. The app receives text
updates through `on_input`, including line breaks inserted by Enter.
Use a separate button with its own action when the user must submit
the text. The field can also show adornments, descriptions, validation
and a counter. The count excludes active IME text from the value.

Up and Down follow visual lines. Page Up and Page Down move by the
number of rows in the editor's viewport. Hold Shift to extend the
selection. With wheel input over the editor, the field uses the
available vertical scroll first. The nearest enclosing scroll region
can use the remaining movement.

In Material Gallery, choose 動作 and 多行文字欄位 to try both appearances.

Inactive textboxes keep their committed value in the Web accessibility
tree, including disabled fields. Chrome checks confirm that this value
preserves line breaks and spaces. When an enabled field gains focus,
its active editor provides the value and keeps the same name and
descriptions.

### Floating label motion

Outlined and Filled fields move the label when focus or the value
changes. The Material Standard Fast Spatial spring moves and resizes
one painted label. An Outlined field opens or closes its border notch
from the same progress. The editor and its action retain their
measured positions.

Reduce Motion settles the label and notch immediately. A label that
needs more space wraps above the field and keeps the outline closed.
While an empty label moves out of the value row, the hint waits for
that row to clear.

The Material Standard Fast Effects spring changes the label color in
both appearances. It blends the colors in Oklab, a color space that
makes changes in color more even to the eye. The spring keeps its
current color and speed when focus or validation changes again.
The above-field label shares that color during a resize.
Reduce Motion, a disabled field, or a new theme settles the color
immediately. The validation state and its description update at once.

### Placeholder motion

An expanding label reveals the hint when an empty field gains focus.
The Material Standard Slow Effects spring fades it in. The Fast Effects
spring fades it out on blur. A label that wraps above leaves space for
the hint at rest. Clearing an unfocused above-label field fades the hint
back in. Reduce Motion applies each endpoint immediately.

The hint uses the existing value area and its own painted opacity.
Committed text or active composition hides it immediately. This also
applies to an empty composition. The editor keeps its value and name.
An empty disabled field with an above-field label uses the disabled ink.

[Material placeholder transition phases](https://android.googlesource.com/platform/frameworks/support/+/26065e0db6b4c61d1779c89ad388ea75ce98235a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/internal/TextFieldImpl.kt#2048)

### Field boundary motion

The Outlined border and Filled indicator keep the Material 3 widths of
1 dp at rest and 3 dp when focused. The Filled indicator changes width
with Standard Fast Spatial. The Outlined border changes width with
Standard Fast Effects. Both use Fast Effects in Oklab for boundary ink.
Each track keeps its current position and speed if the target changes again.

The border still opens around the measured label. The width changes
inside the field, so the editor, caret and action stay in their measured
positions. Disabled fields and Reduce Motion apply width and ink
immediately. A theme change applies the new boundary ink immediately.
A re-enabled field uses enabled ink from its first frame.

[Material 3 text field specifications](https://m3.material.io/components/text-fields/specs) ·
[Filled indicator motion](https://android.googlesource.com/platform/frameworks/support/+/26065e0db6b4c61d1779c89ad388ea75ce98235a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TextField.kt#773) ·
[Outlined border motion](https://android.googlesource.com/platform/frameworks/support/+/26065e0db6b4c61d1779c89ad388ea75ce98235a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TextFieldDefaults.kt#1164)

### System colors for disabled field text

Build fields with `theme.text_fields_for(contrast)` when the application
already follows the host's display preference. Use `single_line` or
`multiline` on the returned builder. In forced colors, both appearances use
the host's disabled ink for their captions, values, hints, supporting text,
counters and disabled glyphs. When the host does not supply that color,
the field uses its Material disabled ink.

On the Web, Surface resolves the browser's `GrayText` system color. For a
repeatable native preview, run the headless Gallery with
`--forced-colors light` or `--forced-colors dark`. The preview uses the
fixed testing palettes in the CSS Color Adjustment specification.

[W3C system colors](https://www.w3.org/TR/css-color-4/#css-system-colors) ·
[Forced colors mode test palettes](https://www.w3.org/TR/css-color-adjust-1/#forced-colors-mode-color-palettes)

## Baseline segmented buttons

Use `Theme::baseline_segmented_button` for two to five related choices in
one outlined row. Pass the actual available width, layout direction and
text scale. Give the group and its items stable IDs. Each item supplies
text, a vector icon with a spoken name, or both. The application applies
the messages and passes the current selection to the next view.

Use `BaselineSegmentedSelection::Single` for one required choice. It
exposes a named radio group. Use `Multiple` for independent checkboxes.
It allows any number of values, including zero. Disabled choices stay
visible and do not accept focus or activation.

Tab enters on the first enabled item. Left and Right move focus in the
visual direction and skip disabled items. Space or Enter activates the
focused choice. The default face is 40 dp tall. The 48 dp minimum target
stays in place when labels wrap and the row grows. When the targets cannot
fit, the row scrolls with touch, a wheel, keyboard focus, or an
accessibility reveal action. A vertical touch drag can scroll the enclosing
page. The icon moves above a wrapped label when this saves space. Choose a Connected button group for the Expressive style.
Existing segmented helpers retain their behavior.

In Material Gallery, choose 動作, then 分段按鈕. The example includes left
and right layout directions, light and dark colors, and 200% text.

[Material 3 segmented buttons](https://m3.material.io/components/segmented-buttons/overview) ·
[Material 3 button groups](https://m3.material.io/components/button-groups/overview)

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

## Lists

Call `Theme::list` with `ListOptions` and an iterator of `ListItem` values.
Set a stable list ID and a name for assistive technology. Give each item its
own stable ID. Keep the ID when the item changes position. Reserve the item's
derived IDs for `-item`, `-overline`, `-headline`, `-supporting`, `-leading`,
`-trailing` and `-mark`.

| `ListStyle` | Presentation |
| --- | --- |
| `Baseline` | Baseline Standard with square unselected rows and 24 dp icons |
| `Standard` | Expressive Standard with 4 dp resting corners and 20 dp icons |
| `Segmented` | Expressive rows with 16 dp outer corners, 4 dp inner corners and 2 dp gaps |

Use `ListLines::One`, `Two` or `Three` to set a 56, 72 or 88 dp minimum row
height. Text wraps and can make a row taller. The row can include an overline,
supporting text, a leading vector icon and a short trailing label. Put essential
information in the headline or supporting text. Set `direction` to mirror the
slots with `LayoutDirection::RightToLeft`.

Set `ListMode::Passive` for readable content and `ListMode::Action` for one
command per row. Use `ListMode::SingleSelect { selected_id }` for one selection.
In SingleSelect mode, set `on_activate` to a message that carries the row ID.
Update `selected_id` in the app when that message arrives. The row displays a
check mark and selection color. Rows with no action are disabled in both
interactive modes.

Tab enters the selected or first enabled row. Up and Down move focus and wrap
at the ends. Home and End move to the first or last enabled row. Enter or Space
runs the row action. Web exposes single selection as a listbox with options.
Native exposes it as a radio group. The selected value and action are shared.

Place a long list in a named clipped container and apply
`View::accessibility_scroll_pages` to that container. Surface reveals focused
rows when they move outside the visible area. The Gallery **清單** scene shows
all three styles with light and dark themes, RTL and 200% text.

Expressive action and selection rows animate to 12 dp corners on hover and
16 dp on press or focus. Selected rows use 16 dp. Fill, state layer and focus
outline use the same presented shape. The row keeps its size and hit area
throughout the change. This also applies to Expressive rows with media.
Surface uses its 200 ms `STATE_CHANGE` timing. Reduce Motion and hosts without
a frame clock apply the target shape immediately.

A pointer press on an enabled action, selection or media row starts a
bounded ripple at the press position. The ripple uses the current row
corners and ends at the scroll clip. A media row paints the ripple over
the image and below its text and focus outline. A cancelled contact clears
the ripple. Reduce Motion and hosts without a frame clock use the static
pressed state. Custom hosts use `Runtime::render_extended` for the ripple.

### Multiple selection

Use `Theme::multi_select_list` with `MultiSelectListOptions` and the same
`ListItem` values. Pass an `is_selected` function that reads the app's set of
selected IDs. Set each row's `on_activate` to a message with its stable ID.
In the app's `update`, add or remove that ID. The other selected IDs stay
selected. The trailing checkbox and the rest of the row use the same action.

Tab enters the first enabled selected row, or the first enabled row when
there is no selection. Arrow keys move focus. Enter and Space toggle the
focused row once per key press. Web exposes named options in a multi-select
listbox. Native assistive technology receives a checkbox and checked state for
each row.
Choose **多選** in the Gallery to try the mode.

### Multiple actions per row

Use `Theme::multi_action_list` with `MultiActionListOptions` and
`MultiActionListItem` values. Give the primary action and each
`ListSupplementaryAction` its own stable ID. Set the supplementary action's
accessible label and icon. Set `checked` for an app-owned toggle. A missing
message disables its own action. The primary and supplementary actions work
independently.

Pass the available width inside the parent padding. Rebuild with the new
width when the parent resizes. Every target needs at least 48 × 48 dp. The
method returns `None` when a finite width of at least 48 dp is unavailable.
When the actions and text need more room, the supplementary buttons wrap
below the primary action. Stable IDs preserve their focus through reflow.

Tab enters at the first enabled action from either direction. Right and
Down move to the next action. Left and Up move to the previous action. The
order wraps across rows and follows the authored order in LTR and RTL.
Enter and Space invoke the focused action. The Gallery **清單多動作** scene
shows independent commands, a controlled toggle, and narrow reflow.

### Expandable rows

Use `Theme::expandable_list` with `ExpandableListOptions` and
`ExpandableListRow` values. A `Leaf` holds one `ListItem`. A `Branch` holds a
header `ListItem`, its child rows, the app's `expanded` value and a reference
to `ListExpansionMotion`. Give the header and each child a stable ID. Give
the header an `on_activate` message that changes `expanded`.

Initialize one `ListExpansionMotion::new(expanded)` per header. Retain it
across builds. In the app's `animation_frame` hook, call
`motion.advance(header_id, expanded, &frame)` and return its update. For
more than one disclosure, combine their two update flags with logical OR.
Keep the child data available during closing so Surface can paint it.

The body grows in place and uses the Material Standard spatial spring.
This is Surface's choice of spring for the Lists animation. Reduce Motion
or a frame without a clock shows the endpoint in one frame. The open list
uses the children's natural height, so changes to width or text scale can
reflow rows. In the Segmented style, each child has a 2 dp gap before it.
The open container has 16 dp exterior corners. The decoration at the end
of the header uses Surface when closed and Surface Container when open.

The header and all enabled rows share one Tab stop. Up and Down move
through the header, its open children and the next row. Enter or Space
runs the focused action. Assistive technology gets the header's expanded
state. The children become interactive after the opening animation.
Closing removes their actions immediately. If a child has focus, it
returns to the enabled header. When the app opens a branch above a
fully visible focused row, the scroll area keeps that row in view.
Try **可展開清單** in the Gallery.

### Reorderable rows

Use `Theme::reorderable_list` with a `ReorderableListOptions` value and
`ReorderableListItem` values. Place the list in the named clipped scroll
area. Pass its available width and a revision of the app's row order.
Give each row a stable ID and localized names for its up and down buttons.
Set `movable` to false to keep a row fixed. The other rows stay on their
side of that row. The primary command works independently.

Retain one `ListReorderState` for each list. Forward its input to
`state.update(input, revision, &order)`. The order is a slice of
`ListReorderEntry` values. If the app accepts the request, remove `item`
and insert it before the returned stable ID. A missing `before` ID means
the end of the list. Change the revision whenever order or movability changes.
Use a visible live status to announce the new position.

Call `state.advance(options, &order, &frame, Some(&queue))` from the app's
animation frame hook to update the preview and scroll at the edge. Use the
returned update flags. A drag changes the preview until release. The drop
uses the measured positions from the last rendered frame, including the
scroll offset. Call `state.cancel()` when leaving the list or replacing the
interaction. Escape and host pointer cancellation discard the drag.

The same order is available through the up and down buttons with a click,
tap, Enter, Space, or an accessibility press. Arrows move focus through
each enabled action, and the group uses one Tab stop. If a move reaches a
boundary, focus stays on the row when another action can receive it. At
narrow widths the separate 48 dp controls wrap below the headline.
Try **可排序清單** in the Gallery.

### Rows with media

Use `Theme::media_list` with `MediaListOptions` and `MediaListItem` values.
Wrap a `ListItem` and give it optional `ListMedia`. A supplied image fills
the leading slot. Text-only rows can keep their leading icon. Retain the
image and pass a reference to it on each build.

Create a `surface::RasterImage` from straight-alpha sRGB RGBA8 pixels in
top-to-bottom row order. `RasterImage::new` takes the pixel dimensions and
tightly packed bytes. The extended renderer crops the image around its
center and clips it to its own corners and the scroll area. For standalone
art, use `Node::raster_image` and size that node through its style.

| Profile | Preferred media frame |
| --- | --- |
| `Thumbnail` | 56 × 56 dp |
| `VideoSmall` | 100 × 56 dp |
| `VideoLarge` | 114 × 64 dp |
| `WideThumbnail` | 80 × 80 dp for a wider layout |

Use the width allocated to the list after insets. The media frame can
shrink with its aspect ratio intact. At very narrow widths, content stacks
and keeps a full row target. Expressive rows use 8 dp media corners. Baseline
rows use square corners. RTL mirrors the slot order and keeps the picture's
orientation. Text can wrap and grow to 200% scale.

At normal widths, a passive Baseline video poster starts at the leading
edge. Interactive Baseline video rows inset the poster by 3 dp to keep the
inside focus outline visible.

Use `Passive` for readable content and `Action` for one app command per
row. Use `MultiSelect` with a selection function for one checkbox action
per row. Tab enters or leaves the group, Up and Down move focus, and Enter
or Space uses the focused row. The image adds no focus stop. Put useful
image information in the row text. The application handles opening details
or playback when a poster row activates.

Raster pixels stay in live resources. DOM snapshots and legacy paint
exports keep the existing wire format. Serializing an extended frame with
a live image returns an error. Use `Page.screenshot` to capture the pixels.
Custom hosts use `Runtime::render_extended` with the extended renderer.
The native and browser Gallery hosts already use this path.

[Material 3 list specs](https://m3.material.io/components/lists/specs) ·
[Material 3 list guidelines](https://m3.material.io/components/lists/guidelines) ·
[Material 3 list accessibility](https://m3.material.io/components/lists/accessibility) ·
[WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/) ·
[WCAG dragging movements](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html)

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

## Search

Mount `Theme::search_overlay` beside the ordinary page. Keep `open`,
`query` and the result data in the application. Pass the viewport and the
un-inset resting bar allocation in window coordinates. Use
`SearchStyle::resting_bar_bounds` for the visible launcher. Recompute the
anchor when the page reflows. Use a unique, stable `id` for each overlay.

Choose Contained to keep the pill while open. Choose Divided for a row
with a divider above the results. Search uses the full safe viewport
below 600 dp or when the anchor cannot fit the docked panel. From 600 dp,
a fitting docked panel has a scrim and a fixed input above scrollable
results. Keep the same editor and result IDs through resize.

Use a short localized placeholder and the matching accessible input
label. Use `results_status` for a count, an empty result or suggestions.
Pass any visible empty state as part of the results view. Give every
result a stable ID. Supply `first_result` with the first enabled result
so ArrowDown can move there. That action moves focus. Selection and
navigation stay in the caller's update messages.

Pointer, Space and Enter activate the launcher. Tab moves to the
launcher in normal focus order. Opening moves focus to the query field.
Enter submits the query after composition. A focused result uses its
normal activation message. Clear returns to the editor. Back, Escape
and an outside click dismiss Search and restore the opener. The
application can supply a fallback when that opener is removed.

For motion, keep one `SearchOverlayMotion` for the view. On every
`Application::animation_frame`, pass `options.motion_target()` to
`motion.advance`. Return its `AnimationUpdate`. Pass the same geometry
and state to `Theme::search_overlay_with_motion`. Opening keeps the same
live editor. Closing immediately returns focus to the opener. The
remaining shell paints while the motion settles. Reduce Motion, a
missing clock and an unfit anchor use the settled view.

In Material Gallery, choose 動作, then 搜尋浮層. Try both styles, dark
colors, RTL, 200% text and the Chinese document list. At 599 and 600 dp,
try a selection and type while the layout changes.

[Material 3 Search](https://m3.material.io/components/search/overview) ·
[Headless preflight](https://github.com/faberline/surface/tree/8500ce31b12126e0268b3da1552f1e857f90435d/docs/evidence/md3/search-adaptive/preflight)

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

## Command menus

Use `Theme::command_menu` next to a button that opens a group of commands.
Give the button, menu and each entry a stable ID. Pass the button's ID and
the accessible menu name in `CommandMenuOptions`. Supply each action as a
`CommandMenuEntry::Item`. Use `CommandMenuEntry::Separator` between groups.
Each item can show a leading or trailing icon. Its label names the action.

Store `open` and one `CommandMenuMeasurement` per menu in your application.
Call `measurement.advance(menu_id, trigger_id, &frame)` in your animation
callback and return its `AnimationUpdate`. The first frame measures the
visible button. Subsequent frames track the natural menu height and changes
to the viewport. Pass the measurement back to the builder. Open on the
button message, then close on a command or the dismissal message.

The menu follows the button's logical leading edge. It can open above a
low button or scroll when rows take more height. The preferred width is
112 to 280 dp. Rows start at 48 dp and grow for wrapped text. The width
can shrink in a narrower viewport.

The trigger exposes its expanded state. Arrow keys, Home and End move
focus. A disabled command receives focus so a user can discover it.
Pressing, clicking or invoking that command leaves it inactive. Escape
and outside clicks close the menu. Tab and Shift+Tab close it and move
to the next or previous control. Set `restore_focus_to` for a fallback
when the button can disappear.

Open **命令選單** in the Gallery's **浮層** tab. Try the bottom trigger
with right-to-left layout and large text.
[Material 3 Menu specifications](https://m3.material.io/components/menus/specs) ·
[WAI menu keyboard guidance](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/#keyboard-interaction)

## Dividers

Use `Theme::decorative_divider` to separate adjacent content with a 1 dp
line. For an 8 dp line, pass `DividerWeight::Heavy` to
`Theme::decorative_divider_with_weight`. Both weights draw with
`outline_variant` and support the same `DividerLayout` values.
[Google Material layout specs](https://m3.material.io/components/divider/specs) · [Google Material Heavy style](https://github.com/material-components/material-components-android/blob/6061a69915d04f11230dbf6f417bc27d9c3056ad/docs/components/Divider.md#adding-heavy-divider)

For horizontal lines, `Full` spans the parent. `Inset` leaves 16 dp at
the leading edge and follows the layout direction. `MiddleInset` leaves
16 dp at both edges. Give a vertical divider a row with a defined height.
Use headings to label content groups. Decorative lines stay out of the
focus order and accessibility tree. The existing semantic separator
builders remain available for callers that need a separator role.

Open **分隔線** in the Gallery's **色彩** tab. Toggle **粗線** to compare both
weights in the same layout. The scene supports light and dark colors,
right-to-left layout and 200% text.

## Dialogs

Use `Theme::basic_dialog` for a short choice or message. Give
`BasicDialogOptions` a stable `id`, an accessible `label`, the viewport
size and the desired focus and dismissal policy. Supply one confirmation
and an optional cancel action in `BasicDialogContent`. Use `extra` for
content in the scrolling body. The heading and actions stay in place.
The layout measures action captions at the current text scale. It stacks
the actions when they do not fit side by side and preserves their key order.

Set `alert` for an urgent message. Use `ModalDismissal::ActionOnly` when
the user must acknowledge it. Choose the initial focus for the actual
task. The framework restores focus to the opener or the configured fallback.

Use `Theme::full_screen_dialog` for a task on a compact display. It keeps
Close, the heading and the commit action above the scrolling body. Pass
the same dialog and editor IDs if the app chooses a Basic dialog at a
wider window size. Keep the draft and its save or discard policy in the
app. Close and commit wait for active text composition to finish.

For a Basic or alert fade, keep a `BasicDialogMotion` for each dialog.
Call `advance` from `Application::animation_frame` and return its
`AnimationUpdate`. Build
the dialog with `Theme::basic_dialog_with_motion`. Keep its view at the
same layer position while it exits. The exit keeps paint until it settles.
Focus, input and accessibility return to the previous scope at closing.

For a Full-screen dialog opened from a floating action button, keep a
`FullScreenDialogMotion`. Read the visible button bounds with
`FullScreenDialogAnchor::measured` while the source is laid out. Pass the
button's visual corner radius and fill. Retain the valid anchor for the
transition and update it when the viewport moves. Call `advance` from
`Application::animation_frame` with the open state, anchor and viewport. Pass those inputs
to `Theme::full_screen_dialog_with_motion` through
`FullScreenDialogTransition`. The panel grows from the button and scales
the body within the changing rounded edge. A missing anchor uses the
settled dialog. Both motion controllers honor Reduce Motion.

Open **對話框** in the Gallery's **浮層** tab for the Basic, alert and
adaptive editor examples. Open the separate **全螢幕轉場** scene
to test the measured launcher. The same examples run in the
[headless native Gallery](devtools-protocol.md#headless-native-gallery).

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

`motion::STATE_CHANGE` is the Surface choice of 200 ms (`duration::SHORT4`)
with the `STANDARD` easing. Components that use it include:

- the filled button's visual container;
- the checkbox box;
- the radio outer ring and its dot;
- the switch track and thumb;
- the segmented button container;
- the chip container;
- the slider active fill and thumb;
- the tab active indicator;
- the Expressive list row's shape and colors.

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

Custom controls can use `Node::animate_interaction_corners` with
`Node::interaction_corners` and a style transition. The Runtime treats the
active interaction corners as the target and uses the current shown corners
when the target changes during a run.

For a custom ripple, add `Node::press_ripple(None)` to a named control
with a press action. Use the style's state layer and the node's press
alpha to set the ink. Pass the stable ID of a passive raster leaf to
paint the ripple after that image. Place the leaf before the row's text
and controls. Surface uses 225 ms for growth, 75 ms for fade-in and
150 ms for fade-out after release and full growth.

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
- `STATE_CHANGE` uses a shared duration and easing. Radial ripple and shape
  morphing still cover a subset of the catalog.
- Other tokens and components do not use a transition yet.

Motion does not complete the Material catalog. The coverage file keeps
the motion foundation at `partial`.

## Tabs

Keep one selected stable ID and one `TabBarMotion` per bar in the app.
Pass them to `Theme::tab_bar` with a unique bar ID and named items.
Each item has its own stable tab and panel ID and sends one selection
message. The app sets the selected ID when it accepts the message.
Set `TabActivation::Manual` for panels that take time to open. Automatic
activation loads the panel when arrow navigation moves focus.

Use a fixed bar when each equal cell can keep its 48 dp minimum.
Supply the width that the caller's layout gives the bar. Choose the
scrollable layout when the labels need more room. The leading edge
follows the text direction. Pass drag messages to `TabBarMotion::apply`.
Call `advance` and return its frame update before drawing. Call
`observe_interaction` from the app's press-detail frame hook so labels
and icons use the current hover, focus and press colors.

Place `Theme::tab_panel` just after its tab bar in the same parent.
Mount the active panel only. `TabPanelEntry::Panel` puts a text-first
panel in the Tab order. Use `TabPanelEntry::Contents` if its first
meaningful child can take focus. The hosts name the panel from its tab.
On Web, each inactive tab has an empty hidden target with the same ID
that its panel uses when mounted.

[Material 3 Tabs](https://m3.material.io/components/tabs/specs) ·
[WAI Tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)

## Chips

Use `Theme::chip_with_options` for a standalone chip. Use
`ChipContent` to supply the actions for its type. The app owns Filter
selection and Input removal. For a split Input chip, supply a body
action and a localized Remove action. Leave the body action empty to
make the whole chip remove the token.

Use `Theme::chip_in_grid` for chips in a named `Theme::chip_grid`.
Each split action gets its own cell and keeps its action role. Keep a
`ChipGridLayout` for each set. Give the chips their available width as
a cap. Call `advance` from the frame hook and pass the resulting row
ranges to the grid. This keeps the rows used by arrow navigation in sync
with the rows that fit on the screen. Pass the same text scale to the
chips and the layout.

Each action keeps a 48 dp minimum target. The 32 dp face grows with
larger text. A split chip shares that face but gives Remove its own
focus ring and press paint. At narrow widths, a Filter menu can use
`MenuOnBody` for a single action. The Gallery makes every row available
in one vertical viewport.

[Material 3 Chips](https://m3.material.io/components/chips/specs) ·
[WAI layout grids](https://www.w3.org/WAI/ARIA/apg/patterns/grid/#layoutGrid)

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
