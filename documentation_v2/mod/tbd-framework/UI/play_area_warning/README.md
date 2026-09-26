**Status:** live

# Play area warning documentation

The documentation of the out-of-bounds warning a player gets when they leave the area of
operations of a round of an [event](/documentation_v2/glossary/a_to_f.md#event).

## Contents

```text
documentation_v2/mod/tbd-framework/UI/play_area_warning/
└── play_area_warning_specification.md  the warning as built, its rules, the design target
```

## Code

- [Mission zones](/apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Zones/) —
  `TBD_PlayAreaComponent`, the enforcer and its messages, and the zone registry

## Boundaries

- Depends on: the [feature doc template](/documentation_v2/standards/templates/feature_doc.md).
- Used by: the in-code README of the zones folder, which links the specification; the
  [mod UI index](/documentation_v2/mod/tbd-framework/UI/README.md).
- Rules: the specification keeps its path, since the in-code README links it; the folder holds no
  design references until a mockup set is drawn.
