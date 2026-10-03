# World line of sight source

The source of `world_line_of_sight`: the occluder and its residency, the chunk pieces, the chunk
walk, the verdict, the read-outs, the occluder library, the error and the crate root.

## Contents

```text
crates/line_of_sight/world_line_of_sight/src/
├── chunk_cells.rs        `cells_on_segment`, the chunk cells a segment crosses in order
├── chunk_occluder.rs     `WorldInstance` rows in the engine frame and `ChunkOccluder`, a chunk's boxes and tree
├── error.rs              `Error` and `Result`: the archive read and descriptor projection errors
├── instance_box_tree.rs  `AabbTlas`, the box tree over one chunk's row boxes, and its brute-force scan
├── lib.rs                the crate root: module header, `mod` lines and the public surface
├── occluder_library/     the prefab descriptor, BLAS manifest and building archive model
├── occluder_readouts.rs  `wanted` (the fetch plan) and the occluder's read-outs
├── prelude.rs            the names most readers import
├── query_types.rs        `map_to_engine`, `BlockPolicy`, `WorldEvent`, `Coverage`, `WorldLos`, `WorldVerdict`
├── raycast.rs            `trace` and `blocked`: the chunk walk, the box-tree candidates, the exact traces
├── residency.rs          what arrives and leaves: catalogue, chunks, descriptors, BLAS files, byte cap
├── sight_line.rs         `evaluate_los`: the world as a sight-line scene, its verdict and coverage
├── tests/                unit tests of the occluder and the chunk walk
└── world_occluder.rs     `WorldOccluder`, the resident state, and `PrefabOccluder`
```

## How it works

`world_occluder.rs` holds the state that `residency.rs` fills; `raycast.rs` walks it chunk by
chunk through `chunk_cells.rs`, `chunk_occluder.rs` and `instance_box_tree.rs`; `sight_line.rs`
hands the crossings to the interior line of sight's shared evaluation and decides the verdict;
`occluder_readouts.rs` answers the host. `occluder_library/` is the file model the host loads
from.

## Boundaries

- Depends on: the crates the package README lists.
- Used by: the crate's callers through `lib.rs` and `prelude.rs`.
- Rules: the crate root re-exports the public surface; the residency calls are the only writers
  of the occluder's state.
