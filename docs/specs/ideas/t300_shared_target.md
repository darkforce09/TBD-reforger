# T-300 — Shared CARGO_TARGET_DIR serves unmerged slice binaries

Owner: command center. Wave-1 incident (T-192): `make api` on :8080 served unmerged slice code because the
shared target considered a worktree-built binary fresh. Live code: xtask/src/wave/mod.rs, xtask/src/platform_preflight.rs.

## Claude Code prompt — T-300

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-300 && pwd && git branch --show-current   # must be slice/T-300
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
xtask/src/wave/mod.rs (header + Ctx::cargo_target_dir :241), xtask/src/platform_preflight.rs:380-420, docs/plans/t-300_plan.md
═══ PROBLEM ═══
One shared target dir for worktrees and main; a run lane from main can execute a binary a worktree built.
═══ SHIPPED ═══
T-742 (shared target), T-853 (bash → xtask port). Keep their semantics; no per-worktree targets.
═══ LANGUAGE GATE ═══
Rust only. No shell scripts.
═══ LOCKED ═══
- check/test/clippy stay on the shared dir; only run lanes move to `run-main`.
- Stamp file name `tbd-built-from`, contents `<sha> <path>`; preflight message names both.
- Verify the defect on main first (build in a worktree, run from main, observe) and paste it.
═══ DO ═══
1. Add Ctx::run_target_dir + stamp writer in wave/mod.rs; route run lanes through it.
2. Preflight step 4: stamp vs HEAD comparison with a named fix.
3. Unit tests for both; perturbation = wrong sha in stamp → red → restore → touch → green.
4. cargo xtask platform wave gate --slice T-300
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; touch only the two owned files.
═══ VERIFY ═══
cargo test -p xtask wave:: platform_preflight ; cargo xtask platform preflight ; cargo xtask platform wave gate --slice T-300
═══ MANUAL ═══
Command center runs `cargo xtask platform preflight` on the next wave open and pastes the stamp line.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
