# T-830 — outliner density pass: taller rows, hover tools

Ticket: .ai/tickets/T-830.toml · Plan: docs/plans/t-830_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-830

```
Read CLAUDE.md first. Implement **T-830** — outliner density pass: taller rows, hover tools.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-830
═══ READ ═══  docs/plans/t-830_plan.md; apps/website/frontend/src/editor/panels/outliner_tree.rs:158 (h-4 recipe, windowing)
═══ PROBLEM ═══  16 px rows with always-visible icon clusters squeeze truncated names at 240 px; Eden shows row tools on hover.
═══ SHIPPED ═══  T-803 drop-target; T-809 vehicle rows; windowing threshold 50.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Keep windowing, rename, drop-target and Alt-click affordances
  - Tools visible on the active row too
═══ DO ═══
  1. Raise row height/padding; tool cluster on hover-or-active
  2. Give the name the freed width
  3. Existing outliner tests green; before/after screenshots
  4. Tag T-830 · commit prefix T-830:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-830
═══ MANUAL ═══  Read a 240 px outliner with typical names.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
