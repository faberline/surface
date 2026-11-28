# Badge placement and readable status

The published Material 3 Badge page lists the Small dot and the Large text badge. It describes 6 dp and 16 dp defaults, a limit of four characters for the visible label, and 999+ for higher counts. It describes a logical upper-trailing icon anchor. The Navigation Bar guide applies the same anchor in both the compact vertical and medium horizontal layouts.

The accessibility guidance puts the badge status after its destination name and describes clearing an unread indicator on selection. The Surface Gallery keeps the full localized count in the button name. When the app handles the destination, it clears the badge and removes the unread status from that name. The library exposes the badge as decoration on the existing button.

The two proposed builders reuse Surface’s navigation badge placement. It uses a 4 dp vertical overlap for the larger badge to preserve icon and adjacent label ink. The published 14 dp reference geometry remains an open detail for the broader Badge family. The horizontal builder holds a separate badge overlay so the existing measured pill can place its leaf icon and label. Both builders retain the destination IDs and keep keyboard focus on the destination.

The candidate covers badges on standalone compact and medium flexible navigation bars. The adaptive scaffold’s rail route, separate modern rail support and live platform assistive speech remain open.

[Google Badge specification](https://m3.material.io/components/badges/specs) · [Google Badge accessibility](https://m3.material.io/components/badges/accessibility) · [Google Navigation Bar guidelines](https://m3.material.io/components/navigation-bar/guidelines) · [Google Compose badges](https://developer.android.com/develop/ui/compose/components/badges)

The published Badge export SHA-256 is `743661fc055b3f95cb9227abe6b2abc15676e495035b1cd972137da16967a707`. The published Navigation Bar export SHA-256 is `62189f1b8b50b69ac49ae1c4e4d8b101a7981a0b8b0b6e90c03f5ca56a57f12e`.

[Badge export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/e18a73ff-b00d-46e7-af20-ab9dd24bf579.json) · [Navigation Bar export](https://m3.material.io/_dsm/content/m3/2026-11-19_06-20-01/9e4cdffa-f7fd-425f-9915-867aafbcc7d6.json)
