# T-935.3 — Plan

## Context
audit.md Finding 1.4: chunk ingest is gzip + serde on the main thread (residency.rs:716-737,
chunk.rs:46-97). residency.rs is an allowlisted SIZE-3 file (3137 lines) so the parser goes into
a new `chunk_bin.rs`; residency gains call sites only (≤ 40 lines). world_host.rs:426 builds the
chunk URLs and gets a binary branch keyed on `manifest.objects.binary.chunks`.

## Approach
1. `crates/map-engine-core/src/world/chunk_bin.rs`: `parse_chunk_bin(&[u8]) -> Result<WorldChunk>`;
   header checks, length check, aligned copy when needed, cast_slice, SoA fill. Register in `world/mod.rs`.
2. `residency.rs`: `ingest_chunk_bin(cx, cy, bytes)` beside the gz ingest, reusing its bookkeeping.
3. `world_host.rs`: branch at :426; `{cx}_{cy}` pattern from the manifest; gz path untouched.
4. Tests: two-row buffer round-trip; truncated / wrong magic / wrong version → Err.
5. Perturbation: skip the length check → truncated-buffer test red; restore, touch, green.

## Risks
- Fetched `Vec<u8>` alignment is 1 in wasm: always go through the aligned-copy path unless
  `bytemuck::try_cast_slice` succeeds.
- residency.rs growth: if > 40 lines, move bookkeeping into chunk_bin.rs, not into residency.

## Verification
- `cargo test -p map-engine-core --all-features`
- `cargo xtask map world-los --cell 18_0 --probe 9350,15,280 9380,15,290` (unchanged output)
- `cargo xtask mk leptos-gates`; `cargo xtask platform wave gate --slice T-935.3`
