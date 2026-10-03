# Line of sight crates

The engine category for every visibility question the map answers, at three scales: over the bare
ground of the elevation model, inside one building, and through every object placed on the
streamed terrain. The three are separate layers, each with its own data structure and cost, and
share their vocabulary rather than their work.

## Contents

```text
crates/line_of_sight/
├── interior_line_of_sight/  `interior_line_of_sight`: compound traces, the shared sight-line evaluation, floor washes
├── terrain_line_of_sight/   `terrain_line_of_sight`: elevation profiles and viewsheds, whole or sliced
└── world_line_of_sight/     `world_line_of_sight`: chunk box trees, the prefab occluder library, verdicts with coverage
```

## How it works

| Layer | Answers | Traces through | Tier |
|---|---|---|---|
| `terrain_line_of_sight` | a profile along a line; a viewshed raster | a radial march over the elevation model | 3 |
| `interior_line_of_sight` | a verdict in one building; floor rasters | the building's own meshes | 4 |
| `world_line_of_sight` | a verdict with its crossings and coverage | per-chunk box trees, then meshes | 5 |

The terrain viewshed's `Visibility` (visible, hidden, unknown) and `ViewshedCapRefused` (the cap,
its limit and the measured value) serve the floor wash too. A sight line through one building
and one through the streamed world are both reduced by the interior crate's one evaluation,
`evaluate_los` over a `SightLineScene`: an opaque surface stops the ray, glass and foliage add
concealment. The world occluder traces each expanded prefab with the interior crate's instance
trace, and its `blocked_fn` has the shape of blocking test the floor wash takes. Both rasters
come in two forms, one call (`compute_viewshed`, `wash_band`) or a job that does the same work in
slices under a time budget and produces the identical raster.

## Boundaries

- Depends on: the terrain crate `terrain_elevation`, the world format crates (`world_chunks`,
  `prefab_catalog`, `world_file_formats`), the world object crate `building_interiors`, the
  geometry crates (`spatial_indexes`, `geometry_primitives`, `map_coordinates`), and external
  crates (`serde`, `rkyv`, `bytemuck`, `thiserror`).
- Used by: `map_editing_tools`, whose line-of-sight tool and visibility scheduler import them
  directly; `map_asset_loading`'s occluder loader and `map_streaming_host`'s queries; the Mission
  Creator's line-of-sight tool, the mortar
  map picker and the debug benches in `apps/frontend/`; the developer tools' world check and
  blueprint tooling.
- Rules: a line of sight crate declares `category = "crates/line_of_sight"`, depends only on
  lower engine categories and on lower line of sight crates, and holds no browser or GPU code
  (`cargo xtask verify crate-tiers`).
