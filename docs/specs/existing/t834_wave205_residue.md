# T-834 — wave-205 residue cleanup (absorbs T-835, T-840)

Ticket: .ai/tickets/T-834.toml · Plan: docs/plans/t-834_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-834

```
Read CLAUDE.md first. Implement **T-834** — wave-205 residue cleanup (absorbs T-835, T-840).
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-834
═══ READ ═══  docs/plans/t-834_plan.md; apps/website/frontend/src/editor/mission_editor.rs:799 (widget_is_rotate); apps/website/frontend/src/editor/panels/top_strip.rs; apps/website/frontend/src/editor/panels/dock_right.rs:3888 (Placing a hint); wave205.md NIT lines
═══ PROBLEM ═══  Two stale pre-renumber comments, a dead dispatch field and an article-grammar hint remain; T-835 (No-Widget glyph) and T-840 (draft chip on boot) are absorbed.
═══ SHIPPED ═══  Barrier reconciliation (widget_digit).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Mechanical; no behaviour change beyond the absorbed acceptances
═══ DO ═══
  1. Fix the two comments to the 1/2/3 map
  2. Remove widget_is_rotate and its registration/call sites
  3. Fix the hint grammar; deliver T-835 and T-840 acceptances
  4. Tag T-834 · commit prefix T-834:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-834
═══ MANUAL ═══  Open the marker picker hint; boot a content-bearing mission and read the draft chip.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
