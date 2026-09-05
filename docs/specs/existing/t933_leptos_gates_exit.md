# T-933 — leptos-gates pre-close exits 0 when the editor suite is green

Ticket: .ai/tickets/T-933.toml · Plan: docs/plans/t-933_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-933

```
Read CLAUDE.md first. Implement **T-933** — leptos-gates pre-close exits 0 when the editor suite is green.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-933
═══ READ ═══  docs/plans/t-933_plan.md; xtask/src/mk_build.rs (leptos-gates recipe); docs/platform/EDITOR_FACTORY_FOR_CURSOR.md; docs/website/EDITOR_GATE_RUNBOOK.md
═══ PROBLEM ═══  mk leptos-gates exits non-zero after editor-suite 20/20 because the v-suite SPA goldens mass-fail independently; the composite exit lies about the editor half.
═══ SHIPPED ═══  T-843 option b recipe.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - If goldens are refreshed, list every diff in the report
  - Docs and xtask comments must agree on editor vs v-suite lanes
═══ DO ═══
  1. Run the gate on main and paste the tail
  2. Refresh goldens or split the recipe so the rect pre-close is editor-suite-only
  3. Update both docs to the live command
  4. Tag T-933 · commit prefix T-933:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates exits 0 on main with editor-suite 20/20; cargo xtask platform wave gate --slice T-933
═══ MANUAL ═══  None.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
