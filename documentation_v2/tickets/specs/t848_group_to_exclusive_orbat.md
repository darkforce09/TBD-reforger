# T-848 — Connect ▸ Group to means exclusive ORBAT membership

Ticket: .ai/tickets/T-848.toml · Plan: docs/plans/t-848_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-848

```
Read CLAUDE.md first. Implement **T-848** — Connect ▸ Group to means exclusive ORBAT membership.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-848
═══ READ ═══  docs/plans/t-848_plan.md; apps/website/frontend/src/editor/panels/context_menu.rs; apps/website/frontend/src/editor/state/operations/entity.rs:1302,:1338 (arm/complete_connect); crates/map-engine-core/src/doc/store.rs:2125 (add_connection), :974 (move_slot_to_squad); crates/map-engine-core/src/doc/place_orbat.rs
═══ PROBLEM ═══  Group to writes a stackable connection-graph edge, not ORBAT membership, so one rifleman shows two squad lines.
═══ SHIPPED ═══  T-672 connection graph; T-647 Ctrl+drag regroup.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - One squad at a time; refuse or replace prior membership
  - Audit Sync to in the same pass
═══ DO ═══
  1. Core test documenting today's stacked edges
  2. Route Group to through move_slot_to_squad; write no group edge
  3. Unassigned source joins or gets a refusal toast; Sync to matrix in the report
  4. Tag T-848 · commit prefix T-848:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-848
═══ MANUAL ═══  Group to onto B then onto C; only one tether at a time.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
