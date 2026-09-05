# T-133 — Plan

## Context
T-936.2 lands authored tasks with trigger-driven transitions. OFCR-style timed objectives need a schedule on the
same model, edited in the same panel, evaluated by the same state machine on the mission clock.

## Approach
1. `crates/map-engine-core/src/mission/tasks.rs`: `Schedule {start_after_s, window_s}`; validator rules; tests for
   each rule and for schema round-trip (the tasks schema block is T-936.2's — extend it only if it lives in tasks.rs).
2. `panels/tasks_panel.rs`: schedule fields with undoable ops; wasm test for refusal copy.
3. `Objectives/TBD_TaskStateMachine.c`: per-tick check of the mission clock; `[TBD][Task] id=<n> t=<s> -> <state>`
   log; `cargo xtask mod compile`.
4. Perturbation: disable the window rule → validator test red; restore, `touch`, green.
## Risks
- If the schedule needs a schema change in mission.schema.json (not owned), report found_not_fixed and ship the
  model behind an editor-only key until a schema slice picks it up.

## Verification
- `cargo test -p map-engine-core --all-features mission::tasks` · `cargo xtask mk ci-local-leptos` · `cargo xtask mod compile`
- `cargo xtask platform wave gate --slice T-133` · human checklist: T+2 min objective activates on time.
