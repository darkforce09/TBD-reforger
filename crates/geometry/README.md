# Geometry crates

The engine's lowest arithmetic: plain f64 geometry with no map concept, the map's coordinate facts
and conversions, and the cameras built on them. No crate here touches a GPU or a browser binding,
so every build of the map engine links them, the API's included.

## Contents

```text
crates/geometry/
├── camera_math/          `camera_math`: the deck.gl-parity orthographic camera, the doll's orbit camera, gl-matrix 4x4
├── geometry_primitives/  `geometry_primitives`: 3D vector products, 2D segment tests, rigid transforms, 3D boxes
└── map_coordinates/      `map_coordinates`: terrain centres and bounds, the chunk grid, rounding, grid references
```

## Boundaries

- Depends on: external crates only (`serde`, `thiserror`), and one edge inside the category:
  `camera_math` depends on `map_coordinates` for JavaScript rounding.
- Used by: the map engine (`legacy/map_engine`); through it, the single-page app and the
  developer tools.
- Rules: a geometry crate declares `category = "crates/geometry"`, and its dependency edges point
  to lower tiers only (`cargo xtask verify crate-tiers`).
