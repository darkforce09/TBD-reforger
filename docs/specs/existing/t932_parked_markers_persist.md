# T-932 — parked briefing markers survive server save/reload

Ticket: .ai/tickets/T-932.toml · Plan: docs/plans/t-932_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-932

```
Read CLAUDE.md first. Implement **T-932** — parked briefing markers survive server save/reload.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-932
═══ READ ═══  docs/plans/t-932_plan.md; crates/map-engine-core/src/doc/store.rs:905,:3998 (pending markers); crates/map-engine-core/src/mission/compile.rs
═══ PROBLEM ═══  pendingBriefingMarkers is session/meta state: local yrs keeps it, a server JSON round-trip drops it before the first faction mint.
═══ SHIPPED ═══  T-826 parking.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Wire shape stays schema-valid; park under meta rather than widen
  - Promotion on first faction mint unchanged
═══ DO ═══
  1. Core round-trip test loses the parked markers (red)
  2. Emit and restore the park on compile/hydrate
  3. Perturbation proof; V1 path intact
  4. Tag T-932 · commit prefix T-932:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-932
═══ MANUAL ═══  Save a marker-only mission to the server and reload.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
