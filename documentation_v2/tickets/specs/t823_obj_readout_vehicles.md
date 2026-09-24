# T-823 — OBJ readout counts vehicles or is renamed honestly

Ticket: .ai/tickets/T-823.toml · Plan: docs/plans/t-823_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-823

```
Read CLAUDE.md first. Implement **T-823** — OBJ readout counts vehicles or is renamed honestly.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-823
═══ READ ═══  docs/plans/t-823_plan.md; apps/website/frontend/src/editor/state/history.rs:350 (slot_count readout)
═══ PROBLEM ═══  OBJ equals slot_count, so a placed vehicle leaves it unchanged while the vehicle really places.
═══ SHIPPED ═══  T-819 crewed-slot derived hide.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - WEST/EAST/IND/TOTAL chips keep slot semantics
  - Pin the decision (count vs rename) in the ticket
═══ DO ═══
  1. Decide count-vs-rename
  2. Implement in history.rs
  3. Test: place a vehicle → readout reflects the decision
  4. Tag T-823 · commit prefix T-823:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-823
═══ MANUAL ═══  Place a vehicle and read the strip.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
