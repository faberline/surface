# Menu coverage amendments

The amendments add the verified desktop and Web evidence to the menu row.
Each patch passes a strict apply check on its own tested source tree.

| Patch | Expected source tree |
| --- | --- |
| `v0.1.75-menu-evidence.patch` | `8046e25a630284411e1ebf9f9d16d1004b02651b` |
| `v0.1.76-menu-evidence.patch` | `b6dc18c4baf8e6b65bddfbc6f59b7b07b4a0b5b9` |

The coverage status stays `partial`. Submenus, system high contrast and
live screen-reader speech still need evidence.
