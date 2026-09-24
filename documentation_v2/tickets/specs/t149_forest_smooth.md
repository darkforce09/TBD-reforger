# T-149 — Forest mass polygon smoothing

Owner: command center. Frozen-scope ticket; proposed scope repo/tools.

## Claude Code prompt — T-149

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-149 && pwd && git branch --show-current   # must be slice/T-149
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
tools/tbd-tools/src/world/build.rs:560-620 and :900-960, tools/tbd-tools/src/density.rs:1-80, docs/plans/t-149_plan.md
═══ PROBLEM ═══
Forest hulls are unsmoothed marching-squares rings; edges are stair-stepped.
═══ SHIPPED ═══
TBDD density (T-935.5 cast_slice decode), forest-regions emit, forest_mass.rs reader — formats stay unchanged.
═══ LANGUAGE GATE ═══
Rust only; edition-2024 rustfmt for tools/*.
═══ LOCKED ═══
- No TBDD or forest-regions.json.gz schema change; only ring geometry changes.
- Area drift per region < 3%, rings under 6 vertices untouched.
═══ DO ═══
1. Paste a blocky ring from the everon fixture (defect on main). 2. forest_smooth.rs + tests. 3. Wire the emit.
4. Perturb (iters = 0) → red → restore → touch → green. 5. Golden gate + wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no frontend edits; do not commit a rebuilt catalogue unless asked.
═══ VERIFY ═══
cargo test -p tbd-tools --lib world::forest_smooth ; cargo xtask ci verify map-object-golden ; cargo xtask platform wave gate --slice T-149
═══ MANUAL ═══
Operator: compare a forest edge before/after in the editor at 1:5000.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
