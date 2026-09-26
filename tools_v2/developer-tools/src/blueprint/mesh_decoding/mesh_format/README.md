# XOB model parsers

The byte-level halves of `mesh_format.rs` in `tools_v2/developer-tools/src/blueprint/mesh_decoding/`:
the reader of an [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) `.xob` model's visual LODs and
the reader of its fire-collision (COLL) chunk, both returning the parent's `XobMesh`.

## Contents

```text
tools_v2/developer-tools/src/blueprint/mesh_decoding/mesh_format/
├── parse_coll.rs  `parse_coll`: every COLL collider record as one triangle soup in the model frame
└── u16le.rs       byte readers, chunk lookup, LZ4 blocks, `parse_xob`, `aabb` and `has_coll`
```

## How it works

A model is an IFF `FORM … XOB9` container. `parse_xob` reads the `HEAD` chunk's material strings
and 116-byte `LZO4` descriptors, one per LOD tier and submesh, decompresses the `LODS` chunk with
`lz4_decompress_chained` (a stream of LZ4 blocks whose matches reach back across block boundaries,
so they decode into one buffer), and cuts each descriptor's region, stored in reverse descriptor
order, into indices, positions and packed normals. It loads every submesh of one quality tier:
the one `--lod` names, or else the tier with the most triangles, which is the full-detail LOD on
the game's models. It returns no UVs, tangents or skinning.

`parse_coll` walks the collider records of the `COLL` chunk. Each carries a shape type, its layer
preset and collider mesh names (as indices into the `HEAD` name table), a rotation and a centre.
A box (type 3) becomes twelve triangles; a convex collider (type 4) keeps only its vertices and
gets its faces from `hull_triangles`; a trimesh (types 5 and 6) keeps its vertices and indices, and
type 6 adds the subrange table that names a `.gamemat` for each run of triangles. Every record
is transformed into the model frame and recorded in `records` with its triangle range, and
`tri_submesh` carries the record index so a caller can isolate one collider. Collision meshes
have no normal stream, so `vert_normals` is all zero. A framing byte other than `0xFF` or a
truncated record fails the parse.

## Boundaries

- Depends on: the parent's `XobMesh`, `CollRecord` and `LodDescriptor`; the convex hull builder in
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/convex_hulls.rs`.
- Used by: `mesh_format.rs`, which re-exports `parse_xob`, `parse_coll`, `aabb` and `has_coll`;
  through it the sidecar commands and the prefab library in
  `tools_v2/developer-tools/src/blueprint/bvh/` and
  `tools_v2/developer-tools/src/blueprint/archive_emission/`, `xob-inspect`, and
  `voxels-from-mesh`.
- Rules: extracted game files are never committed, so the tests build synthetic models
  (`tiny_xob_parses_to_quad`, `coll_box_record_becomes_twelve_triangles`,
  `coll_trimesh_record_parses_and_transforms`, `lz4_match_reaches_across_block_boundary` in
  `tools_v2/developer-tools/src/blueprint/tests/xob_tests.rs`); a wrong magic is refused
  (`wrong_magic_is_rejected`).
