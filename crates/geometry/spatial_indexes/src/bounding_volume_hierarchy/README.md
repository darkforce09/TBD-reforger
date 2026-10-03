# Bounding volume hierarchy

The flat box trees of the workspace and the tests their walks run: the one build core, the
triangle tree over a mesh with its segment queries, the surface kind of each triangle, and the
`.bvh` sidecar file that carries a mesh and its tree from the offline tools to the browser.

## Contents

```text
crates/geometry/spatial_indexes/src/bounding_volume_hierarchy/
├── flat_tree_build.rs      the one build core: `build_flat_tree`, `BuildLimits`, the 32-byte `BvhNode`
├── mod.rs                  the module tree
├── segment_box_window.rs   the slab test: a segment's window inside an axis-aligned box
├── segment_triangle.rs     the both-sided segment-triangle test
├── sidecar.rs              the `.bvh` sidecar: validating parse, deterministic emit, f32 vertices
├── sidecar_parse_error.rs  `BvhParseError`: every way the sidecar parse refuses bytes
├── surface_kind.rs         `SurfaceKind`: opaque, glass or foliage, and which stops a sight line
├── tests/                  unit tests: the build core, intersection, queries, sidecar format
└── triangle_tree.rs        `Bvh`: its build over a mesh and the any-, closest- and all-hits queries
```

## How it works

`build_flat_tree` sorts the items an order names into 32-byte nodes: bounds in f32 padded by
`NODE_BOX_PAD` (1 mm), an internal node's two children adjacent at `left_first` and
`left_first + 1`, a leaf covering `count` entries of the order. It splits at the midpoint of the
longest centroid axis, falls back to a median split when one side comes out empty, and makes a
leaf at `BuildLimits::leaf_max` items, at `BuildLimits::max_depth`, or when every centroid
coincides. `Bvh::build` runs it over a mesh's triangle boxes with `BuildLimits::TRIANGLES` (8,
32); `world_line_of_sight`'s instance box tree runs it over instance boxes with its own limits
(4, 48).

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
60 and every kind code, so no query over a parsed sidecar can panic or loop. `emit_bytes` takes a
tree built over `lift_verts(quantize_verts(verts))`, the f32 vertices exactly as the file stores
them, which makes the output deterministic: two builds emit the same bytes.

## Public surface

- `flat_tree_build::{build_flat_tree, BuildLimits, BvhNode, ItemBounds, FlatTree, NODE_BOX_PAD}`.
- `triangle_tree::{Bvh, Hit}`: the tree and its queries.
- `sidecar::{BvhSidecar, emit_bytes, quantize_verts, lift_verts}`, the format constants
  `SIDECAR_MAGIC`, `SIDECAR_VERSION`, `SIDECAR_VERSION_MIN` and `FLAG_KINDS`, and
  `sidecar_parse_error::BvhParseError`.
- `surface_kind::SurfaceKind`, `segment_triangle::segment_hits_tri`.
- `segment_box_window::segment_aabb_window`: the parametric window `[t_in, t_out]` of a segment
  inside an axis-aligned box (endpoints inclusive), or `None` when it misses.

## Boundaries

- Depends on: `geometry_primitives::vector3` (`sub`, `cross`, `dot`); the standard library.
- Used by: the line of sight crates (`interior_line_of_sight`'s compound walk and floor wash,
  `world_line_of_sight`'s occluder and its box tree); `building_interiors` (blueprints, compounds
  and sections); the map engine's occluder loader; the debug benches and the line-of-sight tool
  of the single-page app; the blueprint tooling in
  `tools/map_assets/blueprint_compiler/src/`, whose emitters write the sidecars.
- Rules:
  - the queries agree with brute force (`bvh_matches_brute_force_on_box_grid`,
    `first_hit_matches_min_t_brute_force_on_box_grid`), and `first_hit` finds nothing exactly when
    `any_hit` finds nothing (`first_hit_none_iff_any_hit_none`), in
    `tests/triangle_tree_tests.rs`;
  - the build core's leaves tile the order and respect both limits, and the build is deterministic
    (`tests/flat_tree_build_tests.rs`);
  - a sidecar round-trips, emits byte-identically twice and has the size the format gives
    (`sidecar_round_trip_box_grid`, `double_emit_is_byte_identical`,
    `emitted_size_matches_formula`); a version-1 file reads as all opaque
    (`v1_sidecar_parses_as_all_opaque_and_upgrades`); malformed and hostile bytes are refused
    (`parse_rejects_malformed_bytes`, `parse_rejects_structural_attacks`);
  - the node is 32 bytes, the sidecar's node stride, which a compile-time assert in
    `flat_tree_build.rs` holds; a new surface kind needs a new wire code, since the parse refuses
    codes above `SurfaceKind::MAX_CODE`.
