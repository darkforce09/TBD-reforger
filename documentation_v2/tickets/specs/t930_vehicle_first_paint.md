# T-930 — vehicle paints its glyph on first paint

Ticket: .ai/tickets/T-930.toml · Plan: docs/plans/t-930_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-930

```
Read CLAUDE.md first. Implement **T-930** — vehicle paints its glyph on first paint.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-930
═══ READ ═══  docs/plans/t-930_plan.md; crates/map-engine-core/src/slots_gpu.rs:230 (pack_vehicle_instances); apps/website/frontend/src/editor/state/operations/entity.rs (place path); apps/website/frontend/src/editor/mission_editor.rs (rebind)
═══ PROBLEM ═══  A freshly placed vehicle paints as a yellow disc until it is moved.
═══ SHIPPED ═══  T-819 crewed-slot hide.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Confirm the disc is not a deliberate placeholder before changing the pack
═══ DO ═══
  1. Test asserting the glyph on first paint (red)
  2. Trace place → pack → upload; fix the missed rebind or pack input
  3. Perturbation proof; T-819 unchanged
  4. Tag T-930 · commit prefix T-930:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-930
═══ MANUAL ═══  Place a vehicle from the catalog and do not move it.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
