# Compact Expressive flexible navigation bar preflight

The inputs start at the exact Menu v0.1.46 candidate. The reports record
their own source manifests and instructions to reproduce the checks.

- Twenty-four scoped Rust tests passed, including existing navigation
  and consumer checks. Two native headless replays returned matching
  two-image sets with the screenshot API, CLI and MCP. All 12 window
  observations found no visible windows. The reports include an early
  200% text failure and the corrected pinned page title.
  [Native and Rust proof](native-and-rust)
- The focused headless Chrome fixture passed 458 checks across M270–M273.
  It measured actual WebGPU paint and verified three, four and five
  destinations, keyboard and touch, and 320 dp RTL with 200% text.
  The bar's decoded pixels remained identical through page scrolling.
  All 376 live window observations found no visible windows.
  [Browser receipt and decoded images](chrome)

The Surface v0.1.47 candidate has a separate full verification record.
