**Status:** live

# Objective capture HUD documentation

The documentation of the objective board and capture bar a player sees during the live stage of
a round of an [event](/documentation_v2/glossary.md#event).

## Contents

```text
documentation_v2/mod/tbd-framework/UI/objective_capture_hud/
└── objective_capture_hud_specification.md  the HUD as built, its wire, design target, open work
```

## Code

- [Objective and task HUD scripts](/apps/mod/tbd-framework/Scripts/Game/TBD/UI/Hud/) — the panel
  and its RPC pair
- [Objectives](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Objectives/) — the server tick
  that builds each player's board
- [HUD layouts](/apps/mod/tbd-framework/UI/layouts/Hud/) — `TBD_ObjectiveHud.layout`

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code READMEs of the folders above, which link the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code READMEs link it; the folder holds no
  design references until a mockup set is drawn.
