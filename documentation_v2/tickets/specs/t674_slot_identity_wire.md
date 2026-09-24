# T-674 — Slot identity reaches the wire (T-216 follow-on)

Program T-674 · slices T-674.1 (engine emit) → T-674.2 (mod reader + validator 1.3) → T-674 (closure). Schema widened in T-706.

## Claude Code prompt — T-674.1

```
Read CLAUDE.md first. Implement **T-674.1** — flatten emit of slot identity + leaderSlotId.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-674.1
═══ READ ═══  docs/plans/t-674_1_plan.md; crates/map-engine-core/src/mission/flatten.rs:2584-2649 (ledger), :2620-2632 (contract delta)
═══ PROBLEM ═══  callsign, rank, stance, unitName, tag and squad leaderSlotId are authored but silently dropped at compile; the ledger tripwire exists, the fix does not.
═══ SHIPPED ═══  T-216 ledger + tripwire; T-706 schema widening.
═══ LANGUAGE GATE ═══  Rust in crates/map-engine-core only; cargo test -p map-engine-core --all-features (never without the flag).
═══ LOCKED ═══
  - Verify on main: a flatten test asserting callsign on the wire is red
  - Perturbation proof: red pasted verbatim, touch after restore
  - schemaVersion 1.3 only when an identity key emits; wire-unsafe values drop whole; rank/stance enum-gated after trim
  - owns = flatten.rs only; T-675.1 lands after you on the same file
═══ DO ═══
  1. Write the red test
  2. Emit the five ModSlot keys and ModGroup.leaderSlotId
  3. Bump schemaVersion to 1.3 conditionally
  4. Golden fixture through schema-validate; perturbation proof
  5. Tag T-674.1 · commit prefix T-674.1:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask ci schema-validate; cargo xtask platform wave gate --slice T-674.1
═══ MANUAL ═══  None.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

## Claude Code prompt — T-674.2

```
Read CLAUDE.md first. Implement **T-674.2** — Enfusion reader for slot identity.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-674.2
═══ READ ═══  docs/plans/t-674_2_plan.md; Backend/TBD_MissionSlotStruct.c; Backend/TBD_MissionLoader.c; Backend/TBD_MissionValidator.c:42; Gamemode/TBD_SpawnManager.c
═══ PROBLEM ═══  The mod has no fields for the identity keys, no stance call, and its validator refuses schemaVersion 1.3, so T-674.1's wire is rejected outright.
═══ SHIPPED ═══  T-674.1 emit (must be shipped).
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Missions without identity keys behave exactly as before
═══ DO ═══
  1. Confirm the validator list lacks 1.3 and the struct lacks the keys
  2. Add struct fields and loader binding
  3. Accept 1.3 in TBD_MissionValidator
  4. Apply callsign/rank/unitName, stance pose and leaderSlotId in TBD_SpawnManager
  5. Compare with salvage/t853-dropped/T-674; reuse only matching hunks
  6. Tag T-674.2 · commit prefix T-674.2:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-674.2
═══ MANUAL ═══  Human checklist: spawned bodies show authored callsign/rank/name and stance; squad leader resolves.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

## Claude Code prompt — T-674

```
Read CLAUDE.md first. Implement **T-674** — program closure.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-674
═══ READ ═══  docs/plans/t-674_plan.md; docs/plans/t-674_plan.md; the two child reports
═══ PROBLEM ═══  Closure after both slices ship; no code unless T-674.2 reported found_not_fixed on TBD_MissionSlotStruct.c.
═══ SHIPPED ═══  T-674.1; T-674.2.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
═══ DO ═══
  1. Confirm struct fields, loader binding and validator entry on main
  2. Compile a fixture mission with every identity key and run the mod compile
  3. Fix only a found_not_fixed item on TBD_MissionSlotStruct.c
  4. Tag T-674 · commit prefix T-674:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-674; cargo xtask ticket check
═══ MANUAL ═══  Human checklist: end-to-end identity in game.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

