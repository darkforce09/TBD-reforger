# T-257 — Undo scope misses loadouts, items, objectives, markers

Owner: command center. store.rs expand_scope (:372-375) covers four roots + vehicles; hydrate clears four more that
nothing undo-scopes.

## Claude Code prompt — T-257

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-257 && pwd && git branch --show-current   # must be slice/T-257
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/doc/store.rs:80-130 and :340-400 plus its tests module, docs/plans/t-257_plan.md
═══ PROBLEM ═══
loadouts, items, objectives, markers are cleared by hydrate but not in the UndoManager scope.
═══ SHIPPED ═══
T-180.2 vehicles undo scope — copy its shape; hydrate origin handling unchanged.
═══ LANGUAGE GATE ═══
Rust only. `cargo test -p map-engine-core --all-features`, never without the flag.
═══ LOCKED ═══
- One test per root; hydrate stays outside history.
- No new files; store.rs is not allowlisted for size — keep the addition small.
═══ DO ═══
1. Four round-trip tests first; paste the red on main. 2. Add the four expand_scope calls.
3. Perturb (drop one call) → red → restore → touch → green. 4. cargo xtask platform wave gate --slice T-257
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; touch only store.rs.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features doc::store ; cargo xtask platform wave gate --slice T-257
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
