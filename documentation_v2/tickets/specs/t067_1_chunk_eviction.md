# T-067.1 — Byte-budget chunk eviction at 1M objects

Owner: command center. Program T-111 (spec docs/specs/Mission_Creator_Architecture/t067_spatial_chunks.md §Deferred),
re-anchored 2026-09-05 on the Rust port: world/residency.rs LRU by count, world_host.rs caches never evicted.

## Claude Code prompt — T-067.1

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-067.1 && pwd && git branch --show-current   # must be slice/T-067.1
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/world/residency.rs:1-80 (header), set_viewport, end_apply_frame, eviction_log; world_host.rs:1-120; docs/plans/t-067_1_plan.md
═══ PROBLEM ═══
Count-based eviction and immortal caches overrun browser memory on a 1M-object terrain.
═══ SHIPPED ═══
T-166 residency port (LRU exactness, pinned never evicted), T-935.13 binary chunks — keep both semantics.
═══ LANGUAGE GATE ═══
Rust only. `cargo test -p map-engine-core --all-features`, never without the flag.
═══ LOCKED ═══
- residency.rs is allowlisted SIZE-3: call sites only; policy lives in eviction.rs.
- Pinned chunks are never evicted; delivery-order determinism (header) is preserved.
- No file-length allowlist edits.
═══ DO ═══
1. Show on main that caches survive eviction (wasm test); paste the red. 2. eviction.rs + tests. 3. Call sites.
4. world_host.rs cache drop + reload. 5. Perturb (ignore budget) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no changes to chunk formats or the fetch layer.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features world::eviction ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-067.1
═══ MANUAL ═══
Operator: pan across everon at max zoom-out with the debug stat open; resident bytes never exceed the budget.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
