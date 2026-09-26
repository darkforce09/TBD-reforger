# Spatial computation

The map engine's spatial computation: the acceleration structures over triangle meshes and map
points, and the line-of-sight queries built on them, from the viewshed of a hill to the sight line
through one building's window. Everything here is CPU work in world metres with no UI or document
state; the one exception is the viewshed texture upload in `los/terrain/`.

## Contents

```text
apps/website/map-engine/src/spatial/
├── bvh/       the triangle-mesh BVH, its segment queries and the `.bvh` sidecar format
├── indexing/  point grids, selection picks, the streamed world's object index, map clusters
├── los/       line of sight over terrain, through the streamed world and inside buildings
└── mod.rs     the module tree
```

## How it works

`bvh/` is the foundation: the flat tree over a triangle mesh that every sight line through
geometry is finally tested against, and the sidecar file that carries each building's or prop's
collision mesh and its tree from the offline tools. `los/` stacks three layers on it and on the world data:
a radial march over the elevation model, the streamed world's per-chunk box trees over placed
objects, and the meshes of one building. `indexing/` stands apart: two-dimensional grids and
clusters over points, for picking and for drawing, with no ray in them.

Two coordinate frames meet here. Map points are world metres with x east and y north, as the
indexes, the terrain march and the streamed rows use them; three-dimensional traces through
meshes use the engine frame `[x, y_up, z_north]`, into which `los::world::map_to_engine` turns a
map point and its elevation.

## Public surface

- `bvh`: `Bvh` and its queries, `BvhSidecar` with the sidecar's parse and emit, `SurfaceKind`.
- `indexing`: `PointIndex`, the `picking` functions, `WorldSpatialIndex`, `ClusterIndex`.
- `los`: the terrain profile, viewshed and `ViewshedJob`; the building traces and washes; the
  world occluder with its verdicts and descriptor data model.

## Boundaries

- Depends on: `crate::world` (the elevation model, building blueprints and compounds, the
  catalogue's prefab rows and object classes); `crate::streaming` (the chunks the world layer
  reads); `crate::io::archives` (the building archive); and, for the viewshed upload only,
  `crate::frame`, `crate::overlay` and `wgpu`.
- Used by:
  - inside the crate: `crate::editing` (picking, selection, the line-of-sight tool and the
    viewshed scheduler), `crate::overlay` (the cluster layer), `crate::streaming` (the world
    object index and the occluder loader), `crate::world::architecture` (the BVH) and
    `crate::world::terrain::dem::sample` (terrain line-of-sight re-exports);
  - the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s line-of-sight tool and
    input handlers in `apps/website/frontend/src/v2/apps/editor/input/`, and the debug benches in
    `apps/website/frontend/src/v2/apps/debug/`;
  - the blueprint tooling and the map checks in `tools_v2/developer-tools/src/`.
- Rules: the module compiles only with the `world` feature (`apps/website/map-engine/src/lib.rs`),
  since the terrain layer reads `crate::world::terrain::dem`; nothing here depends on Leptos or on
  the Mission Creator's state, and the one file that reaches the browser,
  `los/terrain/overlay.rs`, compiles only for wasm32 with the `render` feature; the crate's tests
  run with `--all-features` (`map_engine_tests_require_all_features` in
  `apps/website/map-engine/src/tests/feature_gate_tripwire.rs`), since most of this module is
  compiled out otherwise.
