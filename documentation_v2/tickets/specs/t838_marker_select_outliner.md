# T-838 — markers select on the map, list in the outliner, dblclick opens Attributes

Ticket: .ai/tickets/T-838.toml · Plan: docs/plans/t-838_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-838

```
Read CLAUDE.md first. Implement **T-838** — markers select on the map, list in the outliner, dblclick opens Attributes.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-838
═══ READ ═══  docs/plans/t-838_plan.md; apps/website/frontend/src/editor/mission_editor.rs:53 (pick_comment lane); apps/website/frontend/src/editor/panels/outliner_tree.rs (T-809 placed rows pattern); apps/website/frontend/src/editor/state/operations/entity.rs:4772 (set_selection_ids)
═══ PROBLEM ═══  Markers cannot be selected by map click and do not list in the outliner; the dock is the only surface.
═══ SHIPPED ═══  T-790 pickable markers; T-763 Attributes lineage; T-831 per-side (separate).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - F-07/T-790 guard: no new dock editing lanes
  - Prune selection by doc presence — invisible is not gone
═══ DO ═══
  1. Marker map-click pick after the comment lane; dblclick opens Attributes
  2. Placed markers rows in the outliner; row dblclick opens Attributes
  3. Del removes in one undo; dock tab keeps only the picker
  4. Tag T-838 · commit prefix T-838:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-838
═══ MANUAL ═══  Click a marker glyph; the outliner row highlights.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
