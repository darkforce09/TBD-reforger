**Status:** live

# Lobby bottom bar mockup

Design-phase reference for the session bottom bar that closes every pre-game screen: a full-width 64 px bar with one primary action. It gives layout and colour context and is not an implementation
source; the built panel is the EnfScript and layout code the Code section links.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/lobby/visual_references/lobby_bottom_bar_mockup/
├── lobby_bottom_bar_mockup.html  the Stitch export
└── lobby_bottom_bar_mockup.png   its screenshot
```

## How it works

The set shows the bar with a single primary button, "Select Scenario".

The built bar is `TBD_SessionBottomBar` (`apps/mod/tbd-framework/UI/layouts/Session/Shared/TBD_SessionBottomBar.layout`), shared by all three pre-game screens: the Mission Selector mounts "Select Scenario", while the lobby mounts "Lock Lobby" and the primary "Ready & Continue", which toggle their labels only. The [lobby specification](/documentation_v2/mod/tbd-framework/UI/lobby/lobby_specification.md) feature doc holds the full comparison.

## Code

- [Shared UI primitives](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Common/) — the built panel this set was drawn for.

## Boundaries

- Depends on: the styles, fonts and images the html loads from the network when opened; the png
  needs nothing.
- Used by: the feature doc's Design section and the visual references README.
- Rules: the set is kept as captured; the html and the png stay a pair named after the set.
