# T-935.6 — Plan

## Context
build-roads (build.rs:1106-1183) writes `objects/roads.json.gz`; roads.rs:28 parses it;
store.rs:50/110/120 loads bytes with no format switch. This slice packs after T-935.2 because
both need `tools/tbd-tools/src/world/mod.rs`; the frontend URL switch is T-935.11's.

## Approach
1. `tools/tbd-tools/src/world/roads_emit.rs`: RoadNetworkArchive from the road model → rkyv bytes;
   register in `world/mod.rs`; `bin/world.rs` calls it after the JSON emit.
2. `roads.rs`: `from_archive`; `store.rs`: sniff gzip magic `1f 8b` → JSON, else rkyv.
3. Parity test on everon roads (ids, classes, widths, points bit-equal).
4. Perturbation: drop the last centerline point → parity red; restore, touch, green.

## Risks
- Segment ids are strings in the JSON; keep them as `String` in the archive to stay lossless.
- Magic sniff on an empty buffer: return Err before reading two bytes.

## Verification
- `cargo test -p map-engine-core --all-features roads`
- `cargo test -p tbd-tools roads_emit`
- `cargo xtask platform wave gate --slice T-935.6`
