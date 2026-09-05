# T-924 — Gate verdict receipt required at land

Owner: command center. Shape: T-913.2 token receipt. Forward-only, no backfill.

## Claude Code prompt — T-924

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-924 && pwd && git branch --show-current   # must be slice/T-924
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
xtask/src/wave/{gate.rs:80-120 and :240-300, land.rs:1-120, mod.rs (module list)}, the T-913.2 receipt code, docs/plans/t-924_plan.md
═══ PROBLEM ═══
Land never checks that a gate ran on the sha being landed.
═══ SHIPPED ═══
T-913.2 token receipt (copy the refuse shape), T-300 run-target changes to wave/mod.rs (rebase on them).
═══ LANGUAGE GATE ═══
Rust only. No shell scripts.
═══ LOCKED ═══
- Receipt path `.ai/artifacts/verdicts/<slice>.json`; refusal names the exact re-gate command.
- No backfill; shipped tickets untouched.
═══ DO ═══
1. Show on main that land succeeds with no gate receipt (test); paste the red. 2. verdict.rs + tests. 3. Gate writes.
4. Land refuses. 5. Perturb (ignore sha) → red → restore → touch → green. 6. Wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no changes to gate semantics themselves.
═══ VERIFY ═══
cargo test -p xtask wave::verdict wave::land ; cargo xtask platform wave gate --slice T-924
═══ MANUAL ═══
Command center: next land prints the receipt line.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
