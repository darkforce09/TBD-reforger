# Line of sight through the streamed world

The world occluder: line of sight through every object placed on the terrain, over the chunks the
browser has streamed in. It keeps each resident chunk's placed rows under a box tree, expands a
prefab into its exact collision meshes once its descriptor and BLAS files arrive, stands in a proxy
box until then, and says which verdicts rest on geometry that is not loaded yet.

## Contents

```text
apps/website/map-engine/src/spatial/los/world/
├── coverage_1.rs  the query types, `map_to_engine`, and the occluder's fetch plan and read-outs
├── coverage_2.rs  `root_kind_of`, the instance kind of an expanded prefab's root record
├── dda.rs         `cells_on_segment`, the chunk cells a segment crosses in order, and its reference
├── descriptor/    the prefab descriptor, BLAS manifest and building archive data model
├── los.rs         `evaluate_los`, the verdict with named hits, concealment and coverage
├── mod.rs         the module tree; re-exports the query, descriptor, placement and box-tree types
├── placed.rs      `WorldInstance` rows in the engine frame and `ChunkOccluder`, a chunk's boxes
├── raycast.rs     `trace` and `blocked`: the chunk walk, the box-tree candidates, the exact traces
├── residency.rs   what arrives and leaves: catalogue, chunks, descriptors, BLAS files, byte cap
├── state.rs       `WorldOccluder`, the resident state
├── tests/         unit tests for the occluder and the descriptor model
├── tlas.rs        `AabbTlas`, the box tree over one chunk's rows, and its brute-force reference
└── trace/         a second path to the occluder and its query types
```

## How it works

```text
occluder loader (crate::streaming)            WorldOccluder              queries
  set_prefabs(catalogue rows) ──────────► proxy boxes, labels     trace(obs, tgt)
  insert_chunk(id, chunk) ──────────────► ChunkOccluder:            → crossings + Coverage
                                            rows, world boxes, TLAS  blocked(obs, tgt, policy)
  insert_descriptor(descriptor) ────────► blocks = false: never      → bool
                                            crosses; else waits     evaluate_los(obs, tgt)
  insert_blas(path, sidecar) ───────────► expands every descriptor   → WorldLos
                                            whose BLAS files are all
                                            in; enforces the byte cap
  refresh() ────────────────────────────► rebuilds the boxes and TLAS of changed chunks
  wanted(chunk ids, limit) ◄────────────── descriptors, then BLAS files, most-placed first
```

Positions are metres in the engine frame `[x, y_up, z_north]`: `map_to_engine` turns a map point
`(x, y_north, elevation)` into it, and `rows_of_chunk` turns each streamed row into a
`WorldInstance` with its angles as `[pitch, yaw, roll]` degrees and a uniform scale, placed by
`Rigid::from_enfusion`. Each row's world box comes from the best bounds known for its prefab: the
exact union of its expanded meshes, else its descriptor's bounds, else the catalogue proxy (the
median half extents, standing on its pivot); a prefab that never blocks, or one with no bounds
known at all, gets no box and never crosses anything.

A query walks the chunk cells under the segment (`cells_on_segment`, cells of the loader's chunk
size, 512 m in the browser) and records the ones not resident in `Coverage`. In each resident
chunk the TLAS yields the rows whose boxes the segment crosses (the brute-force scan while the
chunk is marked for `refresh`). An expanded prefab is traced exactly in the row's local frame
through the interior walker; any other crosses as one opaque proxy crossing, and `Coverage` lists
it with the BLAS files it waits for. `evaluate_los` names every crossing
`pid:chunk:row[/inner id]` and reduces them as a building's interior is reduced (opaque stops,
glass and foliage conceal): the verdict is `Blocked` for an exact opaque blocker, `Provisional`
for a proxy blocker or, with no blocker, for a chunk on the segment that is not resident, and
`Clear` otherwise. `blocked` is the faster yes-or-no under a `BlockPolicy` (`BlockPolicy::VISION`:
opaque only, and a proxy counts), judged on what is loaded.

The BLAS byte cap starts at `DEFAULT_BLAS_CAP_BYTES` (48 MiB). Over it, `insert_blas` evicts
sidecars that no placed, expanded prefab needs and un-expands the prefabs that used them, until the
total fits or nothing more may go.

## Public surface

- `state::WorldOccluder`, also re-exported here: its residency calls, `wanted`, the three queries
  and the read-outs (`kind_of`, `label_of`, `descriptor_of`, `memory_bytes`,
  `resident_chunk_ids`, `chunk_rows`, `chunk_boxes`, `expanded_of`, `proxy_rows`,
  `root_kind_of`), driven by the occluder loader and host queries in `crate::streaming` and read by
  the browser tools and the developer tools below.
- `coverage_1::{WorldLos, WorldVerdict, BlockPolicy}` and `map_to_engine`, also re-exported here,
  for every caller of the queries.
- `descriptor`: the descriptor, manifest and archive types, for the occluder loader and the
  blueprint tooling.

## Boundaries

- Depends on: `crate::spatial::bvh` (sidecars, surface kinds), `crate::spatial::los::interior`
  (the walker's trace, blocking test, crossing reduction and concealment fold, and
  `segment_aabb_window`), `crate::world::architecture` (instances, rigid transforms, the `LosHit`
  types), `crate::world::environment::buildings::prefab` (the catalogue's `PrefabRow`),
  `crate::streaming` (`WorldChunk`, `chunk_id`, `TerrainSizeM`) and `crate::io::archives`.
- Used by:
  - `crate::streaming`: the occluder loader
    (`apps/website/map-engine/src/streaming/loaders/occluder_loader.rs`), the world loader's
    viewport and the host queries that lend the occluder out;
  - `crate::editing::tools::line_of_sight`, whose object wash places points with `map_to_engine`;
  - the [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s line-of-sight tool
    (`apps/website/frontend/src/v2/apps/editor/input/tools/los_world_wasm.rs`) and the debug world
    line-of-sight bench (`apps/website/frontend/src/v2/apps/debug/world_los/`);
  - the world line-of-sight check
    (`tools_v2/developer-tools/src/map_verification/world_line_of_sight.rs`) and the blueprint
    tooling in `tools_v2/developer-tools/src/blueprint/archive_emission/`.
- Rules:
  - the TLAS returns exactly the boxes the brute-force scan returns, the observer-inside case
    included (`tlas_matches_brute_force_including_the_observer_inside_case` in
    `tests/occluder.rs`), and `cells_on_segment` matches its brute-force rasteriser
    (`dda_matches_the_brute_force_rasteriser`);
  - a missing chunk on the segment makes the verdict `Provisional` and is named
    (`a_missing_chunk_on_the_segment_is_provisional_and_named`), and `blocked` agrees with the
    verdict (`blocked_agrees_with_the_verdict_on_random_segments`);
  - a `blocks: false` descriptor and an unknown pid never block
    (`a_blocks_false_descriptor_and_an_unknown_pid_never_block`), and glass and foliage follow the
    building interior's semantics
    (`glass_and_foliage_follow_the_compound_semantics_and_the_policy`);
  - the byte cap evicts only sidecars nothing resident needs
    (`the_blas_cap_evicts_only_sidecars_nothing_resident_needs`);
  - the folder compiles only with the `streaming` feature.

## Related documentation

- [Prefab descriptor schema](/contracts_v2/definitions/prefab-descriptor.schema.json) and
  [BLAS manifest schema](/contracts_v2/definitions/blas-manifest.schema.json) — the library files
  the occluder loads.
