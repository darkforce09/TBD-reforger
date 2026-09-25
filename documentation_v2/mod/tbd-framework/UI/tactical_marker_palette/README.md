**Status:** live

# Tactical marker palette documentation

The documentation of the map markers a player sees in game, the briefing and task markers each
side receives, and of the marker palette designed for play but not built.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/tactical_marker_palette/
└── tactical_marker_palette_specification.md  the markers as built, the palette design target, open work
```

## Code

- [Mission map markers](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Markers/) — the
  side-scoped briefing markers and their wire
- [Objective and task HUD](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/) — `TBD_TaskHud`, the
  task markers

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it; the folder holds no
  design references until a mockup set is drawn.
