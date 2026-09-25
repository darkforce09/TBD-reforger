**Status:** live

# Mission selection reference screenshot

Design-phase reference for the [Mission Selector](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md):
an Arma 3 capture of the "CREATE GAME" hosting screen the TBD selector started from. It is not an
implementation source.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/reference_screenshots/
└── mission_selection.png  the Arma 3 create game screen, Altis selected
```

## How it works

The capture shows, over a blurred 3D backdrop:

- an amber header rail with "CREATE GAME" and the host's name ("Mission Maker");
- a mission settings card (minimum and maximum players, game type, respawn model) and the server
  difficulty preset ("Regular");
- "MAPS: 53", a scrolling terrain list with biome icons, where a leading "!" sorts test maps first
  ("!Virtual Reality");
- "Missions: 30" for the chosen map, with a search field, "SHOW ALL MISSIONS" (ignore the map
  filter) and a green "<<New - 3D Editor>>" entry; mission names follow prefix conventions
  (`COOP 12 Combat Patrol`, `SC 48 Warlords (Whole Island)`, `Zeus 16+2 Master Altis (NATO)`);
- a tabbed panel with the chosen mission's summary (title, author, artwork, synopsis) or the
  pre-lobby chat;
- a footer with `BACK`, `GAME OPTIONS`, a Steam Workshop button and `PLAY`, which moves everyone to
  role assignment.

The built selector keeps the terrain-first drill-down, the search and the chosen mission's
summary; the
[mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md)
lists what it drops and adds.

## Code

- [Mission Selector scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/)
  — the screen the capture informed.

## Boundaries

- Depends on: nothing.
- Used by: the mission selection specification's Design section and the visual references README.
- Rules: captures are kept as taken; no screenshot of the built UI belongs here.
