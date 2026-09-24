# T-816 — one Esc per layer with an armed composition and the Controls Hint open

Ticket: .ai/tickets/T-816.toml · Plan: docs/plans/t-816_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-816

```
Read CLAUDE.md first. Implement **T-816** — one Esc per layer with an armed composition and the Controls Hint open.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-816
═══ READ ═══  docs/plans/t-816_plan.md; apps/website/frontend/src/editor/mission_editor.rs:202 (cancel_pending); apps/website/frontend/src/editor/panels/top_strip.rs:117 (hint toggle); apps/website/frontend/src/core/ui.rs
═══ PROBLEM ═══  The Controls Hint is not a modal_stack participant, so one Esc closes the hint and clears the armed composition together.
═══ SHIPPED ═══  T-814 consume-aware guard (stack dialogs); T-791 arm hint.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - T-814 one-layer ladder stays intact
═══ DO ═══
  1. Keydown test: arm + hint open → one Esc clears both (red)
  2. Mark the event consumed when the hint closes or register the hint with the stack
  3. Second Esc clears the arm; test both presses
  4. Tag T-816 · commit prefix T-816:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-816
═══ MANUAL ═══  Arm a composition, open the hint, press Esc twice.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
