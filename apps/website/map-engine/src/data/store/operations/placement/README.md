# Placement geometry

The arrange commands of the [Mission Creator](/documentation_v2/glossary.md#mission-creator) as
pure point math over a selection of the [mission](/documentation_v2/glossary.md#mission)'s placed
entities: the four patterns, align, space and orient, and garrison firing positions around a
building. Nothing here reads or writes the document.

## Contents

```text
apps/website/map-engine/src/data/store/operations/placement/
├── alignment.rs  the six align edges, the three space-equally axes and the six orient commands
├── garrison.rs   evenly spaced firing positions around a building's oriented box, facing out
├── geometry.rs   the principal axis, the convex hull and its containment test, the scatter's seed
├── mod.rs        the module tree; re-exports every type and function of the four files
└── patterns.rs   `Pt`, the four patterns, centroid, spread, bounds, bearing, the confirm threshold
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
| `pattern_fill_area` | a scatter inside the convex hull, drawn from a `SplitMix64` stream |
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

- Depends on: the standard library alone (`DefaultHasher` for the seed).
- Used by:
  - `crate::data::store::operations::transform`, which reads the selection's positions and commits
    the patterns, aligns, spacing and orient;
  - `crate::editing::tools::placement`, which re-exports this vocabulary for the hosted selection
    transforms of `crate::editing::hosted_commands` and for the Mission Creator's arrange menu
    (`apps/website/frontend/src/v2/apps/editor/ui/docks/top_strip/arrange.rs`) and bulk
    confirmation (`apps/website/frontend/src/v2/apps/editor/bridge/host_state/`);
  - `apps/website/map-engine/tests/operation_boundaries.rs`.
- Rules:
  - the patterns, aligns, spacing, orient and garrison positions match their goldens, and a move
    needs confirming only above 10 entities (`confirm_threshold_boundary`), in
    `apps/website/map-engine/src/editing/tools/tests/placement.rs`;
  - the scatter is deterministic, stays inside the hull and ignores the order of the ids
    (`fill_area_deterministic_and_contained`, `fill_area_seed_order_independent`, same file);
  - every source file here is on the place path that `cargo xtask verify editor-orbat-coherency`
    scans for `ensure_default_squad`.
