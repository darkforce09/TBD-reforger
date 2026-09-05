# T-828 — marker captions recompute per frame like icons

Ticket: .ai/tickets/T-828.toml · Plan: docs/plans/t-828_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-828

```
Read CLAUDE.md first. Implement **T-828** — marker captions recompute per frame like icons.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-828
═══ READ ═══  docs/plans/t-828_plan.md; crates/map-engine-render/src/engine.rs:316 (MarkerComposite)
═══ PROBLEM ═══  Caption positions freeze at bind-time zoom and drift past 40 px until a doc change forces a rebind; icons are already per-frame.
═══ SHIPPED ═══  T-790 icon screen-space half.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - One transform source shared by icon and caption; no rebind dependency
═══ DO ═══
  1. Render test: zoom 5.6 → 1.06 m/px with no doc change shows drift > 40 px (red)
  2. Compute caption offsets in the icon pass from the live camera
  3. Sweep test green; perturbation proof
  4. Tag T-828 · commit prefix T-828:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-render; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-828
═══ MANUAL ═══  Zoom continuously; captions stay attached.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
