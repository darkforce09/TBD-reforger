# Prefab library builder

The two submodules of `library_reader.rs` in
`tools_v2/developer-tools/src/blueprint/archive_emission/`: they build the whole-catalogue prefab
occluder library that `cargo xtask map bvh-batch --all-prefabs` writes, and validate and write it.

## Contents

```text
tools_v2/developer-tools/src/blueprint/archive_emission/library_reader/
├── assemble_manifest.rs  the `BlasManifest` with its hot set, schema validation, and `write_library`
└── gunzip_json.rs        catalogue rows, the chunk census, the canopy fallback, and `build_library`
```

## How it works

`load_prefab_rows` reads the terrain's object catalogue (`objects/prefabs.json.gz`) as rows sorted
by prefab id, and `world_census` counts how many chunk rows of `objects/chunks/*.json.gz` place
each prefab. `build_library` walks every selected row (`--only-kind`, `--limit`) through the
batch `Walker` straight out of the game paks and records one `PrefabDescriptor` per prefab: the
root mesh as an instance at identity, plus every collision-bearing child, or `blocks: false` with
a reason (`unresolved`, `no-mesh`, `model-unreadable`, `empty-coll`, `no-fire-geo`, `no-coll`).
Every distinct model becomes one `blas/<stem>.bvh` shared by all the prefabs that place it. A tree
whose COLL chunk carries no foliage triangles gets a canopy from the convex hull of 26 extreme
points of its visual LOD0, written as an all-foliage `blas/<stem>_canopy.bvh`.

`assemble_manifest` indexes every mesh (bytes, triangles, triangles per kind) and every descriptor,
counts the totals, and names the `hot` prefetch set: the most-placed blocking prefabs, 100 by
default (`--hot`). `write_library` validates each descriptor against
`contracts_v2/definitions/prefab-descriptor.schema.json` and the manifest against
`contracts_v2/definitions/blas-manifest.schema.json`, then writes `descriptors/<pid>.json`, the
meshes and `blas-manifest.json` through `write_if_changed`. A filtered run is partial and writes
no manifest, so it never replaces a full one.

## Boundaries

- Depends on: the parent's `PrefabRow`, `LibraryOptions` and `Library`; the batch `Walker`,
  `Asset`, `classify_prefab`, `cover_for_prefab`, `slug_of` and `write_if_changed` in
  `tools_v2/developer-tools/src/blueprint/bvh/batch_processing.rs`; `hull_triangles` in
  `tools_v2/developer-tools/src/blueprint/architectural_analysis/convex_hulls.rs`;
  `website_map_engine::spatial::los::world::descriptor` (`PrefabDescriptor`, `BlasManifest`,
  `BlasEntry`, `DescEntry`, `Totals`) and `website_map_engine::spatial::bvh`; `jsonschema`.
- Used by: `library_reader.rs`, which re-exports `build_library`, `load_prefab_rows`,
  `world_census` and `write_library` (and, to its tests, `validate_against` and `hull_sample`);
  the `--all-prefabs` arm in
  `tools_v2/developer-tools/src/blueprint/archive_emission/archive_command.rs`.
- Rules: the output is deterministic (sorted, timestamp-free, pretty-printed with a trailing
  newline), and a re-run that changes nothing writes nothing
  (`write_is_schema_valid_and_deterministic`); meshes are shared by stem and the hot set is ordered
  by placements (`blas_dedup_by_stem_manifest_entries_and_hot_order`); the committed farmhouse
  descriptor reproduces its instances file
  (`committed_farmhouse_descriptor_reproduces_its_instances_file`); all three are in
  `tools_v2/developer-tools/src/blueprint/tests/library_tests.rs`.

## Related documentation

- [Everon prefab geometry](/assets_v2/terrains/everon/prefabs/README.md) — the library this
  builder writes and how the map engine loads it.
