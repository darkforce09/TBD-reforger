**Status:** live

# Spectator design references

The design references of the [spectator](/documentation_v2/mod/tbd-framework/UI/spectator/spectator_specification.md):
four design-phase Stitch sets, one per overlay, and the Arma 3 captures the design started from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/spectator/visual_references/
├── combat_details_panel_tactical_wireframe_forensics_mockup/  death forensics: hits, killer, kills
├── reference_screenshots/                                     two Arma 3 spectator captures
├── spectator_bottom_bar_split_faction_top_bar_mockup/         the watched player's bar: killer, kills
├── spectator_tactical_roster_panel_sidebar_mockup/            per-side counts, squads and seats
└── spectator_top_bar_broadcast_glass_pod_mockup/              kills to win, clock and objectives
```

## How it works

A mockup set is a folder named `<overlay>_mockup` holding the Stitch export as an html file and its
screenshot as a png, both named after the set, and a README that says what the set shows and how
the built spectator differs. `reference_screenshots/` holds in-game captures of another game. The
built spectator is the code the Code section links; the spectator specification's Design section
lists the differences.

## Code

- [Spectator](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/) and its
  [roster screen](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Spectator/UI/) — the built
  spectator.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from what its html loads
  from the network.
- Used by: the spectator specification and the spectator folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
