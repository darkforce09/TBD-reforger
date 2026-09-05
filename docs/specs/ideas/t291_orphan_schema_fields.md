# T-291 — Resolve five schema fields implemented on no surface

Owner: command center. Frozen-scope ticket; proposed scope engine/core. Operator authorization 2026-09-04 covers the .c readers.

## Claude Code prompt — T-291

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-291 && pwd && git branch --show-current   # must be slice/T-291
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/mission/flatten.rs:2080-2140, Gamemode/TBD_FrameworkManager.c (settings/weather parse), Spectator/TBD_SpectatorController.c, docs/plans/t-291_plan.md
═══ PROBLEM ═══
spectatorPolicy, nightVision, windDirDeg, color, radio, layers are declared and go nowhere.
═══ SHIPPED ═══
Spectator subsystem (7 files), T-181 respawn lineage (do not re-implement respawn here).
═══ LANGUAGE GATE ═══
Rust + Enforce script. No scripts.
═══ LOCKED ═══
- No schema edits (other tickets own mission.schema.json); the fields are already declared.
- Editor-only fields are documented, never silently dropped.
═══ DO ═══
1. Golden test with the three runtime fields; paste the red. 2. Emit. 3. Readers + policy enforcement; mod compile.
4. Ledger rows. 5. Perturb (re-trim) → red → restore → touch → green. 6. Wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits outside the three owned files.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features mission::flatten ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-291
═══ MANUAL ═══
Human checklist: policy own-side → a dead player cannot spectate the enemy; night vision toggle respected.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
