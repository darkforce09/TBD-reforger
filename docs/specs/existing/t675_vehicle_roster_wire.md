# T-675 — Vehicle roster reaches the game (T-076 compile half)

Program T-675 · slices T-675.1 (engine emit, after T-674.1) → T-675.2 (mod reader, after T-674.2) → T-675 (closure). Schema widened in T-706.

## Claude Code prompt — T-675.1

```
Read CLAUDE.md first. Implement **T-675.1** — flatten emit of top-level vehicles[].
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-675.1
═══ READ ═══  docs/plans/t-675_1_plan.md; crates/map-engine-core/src/mission/flatten.rs:2584-2649 (ledger sixth row)
═══ PROBLEM ═══  The authored vehicle roster (seats, crew refs) never leaves the editor: flatten emits no vehicles[] and the entities[] alias cannot carry it (T-200).
═══ SHIPPED ═══  T-076 crew UI; T-706 schema; T-674.1 (1.3 bump, same file).
═══ LANGUAGE GATE ═══  Rust in crates/map-engine-core only; cargo test -p map-engine-core --all-features (never without the flag).
═══ LOCKED ═══
  - Verify on main: a flatten test asserting vehicles[0].seats is red
  - Perturbation proof
  - Rows with a missing prefab or dangling slot ref drop whole with a ledger note
  - owns = flatten.rs only; reuse T-674.1's 1.3 bump
═══ DO ═══
  1. Write the red test
  2. Project the roster onto vehicles[] rows (id, prefab, position, heading, seats)
  3. Golden fixture through schema-validate
  4. Perturbation proof
  5. Tag T-675.1 · commit prefix T-675.1:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask ci schema-validate; cargo xtask platform wave gate --slice T-675.1
═══ MANUAL ═══  None.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

## Claude Code prompt — T-675.2

```
Read CLAUDE.md first. Implement **T-675.2** — Enfusion reader for vehicles[] + crew seating.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-675.2
═══ READ ═══  docs/plans/t-675_2_plan.md; Backend/TBD_MissionLoader.c; Gamemode/TBD_SpawnManager.c; new Backend/TBD_MissionVehicleStruct.c
═══ PROBLEM ═══  The loader has no vehicles[] field and SpawnManager spawns bodies only, so authored crews never sit in vehicles.
═══ SHIPPED ═══  T-675.1 emit; T-674.2 loader binding + validator 1.3 (same files, packs first).
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Unknown prefab or unresolved slot ref logs and skips that row
═══ DO ═══
  1. Confirm no vehicles binding in the loader
  2. Add TBD_MissionVehicleStruct.c and bind vehicles[]
  3. Spawn vehicles after bodies and seat referenced slots in TBD_SpawnManager
  4. Tag T-675.2 · commit prefix T-675.2:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-675.2
═══ MANUAL ═══  Human checklist: crew sits in authored seats; rosterless missions unchanged.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

## Claude Code prompt — T-675

```
Read CLAUDE.md first. Implement **T-675** — program closure.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-675
═══ READ ═══  docs/plans/t-675_plan.md; docs/plans/t-675_plan.md; child reports
═══ PROBLEM ═══  Closure after both slices ship; no code unless T-675.2 reported found_not_fixed on TBD_SpawnManager.c.
═══ SHIPPED ═══  T-675.1; T-675.2.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
═══ DO ═══
  1. Compile a fixture mission with an authored roster and run the mod compile
  2. Confirm the ledger sixth row reads closed
  3. Fix only a found_not_fixed item on TBD_SpawnManager.c
  4. Tag T-675 · commit prefix T-675:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-675; cargo xtask ticket check
═══ MANUAL ═══  Human checklist: end-to-end roster in game.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

