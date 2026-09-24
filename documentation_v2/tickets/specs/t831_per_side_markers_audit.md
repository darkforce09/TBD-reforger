# T-831 — per-side marker authoring: audit then explicit UI

Ticket: .ai/tickets/T-831.toml · Plan: docs/plans/t-831_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-831

```
Read CLAUDE.md first. Implement **T-831** — per-side marker authoring: audit then explicit UI.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-831
═══ READ ═══  docs/plans/t-831_plan.md; apps/website/frontend/src/editor/panels/dock_right.rs:3799 (Markers surface copy); T-069 per-side briefings
═══ PROBLEM ═══  Markers are side-scoped in the data model but how the active side switches and whether OPFOR/INDFOR authoring is reachable is unknown; there is no explicit per-side UI.
═══ SHIPPED ═══  T-069 briefings; T-673 is the runtime enforcement half (not built here).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Audit first and record it in the report
  - Game enforcement is T-673 — cite, do not build
═══ DO ═══
  1. Audit the active-side switch and per-side export
  2. Add a side selector on the Markers surface; map shows the selected side set
  3. Export test with BLUFOR and OPFOR markers
  4. Tag T-831 · commit prefix T-831:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-831
═══ MANUAL ═══  Author one marker per side and inspect the export.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
