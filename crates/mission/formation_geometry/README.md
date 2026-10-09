# Formation geometry

The `formation_geometry` crate: the arrange commands of the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator) as pure point math over a
selection of the [mission](/documentation/glossary/g_to_m.md#mission)'s placed entities. The four
placement patterns, align, space and orient, and garrison firing positions around a building;
nothing here reads or writes a mission document.

## Contents

```text
crates/mission/formation_geometry/
├── Cargo.toml  the package: `deterministic_random`, layout tier 1
└── src/        the patterns, align, space, orient, garrison positions and their shared geometry
```

## How it works

Every command maps a list of `Pt` (world metres, `x` east, `y` north) to a new list of the same
length and order, or to a yaw in degrees clockwise from north. The caller reads the positions from
the document, commits the result and owns any confirmation; `needs_confirm(n)` holds above
`DESTRUCTIVE_MOVE_THRESHOLD` (10) entities. The fill-area scatter draws from a
`deterministic_random::SplitMix64` stream seeded by `seed_from_ids`, an order-independent
combination of the selected ids. The command table is in the [source README](src/README.md).

## Getting started

Run from the repository root:

```bash
cargo test -p formation_geometry   # the placement goldens
```

## Configuration

No features and no environment variables.

## Public surface

- `Pt`, `PatternKind`, `AlignEdge`, `SpaceAxis`, `Orient`, `FiringPosition`,
  `DESTRUCTIVE_MOVE_THRESHOLD`.
- Patterns: `pattern_circular`, `pattern_line`, `pattern_grid`, `pattern_grid_cell`,
  `pattern_fill_area`; measures: `centroid`, `max_spread`, `bounds`, `bearing_from_to`,
  `needs_confirm`.
- Align, space and orient: `align_edge`, `space_equally`, `space_along_line`,
  `space_axis_aligned`, `orient_yaw`.
- Geometry: `principal_axis`, `convex_hull`, `point_in_convex_hull`, `seed_from_ids`.
- Garrison: `garrison_firing_positions`, `perimeter_point`.
- `prelude` holds the types and the commands a caller uses.

## Boundaries

- Depends on: `deterministic_random`.
- Used by: the document operations of `mission_operations` (`transform`), the map engine's
  hosted selection transforms, and the Mission Creator's arrange menu and bulk confirmation.
- Rules: mission tier 1, so the crate depends on foundation crates only
  (`cargo xtask verify crate-tiers`); no source file calls `ensure_default_squad`.

## Related documentation

- [Mission crates](/crates/mission/README.md) — the mission domain's crates and their tiers.
