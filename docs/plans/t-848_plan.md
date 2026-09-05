# T-848 — Plan

## Context

Wave-208 eye-pass: one rifleman with two blue lines to two squads. Connect ▸ Group to arms via `context_menu.rs` → `arm_connect`/`complete_connect` (`operations/entity.rs:1302/1338`) → `add_connection` (`store.rs:2125`), a stackable connection-graph edge — not ORBAT membership (`slot.squadId` + `squad.slotIds`, mutated by `move_slot_to_squad` `store.rs:974`, `place_character_under_side`/`regroup_slot_onto` in `place_orbat.rs`). Both draw as blue lines.

## Approach

1. Verify on main: core test that Group to twice yields two group edges (documents today's behaviour).
2. `operations/entity.rs` + `context_menu.rs`: Group to calls `move_slot_to_squad` (exclusive); no group edge is written; unassigned source joins or gets a refusal toast.
3. `store.rs`/`place_orbat.rs`: refuse a second group-connection edge to another squad; audit Sync to with a pass/fail matrix in the report.

## Risks

- Existing missions with stacked group edges — migrate on load or tolerate; decide and state it.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-848`
