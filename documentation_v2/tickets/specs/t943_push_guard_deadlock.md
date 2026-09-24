# T-943 — `platform wave push` deadlocks on large LFS ranges

Owner: command center. Measured 2026-09-04: `git check-attr --cached -z --stdin filter` (child of
`xtask platform wave push`) sat in state S for 10 minutes on `origin/main..HEAD` (28 commits; one adds
1,691 `packages/map-assets/everon/prefabs/blas/*.bvh`). Operator killed it; `git push origin main`
with hooks succeeded because git-lfs 3.7.1 is installed on the host.

## Claude Code prompt — T-943

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-943 && pwd && git branch --show-current   # must be slice/T-943
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
xtask/src/wave/push.rs (all of it — the header comments are the design record), docs/plans/t-943_plan.md
═══ PROBLEM ═══
lfs_paths_in_range writes the full diff list into check-attr's stdin before reading its stdout; lists
over the 64 KB pipe buffer deadlock. cmd_push always pushes --no-verify and refuses on any LFS path,
which is wrong on a host that has git-lfs.
═══ SHIPPED ═══
T-599 (check-attr guard), T-600 (per-commit walk), T-853 (bash → xtask port). Keep their semantics.
═══ LANGUAGE GATE ═══
Rust only. No shell scripts.
═══ LOCKED ═══
- Fail-closed behaviour and refusal text when git-lfs is absent stay byte-identical.
- Per-commit .gitattributes evaluation (temp GIT_INDEX_FILE) stays.
- No new dependencies.
═══ DO ═══
1. Reproduce: write the failing test first (stub check-attr that echoes each path as `<p>\0filter\0unspecified\0`, feed >64 KB) and watch it hang → wrap with a timeout so RED is a timeout, paste it.
2. Fix with std::thread::scope: writer thread drops stdin; main thread wait_with_output.
3. Add `git lfs version` probe in cmd_push; normal push when present, print the mode.
4. Perturb (restore inline write_all), capture RED, restore, touch, GREEN.
5. cargo xtask platform wave gate --slice T-943
═══ DO NOT ═══
Touch any file other than xtask/src/wave/push.rs (and its test module inside it). No docs edits.
═══ VERIFY ═══
cargo test -p xtask wave::push ; cargo xtask platform wave gate --slice T-943
═══ MANUAL ═══
Command center runs `cargo xtask platform wave push` on the next wave close and pastes the mode line.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
