# T-682 — environment readers: fog, wind, view distance

Ticket: .ai/tickets/T-682.toml · Plan: docs/plans/t-682_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-682

```
Read CLAUDE.md first. Implement **T-682** — environment readers: fog, wind, view distance.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-682
═══ READ ═══  docs/plans/t-682_plan.md; apps/website/frontend/src/editor/panels/env.rs (author_env ordering test); crates/map-engine-core/src/mission/flatten.rs:268-275 (ModEnvironment); Backend/TBD_MissionLoader.c; new Backend/TBD_EnvironmentReader.c
═══ PROBLEM ═══  The editor refuses fog/wind/view-distance controls until a mod reader exists, and ModEnvironment does not even serialise windDirDeg.
═══ SHIPPED ═══  T-193 control removal; T-663 DTO cleanup; T-706 schema.
═══ LANGUAGE GATE ═══  Enfusion script (.c) under apps/mod/tbd-framework only; no schema JSON; no TypeScript. Plus Rust in flatten.rs for the emit.
═══ LOCKED ═══
  - Verify the gap on main first (rg the keys in apps/mod: zero readers)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; new sibling .c files are named in the ticket
  - No packages/tbd-schema edits (T-706 owns widening)
  - In-game behaviour goes on the human checklist; the gate is cargo xtask mod compile
  - Reader before control: do not add editor controls here
  - flatten.rs test with --all-features
═══ DO ═══
  1. Serialise windDirDeg, fog, view distance in ModEnvironment with a red-then-green test
  2. Bind the fields in the loader
  3. Apply them at mission boot in TBD_EnvironmentReader
  4. Tag T-682 · commit prefix T-682:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mod compile; cargo xtask platform wave gate --slice T-682
═══ MANUAL ═══  Human checklist: authored fog visibly applies at boot.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

