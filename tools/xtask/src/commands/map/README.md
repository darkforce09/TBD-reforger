# Map asset commands

The `cargo xtask map` group: the terrain export that turns a staged Workbench world export into
the committed object and road artifacts, the map tile index the offline pack reads, and the
building-blueprint and line-of-sight tools that build and check the occlusion data the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s line-of-sight tool uses. Map
and mod developers run them by hand. This folder holds only the command line and the dispatch: the
export driver and the tile index writer live in the `world_export_pipeline` crate, the blueprint
and line-of-sight work in the `blueprint_compiler` and `map_asset_verification` crates, and the
export runs the `world` binary of `developer_tools`.

## Contents

```text
tools/xtask/src/commands/map/
├── cli.rs             the `MapCmd` clap enum: fourteen commands, each taking its arguments raw
├── dispatch.rs        routes each `MapCmd` to its adapter
└── mod.rs             the module tree; one adapter per map asset crate entry, given the checkout root
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes the parsed `MapCmd` to `dispatch::run`. `export-terrain`
and `tile-index` go straight to `world_export_pipeline::export_terrain_driver::run` and
`world_export_pipeline::map_tile_index::run`; every other command is a one-line adapter in
`mod.rs` that finds the checkout root and hands the raw arguments to a `blueprint_compiler` or
`map_asset_verification` entry, which parses them, does the
work and returns the exit code: the `blueprint_compiler` entries `run` (`blueprint-from-voxels`),
`ingest::run`, `parity_report::run`, `run_mesh_voxelization`, `run_occlusion_sidecar_parity`, `run_occlusion_sidecar_emission`,
`run_occlusion_sidecar_batch`, `run_xob_inspection`, `run_pak_file_print`, `run_instance_verification` and `run_rotation_validation`,
and `map_asset_verification`'s `world_line_of_sight::run` (`world-los`). The adapters
import no map-engine types.

`export-terrain` runs the `world` binary of `developer_tools` three times through
`cargo run -q -p developer_tools --bin world --`, printing each run's output when it ends:

```text
export-terrain <terrain> [--phase Pn]
  ├─ world phase-gate --terrain T --phase P            non-zero: stop with its code
  ├─ <map scratch dir of T>/export/raw-entities.jsonl missing:
  │     print the Workbench export and staging steps, exit 2
  ├─ world build-objects --terrain T --phase P --patch-manifest --ops-log
  ├─ world build-roads --terrain T --ops-log
  └─ print the `world verify-phase` command that checks the result
