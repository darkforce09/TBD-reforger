# T-939.2 — Plan

## Context
attributes_modal.rs:1864-1872 shows the squad read-only; faction only as asset-id text (:1546-1607, :1749-1842).
No multi-slot operation exists. Depends on T-937.2 (`with_batch`).

## Approach
1. Verify on main: test asserting no faction and no squad control in the modal; paste the red.
2. `state/operations/reassign.rs` (new, in state/operations.rs): `reassign_slots(ids, target)` moving slotIds, updating side keys.
3. Modal: faction selector + squad picker for the chosen faction, acting on the whole selection inside `with_batch`.
4. Refuse a squad of another faction with a named reason.
5. Perturbation: skip the side-key update → test red; restore, touch, green.

## Risks
- Emptied squads must survive; never delete implicitly.
- state/operations.rs shared with T-937.2/.5 → later wave.

## Verification
- `cargo xtask mk ci-local-leptos`; `cargo xtask mk leptos-gates`
- `cargo xtask platform wave gate --slice T-939.2`
