# T-679 — placement scatter: radius and area shape

Ticket: .ai/tickets/T-679.toml · Plan: docs/plans/t-679_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-679

```
Read CLAUDE.md first. Implement **T-679** — placement scatter: radius and area shape.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-679
═══ READ ═══  docs/plans/t-679_plan.md; Gamemode/TBD_SpawnManager.c spawn sites; new Backend/TBD_PlacementScatter.c
═══ PROBLEM ═══  Placement radius/shape attrs are on the wire; spawn puts every body on the exact authored point.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Zero radius returns the center exactly
  - Deterministic seed per slot id
═══ DO ═══
  1. Confirm no scatter in the spawn path
  2. Implement Scatter(center, radius, shape, seed) in TBD_PlacementScatter
  3. Call it from the slot and group spawn sites
  4. Tag T-679 · commit prefix T-679:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-679
═══ MANUAL ═══  Human checklist: a squad with radius 20 spawns spread out; radius 0 unchanged.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

