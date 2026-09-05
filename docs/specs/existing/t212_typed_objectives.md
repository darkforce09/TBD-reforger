# T-212 — typed per-side objectives with attributes

Ticket: .ai/tickets/T-212.toml · Plan: docs/plans/t-212_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-212

```
Read CLAUDE.md first. Implement **T-212** — typed per-side objectives with attributes.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-212
═══ READ ═══  docs/plans/t-212_plan.md; Objectives/TBD_ObjectiveRegistry.c:36-42; crates/map-engine-core/src/mission/compile.rs:228 (dead objectivesById); docs research wog.md 7/13.1, fnf_v4.md 7
═══ PROBLEM ═══  objectivesById is a dead container; objectives-as-zones is the only consumer; the corpus converges on typed, placed, per-side objectives with one attribute spine.
═══ SHIPPED ═══  T-685 volumes (packs first, shares the registry); T-241; T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Stable uid identity; no positional identity
  - Inferred WOG semantics stay marked inferred
  - SPA authoring UI is a follow-on slice, not this one
═══ DO ═══
  1. Decide and record the shape in the report
  2. Resolve typed per-side objectives in TBD_ObjectiveRegistry from the T-706 keys
  3. Keep the zone-typed path working
  4. Tag T-212 · commit prefix T-212:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-212
═══ MANUAL ═══  Human checklist: an objective reads differently to attacker and defender in game.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

