**Status:** live

# Ballistics oracle documentation

The deeper document of the export addon's ballistics oracle: what it asks the engine, why it runs
in two halves, and how its outputs become the calibration fixtures of a ballistics catalog.

## Contents

```text
documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/
└── ballistics_oracle.md  feature doc: the two halves, their rules, outputs, decoding and decisions
```

## Code

- [Ballistics oracle plugin](/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/) — the
  Workbench menu entry, the forward-angle writer and the operator procedure.
- [Ballistics oracle game scripts](/mod/tbd-export/Scripts/Game/TBD/Export/BallisticsOracle/)
  — the play-mode simulation component and the classes both halves share.

## Boundaries

- Depends on: the [feature doc](/documentation/standards/templates/feature_doc.md) template;
  the code READMEs above, which hold the file formats and the procedure.
- Used by: the [Workbench exporter documentation](/documentation/mod/tbd-export/Scripts/WorkbenchGame/README.md)
  and the code READMEs, which link the feature doc.
- Rules: the formats and the operator steps stay in the code READMEs, and the feature doc links
  them.
