# macOS tabs in a scroll area

Apply `tab-mac-ax-tabs.patch` separately from `tab-semantics.patch`. It changes only the vendored macOS node lookup and the provenance note. A dry run with `git apply --check --whitespace=error-all` passed on the exact 607-file Radio base.

The native `AXTabs` lookup now follows scroll areas within the tab list. It respects hidden subtrees and stops at other exposed semantic roles. The scroll area still appears in the regular native children and keeps its page action.

The headless native test asserts order and ownership using the AccessKit tree filter called by `AXTabs`. The fixture has a direct tab, an anonymous wrapper, a paged scroll area, a hidden scroll area, a named group, and a nested tab list. All five vendored macOS library tests passed. Vendored formatting, workspace provenance, the native Surface platform check, and the native mounted-tab AccessKit test passed.

The checks did not open a window or measure live VoiceOver speech.
