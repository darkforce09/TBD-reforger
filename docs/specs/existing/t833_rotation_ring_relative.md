# T-833 — rotation ring: relative delta plus live preview

Ticket: .ai/tickets/T-833.toml · Plan: docs/plans/t-833_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-833

```
Read CLAUDE.md first. Implement **T-833** — rotation ring: relative delta plus live preview.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-833
═══ READ ═══  docs/plans/t-833_plan.md; apps/website/frontend/src/editor/tools/select_tool.rs (drag machinery); apps/website/frontend/src/editor/mission_editor.rs:422 (snap ladder); T-796 bind_vehicle_preview_lane; T-788 batch API
═══ PROBLEM ═══  The ring sets facing to the release bearing instead of applying the drag delta, and there is no live preview.
═══ SHIPPED ═══  T-648 (no-preview, superseded); T-808 facing point.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Default orbits the selection centre in one undo step; Ctrl turns each unit in place
  - Zero rotation until the cursor moves
═══ DO ═══
  1. Press 170° off facing, release unmoved → facing jumps (red)
  2. Record grab bearing + heading; apply the delta per move with a per-frame preview
  3. Ctrl modifier; fix the eden_help comment
  4. Tag T-833 · commit prefix T-833:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-833
═══ MANUAL ═══  Drag 30° from any grab point; heading changes by 30.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
