# Surface Divider variant review

The Material 3 catalog includes Divider. The Surface completion contract
requires a family review and matching native and WebGPU execution.

Google's first-party Android guide specifies the Outline Variant color. It
specifies Regular at 1 dp and Heavy at 8 dp. It describes full-width and inset
lines, logical start and end inset values, horizontal and vertical list
orientation, and decorative accessibility semantics. Its Heavy example puts
16 dp above and below the line. The guide's exact source revision is pinned
below.

Surface renders full-width, leading-inset, middle-inset and vertical lines.
The new List seam places a decorative divider inside one named list. It leaves
the rows in a single keyboard group. The Gallery shows an inset in a List,
a Regular full-width rule before unrelated content, and a Heavy rule between
named regions. It also shows the same List inside a modal side sheet. Side sheet
composition is a Surface coverage requirement for this batch.

The accepted headless proof needs the exact source and Gallery builds. It
compares light and dark native PNGs across Page.screenshot, CLI and MCP.
Browser proof checks the CSS CanvasText color in forced colors, returns to
authored Outline Variant, and verifies list and modal keyboard behavior.
The shared accessibility frame and Chrome tree cover the decorative rule.
Live macOS assistive speech is outside the headless evidence.

[Material 3 Divider](https://m3.material.io/components/divider/overview)
[Google Android Divider guide at cc2390f1](https://github.com/material-components/material-components-android/blob/cc2390f1a2b314c4f89114f51ad81fb642e491eb/docs/components/Divider.md)
[Google Android Compose divider documentation](https://developer.android.com/develop/ui/compose/components/divider)
[Historical Surface family issue](https://github.com/faberline/surface/issues/36)

## Exact Google catalog export

The published Material 3 site maps the Divider page to the export identified
below. The export contains Overview, Specs, Guidelines and Accessibility.
Its SHA-256 is 554dcc6159baba5dd39c4a410bde7859c37a4821c6827f34c1c2f9861e2b5309.

The Specs use Outline Variant in light and dark. They give full-width, one-sided
16 dp insets and two-sided 16 dp insets. The Guidelines also show vertical
separation. The Accessibility section classifies the rules as decorative and
leaves the reading structure to the surrounding content. The separate Google
Android style guide supplies the 1 dp and 8 dp weights and the Heavy spacing
example.

[Published Material 3 Divider export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/843ffaa9-e73e-45a7-bff7-f198cac0a7c8.json)
