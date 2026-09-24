**Status:** live

# Personnel roster blueprint

Design-phase reference for the personnel roster page at `/admin/personnel`: a member table beside
one member's dossier, styled like an activity monitor. It gives colour and layout context and is
not an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/administration/personnel/`.

## Contents

```text
documentation_v2/website/frontend/pages/administration/personnel/visual_references/personnel_roster_blueprint/
├── personnel_roster_blueprint.html  the Stitch export of the roster and dossier
└── personnel_roster_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "Personnel Roster" with the line "Manage and audit registered
platform users.", a search field ("Search ID or Name..."), the controls "[ Sort by Warnings ]"
and "[ Filter by Rank ]", a table with the columns User, Arma Character, Rank, Warnings and Status,
member photos, a "Total Records" count, and a dossier with a photo, "Service Telemetry" rows
(Deployments, Current Rank, Warnings, Status) and the buttons "[ Edit Roles ]", "[ Warning ]" and
"[ Ban ]".

The built page differs: no subtitle; the sort and the filter are two controls that cycle, and the
filter picks All, Active or Banned instead of a rank; a "Sync Roles" button joins the header;
initials badges replace photos; the dossier's readings sit in a grid of tiles; its buttons open a
[role](/documentation_v2/glossary.md#role) note and the ban and warning dialogs; and the table shows
no record count. The
[personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md)
feature doc holds the full comparison.

## Code

- [Personnel roster page](/apps/website/frontend/src/v2/pages/administration/personnel/) — the
  page this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted placeholder images, which the
  html loads when opened; the png needs nothing.
- Used by: the personnel roster feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
