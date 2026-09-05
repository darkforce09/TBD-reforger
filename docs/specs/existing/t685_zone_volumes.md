# T-685 — zone volumes: height bounds, capture counts, owner

Ticket: .ai/tickets/T-685.toml · Plan: docs/plans/t-685_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-685

```
Read CLAUDE.md first. Implement **T-685** — zone volumes: height bounds, capture counts, owner.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-685
═══ READ ═══  docs/plans/t-685_plan.md; Backend/TBD_MissionLoader.c zone binding; Objectives/TBD_ObjectiveRegistry.c; new Zones/TBD_ZoneVolume.c; apps/website/frontend/src/editor/panels/zones_panel.rs
═══ PROBLEM ═══  Zones carry no height, counts or starting owner; the corpus (inferred WOG WMT_Task_Point) says they must.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript. Plus Rust/Leptos for the inspector fields.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - WOG semantics are INFERRED — say so in comments
  - Existing zoneRules keys keep behaviour
═══ DO ═══
  1. Bind the new keys in the loader
  2. Implement the volume test and count/owner semantics in TBD_ZoneVolume
  3. Add the resolve branch in TBD_ObjectiveRegistry
  4. Add three numeric fields and a side picker to the zone inspector
  5. Tag T-685 · commit prefix T-685:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-685
═══ MANUAL ═══  Human checklist: a zone with max height 30 ignores aircraft above it.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

