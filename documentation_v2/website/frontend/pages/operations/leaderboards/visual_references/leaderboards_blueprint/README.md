**Status:** live

# Leaderboards blueprint

Design-phase reference for the leaderboards page at `/leaderboards`: category tabs and a search
field over a three-place podium and a ranked roster. It gives colour and layout context and is not
an implementation source; the built UI is the Leptos code under
`apps/website/frontend/src/v2/pages/operations/leaderboards/`.

## Contents

```text
documentation_v2/website/frontend/pages/operations/leaderboards/visual_references/leaderboards_blueprint/
├── leaderboards_blueprint.html  the Stitch export of the leaderboards
└── leaderboards_blueprint.png   its screenshot
```

## How it works

The blueprint shows "Global Leaderboards" with "Real-time tactical performance metrics across all
active theaters.", a "Search operatives..." field, the tabs "K/D Ratio", "Command Win Rate",
"Missions Played", "Longest Kill" and "Wall of Shame (Team Kills)", a podium with portraits and
"#1", "#2" and "#3" badges, a score under each, and "[ VIEW DOSSIER ]" under the first place, and
roster rows from "04" with an avatar or initial, a name, a kill count and a ratio.

The built page follows it closely and differs: the fifth tab reads "Wall of Shame" and the field
"Search operators..."; avatars without an `http(s)` address show initials; a roster row opens the
player's dossier in a slide-over sheet; and the roster stops at the first 20 players. The
[leaderboards page](/documentation_v2/website/frontend/pages/operations/leaderboards/leaderboards_page.md)
feature doc holds the full comparison.

## Code

- [Leaderboards page](/apps/website/frontend/src/v2/pages/operations/leaderboards/) — the page
  this set was drawn for.

## Boundaries

- Depends on: the Tailwind CSS CDN, Google Fonts and Google-hosted images, which the html loads
  when opened; the png needs nothing.
- Used by: the leaderboards feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
