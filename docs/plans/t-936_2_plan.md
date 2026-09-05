# T-936.2 — Plan

## Context
No task state machine exists (audit S1); the mod has no trigger runtime until T-676 ships. The schema has
`task` only as an enum value (:1208). This slice adds tasks with tiers, states, a panel, and two scripts.

## Approach
1. Schema `tasks[]` {id, title, tier, state, triggerId?, markerId?, description?}; golden updated.
2. `mission/tasks.rs` (new, in mission/mod.rs): model + transition table; register in extensions.rs.
3. `panels/tasks_panel.rs` (new, in panels/mod.rs): list + pickers, undoable.
4. `Objectives/TBD_TaskStateMachine.c` (new, driven by T-676 completions); `UI/TBD_TaskHud.c` (new).
5. Perturbation: legalise succeeded→assigned → illegal-transition test red; restore, touch, green.

## Risks
- Overlap with objectives (T-212) and endOn triggers: tasks observe, never fire, endOn.
- Replication of state to clients — one authoritative array, verified on the checklist.

## Verification
- `cargo test -p map-engine-core --all-features`; `cargo xtask ci schema-validate`; `cargo xtask mk ci-local-leptos`
- `cargo xtask mk leptos-gates`; `cargo xtask mod compile`; `cargo xtask platform wave gate --slice T-936.2`
