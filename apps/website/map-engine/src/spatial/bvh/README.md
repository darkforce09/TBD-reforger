# Triangle mesh bounding volume hierarchy

The bounding volume hierarchy (BVH) that every sight line through a building or a placed object is
tested against: a flat binary tree over a triangle mesh, the segment queries over it, the surface
kind of each triangle, and the `.bvh` sidecar file that carries a mesh and its tree from the
offline tools to the browser.

## Contents

```text
apps/website/map-engine/src/spatial/bvh/
├── mod.rs        the module tree
├── node.rs       segment-triangle intersection, the 32-byte flat node, the midpoint-split builder
├── sidecar.rs    the `.bvh` sidecar: validating parse, deterministic emit, the f32 vertex cast
├── surface.rs    `SurfaceKind`: opaque, glass or foliage, and which of them stops a sight line
├── tests/        unit tests for the intersection, the tree, the queries and the sidecar format
├── traversal.rs  `Bvh`: its build and the any-hit, closest-hit and all-hits segment queries
└── tree/         the BVH items under one path, and the mount of the unit tests
```

## How it works

`Bvh::build` sorts a mesh's triangles into 32-byte nodes: bounds in f32 padded by `AABB_PAD`
(1 mm), an internal node's two children adjacent at `left_first` and `left_first + 1`, a leaf
covering `count` entries of `tri_order`. It splits at the midpoint of the longest centroid axis,
falls back to a median split when one side comes out empty, and makes a leaf at `LEAF_MAX` (8)
triangles, at `MAX_DEPTH` (32), or when every centroid coincides.

A query walks the tree with a fixed 64-slot stack along a segment `p→q`, restricted to
`t ∈ [t_lo, t_hi]` (0 at `p`, 1 at `q`), and both faces of a triangle count (`segment_hits_tri`,
Möller–Trumbore). `any_hit` stops at the first accepted crossing, which is enough for occlusion;
`first_hit` returns the nearest; `all_hits` lists every crossing sorted by `t`. The `_where` forms
skip the triangles whose `SurfaceKind` a predicate rejects, so
`any_hit_where(.., SurfaceKind::is_terminal)` asks whether anything opaque stands between: only
`Opaque` stops a sight line, while `Glass` and `Foliage` are crossed and conceal.

A sidecar is little-endian: a 32-byte header (magic `SIDECAR_MAGIC`, `TBVH`; the version; the
vertex, triangle and node counts; the flags; 8 zero bytes), then f32 vertices, u32 triangles, the
nodes, `tri_order`, and, when `FLAG_KINDS` is set, one kind byte per triangle padded to a multiple
of 4. `emit_bytes` writes `SIDECAR_VERSION` (2); `BvhSidecar::parse` also reads version 1, whose
triangles are all opaque. The parse checks the lengths, finite values and index bounds, that every
node is reachable from the root exactly once, that the leaves tile `tri_order`, a depth of at most
`MAX_PARSE_DEPTH` (60) and every kind code, so no query over a parsed sidecar can panic or loop.
`emit_bytes` takes a tree built over `lift_verts(quantize_verts(verts))`, the f32 vertices exactly
as the file stores them, which makes the output deterministic: two builds emit the same bytes.

## Public surface

- `traversal::{Bvh, Hit}`: the tree and its queries.
- `sidecar::{BvhSidecar, BvhParseError, emit_bytes, quantize_verts, lift_verts}` and the format
  constants `SIDECAR_MAGIC`, `SIDECAR_VERSION`, `SIDECAR_VERSION_MIN` and `FLAG_KINDS`.
- `surface::SurfaceKind`, and the vector helpers `node::{segment_hits_tri, sub, cross, dot}`.

## Boundaries

- Depends on: nothing outside the folder; the standard library only.
- Used by:
  - `crate::spatial::los`: the interior walker and wash, and the world occluder;
  - `crate::world::architecture`: blueprint hit attribution, compound assembly and instances, and
    the section cutter and index;
  - the occluder loader (`apps/website/map-engine/src/streaming/loaders/occluder_loader.rs`),
    which parses each fetched `.bvh`;
  - the debug building viewer and building interior benches
    (`apps/website/frontend/src/v2/apps/debug/building_viewer.rs`,
    `apps/website/frontend/src/v2/apps/debug/building_interior.rs`) and the
    [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s line-of-sight tool
    (`apps/website/frontend/src/v2/apps/editor/input/tools/los_world_wasm.rs`);
  - the blueprint tooling in `tools_v2/developer-tools/src/blueprint/`, whose emitters in
    `tools_v2/developer-tools/src/blueprint/bvh/` write the sidecars, and the library checks in
    `tools_v2/developer-tools/src/map_verification/`.
- Rules:
  - the queries agree with brute force (`bvh_matches_brute_force_on_box_grid`,
    `first_hit_matches_min_t_brute_force_on_box_grid`), and `first_hit` finds nothing exactly when
    `any_hit` finds nothing (`first_hit_none_iff_any_hit_none`), all in `tests/tree.rs`;
  - a sidecar round-trips, emits byte-identically twice and has the size the format gives
    (`sidecar_round_trip_box_grid`, `double_emit_is_byte_identical`,
    `emitted_size_matches_formula`); a version-1 file reads as all opaque
    (`v1_sidecar_parses_as_all_opaque_and_upgrades`); malformed and hostile bytes are refused
    (`parse_rejects_malformed_bytes`, `parse_rejects_structural_attacks`);
  - the node is 32 bytes, the sidecar's node stride, which a compile-time assert in
    `traversal.rs` holds; a new surface kind needs a new wire code, since the parse refuses codes
    above `SurfaceKind::MAX_CODE`.
