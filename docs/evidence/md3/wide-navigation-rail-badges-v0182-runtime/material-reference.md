# Material 3 wide navigation rail badge reference

The published rail guide shows a badge on the icon's upper logical trailing
corner in the collapsed rail. The expanded examples put the badge near the
start of the visible text label. They show a dot, a short number, and a longer
count. The Badge specification supplies the 6 dp dot, 16 dp text badge,
short visible label, and 999+ count convention.

Surface reuses the collapsed navigation badge anchor. In the expanded rail
a text badge can grow toward its icon while its trailing edge stops at the
label gap. This clearance preserves the expanded label at 200% text. The dot
keeps its upper corner anchor. The caller supplies the full localized status
and clears it after acknowledgement. The badge shares the destination hit
area and creates no extra accessibility or focus target.

The baseline specification assumes a short badge label. This verification
does not cover arbitrary custom badge strings, live assistive speech, or a
modal or animated badge transition.

[Google navigation rail guidelines](https://m3.material.io/components/navigation-rail/guidelines)
· [Published Navigation Rail export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/98fe42bd-9bb1-4307-9e92-e61e57c3e9da.json)
· [Google badge specification](https://m3.material.io/components/badges/specs)

The unmodified November 19 navigation rail export returned SHA-256
`d33e560852f066798a55c2d71b7dc8ebfac803fb9c247d8e68207d3ca8858a07`.
The original Google figure cited by its Badge section returned SHA-256
`2edfc6dc2d0e03b9d56c239ce884f1868a5d01f3fb315305cd5b68f6c91c5d4a`.
The earlier Badge export returned SHA-256
`743661fc055b3f95cb9227abe6b2abc15676e495035b1cd972137da16967a707`.
