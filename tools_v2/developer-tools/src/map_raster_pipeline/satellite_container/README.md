# Satellite Archive Container (`map_raster_pipeline/satellite_container`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Generates and validates the multi-resolution `.tbd-sat` binary container format.

Replaces cryptic legacy files `tbds_v2.rs` (475 LOC) and `unified.rs` (706 LOC).

---

## Responsibilities

- **`builder.rs`** (<450 LOC): Generates multi-level mip chains down to 1×1 and encodes tiles as VP8L WebP.
- **`archive_writer.rs`** (<400 LOC): Packs the 32-byte header, rkyv index table, and raw tile payloads into `.tbd-sat`.
- **`verifier.rs`** (<300 LOC): Validates container integrity and tile offset alignment.
