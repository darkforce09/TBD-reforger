# T-298 — Gate tbd-tools density tests in CI

Owner: command center. `cargo test -p tbd-tools --lib density::` passes on main (2026-09-05) but no CI lane runs it;
the original failure (partition RHS 401) was invisible for the same reason.

## Claude Code prompt — T-298

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-298 && pwd && git branch --show-current   # must be slice/T-298
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
tools/tbd-tools/src/density.rs (tests module), .github/workflows/ci.yml:60-130, xtask/src/mk_ci_tasks.rs (is ci.yml rendered?), docs/plans/t-298_plan.md
═══ PROBLEM ═══
tbd-tools has no CI test lane; the density partition invariant is only tested by hand.
═══ SHIPPED ═══
density.rs tests (2 passing), T-238 cancelled: `ticket check --strict` already in ci.yml:237.
═══ LANGUAGE GATE ═══
Rust + YAML only. No shell scripts.
═══ LOCKED ═══
- Existing two density tests stay byte-identical; the new test is additive and seeded (deterministic).
- Mod wave gate scope (`--lib enf::`) unchanged.
═══ DO ═══
1. Confirm on main the tests pass and CI has no tbd-tools step; paste both.
2. Add the CI step (or generator change). 3. Add the seeded randomized partition test.
4. Perturb the oracle by one → red → restore → touch → green. 5. cargo xtask platform wave gate --slice T-298
═══ DO NOT ═══
No git add -A, no git stash, no ci-local. Touch only density.rs and ci.yml (plus mk_ci_tasks.rs only if it renders ci.yml — report it).
═══ VERIFY ═══
cargo test -p tbd-tools --lib density:: ; cargo xtask platform wave gate --slice T-298
═══ MANUAL ═══
Command center watches the first CI run after landing for the tbd-tools step.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
