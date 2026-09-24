# T-850 — squad tether follows drag on auto-grouped units

Ticket: .ai/tickets/T-850.toml · Plan: docs/plans/t-850_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-850

```
Read CLAUDE.md first. Implement **T-850** — squad tether follows drag on auto-grouped units.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-850
═══ READ ═══  docs/plans/t-850_plan.md; crates/map-engine-core/src/squad_links.rs:7 (pack_squad_link_drag_preview); apps/website/frontend/src/editor/tools/select_tool.rs:299 (bind_squad_link_preview)
═══ PROBLEM ═══  The tether does not move mid-drag for auto-grouped same-squad units; the verifier could not re-prove it with the 4 px probe.
═══ SHIPPED ═══  T-801 preview lane; T-848 clean membership (may be a prerequisite).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Reproduce on the release build first
  - If membership is the cause, say so and re-test after T-848
═══ DO ═══
  1. Repro single and two-selected drags with probe numbers
  2. Fix the preview path for the auto-group membership shape
  3. Perturbation proof
  4. Tag T-850 · commit prefix T-850:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-850
═══ MANUAL ═══  Drag an auto-grouped unit; the tether follows.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
