# T-845 — selected vehicles get the ringed-twin treatment

Ticket: .ai/tickets/T-845.toml · Plan: docs/plans/t-845_plan.md · Filed from the wave verifier / operator eye-pass named in the ticket.

## Claude Code prompt — T-845

```
Read CLAUDE.md first. Implement **T-845** — selected vehicles get the ringed-twin treatment.
═══ PREFLIGHT ═══  git pull && git lfs pull; cargo xtask ticket show T-845
═══ READ ═══  docs/plans/t-845_plan.md; crates/map-engine-core/src/slots_gpu.rs:127,:150 (T-808 pattern); crates/map-engine-render/src/engine.rs (set_selection patch exits)
═══ PROBLEM ═══  A selected vehicle looks identical to an unselected one; slots and comments got selection treatment.
═══ SHIPPED ═══  T-808 ringed twin; wave-210 vehicle set.
═══ LANGUAGE GATE ═══  Rust/Leptos (edition-2024 rustfmt) and map-engine crates only; cargo test -p map-engine-core --all-features, never without the flag; no TypeScript.
═══ LOCKED ═══
  - Verify the defect on main first (paste the red)
  - Perturbation proof: red pasted verbatim, touch after restore
  - owns = the listed files only; SIZE-3 allowlisted files get minimal diffs, new code goes in new files
  - Class-R byte-parity tests scrub their own source
  - No status changes, no merges, no pushes
  - Extra atlas cells, not extra instances
  - O(delta) patch on set_selection; no re-pack
═══ DO ═══
  1. Render test: one of three selected → identical crops (red)
  2. Ringed-twin cell per silhouette kind + tint; patch the vehicle lane on set_selection
  3. Test exactly one changes; deselect neutral; no re-pack
  4. Tag T-845 · commit prefix T-845:
═══ DO NOT ═══  git add -A; git stash; cargo xtask ci ci-local; extend file-length allowlists; change status; defer in-scope work; skip: = FAIL
═══ VERIFY ═══  cargo test -p map-engine-core --all-features; cargo test -p map-engine-render; cargo xtask mk leptos-gates; cargo xtask platform wave gate --slice T-845
═══ MANUAL ═══  Select a vehicle among three.
═══ RETURN ═══  pwd_branch · defect_verified_on_main · changes · perturbation · gate_verdict_tail · files_outside_owns · found_not_fixed · deviations · commits. Ready for Cursor doc sync.
```
