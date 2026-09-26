# World object and road builders

The builders behind `world build-objects`, `world build-roads`, `world redensify` and `world
gen-density-fixture`: they turn a staged [Workbench](/documentation_v2/glossary/n_to_z.md#workbench)
world export into a terrain's committed object chunks, prefab catalogue, density tiles, forest
regions, census and road network, and re-derive the density tiles from what is committed.

## Contents

```text
tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner/
├── build_world_objects_opt.rs   density, regions, census, archives, manifest patch, log
├── may_clear_density_dir.rs     `build_world_objects`, phase kinds, registry row, density guard
├── object_partitioning.rs       classifies and partitions the staged export; writes chunks
└── redensify_from_committed.rs  density rebuild, density fixture, road build and road census
```

## How it works

`tools_v2/developer-tools/src/world_export_pipeline/chunk_partitioner.rs` declares the four files
with `#[path]`, holds the row types, `CHUNK_SIZE_M` (512) and `PHASE_ORDER` (`P1_buildings` to
`P5_props`), and re-exports the entry points. Paths below are under `assets_v2/`.

```text
scratch/<terrain>/export/raw-entities.jsonl, export-meta.json, staged-meta.json
   │  prepare_world_objects: phase kinds, bounds, prefab-classify.json rules, dedupe, partition
   ▼
terrains/<terrain>/objects/prefabs.json.gz, chunks/<cx>_<cy>.json.gz + .bin, chunks/manifest.json
   │  build_world_objects_opt (tree phases only: density and regions)
   ▼
objects/density/<cx>_<cy>.bin, forest-regions.json.gz, type-inventory.json, and their .rkyv twins
   │  --patch-manifest: manifest.json `objects` block
   │  --ops-log: .ai/artifacts/map_export_<terrain>.json
```

- A phase is cumulative: `phase_kinds` admits buildings for `P1_buildings`, adds trees and water
  for `P2_trees`, then vegetation, rocks, and props with vehicles. A phase that includes trees
  rebuilds all density tiles (the only case `may_clear_density_dir` allows the folder to be
  cleared), derives the forest regions and smooths their rings; other phases leave them alone.
- The terrain must be a square with its origin at 0, 0 in `terrain-registry.json`; a staged file
  that is missing exits the process with code 2.
- `build_roads_from_topo` decodes the terrain's `.topo` file from the game's archives, refuses to
  write an empty network and writes `objects/roads.json.gz`; the `world` CLI then writes its
  archive through `roads_emit`.
- `redensify_from_committed` rebuilds `objects/density/*.bin` from the committed catalogue and
  chunks alone, with the same canopy blur. `gen_density_fixture` rewrites
  `contracts_v2/fixtures/map/density/density-fixture.bin` and the expected corners in its JSON.
- The `_opt` variants take a `quiet` flag; `world verify-phase` calls them into scratch folders to
  prove the build is deterministic.

## Boundaries

- Depends on: the sibling modules `classify`, `binary_emit`, `catalog_emit`, `forest_contours`,
  `forest_smoothing`, `json_number_formatting`, `polygon_geometry`, `topo` and
  `vegetation_density`; `crate::enfusion_pak::PakVfs` for the road build;
  `website-map-engine`'s `io::density::tbdd` encoder; `crate::repository_layout`.
- Used by: `tools_v2/developer-tools/src/world_export_pipeline/cli.rs`; the determinism gate in
  `tools_v2/developer-tools/src/world_export_pipeline/mathematical_verification/`; the road census
  in `world reclassify`; `cargo xtask map export-terrain`, which runs `world build-objects
  --patch-manifest --ops-log` and `world build-roads --ops-log`.
- Rules: only a tree phase or `redensify` clears `objects/density/`
  (`non_density_phase_must_not_clear`, `clear_density_wipes_when_rebuilding`); an empty catalogue,
  density set or road network is refused before it overwrites the committed one, through the
  pipeline's `refuse_empty_write` (`refuse_empty_catalog_write_contract`), all three in
  `tools_v2/developer-tools/src/world_export_pipeline/tests/chunk_partitioner/tests.rs`; each
  chunk's JSON and binary forms are written together.

## Related documentation

- [Everon world objects](/assets_v2/terrains/everon/objects/README.md) — the files these builders
  write.
