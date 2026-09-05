# T-705 — player gadget flags: map, compass, watch, GPS, radio

Ticket: .ai/tickets/T-705.toml · Plan: docs/plans/t-705_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-705

```
Read CLAUDE.md first. Implement **T-705** — player gadget flags: map, compass, watch, GPS, radio.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-705
═══ READ ═══  docs/plans/t-705_plan.md; Backend/TBD_MissionLoader.c; new Backend/TBD_GadgetFlags.c
═══ PROBLEM ═══  Five gadget flags are on the wire with no reader; every player gets every gadget regardless of the scenario.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Unset flags keep today's loadout
  - Hook after loadout is applied
═══ DO ═══
  1. Bind the grouped flag object in the loader
  2. Remove or withhold disabled gadgets on player spawn in TBD_GadgetFlags
  3. Tag T-705 · commit prefix T-705:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-705
═══ MANUAL ═══  Human checklist: GPS = false yields no GPS after spawn.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

