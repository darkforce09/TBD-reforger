**Status:** live

# Players panel mockup

Design-phase reference for the briefing's Players mode: who is slotted on each side. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/briefing/visual_references/players_panel_mockup/
├── players_panel_mockup.html  the Stitch export
└── players_panel_mockup.png   its screenshot
```

## How it works

The set shows "TOTAL: 94 PLAYERS" and a BLUFOR lane ("Defending", "Slotted: 36 / 40") with numbered rows of player, name and ping.

The built panel, `TBD_PlayersPanel` in `apps/mod/tbd-framework/UI/layouts/Session/Shared/TBD_PlayersPanel.layout`, opens in the briefing's `WideDock` with BLUFOR and OPFOR lanes plus Spectators ("Count:") and Unslotted ("Pending:"); its players come from the mock catalog. The [briefing specification](/documentation_v2/mod/tbd-framework/UI/briefing/briefing_specification.md) feature doc holds the full comparison.

## Code

- [Players panel scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Players/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
