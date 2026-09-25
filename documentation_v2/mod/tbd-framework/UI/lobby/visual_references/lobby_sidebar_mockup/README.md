**Status:** live

# Lobby sidebar mockup

Design-phase reference for the Factions column of the lobby: one row per faction, then a Spectators row. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/lobby_sidebar_mockup/
├── lobby_sidebar_mockup.html  the Stitch export
└── lobby_sidebar_mockup.png   its screenshot
```

## How it works

The set shows the "Factions" heading, BLUFOR with a `DEFENDING` chip and `0 / 92`, OPFOR with `ATTACKING` and `0 / 95`, and Spectators with `0 / 10`.

The built column, `TBD_LobbyFactionPanel` in the `TBD_LobbyFactionList` layout, draws the same rows from the mock `TBD_LobbyCatalog`; its `VoiceDock` below the rows stays empty, since no voice panel is built. The [lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) feature doc holds the full comparison.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
