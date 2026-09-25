# Map asset commands

The `cargo xtask map` group: the terrain export that turns a staged Workbench world export into
the committed object and road artifacts, and the building-blueprint and line-of-sight tools that
build and check the occlusion data the
[Mission Creator](/documentation_v2/glossary.md#mission-creator)'s line-of-sight tool uses. Map
and mod developers run them by hand; the work itself lives in the `developer-tools` crate.

## Contents

```text
tools_v2/xtask/src/commands/map/
├── cli.rs             the `MapCmd` clap enum: thirteen commands, each taking its arguments raw
├── dispatch.rs        routes each `MapCmd` to its adapter
├── mod.rs             the module tree; one adapter per developer-tools entry, given the checkout root
├── terrain_export.rs  `export-terrain`: phase gate, staged export check, object and road builds
└── tests/             unit tests for the export-terrain argument parser
```

## How it works

`tools_v2/xtask/src/cli/dispatch.rs` passes the parsed `MapCmd` to `dispatch::run`. Every command
but `export-terrain` is a one-line adapter in `mod.rs` that finds the checkout root and hands the
raw arguments to a `developer_tools` entry, which parses them, does the work and returns the exit
code: `blueprint::run` (`blueprint-from-voxels`), `blueprint::ingest::run`,
`blueprint::parity_report::run`, `blueprint::run_voxels_from_mesh`, `run_bvh_parity`,
`run_bvh_emit`, `run_bvh_batch`, `run_xob_inspect`, `run_pak_cat`, `run_instances_verify`,
`run_rotation_pin`, and `map_verification::world_line_of_sight::run` (`world-los`). The adapters
import no map-engine types.

`export-terrain` runs the `world` binary of `developer-tools` three times through
`cargo run -q -p developer-tools --bin world --`, printing each run's output when it ends:

```text
export-terrain <terrain> [--phase Pn]
  ├─ world phase-gate --terrain T --phase P            non-zero: stop with its code
  ├─ <map scratch dir of T>/export/raw-entities.jsonl missing:
  │     print the Workbench export and staging steps, exit 2
  ├─ world build-objects --terrain T --phase P --patch-manifest --ops-log
  ├─ world build-roads --terrain T --ops-log
  └─ print the `world verify-phase` command that checks the result
```

The staged export sits under `assets_v2/scratch/<terrain>/`, which git ignores.

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

### Building blueprints

- Synopsis: `cargo xtask map ingest-blueprints [--src <dir>] [--filter <substr>]`;
  `cargo xtask map blueprint-from-voxels [--filter <substr>] [--algo segments|grid] [--src <dir>]
  [--out <dir>] [--params <file.json>] [--debug-dir <dir>]`;
  `cargo xtask map voxels-from-mesh --mesh <file.xob> --slug <s> [options]`.
- Does: `ingest-blueprints` copies the building blueprints the `TBD_Export` Workbench profile wrote
  into `assets_v2/terrains`, validated against the `BuildingBlueprint` contract;
  `blueprint-from-voxels` interprets raw Workbench voxel dumps into blueprint JSON offline;
  `voxels-from-mesh` writes the same voxel dump by ray-marching a game `.xob` model, the fire
  collision geometry by default.
- Exit codes: those of the `developer-tools` entry: 0 done, non-zero a failure.
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
- Exit codes: those of the `developer-tools` entry: 0 done, non-zero a failure.
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
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; people, following the export and blueprint steps
  in `assets_v2/terrains/README.md` and the `apps/mod/tbd-export/` plugins, whose output these
  commands read.
- Rules: `export-terrain` runs the phase gate before anything is built, and a missing staged
  export is exit 2, never a build over nothing; the argument parser keeps its defaults and refusals
  (`parse_phase_and_default`, `parse_unknown_arg` in `tests/terrain_export/tests.rs`); the crate
  takes no dependency on `website-map-engine`
  (`tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`), so engine-backed work stays in
  `developer-tools`.

## Related documentation

- [Building blueprint pipeline](/tools_v2/developer-tools/src/blueprint/README.md) — the code
  behind the blueprint, sidecar and parity commands.
- [World export pipeline](/tools_v2/developer-tools/src/world_export_pipeline/README.md) — the
  `world` binary that `export-terrain` runs.
- [Terrains](/assets_v2/terrains/README.md) — the terrain artifacts these commands write.
