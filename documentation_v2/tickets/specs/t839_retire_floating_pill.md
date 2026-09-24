# T-839 — retire the floating Select/Ruler/LoS pill

Ticket: .ai/tickets/T-839.toml · Plan: docs/plans/t-839_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-839

```
Read CLAUDE.md first. Implement **T-839** — retire the floating Select/Ruler/LoS pill.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-839
═══ READ ═══  docs/plans/t-839_plan.md; apps/website/frontend/src/editor/panels/toolbelt.rs (T-636 pill mounts); apps/website/frontend/src/editor/mission_editor.rs:2749-2757; apps/website/frontend/src/editor/panels/top_strip.rs (row-2 toolbar)
═══ PROBLEM ═══  Decision 1 retired the pill but T-797/T-798 never removed it; Ruler and LoS must move to the row-2 toolbar.
═══ SHIPPED ═══  T-642/T-643 arming; T-835 No-Widget arrow glyph.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - No separate Select button
  - Backspace hide-set shrinks accordingly
═══ DO ═══
  1. Row-2 Ruler and LoS buttons with chord tooltips
  2. Delete the pill component and mounts
  3. Test: no pill in any mode; arming works; Backspace hides chrome
  4. Tag T-839 · commit prefix T-839:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-839
═══ MANUAL ═══  Look at the bottom centre in every mode.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
