# T-937.2 — Plan

## Context
store.rs:361-371 UndoOptions{capture_timeout_millis: 0, timestamp: ZeroClock}, deliberate Yjs parity per
:350-360 (T-159.22.1); no depth cap. Every transaction is an undo step. T-257 owns undo scope.

## Approach
1. Verify on main: three ops within 50 ms need three undos → paste the red.
2. `doc/undo_groups.rs` (new, in doc/mod.rs): gesture window (300 ms, injectable clock), begin/end group, cap 200.
3. store.rs: build UndoOptions from undo_groups; replace the :350-371 comment with the dated decision.
4. `state/operations/batch.rs` (new, in operations.rs): with_batch(label, f); use from one multi-op path.
5. Perturbation: window 0 → grouping test red; restore, touch, green.

## Risks
- Real clock on wasm vs tests — injectable clock trait.
- Cap eviction must drop whole groups, never split one.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask mk ci-local-leptos`
- `cargo xtask platform wave gate --slice T-937.2`
