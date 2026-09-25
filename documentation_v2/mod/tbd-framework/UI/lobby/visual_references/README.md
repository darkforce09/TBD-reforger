**Status:** live

# Lobby design references

The design references of the [lobby](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md):
four design-phase Stitch sets, one per panel, and the Arma 3 capture the design started from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/
├── lobby_bottom_bar_mockup/    the session bottom bar with one primary action
├── lobby_sidebar_mockup/       the Factions column with a Spectators row
├── orbat_panel_blufor_mockup/  the Roles column: squad cards and seat rows
├── reference_screenshots/      the Arma 3 role assignment screen
└── slot_kit_inspector_mockup/  the kit inspector for one seat
```

## How it works

A mockup set is a folder named `<panel>_mockup` holding the Stitch export as an html file and its
screenshot as a png, both named after the set, and a README that says what the set shows and how
the built panel differs. `reference_screenshots/` holds in-game captures of another game. The
built UI is the code the Code section links; the lobby specification's Design section lists the
differences.

## Code

- [Lobby screen scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Lobby/UI/) and
  [layouts](/apps/mod/tbd-framework/UI/layouts/Session/Lobby/) — the built screen.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from what its html loads
  from the network.
- Used by: the lobby specification and the lobby folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
