# Material 3 badges on the flexible navigation bar

The tested candidate is commit `b9dc4ad7a5c6cdddd71196a669354f17984af7aa`. Its Git tree is `ea61008a90b12a3f753398b9e5c1ff58f5813b3d`. The source bundle and 714-file manifest reproduce it. This record covers a queued Surface candidate.

The additive compact and horizontal navigation bar adapters accept Hidden, Dot, a localized label, and a count. The count paints `999+` when the full count is greater than 999. The application provides the full localized accessible name and clears unread state on activation. The badge adds neither an accessibility control nor a focus stop. Existing navigation bar methods still accept their original items. The new methods preserve the destination IDs, selection, focus, and hit targets.

## Verification

- Workspace checks passed with 1,629 Rust tests and three Wasm checks. The ordinary workspace run ignored 20 GPU-dependent tests.
- The separate offscreen Apple Metal run passed 18 tests and produced 21 original PNGs.
- Two native headless replays took ten original PNGs. Page.screenshot, CLI, and MCP returned identical PNG bytes at each capture. The replays cover 599 dp and 600 dp light, 320 dp and 839 dp dark RTL at 200% text, and acknowledgement on a focused Inbox. The independent window check recorded 18 zero-window samples and all eight Gallery children exited.
- The full Material suite passed 385 steps and 32,193 checks with the stock Web release and stock E2E runner. It recorded 4,947 Chrome samples, zero owned visible windows, and no remaining observed processes. It exercises the 599/600 dp layout change, retained selection and keyboard focus, and acknowledgement. The archived PNGs show the original light, dark RTL 200%, and acknowledgement frames. The verifier reads actual Error and On Error pixels inside the destination bounds.
- The first full Browser run passed all 385 Material steps and 32,193 checks. Its wrapper could not give that run a complete input receipt because the verifier file was edited after launch. The sealed record below uses a fresh run with the verifier held fixed.

## Reproduce

Restore the commit using [the source bundle](source/README.md). Run the repository's Rust build rules. Keep the test in headless mode.

```sh
cap sh scripts/check.sh
cap sh scripts/check-gpu.sh
cap cargo xtask web e2e --artifacts "$evidence" material
```

In the Material Gallery, go to section 3 and open the navigation bar Badge demo. Use the theme, direction, and text controls to reproduce the dark RTL 200% view. Use the native headless Gallery with a private DevTools session for the PNG captures. The replay and verifier source are included under `native/tools` and `browser/tools`. Config and receipts show the specific pinned commands and inputs used on the tested host.

This follows the Material badge export for size and count presentation. The navigation-specific label clearance uses 4 dp of vertical overlap so more icon artwork stays visible. The Material reference pinned here uses greater vertical overlap for the general Badge anchor. Badges in wide rails and the full adaptive scaffold, actual screen-reader speech, and physical touch stay outside this evidence scope.

[Primary specification and pins](spec/README.md)
