# T-654 — conditional inclusion: variant-gated document subtrees

Ticket: .ai/tickets/T-654.toml · Plan: docs/plans/t-654_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-654

```
Read CLAUDE.md first. Implement **T-654** — conditional inclusion: variant-gated document subtrees.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-654
═══ READ ═══  docs/plans/t-654_plan.md; Backend/TBD_MissionLoader.c; salvage/t853-dropped/T-654
═══ PROBLEM ═══  Mode variants (day/night, player-count bands) cannot gate entity subtrees; everything always spawns.
═══ SHIPPED ═══  T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - No predicate = always included
  - Excluded subtrees drop their dependent refs
═══ DO ═══
  1. Read the selected variant at launch
  2. Evaluate each subtree predicate and drop non-matching subtrees before spawn
  3. Reuse matching salvage hunks
  4. Tag T-654 · commit prefix T-654:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mod compile; cargo xtask platform wave gate --slice T-654
═══ MANUAL ═══  Human checklist: a night-only subtree is absent in the day variant.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

