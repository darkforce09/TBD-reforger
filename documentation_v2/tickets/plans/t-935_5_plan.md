# T-935.5 — Plan

## Context
`crates/map-engine-core/src/geometry/tbdd.rs:51` decodes forest density tiles byte by byte;
`tools/tbd-tools/src/density.rs:24` emits them. 625 everon tiles are committed and must stay
valid, so the on-disk layout does not change — only the decoder does.

## Approach
1. Copy the current loop verbatim into `mod parity_reference` under `cfg(test)` first.
2. Define the TBDD header as a `#[repr(C)]` Pod struct; parse it, cast the payload with
   `bytemuck::cast_slice` (aligned copy on failure; short payload → Err).
3. Class-R test: decode all 625 `objects/density/*.bin` with both paths; assert identical.
4. `density.rs`: unchanged bytes; add an emit → decode round-trip on a synthetic tile.
5. Perturbation: off-by-one the header length → Class-R red; restore, touch, green.

## Risks
- If a TBDD field is not naturally aligned, cast the cell payload only and parse the header
  manually — parity test still decides.
- The test reads the repo's data directory: locate it relative to `CARGO_MANIFEST_DIR`.

## Verification
- `cargo test -p map-engine-core --all-features tbdd`
- `cargo test -p tbd-tools density`
- `cargo xtask platform wave gate --slice T-935.5`
