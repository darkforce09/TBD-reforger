# T-820 — catalog failure names the no-modpack cause and hides chips

Ticket: .ai/tickets/T-820.toml · Plan: docs/plans/t-820_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-820

```
Read CLAUDE.md first. Implement **T-820** — catalog failure names the no-modpack cause and hides chips.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-820
═══ READ ═══  docs/plans/t-820_plan.md; apps/website/frontend/src/editor/panels/dock_right.rs:749 (No modpack is configured), re-probe Effect
═══ PROBLEM ═══  With no current modpack the failure view shows the generic cause with chips visible instead of the named cause.
═══ SHIPPED ═══  T-809 restructure; wave-202 proved the cause live pre-restructure.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Reproduce in a real browser first; headless-only = close with evidence, no code
═══ DO ═══
  1. Repro with is_current = false in a browser
  2. If real: keep the 404 cause through the re-probe Effect and hide chips on failure
  3. Restore a modpack: Retry repopulates
  4. Tag T-820 · commit prefix T-820:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-820
═══ MANUAL ═══  Unset the current modpack and open the catalog.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
