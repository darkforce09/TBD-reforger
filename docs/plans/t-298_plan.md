# T-298 — Plan

## Context
The density tests pass on main today (re-run 2026-09-05: 2 passed), but CI never runs tbd-tools, so the original
red (401 on the partition RHS) went unnoticed for weeks and could return the same way.

## Approach
1. `.github/workflows/ci.yml`: add `cargo test -p tbd-tools --lib` after the website-api test step, same cache.
   Check first whether `xtask/src/mk_ci_tasks.rs` renders this file; if so edit the generator and regenerate.
2. `tools/tbd-tools/src/density.rs`: seeded randomized test (fixed seed, 64 grids) asserting corner partition sums
   equal the full-cell sum; keep the existing two tests.
3. Perturbation: alter the oracle by one → red; restore, `touch`, green. Edition-2024 rustfmt.

## Risks
- CI wall time grows by one crate build; tbd-tools already compiles for the mod gate cache.
- If ci.yml is generated, the generator must own the step (owns then covers mk_ci_tasks.rs — say so in the report).

## Verification
- `cargo test -p tbd-tools --lib density::`
- `cargo xtask platform wave gate --slice T-298`
