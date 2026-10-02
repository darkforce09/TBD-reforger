# Map asset commands

The `cargo xtask map` group: the terrain export that turns a staged Workbench world export into
the committed object and road artifacts, the map tile index the offline pack reads, and the building-blueprint and line-of-sight tools that
build and check the occlusion data the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s line-of-sight tool uses. Map
and mod developers run them by hand; the work itself lives in the `developer_tools` crate.

## Contents

```text
tools/xtask/src/commands/map/
├── cli.rs             the `MapCmd` clap enum: fourteen commands, each taking its arguments raw
├── dispatch.rs        routes each `MapCmd` to its adapter
├── mod.rs             the module tree; one adapter per developer_tools entry, given the checkout root
├── terrain_export.rs  `export-terrain`: phase gate, staged export check, object and road builds
├── tile_index.rs      `tile-index`: the map tile pyramid's `index.json`, for the offline pack
└── tests/             unit tests for the export-terrain parser and the tile index writer
```

## How it works

`tools/xtask/src/cli/dispatch.rs` passes the parsed `MapCmd` to `dispatch::run`. Every command
but `export-terrain` and `tile-index` is a one-line adapter in `mod.rs` that finds the checkout root and hands the
raw arguments to a `developer_tools` entry, which parses them, does the work and returns the exit
code: `blueprint::run` (`blueprint-from-voxels`), `blueprint::ingest::run`,
`blueprint::parity_report::run`, `blueprint::run_voxels_from_mesh`, `run_bvh_parity`,
`run_bvh_emit`, `run_bvh_batch`, `run_xob_inspect`, `run_pak_cat`, `run_instances_verify`,
`run_rotation_pin`, and `map_verification::world_line_of_sight::run` (`world-los`). The adapters
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

`tile-index` runs in-process: it reads the terrain's `manifest.json` for `tiles.map.path`, the
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
- Exit codes: those of the `developer_tools` entry: 0 done, non-zero a failure.
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
- Exit codes: those of the `developer_tools` entry: 0 done, non-zero a failure.
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

- Depends on: `developer_tools::blueprint`, `developer_tools::map_verification` and
  `developer_tools::repository_layout::map_scratch_dir`; `crate::core::repository_root`;
  `verification_core::proc`; cargo, for the `world` binary; the game paks and the Workbench
  exports each command reads.
- Used by: `tools/xtask/src/cli/dispatch.rs`; people, following the export and blueprint steps
  in `assets/terrains/README.md` and the `apps/mod/tbd-export/` plugins, whose output these
  commands read.
- Rules: `export-terrain` runs the phase gate before anything is built, and a missing staged
  export is exit 2, never a build over nothing; the argument parser keeps its defaults and refusals
  (`parse_phase_and_default`, `parse_unknown_arg` in `tests/terrain_export/tests.rs`); `tile-index`
  never writes an index over a missing or empty pyramid, and what it writes validates against
  `map-tile-index.schema.json` (`tests/tile_index/tests.rs`); the crate
  takes no dependency on `map_engine`
  (`tools/xtask/src/tests/tooling_dependency_boundaries.rs`), so engine-backed work stays in
  `developer_tools`.

## Related documentation

- [Building blueprint pipeline](/tools/developer_tools/src/blueprint/README.md) — the code
  behind the blueprint, sidecar and parity commands.
- [World export pipeline](/tools/developer_tools/src/world_export_pipeline/README.md) — the
  `world` binary that `export-terrain` runs.
- [Terrains](/assets/terrains/README.md) — the terrain artifacts these commands write.
