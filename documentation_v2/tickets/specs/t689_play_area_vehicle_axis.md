# T-689 — play-area vehicle-class axis: the aircraft exemption

Ticket: .ai/tickets/T-689.toml · Plan: docs/plans/t-689_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-689

```
Read CLAUDE.md first. Implement **T-689** — play-area vehicle-class axis: the aircraft exemption.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-689
═══ READ ═══  docs/plans/t-689_plan.md; Backend/TBD_MissionLoader.c; Zones/TBD_ZoneRegistry.c; new Zones/TBD_PlayAreaVehicleAxis.c
═══ PROBLEM ═══  Play-area enforcement penalises every occupant alike; FNF v4's AIR flag shows aircraft must be exemptable.
═══ SHIPPED ═══  T-685 (same reader, packs first); T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Defaults reproduce today's behaviour exactly
  - Unknown vehicle class → ground
═══ DO ═══
  1. Bind the per-class penalty axis on boundary/base_protection zones
  2. Classify the occupant and return the effective penalty in TBD_PlayAreaVehicleAxis
  3. Branch in TBD_ZoneRegistry
  4. Tag T-689 · commit prefix T-689:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-689
═══ MANUAL ═══  Human checklist: a helicopter leaves the play area unpenalised when the author set air = exempt.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

