# Delivery stages

## Current checkpoint: surface@0.1.0

[Milestone](https://github.com/faberline/surface/milestone/1) · [Landing issue #5](https://github.com/faberline/surface/issues/5)

This stage lands UI = f(state), the shared native/WASM renderer, the current
partial Material controls, controlled Unicode editing, current DOM export,
opt-in CLI/MCP access and focus scrolling. It does not qualify the full
Material catalog or the physical native OS input-method journey.

The source checkpoint passes 45 Rust cases, native/WASM checks, compact
macOS GPU and Chrome 157 WebGPU bridge journeys, the existing Material
browser journey and the 40-event browser composition journey. Exact-head
CI must pass for the final landing. No crate release or tag is implied.

## Full framework target: surface@0.2.0

[Milestone](https://github.com/faberline/surface/milestone/2) tracks the
[completion contract](completion-contract.md). There are six foundation
issues, 36 component-family issues and seven input/platform/accessibility
issues. These are backlog items; this planning pass starts no worker loop.

Native OS Zhuyin verification is deferred at the user request in
[issue #48](https://github.com/faberline/surface/issues/48). Semantic fill and
synthetic browser events do not qualify an OS candidate window.

| Work | Issue |
| --- | --- |
| foundation: Material typography and matching text metrics | [#6](https://github.com/faberline/surface/issues/6) |
| foundation: Material shape tokens and per-corner GPU geometry | [#7](https://github.com/faberline/surface/issues/7) |
| foundation: Material elevation levels and shadows | [#8](https://github.com/faberline/surface/issues/8) |
| foundation: Material motion tokens and reduced motion | [#9](https://github.com/faberline/surface/issues/9) |
| foundation: Material ripple interaction and clipping | [#10](https://github.com/faberline/surface/issues/10) |
| foundation: Material focus, hover, pressed and disabled state coverage | [#11](https://github.com/faberline/surface/issues/11) |
| material: complete button-groups variants and behavior | [#12](https://github.com/faberline/surface/issues/12) |
| material: complete buttons variants and behavior | [#13](https://github.com/faberline/surface/issues/13) |
| material: complete extended-fab variants and behavior | [#14](https://github.com/faberline/surface/issues/14) |
| material: complete fab-menu variants and behavior | [#15](https://github.com/faberline/surface/issues/15) |
| material: complete floating-action-button variants and behavior | [#16](https://github.com/faberline/surface/issues/16) |
| material: complete icon-buttons variants and behavior | [#17](https://github.com/faberline/surface/issues/17) |
| material: complete segmented-buttons variants and behavior | [#18](https://github.com/faberline/surface/issues/18) |
| material: complete split-button variants and behavior | [#19](https://github.com/faberline/surface/issues/19) |
| material: complete date-pickers variants and behavior | [#20](https://github.com/faberline/surface/issues/20) |
| material: complete time-pickers variants and behavior | [#21](https://github.com/faberline/surface/issues/21) |
| material: complete loading-indicator variants and behavior | [#22](https://github.com/faberline/surface/issues/22) |
| material: complete progress-indicators variants and behavior | [#23](https://github.com/faberline/surface/issues/23) |
| material: complete navigation-bar variants and behavior | [#24](https://github.com/faberline/surface/issues/24) |
| material: complete navigation-drawer variants and behavior | [#25](https://github.com/faberline/surface/issues/25) |
| material: complete navigation-rail variants and behavior | [#26](https://github.com/faberline/surface/issues/26) |
| material: complete bottom-sheets variants and behavior | [#27](https://github.com/faberline/surface/issues/27) |
| material: complete side-sheets variants and behavior | [#28](https://github.com/faberline/surface/issues/28) |
| material: complete app-bars variants and behavior | [#29](https://github.com/faberline/surface/issues/29) |
| material: complete badges variants and behavior | [#30](https://github.com/faberline/surface/issues/30) |
| material: complete cards variants and behavior | [#31](https://github.com/faberline/surface/issues/31) |
| material: complete carousel variants and behavior | [#32](https://github.com/faberline/surface/issues/32) |
| material: complete checkbox variants and behavior | [#33](https://github.com/faberline/surface/issues/33) |
| material: complete chips variants and behavior | [#34](https://github.com/faberline/surface/issues/34) |
| material: complete dialogs variants and behavior | [#35](https://github.com/faberline/surface/issues/35) |
| material: complete divider variants and behavior | [#36](https://github.com/faberline/surface/issues/36) |
| material: complete lists variants and behavior | [#37](https://github.com/faberline/surface/issues/37) |
| material: complete menus variants and behavior | [#38](https://github.com/faberline/surface/issues/38) |
| material: complete radio-button variants and behavior | [#39](https://github.com/faberline/surface/issues/39) |
| material: complete search variants and behavior | [#40](https://github.com/faberline/surface/issues/40) |
| material: complete sliders variants and behavior | [#41](https://github.com/faberline/surface/issues/41) |
| material: complete snackbar variants and behavior | [#42](https://github.com/faberline/surface/issues/42) |
| material: complete switch variants and behavior | [#43](https://github.com/faberline/surface/issues/43) |
| material: complete tabs variants and behavior | [#44](https://github.com/faberline/surface/issues/44) |
| material: complete text-fields variants and behavior | [#45](https://github.com/faberline/surface/issues/45) |
| material: complete toolbars variants and behavior | [#46](https://github.com/faberline/surface/issues/46) |
| material: complete tooltips variants and behavior | [#47](https://github.com/faberline/surface/issues/47) |
| input: qualify real native Chinese composition and candidate placement | [#48](https://github.com/faberline/surface/issues/48) |
| platform: qualify Linux native rendering and input | [#49](https://github.com/faberline/surface/issues/49) |
| platform: qualify Windows native rendering and input | [#50](https://github.com/faberline/surface/issues/50) |
| input: qualify real browser OS composition and selection | [#51](https://github.com/faberline/surface/issues/51) |
| input: qualify mobile keyboards and changing visual viewports | [#52](https://github.com/faberline/surface/issues/52) |
| accessibility: expose the native semantic tree and actions | [#53](https://github.com/faberline/surface/issues/53) |
| accessibility: expose a browser companion tree for the canvas | [#54](https://github.com/faberline/surface/issues/54) |

## Consumer migration

[Core #95](https://github.com/faberline/core/pull/95) preserves the old
package names through forwarding crates. Its required development checks
must pass before landing; the old EC compiler/digest qualification is open.
The broader Core MicroDOM/rendering issues remain independently scoped.

[Jet #76](https://github.com/faberline/jet/pull/76), including #75, switches
to the public standalone dependencies. Its hosted parity wrapper currently
reports an unwired oracle. That exit is not hosted parity acceptance. The
local browser/WASM evidence and any required current-head gates stay visible.

Core #95 and Jet #75 pin `1754a2e`, which is not on Surface main. The
[agreed Jet and Surface split](migration.md#jet-and-surface-split) replaces
that pin with a tag cut from main. Whether to rebase or replace the PRs is
an open decision. Branches and the separate native IME experiment remain
preserved.
