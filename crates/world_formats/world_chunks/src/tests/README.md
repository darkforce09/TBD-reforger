# world_chunks/tests

Unit tests of the chunk JSON decoder, the `TBDC` container decoder, the chunk identifier and the
terrain manifest.

## Contents

- `chunk_container_tests.rs`
- `chunk_id_tests.rs`
- `terrain_manifest_tests.rs`
- `world_chunk_tests.rs`

## Boundaries

Run with `cargo test -p world_chunks`; the Everon cases read `assets/terrains/everon/` and the
goldens in `contracts/fixtures/map/`.
