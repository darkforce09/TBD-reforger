# Game model decoding

The blueprint compiler's reader of [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) `.xob`
models: their triangles, their fire-collision (COLL) colliders with each triangle's game material,
and their node table of sockets, plus the two inspection commands that print what the reader sees
and peek into the game paks.

## Contents

```text
tools_v2/developer-tools/src/blueprint/mesh_decoding/
├── archive_inspection.rs  the `xob-inspect` and `pak-cat` commands
├── mesh_format/           the byte-level visual LOD and COLL parsers
├── mesh_format.rs         `XobMesh`, `CollRecord` and `LodDescriptor`; re-exports the parsers
└── node_records.rs        `parse_head_nodes`: the HEAD name table, node records and socket transforms
```

## How it works

The files are modules of the blueprint root, declared in
`tools_v2/developer-tools/src/blueprint/mod.rs` by `#[path]` as `inspect`, `xob` and `xob_nodes`.

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

- Depends on: the pak reader in `tools_v2/developer-tools/src/enfusion_pak/` (`PakSet`,
  `AssetSource`); the source stack and asset decoder of
  `tools_v2/developer-tools/src/blueprint/bvh/batch_processing.rs`; the surface classification and
  convex hulls in `tools_v2/developer-tools/src/blueprint/architectural_analysis/`;
  `website_map_engine::world::architecture::compound::transform::Rigid` and
  `website_map_engine::spatial::bvh::surface::SurfaceKind`.
- Used by: the sidecar and prefab commands in `tools_v2/developer-tools/src/blueprint/bvh/`, the
  prefab library in `tools_v2/developer-tools/src/blueprint/archive_emission/`, and
  `voxels-from-mesh` in `tools_v2/developer-tools/src/blueprint/voxel_processing/`; the
  `xob-inspect` and `pak-cat` commands of `cargo xtask map`.
- Rules: extracted game files are scratch material and never committed, `--save` copies
  included; the node table's name base is recovered from `Scene_Root`, never assumed
  (`node_table_decodes_sockets_and_the_name_space_starts_at_the_first_material` in
  `tools_v2/developer-tools/src/blueprint/tests/xob_nodes/tests.rs`); the tests pin the decoders
  on synthetic models (`tools_v2/developer-tools/src/blueprint/tests/xob_tests.rs`), and the one
  test on the real farmhouse model is `#[ignore]`d because it needs a local game extract.
