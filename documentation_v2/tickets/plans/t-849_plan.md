# T-849 — Plan

## Context

Asset-browser placement auto-joins the open squad (`place_character_under_side`, `place_orbat.rs:56`); only Remove Slot (delete), Remove Squad or refile exist — no way to leave a squad and keep the slot. Pairs with T-848 (exclusive membership).

## Approach

1. `place_orbat.rs`: `ungroup_slot` clearing membership into a solo/unassigned state without deleting the slot (core test).
2. `context_menu.rs`: Ungroup / Leave squad verb; `orbat_manager.rs`: the same verb on the ORBAT page.
3. Test: after auto-place, Ungroup leaves the slot on the map with no SQUAD_LINKS tether; it can Group to again.

## Risks

- "Unassigned" may not be a valid ORBAT state; fallback is a fresh solo squad.

## Verification

- `cargo test -p map-engine-core --all-features` · `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-849`
