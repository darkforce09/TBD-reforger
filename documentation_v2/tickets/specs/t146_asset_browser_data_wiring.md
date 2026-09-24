# T-146 — Asset Browser data wiring (registry vehicles/crates → drag-place)

Ticket: `.ai/tickets/T-146.toml` · Plan: `docs/plans/t-146_plan.md` · Parent spec: `docs/specs/Mission_Creator_Architecture/t150_universal_registry_export.md`

## Claude Code prompt — T-146

```
Read CLAUDE.md first. Implement **T-146** — Asset Browser data wiring.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-146
═══ READ ═══  docs/plans/t-146_plan.md; apps/website/frontend/src/editor/arsenal/asset_catalog.rs (build_catalog_tree :146); apps/website/frontend/src/editor/panels/dock_right.rs (registry_items :391-408)
═══ PROBLEM ═══  Registry rows for vehicles and crates never become Asset Browser leaves, so they cannot be drag-placed; characters can.
═══ SHIPPED ═══  T-150 registry export; T-809 per-faction tree; T-084/T-646 catalog search.
═══ LANGUAGE GATE ═══  Rust/Leptos only; no TypeScript.
═══ LOCKED ═══
  - owns = asset_catalog.rs only; new code that does not fit goes in a sibling module you name.
  - Reuse the palette-drag place path; no second placement lane.
  - Verify the gap on main first; perturbation proof (red pasted verbatim, `touch` after restore).
═══ DO ═══
  1. Extend build_catalog_tree with vehicle/crate leaves keyed by registry kind.
  2. Wire the leaves into the existing drag-place path.
  3. Native tests: tree fixture + search hit for the new leaves.
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status.
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-146
═══ MANUAL ═══  Drag a vehicle and a crate from the browser onto the map; both place and appear in the outliner.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
