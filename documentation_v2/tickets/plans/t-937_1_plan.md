# T-937.1 — Plan

## Context
store.rs:5171-5191 append_id and :5162 retain_ids clone-rewrite the whole Any::Array; concurrent appends
and undo lose entries (audit S2, priority 0).

## Approach
1. Verify on main: two-peer concurrent append test loses one id → paste the red.
2. `doc/id_arrays.rs` (new, in doc/mod.rs): read/append/retain/move over a yrs array.
3. store.rs hydrate: migrate legacy Any::Array once; readers accept both forms.
4. Route every append_id/retain_ids caller through id_arrays; Class-R oracle suite unchanged.
5. Perturbation: skip the migration branch → legacy-document test red; restore, touch, green.

## Risks
- Oracle byte-parity: representation changes only for the two keys — fixtures must not move.
- Undo interaction with arrays — covered by the two-peer + undo test.

## Verification
- `cargo test -p map-engine-core --all-features`
- `cargo xtask platform wave gate --slice T-937.1`
