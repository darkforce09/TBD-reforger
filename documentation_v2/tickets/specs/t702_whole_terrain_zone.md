# T-702 — whole-terrain zone: one zone sized to the map

Ticket: .ai/tickets/T-702.toml · Plan: docs/plans/t-702_plan.md · Schema half shipped in T-706.

## Claude Code prompt — T-702

```
Read CLAUDE.md first. Implement **T-702** — whole-terrain zone: one zone sized to the map.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-702
═══ READ ═══  docs/plans/t-702_plan.md; apps/website/frontend/src/editor/panels/zones_panel.rs:40-42; apps/website/frontend/src/editor/state/operations/entity.rs:3453 (begin_zone_draw); salvage/t853-dropped/T-702 (4e4eefd7)
═══ PROBLEM ═══  Every mission needs a whole-terrain play-area zone and authors draw a 12.8 km polygon by hand.
═══ SHIPPED ═══  T-582 zone draw tool.
═══ LANGUAGE GATE ═══  Rust/Leptos only; edition-2024 rustfmt; no TypeScript.
═══ LOCKED ═══
  - Verify the gap on main
  - Perturbation proof
  - One undo step; label Play Area; selected with Attributes open
  - terrain_bounds from the same source the flatten/export path reads
═══ DO ═══
  1. terrain_rect_ring + terrain_rect_is_authorable with golden tests for both terrains
  2. add_whole_terrain_zone in operations/entity.rs
  3. One button in zones_panel.rs
  4. Tag T-702 · commit prefix T-702:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-702
═══ MANUAL ═══  Press the button: polygon matches the terrain edge, one undo removes it.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```

