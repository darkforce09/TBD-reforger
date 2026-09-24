# T-821 — save version prefill auto-bumps

Ticket: .ai/tickets/T-821.toml · Plan: docs/plans/t-821_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-821

```
Read CLAUDE.md first. Implement **T-821** — save version prefill auto-bumps.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-821
═══ READ ═══  docs/plans/t-821_plan.md; apps/website/frontend/src/editor/mission_editor.rs:1012,:2694 (save_semver); apps/website/frontend/src/editor/state/commands_hotkeys.rs:955 (save_now)
═══ PROBLEM ═══  Save Version prefill is a static 0.1.0 and save_now never bumps it, so the second save 409s.
═══ SHIPPED ═══  T-789 (claimed auto-bump; refuted).
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Manual semver input still wins
  - Pin the chosen scheme in the ticket
═══ DO ═══
  1. Save 0.1.0, reopen → prefill 0.1.0 (red)
  2. Prefill from the persisted latest version patch+1, or bump on the save-success arm
  3. Test default-accept saves without 409 and manual override
  4. Tag T-821 · commit prefix T-821:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-821
═══ MANUAL ═══  Save twice accepting the default.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
