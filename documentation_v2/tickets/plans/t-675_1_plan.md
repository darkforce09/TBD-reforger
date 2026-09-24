# T-675.1 — Plan

## Context

The authored vehicle roster (T-076 crew UI) never leaves the editor: `flatten.rs` emits no top-level `vehicles[]`, the ledger at `:2584-2649` pins the drop, and the `entities[]` alias cannot carry it (T-200). T-706 already widened the schema for `vehicles[]`.

## Approach

1. Verify on main: a flatten test with one authored vehicle asserts `vehicles[0].seats` on the wire — red.
2. Project the roster beside the slot emit in `crates/map-engine-core/src/mission/flatten.rs`: id, prefab, position, heading, seats with slot refs; rows with a missing prefab or dangling ref drop whole with a ledger note.
3. Reuse the 1.3 bump T-674.1 landed; golden fixture through schema-validate; perturbation proof.

## Risks

- Packs after T-674.1 on the same file; rebase conflicts are expected and small.
- Salvage commit 113108a1 overlaps T-674 — reuse only roster hunks.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask ci schema-validate` · `cargo xtask platform wave gate --slice T-675.1`
