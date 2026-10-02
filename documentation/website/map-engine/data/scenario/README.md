**Status:** live

# Mission domain documentation

The feature documentation of the map engine's mission domain module, `data/scenario/` (the code
keeps the spelling [scenario](/documentation/glossary/n_to_z.md#scenario); prose says
[mission](/documentation/glossary/g_to_m.md#mission)), for the subjects that outgrow its code
READMEs.

## Contents

```text
documentation/website/map-engine/data/scenario/
└── ballistics/  the game ballistics: flight model, solver, calibration, fire-mission assembly
```

## Code

- [Mission domain](/apps/website/map-engine/src/data/scenario/) — the module whose subjects the
  documents below describe.

## Boundaries

- Depends on: the code of `apps/website/map-engine/src/data/scenario/`; the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [mission data documentation](/documentation/website/map-engine/data/README.md)
  index.
- Rules: the folders mirror the code path and keep its spelling; a folder exists only when a
  feature doc lives below it.

## Related documentation

- [Map engine overview](/documentation/website/map-engine/map_engine_overview.md) — the crate's
  layers and feature tiers, the `scenario` tier included.
