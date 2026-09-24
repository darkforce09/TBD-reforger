# T-837 — vehicles can be deleted like slots

Ticket: .ai/tickets/T-837.toml · Plan: docs/plans/t-837_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-837

```
Read CLAUDE.md first. Implement **T-837** — vehicles can be deleted like slots.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-837
═══ READ ═══  docs/plans/t-837_plan.md; apps/website/frontend/src/editor/state/operations/entity.rs:64 (delete_selection); apps/website/frontend/src/editor/panels/context_menu.rs; apps/website/frontend/src/editor/panels/outliner_tree.rs:1129-1139 (PLACED VEHICLES); apps/website/frontend/src/editor/mission_editor.rs (Del key)
═══ PROBLEM ═══  A placed vehicle cannot be deleted by Del, the context menu or the outliner row; slots can.
═══ SHIPPED ═══  T-800 seeded vehicle test; T-819 crew derived hide; T-844 absorbed.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Audit which paths skip vs are absent first
  - One undo step restores the vehicle with crew
═══ DO ═══
  1. Audit and record the three paths
  2. Vehicle partition in delete_selection releasing crew; wire menu, outliner row and Del
  3. Test map/outliner/compiled export agree; undo restores crew
  4. Tag T-837 · commit prefix T-837:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-837
═══ MANUAL ═══  Place, select, Del, undo.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
