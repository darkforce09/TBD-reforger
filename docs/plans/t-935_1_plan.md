# T-935.1 — Plan

## Context
No binary format definitions exist in map-engine-core (rkyv nowhere, bytemuck only in render).
Every later slice needs the same POD, headers and archive types, so they are defined once here
behind a `binary` feature that `world` enables. The POD is 32 B: yaw, pitch, roll, scale are on
the wire (manifest transforms `yaw+pitch+roll+scale`), unlike the operator's 24 B sketch.

## Approach
1. `crates/map-engine-core/Cargo.toml`: feature `binary` = rkyv 0.8 (+bytecheck) + bytemuck derive.
2. `src/world/binary/{mod,pod,chunk_container,archives}.rs`: ObjectInstancePod, TBDC/TBDE/TBDB
   32-byte Pod headers (padding-free field order), rkyv archive types, `access_checked`.
3. `src/world/manifest.rs`: optional `objects.binary`, `dem.raw`, `labels`, `water`, `buildings`.
4. Round-trip + corruption tests for every type; compile-time size assertions.

## Risks
- rkyv 0.8 API drift (`access`, `to_bytes`, bytecheck attrs): pin exact version in Cargo.toml.
- wasm32 target: rkyv/bytemuck are no_std-friendly; verify with `cargo xtask mk leptos-gates`
  if the frontend fails to build, fall back to `rkyv` default features minus `std` extras.

## Verification
- `cargo test -p map-engine-core --all-features`
- `cargo xtask ci verify file-length`
- `cargo xtask platform wave gate --slice T-935.1`
