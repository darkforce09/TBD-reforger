**Status:** live

# BLUFOR ORBAT panel mockup

Design-phase reference for the Roles column of the lobby: one card per squad with its seats, for the BLUFOR side. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/orbat_panel_blufor_mockup/
├── orbat_panel_blufor_mockup.html  the Stitch export
└── orbat_panel_blufor_mockup.png   its screenshot
```

## How it works

The set shows the "ROLES" heading and squad cards such as "Alpha 1-1" with a `UAZ-3151` chip and `2/3`, and "Alpha 2-1" with `BMP-2` and `7/8`; each seat row carries a numbered role ("1: Platoon Commander"), weapon chips (`AK-74`, `RPK-74`, `PKM`), a `MED` trait chip, the holder ("[1stID] Miller") or an `Unslotted` status.

The built column, `TBD_LobbyRosterPanel` with `TBD_LobbySquadCard` and `TBD_LobbySlotRow`, follows the set with role text in capitals, a fold chevron per card, a `DEAD` status for a spent seat, and a Locate button per card when the briefing reuses it read only; its data is the mock catalog. The [lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
