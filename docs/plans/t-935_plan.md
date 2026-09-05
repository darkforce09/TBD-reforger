# T-935 — Plan (program)

## Context
Operator spec 2026-09-04: hybrid rkyv (structured metadata) + raw `#[repr(C)]` POD (bulk GPU data),
flatbuffers rejected. audit.md Finding 1.4 names the main-thread gzip-JSON chunk ingest
(residency.rs:716-737, chunk.rs:46-97) as the boot bottleneck. Today rkyv is used nowhere and
bytemuck only in map-engine-render; flate2 + serde_json are `world` deps of map-engine-core.

## Approach
1. T-935.1 defines every format once in `crates/map-engine-core/src/world/binary/` and adds
   optional manifest blocks (serde default) so loaders can be keyed on the manifest.
2. Wave 2 (independent emitters + dormant loaders): .2 chunks, .4 DEM, .5 density, .7 labels,
   .8 buildings, .10 satellite v2. Wave 3: .3 chunk ingest, .6 roads. Wave 4: .9 water, .11 catalog.
3. T-935.12 schemas, golden `.bin`, gates, LFS rules. T-935.13 regenerates everon, flips the
   manifest, deletes gz-JSON emit + flate2, records before/after sizes and timings.

## Risks
- POD alignment on fetched buffers: every cast copies to an aligned Vec on failure, never unwraps.
- 32 B POD vs the operator's 24 B sketch: pitch/roll/scale are on the wire since T-090.12.1; the
  spec records the correction. Fallback: none needed, the JSON decode is the parity oracle.
- LFS bloat: binaries are committed only in .12/.13 after the `.gitattributes` rules land.

## Verification
- `cargo test -p map-engine-core --all-features` on every slice
- `cargo xtask platform wave gate --slice T-935.N`
- `cargo xtask ci verify-terrain-strict` (.12, .13); world-los cell 18_0 probe (.3, .8, .13)
