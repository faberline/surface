# Surface tab and panel semantics

Base: 607 Radio v0.1.50 source files, manifest SHA-256 `a031c8b5dc2700f69cb131182842722976d57726dd01c2e0a2d99e643e406544`.

Apply `tab-semantics.patch` in that source root. A dry run with `git apply --check --whitespace=error-all` passed against the untouched base.

Author a named `Role::TabList`. Add `tab_controls(panel_id)` to each named `Role::Tab` under that list. Mount only the active panel as a direct sibling of the tab list. Give the panel `Role::Group`, its declared stable ID, and `tab_panel_labelled_by(tablist_id, tab_id)`. Give the panel a Tab stop when its contents need that entry point.

The frame rejects mismatched, ambiguous, or out-of-scope declarations. Relationships connect projected targets. On Web, explicit inactive tabs with unused panel IDs keep empty hidden panel targets. When a selected tab is clipped, the mounted panel retains its name. On macOS, the native AccessKit tree links projected tabs and panels. Clipped tabs with an existing Runtime reveal action can stay in that tree.

Verified with `CARGO_BUILD_JOBS=2`, `cap`, pinned dependencies, and an isolated Cargo target. The focused core tests passed (2), frame tests passed (4), native AccessKit tree test passed (1), and Web projection planner tests passed (2). The three affected crates passed 619 tests with all features. The manual native clipboard test stayed ignored. Provenance, formatting, all-feature native and Wasm checks, and scoped all-target Clippy passed. The Wasm check reports the existing unused `should_exit` warning in `close.rs`. Cargo reports a future compatibility warning for `block 0.1.6` in the native lane.

The tests do not open a window. The native test inspects AccessKit updates, and the Web tests inspect the projection plan. Browser DOM and Chrome accessibility behavior need the integrated Gallery run. Live assistive-technology speech remains unverified.
