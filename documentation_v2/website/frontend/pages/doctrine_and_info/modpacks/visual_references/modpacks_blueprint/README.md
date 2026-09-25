**Status:** live

# Modpacks blueprint

Design-phase reference for the modpacks page at `/modpacks`: one wide card for the server's
modpack, its size, its key dependencies and a button that connects and syncs the
[mods](/documentation_v2/glossary.md#mod). It gives colour and layout context and is not an
implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/`.

## Contents

```text
documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/visual_references/modpacks_blueprint/
├── modpacks_blueprint.html  the Stitch export of the modpack card
└── modpacks_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "Server Modpacks" with a line saying Arma Reforger downloads the
mods on connect, a card for "Core Modern Expansion" "v2.1" with "45.2 GB" "Total Size" and "32"
"Included Mods", four dependencies with an icon each tagged "Required" or "Verified", a "DIRECT
CONNECT & AUTO-SYNC" button and "View Collection in Reforger Workshop".

The built page differs: a searchable list of every pack, with an "Active" chip on the current one,
sits beside one pack's dossier; there is no explanatory line; each addon shows its Workshop id and
"[ REQUIRED ]" instead of an icon and a tag; "[ Launch Game & Auto-Download ]" only toasts that
the Reforger client is needed; and administrators get create, edit, make-current and delete
controls. The
[modpacks page](/documentation_v2/website/frontend/pages/doctrine_and_info/modpacks/modpacks_page.md)
feature doc holds the full comparison.

## Code

- [Modpacks page](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/) — the page
  this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN and Google Fonts, which the html loads when opened; the png
  needs nothing.
- Used by: the modpacks feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
