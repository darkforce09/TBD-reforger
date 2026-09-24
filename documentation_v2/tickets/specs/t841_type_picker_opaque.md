# T-841 — Type picker popover on an opaque panel

Ticket: .ai/tickets/T-841.toml · Plan: docs/plans/t-841_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-841

```
Read CLAUDE.md first. Implement **T-841** — Type picker popover on an opaque panel.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-841
═══ READ ═══  docs/plans/t-841_plan.md; apps/website/frontend/src/editor/panels/attributes_modal.rs:1579 (type_picker)
═══ PROBLEM ═══  The T-810 Type picker popover is translucent, so option text blurs over the map.
═══ SHIPPED ═══  T-810 picker; T-827 live-composite lesson.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Solid modal surface token; no alpha, no backdrop-blur
  - Measure on the live composite
═══ DO ═══
  1. Opaque plate for the popover and its search input
  2. Measure option text ≥ 4.5:1 live
  3. No other picker behaviour changes
  4. Tag T-841 · commit prefix T-841:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-841
═══ MANUAL ═══  Open the Type picker over a busy map area.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
