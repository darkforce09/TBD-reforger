# T-676 — trigger activation and effects runtime

Ticket: .ai/tickets/T-676.toml · Plan: docs/plans/t-676_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-676

```
Read CLAUDE.md first. Implement **T-676** — trigger activation and effects runtime.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-676
═══ READ ═══  docs/plans/t-676_plan.md; Zones/TBD_ZoneRegistry.c; Backend/TBD_MissionLoader.c (zoneRules binding); new Zones/TBD_TriggerRuntime.c
═══ PROBLEM ═══  Twelve TRG attrs and sixteen zoneRules keys are on the wire; nothing activates triggers or fires effects in game.
═══ SHIPPED ═══  T-079 geometry palette; T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Server-authoritative only
  - Missing loader field → report under files_outside_owns, do not widen
═══ DO ═══
  1. Map every trigger key to a semantic from the loader/registry
  2. Implement activation (condition, repeat, timeout, owner side) in TBD_TriggerRuntime
  3. Implement the effects model
  4. Tag T-676 · commit prefix T-676:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-676
═══ MANUAL ═══  Human checklist: a trigger authored in the editor activates and fires in game.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

