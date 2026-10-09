# Spatial indexes

The `spatial_indexes` crate: the triangle bounding volume hierarchy (BVH) every sight line through
a building or a placed object is tested against, with its `.bvh` sidecar file format and the one
flat-tree build core every box tree of the workspace is built with, and the two-dimensional point
grid, picks and zoom-level clusters the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)
selects and draws map points with.

## Contents

```text
crates/geometry/spatial_indexes/
├── Cargo.toml  the package: `geometry_primitives`, `thiserror`, the dev-only `test_fixtures` feature, layout tier 1
└── src/        the bounding volume hierarchy, the point indexes, the error, the prelude and the test fixtures
```

## How it works

`build_flat_tree` sorts items given by their box and centroid into 32-byte nodes: bounds in f32
padded by `NODE_BOX_PAD` (1 mm), an internal node's two children adjacent, a leaf covering a run
of the item order. It splits at the midpoint of the longest centroid axis, falls back to a median
split when one side comes out empty, and makes a leaf at the caller's `BuildLimits`: `Bvh::build`
uses `BuildLimits::TRIANGLES` (8 triangles, depth 32), the map engine's world box tree its own (4
boxes, depth 48). A `Bvh` answers segment queries over a mesh (any hit, nearest hit, every hit,
each optionally restricted to the `SurfaceKind`s a predicate accepts); `emit_bytes` and
`BvhSidecar::parse` write and validate the sidecar that carries a mesh and its tree from the
developer tools to the browser. `PointIndex`, the `picking` functions and `ClusterIndex` index map
points in world metres. The modules' READMEs hold the details.

## Getting started

Run from the repository root:

```bash
cargo test -p spatial_indexes   # the build core, the triangle tree and sidecar, the point indexes
```

## Configuration

One feature, `test_fixtures`, off by default: it compiles the `test_fixtures` module (the cuboid
scenes `Scene`, `cube` and `concat`) for the tests of other crates and is enabled only from their
`[dev-dependencies]`. The crate reads no environment variable.

## Public surface

- `bounding_volume_hierarchy`: `flat_tree_build::{build_flat_tree, BuildLimits, BvhNode,
  ItemBounds, FlatTree, NODE_BOX_PAD}`, `triangle_tree::{Bvh, Hit}`, `sidecar::{BvhSidecar,
  emit_bytes, quantize_verts, lift_verts}` with the format constants,
  `sidecar_parse_error::BvhParseError`, `surface_kind::SurfaceKind`,
  `segment_triangle::segment_hits_tri`, `segment_box_window::segment_aabb_window`.
- `point_indexes`: `point_index::PointIndex`, the `picking` functions, `cluster::{ClusterIndex,
  ClusterMarker, deck_zoom_to_super_zoom}`.
- `Error` and `Result` at the crate root; the common names in `prelude`.
- `test_fixtures`, under `cfg(test)` or the `test_fixtures` feature.

## Boundaries

- Depends on: `geometry_primitives::vector3` (`sub`, `cross`, `dot`), `thiserror`.
- Used by: `building_interiors`, `interior_line_of_sight` and `world_line_of_sight`;
  `map_editing_tools` and `mission_editing_session` (picking and the line-of-sight tool),
  `symbology_layers_gpu` (the slot cluster lane), `map_asset_loading` (the occluder loader) and
  `chunk_scheduler` (the world object index); the single-page app's
  Mission Creator input and debug building benches in `crates/frontend/shell/frontend_application/`; and the developer tools,
  whose BVH emitters in `tools/map_assets/blueprint_compiler/src/occlusion_sidecars/` write the sidecars.
- Rules: geometry category, tier 1 (`cargo xtask verify crate-tiers`); no map, GPU or browser
  concept; the sidecar format and the triangle tree's build are deterministic, so the committed
  sidecars stay byte-identical (`double_emit_is_byte_identical`, and the developer tools'
  `farmhouse_bvh_sidecar_parity_is_pinned`).

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
