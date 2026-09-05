# T-674.1 — Plan

## Context

`crates/map-engine-core/src/mission/flatten.rs:2584-2649` is the T-216 ledger: callsign, rank, stance, unitName, tag and squad `leaderSlotId` are authored but silently dropped at compile; the contract delta is already written out at `:2620-2632`. T-706 widened `mission.schema.json` for these keys, so the emit is unblocked.

## Approach

1. Verify on main: a flatten test authoring a callsign asserts it on the wire — red.
2. Emit the five `ModSlot` keys and `ModGroup.leaderSlotId`; bump `schemaVersion` to 1.3 only when any identity key emits. Wire-unsafe values drop whole; rank/stance are enum-gated after trim.
3. Golden fixture through `cargo xtask ci schema-validate`; perturbation proof on the new test.

## Risks

- A wire shape that disagrees with T-706's schema 500s `/compiled` for every mission — schema-validate runs before the gate.
- T-675.1 edits the same file afterwards (order 4320); keep the 1.3 bump in one place it can reuse.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask ci schema-validate` · `cargo xtask platform wave gate --slice T-674.1`
