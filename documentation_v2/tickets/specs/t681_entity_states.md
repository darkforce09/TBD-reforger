# T-681 — entity states: health, allow-damage, show-model, size, stamina

Ticket: .ai/tickets/T-681.toml · Plan: docs/plans/t-681_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-681

```
Read CLAUDE.md first. Implement **T-681** — entity states: health, allow-damage, show-model, size, stamina.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-681
═══ READ ═══  docs/plans/t-681_plan.md; Backend/TBD_MissionLoader.c (entities[] has no consumer, mission.schema.json:72); new Backend/TBD_EntityState.c; Gamemode/TBD_SpawnManager.c
═══ PROBLEM ═══  Five OBJ attrs have no runtime destination because entities[] has no consumer in the mod.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Answer the stamina API question in the report before coding
  - Unsupported state on a prefab logs and skips
═══ DO ═══
  1. Bind entities[] in the loader
  2. Implement the five state applications in TBD_EntityState
  3. Hook it from the entity spawn site
  4. Tag T-681 · commit prefix T-681:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-681
═══ MANUAL ═══  Human checklist: an entity with health 50 and allow-damage false behaves so.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

