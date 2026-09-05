# T-822 — outliner dblclick does not open the asset picker

Ticket: .ai/tickets/T-822.toml · Plan: docs/plans/t-822_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-822

```
Read CLAUDE.md first. Implement **T-822** — outliner dblclick does not open the asset picker.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-822
═══ READ ═══  docs/plans/t-822_plan.md; apps/website/frontend/src/editor/mission_editor.rs:1208 (dblclick handler); pointerdown chrome guard pattern
═══ PROBLEM ═══  The left dock is inside the map dblclick container, so a row dblclick also opens the empty-ground asset picker under Attributes.
═══ SHIPPED ═══  T-647 PLACE-003 picker; T-809 outliner footer rows.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Map-ground dblclick still opens the picker
═══ DO ═══
  1. Container-dblclick at a dock row → attrH2 = 1 and placeAsset = 1 (red)
  2. Ignore dblclick targets inside dock subtrees via composedPath/closest
  3. Test both outliner and ground dblclick
  4. Tag T-822 · commit prefix T-822:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-822
═══ MANUAL ═══  Double-click an outliner row.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
