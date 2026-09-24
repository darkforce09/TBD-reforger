# T-067.1 — Plan

## Context
WorldResidency (residency.rs, 3137 lines, allowlisted) evicts by chunk count; the occluder/label caches in
world_host.rs are never evicted with their chunks. On a 1M-object terrain that overruns the wasm heap while every
count is legal. T-935.13 (binary cutover) lands first and changes per-chunk byte sizes.

## Approach
1. New `crates/map-engine-core/src/world/eviction.rs` (register in `world/mod.rs`): `EvictionPolicy::new(budget)`,
   `victims(&[(id, bytes, last_tick, pinned)]) -> Vec<id>`; tests: budget respected, pinned kept, LRU order.
2. `world/residency.rs`: call the policy in set_viewport/end_apply_frame; chunk bytes from the SoA lengths
   (call sites only — no new logic in the allowlisted file).
3. `world_assets/world_host.rs`: on eviction_log entries drop occluder/label caches; on viewport re-entry refetch.
4. Perturbation: budget = usize::MAX in the policy → budget test red; restore, `touch`, green.
## Risks
- Byte estimates drift from real heap; add a debug stat and compare with performance.memory in the report.
- Cache drop must not race in-flight fetches; key drops by chunk id + generation.

## Verification
- `cargo test -p map-engine-core --all-features world::eviction` · leptos gates · `cargo xtask platform wave gate --slice T-067.1`
