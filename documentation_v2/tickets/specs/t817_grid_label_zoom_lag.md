# T-817 — grid labels follow wheel zoom without the heartbeat lag

Ticket: .ai/tickets/T-817.toml · Plan: docs/plans/t-817_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-817

```
Read CLAUDE.md first. Implement **T-817** — grid labels follow wheel zoom without the heartbeat lag.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-817
═══ READ ═══  docs/plans/t-817_plan.md; apps/website/frontend/src/editor/panels/toolbelt.rs:40 (m_per_px, label memo)
═══ PROBLEM ═══  Labels recompute on cursor movement plus a ~1 Hz heartbeat, so a stationary-pointer wheel zoom waits up to ~1.4 s.
═══ SHIPPED ═══  T-793 position-quantised For key.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Keep the T-793 For key; pan behaviour unchanged
═══ DO ═══
  1. Scripted stationary wheel zoom shows a stale window > 100 ms (red)
  2. Subscribe the label memo to the camera m_per_px edge
  3. Assert ≤ 2 px vs the unproject oracle within one settled frame
  4. Tag T-817 · commit prefix T-817:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-817
═══ MANUAL ═══  Wheel-zoom without moving the mouse; labels snap immediately.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
