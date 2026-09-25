**Status:** live

# Mission library blueprint

Design-phase reference for the mission library page at `/missions`: a featured
[mission](/documentation_v2/glossary.md#mission) hero above a search and filter toolbar and a grid
of mission cards. It gives colour and layout context and is not an implementation source; the
built UI is the Leptos code under `apps/website/frontend/src/v2/pages/mission_hub/library/`.

## Contents

```text
documentation_v2/website/frontend/pages/mission_hub/library/visual_references/mission_library_blueprint/
├── mission_library_blueprint.html  the Stitch export of the library
└── mission_library_blueprint.png   its screenshot
```

## How it works

The blueprint shows the heading "Mission Library" with the line "Browse, filter, and deploy active
operations.", the tabs "Global Missions", "My Missions" and "Bookmarked", a hero ("Live
Operation", "Operation Vanguard", a briefing, "View Dossier", a player count and the terrain),
a search field beside labelled Terrain (All, Everon, Arland, Custom Map), Mode (All, PvE / COOP,
PvP, Zeus) and Players (All, 1-16, 17-32, 33-64) selects, and three cards, each with a mode badge,
an author, a title, a terrain and "max" players.

The built page differs: a "New Mission" button joins the header and the subtitle ends "across the
theater."; the selects are unlabelled, offer no custom map and split players into 1–8, 9–16, 17–32
and 33–64; the hero reads "<n> OPERATORS" and pulses "Live Operation" only for a live mission;
each card adds a status badge and a bookmark star; and a card opens a slide-over dossier. The
[mission library page](/documentation_v2/website/frontend/pages/mission_hub/library/mission_library_page.md)
feature doc holds the full comparison.

## Code

- [Mission library page](/apps/website/frontend/src/v2/pages/mission_hub/library/) — the page
  this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted placeholder images, which the
  html loads when opened; the png needs nothing.
- Used by: the mission library feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
