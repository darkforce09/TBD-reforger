# T-937.3 — Plan

## Context
store.rs:772-812 resolves position, three strings, stance and side key per slot on every materialize;
entity.rs:1885 slot_attrs_exists materializes to answer a boolean and misses hidden slots (:1795/:1847 warn).

## Approach
1. Verify on main: hidden-slot existence test returns false → paste the red.
2. store.rs: side-key cache keyed by slot id, invalidated by the existing change observer.
3. store.rs: slot_exists(id) on the raw map; entity.rs:1885 switches to it; comments updated.
4. Test-only resolution counter proves one resolution per distinct side over 500 slots.
5. Perturbation: skip invalidation → stale-side test red; restore, touch, green.

## Risks
- Cache staleness — observer wiring is the whole correctness story; test it directly.
- materialize output must be identical — compare against a fixture snapshot.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-937.3`
