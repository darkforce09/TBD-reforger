# T-680 — vehicle states: lock, fuel, ammo

Ticket: .ai/tickets/T-680.toml · Plan: docs/plans/t-680_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-680

```
Read CLAUDE.md first. Implement **T-680** — vehicle states: lock, fuel, ammo.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-680
═══ READ ═══  docs/plans/t-680_plan.md; Gamemode/TBD_SpawnManager.c vehicle spawn site (T-675.2); new Vehicles/TBD_VehicleState.c
═══ PROBLEM ═══  Lock, fuel and ammo attrs are on the wire with no reader; spawned vehicles ignore them.
═══ SHIPPED ═══  T-215 predecessor; T-706 schema; T-675.2 vehicle spawn (packs first).
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Unset attributes leave engine defaults
═══ DO ═══
  1. Confirm no lock/fuel/ammo application in the spawn path
  2. Implement Apply(vehicle, lock, fuel, ammo) in TBD_VehicleState
  3. Call it from the vehicle spawn site
  4. Tag T-680 · commit prefix T-680:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-680
═══ MANUAL ═══  Human checklist: a locked, half-fuel vehicle spawns locked with half fuel.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

