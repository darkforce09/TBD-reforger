# Prefab occluder descriptors

The data model of the prefab occluder library the world line of sight reads: one descriptor
per catalogue prefab naming the collision meshes it places, the manifest that indexes the library,
and the projection of both into and out of the binary building archive the browser boots from.

## Contents

```text
crates/line_of_sight/world_line_of_sight/src/occluder_library/
├── archive_projection.rs  conversions between the JSON rows and the archive rows
├── blas_manifest.rs       `BlasManifest`, the library index, with its rows, census and schema versions
├── building_archive.rs    the 8-aligned archive holder, its validated read, the boot split, the projection error
├── mod.rs                 the module tree; re-exports the manifest, descriptor and archive types
├── prefab_descriptor.rs   `PrefabDescriptor`, one catalogue prefab's collision closure
└── tests/                 unit tests of the descriptor, manifest and archive model
```

## How it works

The files sit in a terrain's asset folder, such as `assets/terrains/everon/`, and the browser
fetches them from the same paths under `/map-assets/<terrain>/`. A `PrefabDescriptor` is the JSON
of `prefabs/descriptors/<pid>.json`: the prefab's catalogue id, slug, resource name and kind;
whether anything in it collides (`blocks`, with a `reason` when it does not); whether it is a
tree with foliage triangles (`canopy`); the union of its placed bounds in its object frame; and
its `instances`, every placed mesh with the root first, each naming a BLAS file
`blas/<stem>.bvh` and a local transform. A `BlasManifest` is
`prefabs/blas-manifest.json`: every BLAS file with its size and its triangle counts per surface
kind, sorted by path; every descriptor, sorted by pid; the `hot` prefabs (the most-placed blocking
ones, most first); and the census totals. Its two lookups, `descriptor(pid)` and `blas(path)`,
are binary searches over those sorted lists.

The archive `prefabs/building_blueprints.rkyv` carries every descriptor as a row of its id, slug,
kind, the two flags, its bounds in f32 and its BLAS paths as indexes into one shared library table:

```text
PrefabDescriptor ─ to_archived ─► archive row             (refuses rather than approximates)
archive bytes ─ BuildingArchiveBytes::new ─ archive() ─► validated archive, borrowed in place
validated archive ─ ArchiveBoot::from_archive ─┬► census       rows with blocks = false, rebuilt
                                               ├► blocking     count of rows still read as JSON
                                               ├► blas_by_pid  every usable row's BLAS paths
                                               └► unusable     count of rows dropped
```

A descriptor read back from a row (`from_archived`, equal to `archive_census` of the original)
keeps the id, slug, kind, flags and f32 bounds and clears everything else; the row's BLAS paths
come back separately through `archived_blas_paths`.

## Boundaries

- Depends on: `world_file_formats` (the archive structs, `access_checked`,
  `ARCHIVE_SCHEMA_VERSION`, `PrefabId`, `TerrainId`), `building_interiors` (`InstanceRecord`),
  `geometry_primitives` (`Bounds3`), `serde`, `rkyv`, `bytemuck` and `thiserror`.
- Used by: the rest of the crate, which re-exports the types from its root and expands
  descriptors into traceable occluders; the occluder loader
  (`legacy/map_engine/src/streaming/loaders/occluder_loader.rs`), which boots from the
  archive and fetches the manifest and the blocking descriptors; the blueprint tooling in
  `tools/map_assets/blueprint_compiler/src/archive_emission/`, which writes the descriptors, the
  manifest and the archive; and the library checks in
  `tools/map_assets/map_asset_verification/src/`.
- Rules: `blocks` is true exactly when `localBounds` is present, and the projection refuses a
  descriptor that breaks this or names a BLAS outside the library
  (`projection_refuses_bounds_that_disagree_with_blocks_and_an_unknown_blas` in
  `tests/occluder_library_tests.rs`); an archive of another schema version is refused, never read
  (`a_future_archive_schema_is_refused_rather_than_read`); a row whose BLAS list does not fully
  resolve is dropped whole, never placed with a part missing (`archived_blas_paths`); the JSON
  shapes follow the two schemas below, which
  `tools/map_assets/map_asset_verification/src/blas_manifest.rs` checks the committed library
  against.

## Related documentation

- [Prefab descriptor schema](/contracts/definitions/prefab-descriptor.schema.json) — the shape
  of `prefabs/descriptors/<pid>.json`.
- [BLAS manifest schema](/contracts/definitions/blas-manifest.schema.json) — the shape of
  `prefabs/blas-manifest.json`.
