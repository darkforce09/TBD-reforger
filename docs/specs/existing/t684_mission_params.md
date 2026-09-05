# T-684 — mission parameters as first-class document objects

Ticket: .ai/tickets/T-684.toml · Plan: docs/plans/t-684_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-684

```
Read CLAUDE.md first. Implement **T-684** — mission parameters as first-class document objects.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-684
═══ READ ═══  docs/plans/t-684_plan.md; docs framework_synthesis C.3/C.5; Backend/TBD_MissionLoader.c; new Backend/TBD_MissionParams.c
═══ PROBLEM ═══  Launch parameters are chosen at launch, not authoring time; TBD has no reader or selection path for them.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - State the launch-selection surface honestly (server config unless a lobby exists)
═══ DO ═══
  1. Read the corpus evidence
  2. Bind params[] in the loader
  3. Resolve the launch selection and expose Get(symbol) in TBD_MissionParams
  4. Tag T-684 · commit prefix T-684:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-684
═══ MANUAL ═══  Human checklist: changing a parameter at launch changes behaviour without re-baking.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

