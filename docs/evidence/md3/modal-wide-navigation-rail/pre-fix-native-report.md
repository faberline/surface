# Modal wide navigation rail before the compact height fix

The first native run used the production Gallery, renderer, screenshot transport, CLI and MCP. The public key checks passed, including initial Close focus, modal scope, skipping the disabled Archive, independent destination scroll, selection and restoration. The run confirmed both collapse layouts and the current page semantics in AppKit.

The focused Inbox label at 320 × 240 dp and 200% text was visibly cropped. The scroll viewport could not fit the full wrapped row. The [dark narrow PNG](pre-fix-native-clipped.png) shows this defect.

The original runner accepted a partially visible row. The new runner verifies row and label containment and checks for foreground pixels on both lines before accepting the production frame.
