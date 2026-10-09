# Formation geometry source

The arrange commands of the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) as
pure point math over a selection of the [mission](/documentation/glossary/g_to_m.md#mission)'s placed
entities: the four patterns, align, space and orient, and garrison firing positions around a
building. Nothing here reads or writes the document.

## Contents

```text
crates/mission/formation_geometry/src/
├── alignment.rs  the six align edges, the three space-equally axes and the six orient commands
├── garrison.rs   evenly spaced firing positions around a building's oriented box, facing out
├── geometry.rs   the principal axis, the convex hull and its containment test, the scatter's seed
├── lib.rs        the crate root: module header, `mod` lines, re-exports of every type and function
├── patterns.rs   `Pt`, the four patterns, centroid, spread, bounds, bearing, the confirm threshold
├── prelude.rs    the types and commands a caller glob-imports
└── tests/        the goldens of the patterns, aligns, spacing, orient, hull and scatter
```

## How it works

The commands map a list of `Pt` (world metres, `x` east, `y` north) to a new list of the same
length and order, or to a yaw in degrees clockwise from north; the caller reads the positions from
the document, commits the result and owns any confirmation.

| Command | Result |
|---|---|
| `pattern_circular` | a ring around the centroid, radius the largest spread (5 m at least), index 0 due north |
| `pattern_line` | even spacing on the principal axis through the centroid, over the same span, in projection order |
| `pattern_grid` | the nearest-square grid of 5 m cells round the centroid, filled row by row from the north-west |
| `pattern_fill_area` | a scatter inside the convex hull, drawn from a `deterministic_random::SplitMix64` stream |
| `align_edge` | one axis snapped to a bounding-box edge or mid-line, the other kept |
| `space_equally` | interior points evened between the extremes on x, on y, or along the principal axis |
| `orient_yaw` | a cardinal heading, or the bearing to (or away from) the pivot, `None` on the pivot itself |
| `garrison_firing_positions` | `count` points spread along a perimeter 1 m inside the walls, each facing out; none for a box too small |

The scatter's seed is `seed_from_ids`, an order-independent combination of the selected ids, so a
selection scatters the same way however it was picked. `needs_confirm(n)` holds for more than
`DESTRUCTIVE_MOVE_THRESHOLD` (10) entities, and every pattern leaves fewer than two points as they
are. A degenerate spread falls back to a due-east axis, so no result is NaN. Only the tests call
`garrison_firing_positions`.

## Boundaries

- Depends on: `deterministic_random` (the scatter's generator) and the standard library
  (`DefaultHasher` for the seed).
- Used by:
  - `mission_operations::transform`, which reads the selection's positions and commits the
    patterns, aligns, spacing and orient;
  - the hosted selection transforms of `mission_editing_commands::hosted_commands`, the Mission
    Creator's arrange menu (`crates/frontend/workspaces/mission_creator_workspace/src/ui/docks/top_strip/arrange.rs`)
    and its bulk confirmation (`crates/frontend/workspaces/mission_creator_engine_bridge/src/bridge/host_state/`), which
    name its vocabulary directly;
  - `crates/mission/mission_operations/tests/operation_boundaries.rs`.
- Rules:
  - the patterns, aligns, spacing, orient and garrison positions match their goldens, and a move
    needs confirming only above 10 entities (`confirm_threshold_boundary`), in
    `crates/mission/formation_geometry/src/tests/placement_goldens.rs`;
  - the scatter is deterministic, stays inside the hull and ignores the order of the ids
    (`fill_area_deterministic_and_contained`, `fill_area_seed_order_independent`, same file);
  - no source file here calls `ensure_default_squad`.
