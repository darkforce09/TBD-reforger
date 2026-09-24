# T-827 — validation chip clears 4.5:1 live-effective

Ticket: .ai/tickets/T-827.toml · Plan: docs/plans/t-827_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-827

```
Read CLAUDE.md first. Implement **T-827** — validation chip clears 4.5:1 live-effective.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-827
═══ READ ═══  docs/plans/t-827_plan.md; apps/website/frontend/src/editor/panels/top_strip.rs:63 (chip geometry)
═══ PROBLEM ═══  The chip passes 5.31:1 by plate-calc but measures 4.01–4.34:1 live over the backdrop-blur glass on four backgrounds.
═══ SHIPPED ═══  T-798 chip geometry; F-36 target.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Measure live-effective with the 4-sample screenshot method
  - Keep the error text colour and T-798 dimensions
═══ DO ═══
  1. Paste the four red measurements
  2. Raise plate alpha or add a solid pill behind the count
  3. Re-measure ≥ 4.5:1 live and plate-calc
  4. Tag T-827 · commit prefix T-827:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-827
═══ MANUAL ═══  Sample the chip over the darkest and lightest map areas.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
