**Status:** live

# Mission selection design references

The design references of the [Mission Selector](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md):
four design-phase Stitch sets, one per panel, and the Arma 3 capture the design started from.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/
├── mission_header_browser_mockup/    the mission column: search, mode filter, mission cards
├── mission_inspector_mockup/         one mission: hero, versions, modset, summary, ORBAT, objectives
├── mission_selector_top_bar_mockup/  the session top bar with the three tabs
├── reference_screenshots/            the Arma 3 create game screen
└── terrain_selector_mockup/          the terrain column with mission counts
```

## How it works

A mockup set is a folder named `<panel>_mockup` holding the Stitch export as an html file and its
screenshot as a png, both named after the set, and a README that says what the set shows and how
the built panel differs. `reference_screenshots/` holds in-game captures of another game. The
built UI is the code the Code section links; the specification's Design section lists the
differences.

## Code

- [Mission Selector scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/UI/)
  and [layouts](/apps/mod/tbd-framework/UI/layouts/Session/MissionSelector/) — the built screen.

## Boundaries

- Depends on: nothing in the repository; each set is self-contained apart from what its html loads
  from the network.
- Used by: the mission selection specification and the mission selection folder README.
- Rules: a set is kept as captured and never edited to match the built screen; a new set gets its
  own folder and README; no screenshot of the built UI belongs here.
