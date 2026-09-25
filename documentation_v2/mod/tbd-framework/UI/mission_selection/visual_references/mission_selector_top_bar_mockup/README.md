**Status:** live

# Mission Selector top bar mockup

Design-phase reference for the session top bar over every pre-game screen: title, the three tabs, the player's identity and the connected count. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/mission_selection/visual_references/mission_selector_top_bar_mockup/
├── mission_selector_top_bar_mockup.html  the Stitch export
└── mission_selector_top_bar_mockup.png   its screenshot
```

## How it works

The set shows the mission id `wog_187_chollima_on_the_wing_10` as the title, the tabs "Scenario Browser", "Lobby" and "Briefing", "Mission Maker" with an `ADMIN` chip, and a count of 1.

The built bar is `TBD_SessionTopBar` (`apps/mod/tbd-framework/UI/layouts/Session/Shared/TBD_SessionTopBar.layout`), shared by the three screens: the Mission Selector titles it "Scenario Browser" and the lobby and briefing show the mission id in a mono face; the count reads `connected / capacity`, and its icon key `group` has no texture, so the icon stays hidden. The [mission selection specification](/documentation_v2/mod/tbd-framework/UI/mission_selection/mission_selection_specification.md) feature doc holds the full comparison.

## Code

- [Shared UI primitives](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
