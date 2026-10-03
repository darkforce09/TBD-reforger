**Status:** live

# Library crate documentation

The feature documentation of the library crates under `crates/`, for the subjects that outgrow
their code READMEs. Developers and AI agents read it below those READMEs.

## Contents

```text
documentation/crates/
├── ballistics/       the game ballistics: flight model, solver, calibration, fire-mission assembly
├── graphics/         the GPU device and frame crates: one frame, the GPU context, sprite culling
├── map_rendering/    the map renderer and its typed GPU layers: from a mounted canvas to a drawn frame
├── streaming/        the streaming crates: boot, viewport passes, residency, memory budget, loaders
└── mission_editing/  the headless editing layer of the Mission Creator and its local drafts
```

## Code

- [Library crates](/crates/README.md) — the crates the documents below describe.

## Boundaries

- Depends on: the code of `crates/`; the
  [documentation folder README template](/documentation/standards/templates/readme_documentation_folder.md).
- Used by: the [documentation](/documentation/README.md) index.
- Rules: the folders mirror the crate categories under `crates/` and keep their spelling; a folder
  exists only when a feature doc lives below it.

## Related documentation

- [Workspace layout](/documentation/architecture/workspace_layout.md) — the library crates among
  the workspace members.
