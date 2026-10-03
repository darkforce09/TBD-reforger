# Interior line of sight

The `interior_line_of_sight` crate: line of sight inside one building. It traces a sight line
through a building compound's shell and placed instances, says whether anything opaque stands on
it, and evaluates it into a verdict that names every wall, door, pane, piece of furniture and
foliage volume it met; it rasters each floor's visibility around an observer. The one sight-line
evaluation, the reduction of a scene's crossings into named hits, a blocker and a concealment,
lives here, and the world line of sight evaluates its placed prefabs through it.

## Contents

```text
crates/line_of_sight/interior_line_of_sight/
├── Cargo.toml  the package: `building_interiors`, `terrain_line_of_sight`, `spatial_indexes`, layout tier 4
└── src/        the compound walk, the sight-line evaluation, the floor wash
```

## How it works

```text
CompoundBuilding ── trace / blocked ──────────────► crossings, yes-or-no      (compound_walk)
       │
       └── evaluate_los ── CompoundSightLine ──┐
                                               ├─► evaluate_los(scene) ──► SightLineEvaluation
world_line_of_sight ── WorldSightLine ─────────┘    (sight_line_evaluation)    └─► LosResult / WorldLos
any blocking test ── wash_band / WashJob ─────────► LevelWash raster          (floor_wash)
```

A segment is traced in the building's frame (`t` 0 at the observer, 1 at the target). The first
opaque crossing stops it and is named after the blueprint feature the shell hit belongs to, or
after the instance hit; each glass pane adds `GLASS_CONCEALMENT` (0.05), its two collider faces
within `PANE_MERGE_M` counting once; foliage adds `1 − exp(−FOLIAGE_K · depth)` for the metres of
canopy crossed; an open door leaf the ray passes adds an aperture hit. The concealment is
`1 − Π(1 − cᵢ)` over the pass-through hits, and 1 when blocked.

A floor wash is a square grid of `WASH_CELL_M` cells (0.25 m) over a disc of `WASH_RADIUS_M`
(25 m) by default, rows north first, each cell decided by one ray from the observer to a point
`WASH_EYE_M` (1 m) above the floor; a disc wider than `MAX_WASH_DIM` (2048) cells a side coarsens
the cell to fit. `WashJob` decides the same cells in batches of `WASH_BATCH_CELLS` (256).

## Getting started

Run from the repository root:

```bash
cargo test -p interior_line_of_sight   # compound walk, verdicts, washes, sliced wash
```

## Public surface

- `compound_walk`: the `CompoundLineOfSight` trait on `CompoundBuilding` (`trace`,
  `trace_range`, `blocked`, `blocked_range`, `evaluate_los`), `Owner`, `TraceEvent`, and the
  instance functions the world line of sight reuses: `trace_instances`,
  `blocked_instances_where`, `hit_kind_of`.
- `sight_line_evaluation`: `SightLineScene`, `evaluate_los`, `SightLineEvaluation`,
  `GLASS_CONCEALMENT`, `FOLIAGE_K`, `PANE_MERGE_M`.
- `floor_wash`: `wash_band`, `WashJob`, `WashParams`, `LevelWash`, `level_wash`,
  `level_washes`, `level_wash_compound`, `level_washes_compound`, `compound_wash`,
  `wash_cap_check` and the wash constants.
- `Error` and `Result` (`error`), and `prelude`, which re-exports the main items.

## Boundaries

- Depends on: `building_interiors` (the compound, the blueprint attribution and `LosResult`),
  `spatial_indexes` (BVH traversal, sidecars, surface kinds, the segment-box window),
  `terrain_line_of_sight` (`Visibility`, `ViewshedCapRefused`), `geometry_primitives` and
  `thiserror`.
- Used by:
  - `world_line_of_sight`, which traces expanded prefabs and evaluates its verdicts through it;
  - `map_editing_tools`: its line-of-sight tool and visibility scheduler's wash lane
    (`crates/mission_editing/map_editing_tools/src/`);
  - the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s input handlers,
    the debug building viewer and interior bench in `apps/frontend/`, and the blueprint tooling
    in `tools/map_assets/blueprint_compiler/src/`.
- Rules: glass and foliage conceal but never block
  (`glass_conceals_five_percent_per_pane_and_never_blocks`,
  `foliage_conceals_by_depth_and_trunks_block`); a wash radius over `MAX_WASH_RADIUS_M` (400 m)
  is refused without casting a ray (`over_cap_wash_radius_is_refused_with_a_message`); a sliced
  wash equals the one-call wash (`sliced_wash_is_bit_identical_to_the_sync_path`); line of sight
  tier 4 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Line of sight crates](/crates/line_of_sight/README.md) — the three layers and how they share
  their vocabulary.
- [Building interiors](/crates/world_objects/building_interiors/README.md) — the compound and
  blueprint model the walk traces.
