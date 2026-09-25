**Status:** live

# Safe start HUD documentation

The documentation of the safe start notices: the countdown and the weapons-cold and weapons-live
messages players see before a round of an [event](/documentation_v2/glossary.md#event) goes live.

## Contents

```text
documentation_v2/mod/tbd-framework/UI/safe_start_hud/
└── safe_start_hud_specification.md  the notices as built, the shield they report, the design target
```

## Code

- [Game stages](/apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/Stages/) —
  `TBD_SafestartManager`: the shield, the countdown and every notice
- [Admin](/apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/) — the `#tbd safestart` command

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code README of the stage scripts, which links the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code README links it; the folder holds no
  design references until a mockup set is drawn.
