# 512m Chunk Spatial Partitioner (`world_export_pipeline/chunk_partitioner`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Buckets macro-world entities into 512m spatial chunk containers (`TBDC`).

Decomposed from the 1,407-line legacy `build.rs` file into modules strictly under **480 LOC**.

---

## Responsibilities

- **`partitioner.rs`** (<480 LOC): Maps floating-point entity coordinates into `cx_cy` grid buckets.
- **`binary_writer.rs`** (<400 LOC): Packs entity POD rows into binary chunk files with aligned headers.
- **`roads_builder.rs`** (<400 LOC): Traverses road vectors and partitions road segments across intersecting chunk boundaries.
