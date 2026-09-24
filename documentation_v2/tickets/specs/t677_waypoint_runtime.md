# T-677 — waypoints: group movement orders

Ticket: .ai/tickets/T-677.toml · Plan: docs/plans/t-677_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-677

```
Read CLAUDE.md first. Implement **T-677** — waypoints: group movement orders.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-677
═══ READ ═══  docs/plans/t-677_plan.md; Gamemode/TBD_SpawnManager.c:963,:1166 (AI disabled); new AI/TBD_WaypointRuntime.c
═══ PROBLEM ═══  Every body spawns with AI disabled so waypoints have no subject; nine WP attrs and six interaction keys go unread.
═══ SHIPPED ═══  T-706 schema; operator 2026-08-02: AI units are coming.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Enable AI only for groups whose payload carries waypoints or T-678 attrs
  - Players unchanged
═══ DO ═══
  1. Confirm both spawn sites pass AI disabled
  2. Gate AI enable on payload presence in TBD_SpawnManager
  3. Implement the per-group ordered waypoint queue and interaction wiring
  4. Tag T-677 · commit prefix T-677:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-677
═══ MANUAL ═══  Human checklist: an AI group follows its waypoints on a dedicated server.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