```

The staged export sits under `assets/scratch/<terrain>/`, which git ignores.

`tile-index` runs in process (`map_tile_index` is bookkeeping with no raster code, so xtask's
closure stays free of the raster pipeline): it reads the terrain's `manifest.json` for `tiles.map.path`, the
tile extension of `tiles.map.urlTemplate` and the zoom range, walks `<z>/<x>/<y>.<extension>`
under the pyramid, and writes `index.json` (`contracts/definitions/map-tile-index.schema.json`)
beside it, where `/map-assets/<terrain>/tiles/map/index.json` serves it from the API and the gate
server.

## Commands

Each runs as `cargo xtask map <command> <arguments>`. `--help` prints the argument summary clap
holds for each command, except on `export-terrain`, which takes `--help` as an argument.

### export-terrain

- Synopsis: `cargo xtask map export-terrain <terrain> [--phase <phase>]`; the terrain may come
  from `TERRAIN` instead, and the phase defaults to `P1_buildings`.
- Does: checks the phase against the terrain's registry limit, requires the staged raw export,
  then builds the object catalogue (patching the manifest) and the road network.
- Exit codes: 0 built; 1 no terrain, an unknown argument or `--phase` without a value, or a failed
  build; 2 the staged raw export is missing; 127 cargo is not installed; the phase gate's own code.
- Example: `cargo xtask map export-terrain everon --phase P1_buildings`

### tile-index

- Synopsis: `cargo xtask map tile-index --terrain <terrain>`.
- Does: writes `assets/terrains/<terrain>/tiles/map/index.json`, every tile of the map tile
  pyramid sorted by zoom, column and row with its size in bytes; warns when a zoom level of the
  manifest's range has no tile, because the offline pack then stays incomplete. The index is
  gitignored with the pyramid.
- Exit codes: 0 written; 1 no terrain, an unknown argument, an unreadable manifest, a template
  without a tile extension, or a tile outside the zoom range or its level's grid; 2 the pyramid
  is missing or holds no tile (nothing is written).
- Example: `cargo xtask map tile-index --terrain everon`

### Building blueprints

- Synopsis: `cargo xtask map ingest-blueprints [--src <dir>] [--filter <substr>]`;
  `cargo xtask map blueprint-from-voxels [--filter <substr>] [--algo segments|grid] [--src <dir>]
  [--out <dir>] [--params <file.json>] [--debug-dir <dir>]`;
  `cargo xtask map voxels-from-mesh --mesh <file.xob> --slug <s> [options]`.
- Does: `ingest-blueprints` copies the building blueprints the `TBD_Export` Workbench profile wrote
  into `assets/terrains`, validated against the `BuildingBlueprint` contract;
  `blueprint-from-voxels` interprets raw Workbench voxel dumps into blueprint JSON offline;
  `voxels-from-mesh` writes the same voxel dump by ray-marching a game `.xob` model, the fire
  collision geometry by default.
- Exit codes: those of the `blueprint_compiler` entry: 0 done, non-zero a failure.
- Example: `cargo xtask map ingest-blueprints --filter <substr>`

### Occlusion sidecars and game files

- Synopsis: `cargo xtask map bvh-emit --mesh <file.xob> --slug <slug> [--out <dir>]`;
  `cargo xtask map bvh-batch --prefab <Prefabs/…/X.et> [options]` or
  `cargo xtask map bvh-batch --all-prefabs [--terrain everon] [options]`;
  `cargo xtask map xob-inspect <file.xob | in-pak path> [options]`;
  `cargo xtask map pak-cat <in-pak path> [--paks <dir>] [--head <bytes>] [--out <file>]`.
- Does: `bvh-emit` writes a building's binary `.bvh` sidecar beside its blueprint; `bvh-batch`
  walks a prefab, or every catalogue prefab, straight out of the game paks into the shell
  sidecar, one BLAS per child model and the instance list; `xob-inspect` prints what the model
  decoder sees; `pak-cat` reads one entry of the game paks.
- Exit codes: those of the `blueprint_compiler` entry: 0 done, non-zero a failure.
- Example: `cargo xtask map pak-cat <in-pak path> --head 256`

### Parity and placement checks

- Synopsis: `cargo xtask map parity-report --pairs <parity.json> --blueprint <blueprint.json>
  --sidecar <file.bvh>`; `cargo xtask map bvh-parity (--mesh <file.xob> | --sidecar <file.bvh>)
  --pairs <parity.json> [options]`; `cargo xtask map world-los --cell <cx_cy> [options]`;
  `cargo xtask map instances-verify --instances <slug>.instances.json --recon <slug>_children.json
  [--world-row --chunk <cx_cy.json.gz> --prefabs <prefabs.json.gz>]`;
  `cargo xtask map rotation-pin --fixture <json>`.
- Does: replay the Workbench line-of-sight parity oracle against the offline raycasts, per
  building (`parity-report`, `bvh-parity`) and across the world catalogue (`world-los`); match
  every socket instance against a Workbench recon dump; and rank the 48 Euler compositions against
  a recon sample.
- Exit codes: 0 agreement; 1 a mismatch (`instances-verify`: over 2 cm or 1°, or an unmatched
  instance; `rotation-pin`: `Rigid::from_enfusion` does not win); other non-zero codes from the
  entry.
- Example: `cargo xtask map world-los --cell <cx_cy> --census`

## Boundaries

- Depends on: the `world_export_pipeline` (`export_terrain_driver`, `map_tile_index`),
  `blueprint_compiler` and `map_asset_verification` crates and `repository_layout`; cargo, for the
  `world` binary the export driver runs; the game paks and the Workbench exports each command
  reads.
- Used by: `tools/xtask/src/cli/dispatch.rs`; people, following the export and blueprint steps
  in `assets/terrains/README.md` and the `apps/mod/tbd-export/` plugins, whose output these
  commands read.
- Rules: `export-terrain` runs the phase gate before anything is built, and a missing staged
  export is exit 2, never a build over nothing; the argument parser keeps its defaults and refusals
  (`parse_phase_and_default`, `parse_unknown_arg` in
  `tools/map_assets/world_export_pipeline/src/tests/export_terrain_driver/tests.rs`); `tile-index`
  never writes an index over a missing or empty pyramid, and what it writes validates against
  `map-tile-index.schema.json`
  (`tools/map_assets/world_export_pipeline/src/tests/map_tile_index/tests.rs`); this folder holds
  no command logic, and xtask depends only on tool crates and the checkout-root finder
  (`tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`), so the raster
  work stays in the `map` binary, outside xtask's dependency closure.

## Related documentation

- [Building blueprint pipeline](/tools/map_assets/blueprint_compiler/src/README.md) — the code
  behind the blueprint, sidecar and parity commands.
- [World export pipeline](/tools/map_assets/world_export_pipeline/src/README.md) — the export
  driver, the tile index writer and the `world` binary that `export-terrain` runs.
- [Terrains](/assets/terrains/README.md) — the terrain artifacts these commands write.
