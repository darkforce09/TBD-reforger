**Status:** live

# Map engine mission data documentation

The feature documentation of the map engine's mission data module, `data/`, for the subjects that
outgrow its code READMEs. Developers and AI agents read it below those READMEs.

## Contents

```text
documentation_v2/website/map-engine/data/
└── scenario/  the mission domain module: the game ballistics under it
```

## Code

- [Mission data](/apps/website/map-engine/src/data/) — the mission domain and the CRDT store the
  documents below describe.

## Boundaries

- Depends on: the code of `apps/website/map-engine/src/data/`; the
  [documentation folder README template](/documentation_v2/standards/templates/readme_documentation_folder.md).
- Used by: the [map engine documentation](/documentation_v2/website/map-engine/README.md) index.
- Rules: the folders mirror the code path under `apps/website/map-engine/src/` and keep its
  spelling; a folder exists only when a feature doc lives below it.

## Related documentation

- [Map engine overview](/documentation_v2/website/map-engine/map_engine_overview.md) — the crate's
  layers and where the mission data sits among them.
