# Material 3 catalog check

Checked the published Google site on 2026-11-23 against Surface PR #207
head `ec2eecef5468f31fbd998a6fb1cad253e98675a3`. The 36 visible
component cards in Google's overview match the 36 Surface family IDs,
including the same order. All 36 Surface entries still have partial
support and variant review pending. [Google catalog](https://m3.material.io/components) ·
[Surface ledger](https://github.com/faberline/surface/blob/v0.1.33/docs/material-coverage.json)

The current site has 38 component page routes. The All Buttons page
is a selection guide covering existing button families. The navigation
and page map publish Scroll fields as a separate family. This gives 37
published families in the checked scope. [All Buttons guide](https://m3.material.io/components/all-buttons)

Scroll fields has minimum widths of 100, 144 and 186 dp, finite and looping value sequences,
and Scroll and Input modes for generic values. The Material time picker
uses time-specific fields. Qualify the generic family separately as part
of the full Material goal. [Scroll fields specs](https://m3.material.io/components/scroll-fields/specs) ·
[Scroll fields guidelines](https://m3.material.io/components/scroll-fields/guidelines)

| Official input | SHA-256 |
| --- | --- |
| [Overview export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/763ccaaf-ec58-403b-b025-a10cd080fd97.json) | `d1d628da4c00d08506873ceba343ff8940969545b08b79198da7348895304e25` |
| [Navigation and route map](https://m3.material.io/static/angular/main.2bcb6530e93abb29.js) | `f5db4772862bc48f3b0cf9ab7dfb134c5abaa69acc5248c741ed5721c171ded2` |
| [All Buttons guide export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/276f1bd7-d048-49fd-8483-f267f2ece13d.json) | `501d2112359db2a0fda075bc2b35357c0b4b0b50931dff2fb742c15e941dee63` |
| [Scroll fields export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/1624d6a1-96ea-437d-b846-dc6f00ebb49c.json) | `3d7d5d816080d044cfc1a67cc7c33b7533cd3e91682ea6f99ab43e6c36c5a390` |

Compare the overview's visible linked image cards with the family slugs
in the ledger's `spec` URLs. Check `components/scroll-fields` in the
navigation and use the route's export for the separate specification.
