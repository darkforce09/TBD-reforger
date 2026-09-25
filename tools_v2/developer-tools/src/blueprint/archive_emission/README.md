# Blueprint and library emission

The blueprint compiler's writers: the assembly of one building's interpretation into a
schema-checked blueprint JSON, the whole-catalogue prefab occluder library of
`bvh-batch --all-prefabs`, and the archive that folds the library and every blueprint into
`prefabs/building_blueprints.rkyv` for the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s line of sight.

## Contents

```text
tools_v2/developer-tools/src/blueprint/archive_emission/
├── archive_command.rs     the `bvh-batch --all-prefabs` arm and its report; `run_archive`
├── archive_writer.rs      `build` and `run`: descriptors, mesh index and blueprints folded into rkyv
├── blueprint_assembly.rs  `assemble`, `validate_and_write`: bands to a checked `BuildingBlueprint`
├── library_reader/        the library builder, its manifest and its writer
└── library_reader.rs      `PrefabRow`, `LibraryOptions` and `Library`; re-exports the builder
```

## How it works

The files are modules of the blueprint root, declared in
`tools_v2/developer-tools/src/blueprint/mod.rs` by `#[path]` as `emit`, `library_cli`,
`archive_emit` and `library`.

```text
blueprint-from-voxels
  bands ──assemble──▶ BuildingBlueprint ──validate_and_write──▶ buildings/<slug>.json
bvh-batch --all-prefabs
  catalogue ──build_library──▶ descriptors/*.json, blas/*.bvh, blas-manifest.json
blueprint-from-voxels archive
  blas-manifest.json + descriptors/ + buildings/ ──build──▶ building_blueprints.rkyv
```

`blueprint_assembly.rs` turns each band's walls, footprint, floor polygons and plate heights into a
level in the building's local frame, adds the roof grid, the overall footprint, the vertical
profile and the furniture records, and emits the legal categories `generic` and `prop`.
`validate_and_write` drops the null optionals the contract types by omission, validates the
document against `contracts_v2/definitions/building-blueprint.schema.json`, and writes it.

`archive_command.rs` parses `bvh-batch --all-prefabs [--terrain everon] [--only-kind K]…
[--limit N] [--hot N] [--dry-run] [--all-layers] [--paks <dir>] [--extract <dir>] [--out <dir>]`,
builds the library through `library_reader/`, prints the census, and writes it under
`assets_v2/terrains/<terrain>/prefabs/` unless `--dry-run`. Its `run_archive` hands
`blueprint-from-voxels archive` to `archive_writer.rs`.

`archive_writer.rs` reads `blas-manifest.json`, every `descriptors/<pid>.json` and every blueprint
in `buildings/` (skipping `.instances.json` and `.scene.json` by name) and folds them into a
`BuildingBlueprintArchive`: the descriptor census, the shared mesh index the descriptors point
into, and each blueprint's tactical levels. It refuses rather than approximates: the descriptor
files and the manifest's list must be the same set, every mesh a descriptor names must be in the
library, `blocks` and `localBounds` must agree, and every blueprint's prefab slug must name a
descriptor. It reads the bytes back through `access_checked` before writing them with
`write_if_changed`, and it prints the building prefabs that still have no blueprint levels with
the Workbench and `blueprint-from-voxels` loop that fills them. The archive keeps a tactical
subset of each blueprint: among what it drops are each door's hinge side, swing direction and
default state and each furniture record's footprint size (`wire_blueprint`).

## Public surface

- `developer_tools::blueprint::run`, the blueprint root's `blueprint-from-voxels` entry, reaches
  `archive` through `run_archive`; `developer_tools::blueprint::run_bvh_batch` reaches the
  `--all-prefabs` arm. Both are `cargo xtask map` commands.
- Nothing else crosses the blueprint compiler's boundary.

## Boundaries

- Depends on:
  - the blueprint root's `types`, `params`, `walls`, `roof`, `march` and `hull` modules, and the
    batch walk, sources and writer in `tools_v2/developer-tools/src/blueprint/bvh/`;
  - `website_map_engine::world::architecture::blueprint` (the JSON blueprint),
    `website_map_engine::io::archives` (`BuildingBlueprintArchive`, the codec, the archive schema
    version) and `website_map_engine::spatial::los::world::descriptor` (descriptors, manifest);
  - `crate::repository_layout`, `jsonschema`, and the schemas in `contracts_v2/definitions/`.
- Used by: the blueprint root's `run` and `interpret_one`; `run_bvh_batch` in
  `tools_v2/developer-tools/src/blueprint/bvh/batch_processing/run_bvh_batch.rs`; through them
  `cargo xtask map blueprint-from-voxels` and `cargo xtask map bvh-batch --all-prefabs`.
- Rules:
  - the archive round-trips every committed descriptor and carries every committed blueprint level
    (`archive_round_trips_every_committed_descriptor`,
    `archive_carries_every_committed_blueprint_level` in
    `tools_v2/developer-tools/src/blueprint/tests/archive_emit/tests.rs`), so a change to the
    manifest, a descriptor or a blueprint is followed by a rebuilt archive;
  - a blueprint passes the schema before it is written
    (`box_room_blueprint_passes_the_schema_contract` in
    `tools_v2/developer-tools/src/blueprint/tests/emit/tests.rs`);
  - a filtered library run never writes a manifest.

## Related documentation

- [Everon prefab geometry](/assets_v2/terrains/everon/prefabs/README.md) — the library and archive
  these writers produce.
- [Prefab occluder descriptors](/apps/website/map-engine/src/spatial/los/world/descriptor/README.md)
  — the descriptor, manifest and archive model the map engine reads.
