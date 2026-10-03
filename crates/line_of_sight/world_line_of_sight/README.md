# World line of sight

The `world_line_of_sight` crate: line of sight through every object placed on a terrain, over the
chunks a host has made resident. It keeps each resident chunk's placed rows under a box tree,
expands a prefab into its exact collision meshes once its descriptor and BLAS files arrive, stands
a proxy box in until then, and says which verdicts rest on geometry that is not loaded yet. It
holds no browser code: the map engine's occluder loader fetches the files and feeds them in.

## Contents

```text
crates/line_of_sight/world_line_of_sight/
├── Cargo.toml  the package: `world_chunks`, `interior_line_of_sight`, `prefab_catalog`, layout tier 5
└── src/        the occluder, its residency, chunk walk, verdict and read-outs, the occluder library
```

## How it works

```text
host (the map engine's occluder loader)         WorldOccluder              queries
  set_prefabs(catalogue rows) ──────────► proxy boxes, labels     trace(obs, tgt)
  insert_chunk(id, chunk) ──────────────► ChunkOccluder:            → crossings + Coverage
                                            rows, world boxes, tree  blocked(obs, tgt, policy)
  insert_descriptor(descriptor) ────────► blocks = false: never      → bool
                                            crosses; else waits     evaluate_los(obs, tgt)
  insert_blas(path, sidecar) ───────────► expands every descriptor   → WorldLos
                                            whose BLAS files are all
                                            in; enforces the byte cap
  refresh() ────────────────────────────► rebuilds the boxes and trees of changed chunks
  wanted(chunk ids, limit) ◄────────────── descriptors, then BLAS files, most-placed first
```

Positions are metres in the engine frame `[x, y_up, z_north]`: `map_to_engine` turns a map point
`(x, y_north, elevation)` into it, and `rows_of_chunk` turns each streamed row into a
`WorldInstance` with its angles as `[pitch, yaw, roll]` degrees and a uniform scale. Each row's
world box comes from the best bounds known for its prefab: the exact union of its expanded meshes,
else its descriptor's bounds, else the catalogue proxy (the median half extents, standing on its
pivot); a prefab that never blocks, or one with no bounds known, gets no box.

A query walks the chunk cells under the segment (`cells_on_segment`) and records the ones not
resident in `Coverage`. In each resident chunk the box tree yields the rows the segment crosses
(the brute-force scan while the chunk awaits `refresh`). An expanded prefab is traced exactly in
the row's frame through the interior line of sight's instance trace; any other crosses as one
opaque proxy crossing. `evaluate_los` names every crossing `pid:chunk:row[/inner id]` and reduces
them through the interior line of sight's shared evaluation: the verdict is `Blocked` for an exact
opaque blocker, `Provisional` for a proxy blocker or, with no blocker, for a chunk on the segment
that is not resident, and `Clear` otherwise. `blocked` is the faster yes-or-no under a
`BlockPolicy`, judged on what is loaded. Over `DEFAULT_BLAS_CAP_BYTES` (48 MiB) `insert_blas`
evicts sidecars no placed, expanded prefab needs and un-expands the prefabs that used them.

## Getting started

Run from the repository root:

```bash
cargo test -p world_line_of_sight   # box trees, chunk walk, verdicts, budget, occluder library
```

## Public surface

The crate root is the public surface:

- `WorldOccluder` with its residency calls, `wanted`, the three queries and the read-outs
  (`kind_of`, `label_of`, `descriptor_of`, `memory_bytes`, `resident_chunk_ids`, `chunk_rows`,
  `chunk_boxes`, `expanded_of`, `proxy_rows`, `root_kind_of`), `PrefabOccluder`, `Wanted`,
  `DEFAULT_BLAS_CAP_BYTES`;
- the query types `WorldLos`, `WorldVerdict`, `WorldEvent`, `Fidelity`, `Coverage`,
  `BlockPolicy` and `map_to_engine`;
- the chunk pieces `ChunkOccluder`, `WorldInstance`, `rows_of_chunk`, `NO_BOX`, `AabbTlas`,
  `Candidate` and `cells_on_segment`;
- the occluder library (`occluder_library`): `PrefabDescriptor`, `BlasManifest` and its rows,
  `BuildingArchiveBytes`, `ArchiveBoot`, `ArchiveProjectionError` and the schema versions;
- `Error` and `Result` (`error`), and `prelude`.

## Boundaries

- Depends on: `world_chunks` (`WorldChunk`, `ChunkId`), `interior_line_of_sight` (the instance
  trace, `Owner`, `TraceEvent` and the shared sight-line evaluation), `building_interiors` (the
  compound instances and `LosHit`), `prefab_catalog` (`PrefabRow`), `spatial_indexes` (sidecars,
  surface kinds, the flat box-tree core), `world_file_formats` (the building archive, `PrefabId`,
  `TerrainId`), `map_coordinates`, `geometry_primitives`, `serde`, `rkyv`, `bytemuck` and
  `thiserror`.
- Used by:
  - the map engine (`legacy/map_engine`): the occluder loader and host queries in
    `legacy/map_engine/src/streaming/` and the object wash of its line-of-sight tool;
  - the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s line-of-sight
    tool and the debug world line-of-sight bench in `apps/frontend/`;
  - the world line-of-sight check and the blueprint tooling in `tools/developer_tools/src/`.
- Rules: the box tree returns exactly what the brute-force scan returns
  (`tlas_matches_brute_force_including_the_observer_inside_case`) and `cells_on_segment` matches
  its brute-force rasteriser (`dda_matches_the_brute_force_rasteriser`); a missing chunk on the
  segment makes the verdict `Provisional` and is named
  (`a_missing_chunk_on_the_segment_is_provisional_and_named`); `blocked` agrees with the verdict
  (`blocked_agrees_with_the_verdict_on_random_segments`); the byte cap evicts only sidecars
  nothing resident needs (`the_blas_cap_evicts_only_sidecars_nothing_resident_needs`); line of
  sight tier 5 (`cargo xtask verify crate-tiers`).

## Related documentation

- [Line of sight crates](/crates/line_of_sight/README.md) — the three layers and how they share
  their vocabulary.
- [Prefab descriptor schema](/contracts/definitions/prefab-descriptor.schema.json) and
  [BLAS manifest schema](/contracts/definitions/blas-manifest.schema.json) — the library files
  the occluder loads.
