# Game model decoding

The blueprint compiler's reader of [Enfusion](/documentation/glossary/a_to_f.md#enfusion) `.xob`
models: their triangles, their fire-collision (COLL) colliders with each triangle's game material,
and their node table of sockets, plus the two inspection commands that print what the reader sees
and peek into the game paks.

## Contents

```text
tools/map_assets/blueprint_compiler/src/mesh_decoding/
├── archive_inspection.rs  the `xob-inspect` and `pak-cat` commands
├── mesh_format/           the byte-level visual LOD and COLL parsers
├── mesh_format.rs         `XobMesh`, `CollRecord` and `LodDescriptor`; re-exports the parsers
├── node_records.rs        `parse_head_nodes`: the HEAD name table, node records and socket transforms
└── tests/                 the unit tests of the mesh and node table decoders
```

## How it works

The files are modules of `mesh_decoding`, declared in `tools/map_assets/blueprint_compiler/src/mesh_decoding.rs`; the unit tests
are in `tests/`, one file per module.

`mesh_format.rs` holds the mesh a model decodes to: vertices, packed normals, triangles, each
triangle's submesh or collider record, and for a COLL mesh each triangle's material index in the
`HEAD` name space and the collider records with their triangle ranges. Its `mesh_format/` parsers
fill it from the visual LODs (`parse_xob`) or from the COLL chunk (`parse_coll`).

`node_records.rs` reads the `HEAD` payload's string table (material names and `.emat` paths,
`Scene_Root`, the socket names, collider mesh names, `.gamemat` paths, layer preset names) and the
36-byte node records before the first `LZO4` descriptor: a name index, a parent-relative position
and quaternion, and sibling and child links. It composes each `socket_*` node's chain into a
`Rigid` transform from the model root, where a child prefab attaches. The COLL chunk's layer and
material indices resolve through the same name table.

`archive_inspection.rs` is the reverse-engineering instrument. `xob-inspect` takes a file or an
in-pak path, opens the paks and the loose extract through the batch's source stack, and prints
the chunk ids, the node records and sockets, each COLL record's layer preset and material runs,
and the resulting surface-kind histogram; `--strings` adds the name table, `--find <substr>`
lists matching in-pak paths, `--save <file>` keeps a raw copy, and `--kind <record>=<kind>`
previews an override. `pak-cat` prints one pak entry of any type to stdout, the first
`--head <bytes>` of it, or writes it whole to `--out <file>`.

## Public surface

- `developer_tools::blueprint::run_xob_inspect` and `run_pak_cat`, re-exported by the blueprint
  root and run by `cargo xtask map xob-inspect` and `cargo xtask map pak-cat`.
- The decoders stay inside the blueprint compiler.

## Boundaries

- Depends on: the pak reader in `tools/enfusion/enfusion_pak/src/` (`PakSet`,
  `AssetSource`); the source stack and asset decoder of
  `tools/map_assets/blueprint_compiler/src/bvh/batch_processing.rs`; the surface classification and
  convex hulls in `tools/map_assets/blueprint_compiler/src/architectural_analysis/`;
  `geometry_primitives::rigid_transform::Rigid` and
  `spatial_indexes::bounding_volume_hierarchy::surface_kind::SurfaceKind`.
- Used by: the sidecar and prefab commands in `tools/map_assets/blueprint_compiler/src/bvh/`, the
  prefab library in `tools/map_assets/blueprint_compiler/src/archive_emission/`, and
  `voxels-from-mesh` in `tools/map_assets/blueprint_compiler/src/voxel_processing/`; the
  `xob-inspect` and `pak-cat` commands of `cargo xtask map`.
- Rules: extracted game files are scratch material and never committed, `--save` copies
  included; the node table's name base is recovered from `Scene_Root`, never assumed
  (`node_table_decodes_sockets_and_the_name_space_starts_at_the_first_material` in
  `tools/map_assets/blueprint_compiler/src/mesh_decoding/tests/node_records_tests.rs`); the tests pin the decoders
  on synthetic models (`tools/map_assets/blueprint_compiler/src/mesh_decoding/tests/mesh_format_tests.rs`), and the one
  test on the real farmhouse model is `#[ignore]`d because it needs a local game extract.
