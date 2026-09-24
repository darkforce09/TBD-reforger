# T-935.2 — Plan

## Context
`tools/tbd-tools/src/world/build.rs:503` writes each of the 315 everon chunks as gzip-9 JSON.
The loader (residency.rs:716-737) inflates and parses it on the main thread. This slice adds the
binary emitter only; the loader lands in T-935.3 and the manifest flip in T-935.13.

## Approach
1. `tools/tbd-tools/src/world/binary_emit.rs`: `write_chunk_bin(path, cx, cy, rows)` =
   TbdcHeader (32 B) + N × ObjectInstancePod (32 B), little-endian, uncompressed.
2. `build.rs`: call it next to the gz write; `world/mod.rs` registers the module;
   `tools/tbd-tools/Cargo.toml` enables map-engine-core `binary`.
3. Parity test over every committed `.json.gz`: emit to tempdir, cast_slice the `.bin`, decode
   the JSON via `chunk.rs`, compare column by column (f32 bit-equality).
4. Perturbation: swap yaw/pitch in the emitter → red on real data; restore, touch, green.

## Risks
- Chunk rows with 5 numbers (no pitch/roll/scale): emitter fills pitch = roll = 0, scale = 1,
  which is exactly what chunk.rs does — the parity test proves it.
- Generated `.bin` files must not be committed before the LFS rules (T-935.12).

## Verification
- `cargo test -p tbd-tools binary_emit`
- `cargo test -p map-engine-core --all-features`
- `cargo xtask platform wave gate --slice T-935.2`
