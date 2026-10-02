**Status:** live

# Game ballistics documentation

The feature documentation of the map engine's game ballistics, the mortar model the mortar
calculator and the [API](/documentation/glossary/a_to_f.md#api) share: a flight model identified
from the game engine, the firing solver, the calibration a catalog must pass and the assembled
fire-mission solution. Developers and AI agents read it before changing the model or its
consumers.

## Contents

```text
documentation/website/map-engine/data/scenario/ballistics/
└── game_ballistics_engine.md  the feature doc: catalog to solution, calibration, consumers, decisions
```

## Code

- [Game ballistics](/apps/website/map-engine/src/data/scenario/ballistics/) — the module the
  feature doc describes; its README and the READMEs of `flight_model/`, `solver/`, `catalog/` and
  `calibration/` hold the exact rules and the tests that pin them.

## Boundaries

- Depends on: the [feature doc template](/documentation/standards/templates/feature_doc.md); the
  code of the ballistics module, the API's catalog and fire-mission handlers, the mortar page and
  the ticket registry the feature doc is written from.
- Used by: the mortar calculator and ballistics catalogs page feature docs, the game ballistics
  design note and the API overview, which link the feature doc.
- Rules: the feature doc keeps its name, which those links use, and stays within 500 lines; the
  design note in the verification evidence keeps the operator decisions and the evidence, and the
  feature doc links it rather than repeating it.

## Related documentation

- [Game ballistics design note](/documentation/website/api_v2/verification_evidence/game_ballistics.md)
  — the operator decisions, the identified engine scheme, the calibration criterion and the
  register.
- [Ballistics oracle](/documentation/mod/tbd-export/Scripts/WorkbenchGame/BallisticsOracle/ballistics_oracle.md)
  — the engine measurements the model is calibrated against.
