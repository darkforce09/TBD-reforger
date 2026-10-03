# Terrain relief source

The source of `terrain_relief`: the three relief computations and the crate root that declares
them.

## Contents

```text
crates/terrain/terrain_relief/src/
├── contours.rs   contour levels, marched rings and segments, and the pick of each summit's ring
├── hillshade.rs  the hillshade image of the metres cache, at most 1024 pixels a side
├── lib.rs        the crate root: module header and `mod` lines
├── prelude.rs    the names most readers import
├── sea_band.rs   the sea band fills at four heights around the waterline, and their opacity by zoom
└── tests/        unit tests for the contours, the hillshade and the sea band
```

## How it works

Each module is a pure function of its input: `contours` and `sea_band` march the vector grid of
`terrain_elevation`, `hillshade` shades the metres cache. None keeps state between calls; the map
engine's relief host decides when to rebuild.

## Boundaries

- Depends on: `terrain_elevation` and `map_coordinates`.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: no module here touches the GPU or a lane; the host that does stays in the map engine.
