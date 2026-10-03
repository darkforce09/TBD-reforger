# Paper doll scene

The `paper_doll_scene` crate: the schematic soldier the
[arsenal](/documentation/glossary/a_to_f.md#arsenal)'s paper doll shows in the
[Mission Creator](/documentation/glossary/g_to_m.md#mission-creator). It names the 14 clickable
equipment regions, builds the soldier from scaled unit cubes and one cylinder, colours each region
by its loadout state, and answers which region lies under a pixel and where a region's callout
belongs. It has no GPU and no browser code; `paper_doll_renderer` draws it.

## Contents

```text
crates/paper_doll/paper_doll_scene/
├── Cargo.toml  the package: `camera_math`; layout tier 2, any target
└── src/        the soldier's parts and regions, the unit meshes, the picking and their tests
```

## How it works

A region's index in `REGION_KEYS` is its number in a part, in the state bytes a renderer takes
and in the index a pick returns. `instances()` lists the soldier's 22 parts, each a unit mesh and
a column-major model matrix; `pick` casts a ray through a pixel with the orbit camera of
`camera_math` and returns the nearest clickable part's region, and `anchor_px` projects a region's
anchor with the same camera. The source README details the regions, the parts and the picking.

## Getting started

Run from the repository root:

```bash
cargo test -p paper_doll_scene   # regions, parts, meshes, colours, picks and anchors
```

## Configuration

None: no feature and no environment variable.

## Public surface

- `soldier_parts`, `part_meshes` and `region_picking`, and `prelude` with all their names.

## Boundaries

- Depends on: `camera_math` (the 4x4 matrix helpers and the orbit camera's projections).
- Used by: `paper_doll_renderer` (`crates/paper_doll/paper_doll_renderer/`).
- Rules: paper doll category, tier 2 (`cargo xtask verify crate-tiers`); no GPU, browser or UI
  crate; the picks use the camera the renderer draws with.

## Related documentation

- [Paper doll crates](/crates/paper_doll/README.md) — the scene and the renderer together.
