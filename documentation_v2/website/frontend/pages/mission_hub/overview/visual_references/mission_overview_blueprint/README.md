**Status:** live

# Mission overview blueprint

Design-phase reference for the mission dossier of the mission overview page at `/missions/:id`,
drawn as the slide-over the library opens: one [mission](/documentation_v2/glossary/g_to_m.md#mission)'s
briefing, required assets and order of battle. It gives colour and layout context and is not an
implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/mission_hub/overview/`.

## Contents

```text
documentation_v2/website/frontend/pages/mission_hub/overview/visual_references/mission_overview_blueprint/
├── mission_overview_blueprint.html  the Stitch export of the slide-over dossier
└── mission_overview_blueprint.png   its screenshot
```

## How it works

The blueprint shows a sheet titled "Mission Dossier" over "Operation Black Waves", a zone line
with latitude and longitude, "Tactical Briefing", "Required Assets" as requisition rows ("REQ-04",
"M4A1 Carbine", "QTY: 12"), "Order of Battle" with squad rows and their fill ("Alpha 1-1
(Assault)", "READY", "8/8"), and a "LAUNCH TACTICAL PLANNER" button.

The built dossier differs: no zone or coordinates; "The Armory" with one tab per faction replaces
the required assets; there is no order of battle, map preview or briefing tabs; the detail rows
(weather, time, players, status) and, for the author and administrators, the review record and
the "Edit Armory" button are additions; and the planner button sits only in the library sheet's
footer. The
[mission overview page](/documentation_v2/website/frontend/pages/mission_hub/overview/mission_overview_page.md)
feature doc holds the full comparison.

## Code

- [Mission overview page](/apps/website/frontend/src/v2/pages/mission_hub/overview/) — the
  dossier this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted placeholder images, which the
  html loads when opened; the png needs nothing.
- Used by: the mission overview feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
