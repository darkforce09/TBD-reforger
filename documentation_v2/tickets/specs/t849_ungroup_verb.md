# T-849 — Ungroup / leave squad without deleting the slot

Ticket: .ai/tickets/T-849.toml · Plan: docs/plans/t-849_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-849

```
Read CLAUDE.md first. Implement **T-849** — Ungroup / leave squad without deleting the slot.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-849
═══ READ ═══  docs/plans/t-849_plan.md; crates/map-engine-core/src/doc/place_orbat.rs:56 (place_character_under_side); apps/website/frontend/src/editor/panels/context_menu.rs; apps/website/frontend/src/pages/operations/orbat_manager.rs
═══ PROBLEM ═══  Auto-placement joins the open squad and there is no verb to leave it without deleting the slot.
═══ SHIPPED ═══  T-848 exclusive membership (pairs).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Clears into a solo squad or unassigned; slot survives
═══ DO ═══
  1. ungroup_slot in place_orbat.rs with a core test
  2. Context-menu and ORBAT-page verb
  3. Test: no tether to the old leader; can Group to again
  4. Tag T-849 · commit prefix T-849:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-849
═══ MANUAL ═══  Auto-place, Ungroup, drag the slot.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
