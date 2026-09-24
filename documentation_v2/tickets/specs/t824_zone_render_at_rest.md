# T-824 — placed zones render visibly at rest

Ticket: .ai/tickets/T-824.toml · Plan: docs/plans/t-824_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-824

```
Read CLAUDE.md first. Implement **T-824** — placed zones render visibly at rest.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-824
═══ READ ═══  docs/plans/t-824_plan.md; apps/website/frontend/src/editor/panels/zones_panel.rs; crates/map-engine-render/src/scene.rs:195; crates/map-engine-render/src/engine.rs (T-760 lane template); apps/website/frontend/src/editor/mission_editor.rs:1977 (after_doc_change)
═══ PROBLEM ═══  A placed zone is not visible on the map at rest; drawing works, so the lane is gated selection-only or missing.
═══ SHIPPED ═══  T-760/T-790 marker lane recipe; wave-130/141 rebind pins.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Measure at rest and selected at 2–3 zooms before coding
  - Follow the lane template exactly if a lane is added
═══ DO ═══
  1. Measure and record which case renders
  2. Fix the gate or add the zone lane (draw_order rebind tail + after_doc_change feed)
  3. Test circle and polygon at swept zooms; toggles round-trip; count chip matches
  4. Tag T-824 · commit prefix T-824:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo test -p map-engine-render; cargo xtask platform wave gate --slice T-824
═══ MANUAL ═══  Draw a zone, deselect, zoom out — it stays visible.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
