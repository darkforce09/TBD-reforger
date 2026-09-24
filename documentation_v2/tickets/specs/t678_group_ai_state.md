# T-678 — group AI state: combat mode, behaviour, formation, speed

Ticket: .ai/tickets/T-678.toml · Plan: docs/plans/t-678_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-678

```
Read CLAUDE.md first. Implement **T-678** — group AI state: combat mode, behaviour, formation, speed.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-678
═══ READ ═══  docs/plans/t-678_plan.md; AI/TBD_WaypointRuntime.c (T-677 gate); new AI/TBD_GroupState.c
═══ PROBLEM ═══  Four GRP attrs have zero readers; groups spawn with engine defaults regardless of what the author set.
═══ SHIPPED ═══  T-677 AI gate (packs first); T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Absent attrs leave engine defaults
═══ DO ═══
  1. Confirm zero readers for combatMode/speedMode
  2. Apply the four attrs post-spawn through the AI group API in TBD_GroupState
  3. Record the exact API calls in the report
  4. Tag T-678 · commit prefix T-678:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-678
═══ MANUAL ═══  Human checklist: formation and speed visibly differ per authored value.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

