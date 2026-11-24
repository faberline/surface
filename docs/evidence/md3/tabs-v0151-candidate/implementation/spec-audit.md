# Surface Material 3 Tabs audit

This read-only audit uses the exact Radio v0.1.50 candidate. Its 607-file
manifest SHA-256 is `a031c8b5dc2700f69cb131182842722976d57726dd01c2e0a2d99e643e406544`.

## Primary sources

- Google [Overview](https://m3.material.io/components/tabs/overview), [Specs](https://m3.material.io/components/tabs/specs), [Guidelines](https://m3.material.io/components/tabs/guidelines) and [Accessibility](https://m3.material.io/components/tabs/accessibility) are all in the [current page export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/f37f47ba-8e67-4f1e-98a3-3bea94879e52.json). SHA-256 `3b7a96ed176b32c403e78ec3d2b2d5d8dde459d889c406d9a6178f286079cba9`.
- The official [component token export](https://m3.material.io/_dsm/data/dsdb-m3/2026-11-19_06-20-01/TOKEN_TABLE.6039f0080a95df1e.json) identifies DSDB `41.1.12`, system revision `7989` and Tabs revision `6134`. SHA-256 `de958506ea78b70410b73d36515c8057ca8227c54b247d4f98e99cdf045a47b9`. The [viewer](https://m3.material.io/static/angular/main.2bcb6530e93abb29.js) resolves the exact paths. All observed source hashes appear in `SHA256SUMS`.
- The [Android Developers Tabs guide](https://developer.android.com/develop/ui/compose/components/tabs) gives controlled tab activation and text, icon and disabled options. Its sample uses two lines of label text and truncates longer copy. The pinned [Tab](https://android.googlesource.com/platform/frameworks/support/+/de12189a63496bd8aed010be245ce96a94f5873a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/Tab.kt), [TabRow](https://android.googlesource.com/platform/frameworks/support/+/de12189a63496bd8aed010be245ce96a94f5873a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TabRow.kt), [Primary tokens](https://android.googlesource.com/platform/frameworks/support/+/de12189a63496bd8aed010be245ce96a94f5873a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/PrimaryNavigationTabTokens.kt) and [Secondary tokens](https://android.googlesource.com/platform/frameworks/support/+/de12189a63496bd8aed010be245ce96a94f5873a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/SecondaryNavigationTabTokens.kt) separate fixed and scrollable implementations and show component motion.
- The [WAI APG Tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/) specifies the tablist, tab, and tabpanel relationships. It distinguishes manual activation from automatically loading content on focus and gives a condition for making the active panel a Tab stop.

## Kind, size, and paint

Both variants use a single horizontal strip and one active destination.
Primary tabs group top-level content below an app bar. Secondary tabs
subdivide content farther down. This is one family with two visual kinds.
Fixed and scrollable are separate layout choices for either kind.

| | Primary | Secondary |
| --- | --- | --- |
| Resting active label and icon | Primary | OnSurface |
| Resting inactive label and icon | OnSurfaceVariant | OnSurfaceVariant |
| Label-only row at normal text | 48 dp | 48 dp |
| Optional icon and label | 64 dp stacked, 24 dp icon | Official illustration shows a 48 dp inline row, 24 dp icon and 8 dp gap |
| Active indicator | Primary, 3 dp high, content-matched width with 24 dp minimum. Top corners 3 dp, bottom corners square. | Primary, 2 dp high, full tab width, square |
| Container | Surface, no elevation, square | Same |
| Bottom divider | 1 dp SurfaceVariant | Same |
| Keyboard focus | Secondary, 3 dp inward stroke | Same |

Both use Title Small. Base size and line height are 14 and 20, with medium
weight in the default context. Keep the full visible label and accessible
name through layout negotiation. Respect a 48 × 48 minimum hit area. Add
height for large text or select a suitable scrollable presentation. Keep
the accepted selection indicator below the tab label and focus paint
independent of selection. Google explicitly tokens the shared focus
`inner-offset` as −3 dp. Its focus figure has a rounded rectangular
outline. Measure its corner appearance separately if a release claims
that exact contour. [Google dimensions](https://firebasestorage.googleapis.com/v0/b/design-spec/o/projects%2Fm3%2Fimages%2Fm92tisv3-07-3P.png?alt=media) · [Primary width and corners](https://firebasestorage.googleapis.com/v0/b/design-spec/o/projects%2Fm3%2Fimages%2Fm92thj1h-08-3P.png?alt=media) · [Primary focus and states](https://firebasestorage.googleapis.com/v0/b/design-spec/o/projects%2Fm3%2Fimages%2Fm8t9w5tr-03-3P.png?alt=media) · [Secondary focus and states](https://firebasestorage.googleapis.com/v0/b/design-spec/o/projects%2Fm3%2Fimages%2Fm8t9wue2-06-3P.png?alt=media)

Primary active hover, focus and press keep Primary ink. Inactive states
lift label/icon to OnSurface. Inactive hover and focus use OnSurface
state tint. Active states and inactive press use Primary tint. Secondary
uses OnSurface for interactive label/icon and tint. Opacities are 8%
hover and 10% focus/press. Google illustrates a drawn touch and click
ripple on both active and inactive tabs. Disabled is supported by the
pinned Android Tab API. The Google component export does not assign a
disabled palette, so identify any disabled ink rule as a Surface decision.

Google wins exact paint discrepancies. The pinned Compose secondary helper
uses the 3 dp Primary height by default and its rows use generic
`HorizontalDivider()`. Google specifies 2 dp Secondary and SurfaceVariant.
The general Google measurement table mentions 64 dp for icon and text.
Its dimension drawing supplies the narrower 48 dp Secondary inline case.

## Fixed, scrolling, and motion

Fixed lays out one row of equal cells. The whole strip can span its caller's
content surface. Google advises up to four fixed tabs and shows fluid side
space at wide widths. Never wrap into a second row. At 320 dp, use a valid
minimum cell size and enough label width. If authored choices cannot fit,
a caller can opt into a scrollable strip. Compose uses equal allocation for
fixed. Its reference scrollable row adds a 90 dp minimum per tab and uses
52 dp logical edge padding. Google directly specifies 52 dp emerging at
the first tab and widths driven by labels. A symmetric tail uses Compose
behavior. Mirror the logical start in RTL. Bound both scroll ends. Keyboard
focus must reveal a newly focused clipped tab. Touch and trackpad should
move the real stripe without selecting. Page content can optionally swipe
to switch tabs through caller-owned selection. Keep that distinct from
strip scrolling, which moves content without activation. Google also
allows an optional badge, only after existing badge semantics and bounded
label sizing can be explained. [Google Guidelines](https://m3.material.io/components/tabs/guidelines) · [Compose TabRow](https://android.googlesource.com/platform/frameworks/support/+/de12189a63496bd8aed010be245ce96a94f5873a/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/TabRow.kt)

One passive group indicator should travel between measured tab locations.
Animate its offset and size from the currently presented pose and keep
its bottom edge pinned. Stable checked/selected semantics and displayed
panel must update immediately when the caller accepts a selection. Moving
the indicator alone should preserve focus and the accessibility generation.
Snap with Reduce Motion, missing clock and unavailable geometry. Retarget
continuously under reversal, resize and direction change. Measure in the
scroll content's own coordinates so viewport motion does not restart the
indicator. Google demonstrates travel without specifying a release curve.
Compose uses Standard Default Spatial. Surface's `SpatialMotion::new`
currently selects Fast Spatial, so an additive preset would be needed for
exact Compose timing. A documented Surface transition is another valid
choice when claiming only Google behavior.

## Current Surface and compatibility gaps

- Legacy `Theme::tab` provides fixed-height text, selected color, a local
  3 dp indicator for only the selected tab and disabled/input hooks. Its
  indicator is inset 12 dp from the cell, with four rounded corners. Legacy
  `Theme::tabs` wraps a 4 dp gap row, has manual horizontal roving, and gives
  a named TabList role. It has no Primary/Secondary split, measured strip,
  divider, cross-tab indicator, scroller or linked panels. Keep both
  legacy builders and the Gallery section controls compatible.
  [Legacy tab and group](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-material/src/components.rs:743) · [Coverage](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/docs/material-coverage.json:326)
- Runtime already supports selected-first entry, disabled skip, one group
  Tab stop, Home and End, wrap, manual activation, explicit automatic
  arrows, and an RTL reversed horizontal row. Use retained layout roving
  for stable focus and reveal. Existing tests separate key movement from
  acceptance. Ordinary horizontal tabs should not consume Up/Down. Native
  and Web already project Tab/TabList and selected state.
  [Runtime tests](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-app/tests/controls.rs:182) · [Retained roving](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-app/src/lib.rs:519) · [Chrome projection](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-platform/src/web_accessibility/semantics.rs:181)
- Surface currently lacks TabPanel and tab-panel relations. Native AccessKit
  already has a TabPanel role, controls and labeled-by properties. Its
  macOS translation is a group with AXTabPanel subrole. Web needs proper
  stable DOM `aria-controls` from each tab and active panel
  `aria-labelledby` back to its tab. A relation resolver should accept only
  linked roles in the appropriate scope and avoid duplicate IDs. Decide how
  to keep stable hidden panel reference targets. Mount programming-heavy
  content only for the chosen tab. Inactive content must have no input,
  sequential focus or accessibility exposure. A plain active panel is a Tab
  stop if it starts with non-focusable content. An active panel starting
  with an eligible control can let that control take Tab. WAI clarifies
  selected-first entry, which Google's menu-style keyboard captions leave
  ambiguous. [Surface roles](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface/src/microdom.rs:20) · [Native mapping](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/third-party/accesskit-macos/src/node.rs:174) · [WAI](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)
- Ordinary WGPU Quad paint caps radii at half the shorter side. A 3 dp high
  Primary indicator can't show top corners with a full 3 dp radius through
  that path. A scoped passive `FillContour` can represent the true shape.
  Use ordinary square paint for the Secondary underline and divider.
  Existing `press_ripple` has no special role restriction, but requires an
  activation binding. If selected press should provide feedback without a
  second request, use an opt-in accepted press observer or a narrowly
  proven ripple seam. Never opt in all existing Tabs by default.
  [Quad clamp](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-wgpu/src/quad.wgsl:33) · [Press ripple acceptance](/tmp/surface-md3-radio-v0150.rTTLvfRB/source/crates/surface-app/src/press_ripple.rs:107)

## Additive API and first release

Recommend one coherent fixed-tabs release with both exact Primary and
Secondary paint, ordinary manual activation, and real linked panel semantics.
Use a `TabBarOptions {id, name, kind, layout, direction, available_width,
text_scale}` API that leaves the existing `tab` and `tabs` unchanged.
`TabBarKind` is Primary or Secondary. `TabBarLayout` can initially expose
Fixed and grow to Scrollable when its behavior and evidence exist. Use
`TabItem {id, label, name, icon, disabled, panel_id, on_select}`. Derive the
single active tab from one caller-supplied stable ID, not separate boolean
states. Keep `TabIndicatorMotion` in the application, updated from measured
geometry. Add a bounded `tab_panel` builder or dedicated authored semantics
that lets the caller supply active content and an explicit Tab entry policy.
Do not add badge-only or page-swipe API until those paths have their own tests.

For the first fixed release, headless proof should exercise both kinds and
both levels of content, text and vector icons, light/dark token reads,
320 dp RTL and 200% long text, exact 3/2/1 dp dimensions, 24 dp Primary
minimum, actual focus contour, accepted selection and indicator midpoint
and reversal, then Reduce Motion endpoints. Trusted Left/Right, Home/End,
Tab/Shift+Tab and Space/Enter prove focus, selection and the active panel.
Verify disabled and canceled pointer input, trusted touch, linked Chrome AX
and native AX groups, and unchanged legacy test journeys. Require direct PNG
checks on both headless native and WebGPU. When Scrollable is delivered, add
wheel/touch clipped geometry, leading 52 dp, physical scroll ends in both
directions, focus reveal of off-screen tabs, active indicator while scrolling,
and independent page swipe. Keep live screen-reader speech and system high
contrast separate from authored metadata and normal palette checks.
