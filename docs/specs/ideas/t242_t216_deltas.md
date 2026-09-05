# T-242 — Emit the T-216 slot deltas through flatten

Owner: command center. Frozen-scope ticket re-scoped 2026-09-05; proposed scope schema/mission.

## Claude Code prompt — T-242

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-242 && pwd && git branch --show-current   # must be slice/T-242
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/mission/flatten.rs:640-800 (drop table) and the slot writer, packages/tbd-schema/schema/mission.schema.json:330-450, docs/plans/t-242_plan.md
═══ PROBLEM ═══
Schema declares slot tag/callsign/rank/stance; flatten drops them with DIAG_DROP_SLOT_*.
═══ SHIPPED ═══
T-216 schema deltas (all landed), T-674 leaderSlotId (emission is T-674.1's — leave it).
═══ LANGUAGE GATE ═══
Rust + JSON schema. No scripts. generated/ is codegen output only.
═══ LOCKED ═══
- Goldens validate under schema-validate before and after.
- No hand edits under apps/website/api/src/contract/generated/.
═══ DO ═══
1. Golden with the four fields; paste the drop diagnostic (red on main). 2. Emit; retire the four DIAG rows.
3. schema-validate + schema-codegen. 4. Perturb (skip stance) → red → restore → touch → green. 5. Wave gate.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no leaderSlotId changes; no mod edits.
═══ VERIFY ═══
cargo test -p map-engine-core --all-features mission::flatten ; cargo xtask ci schema-validate ; cargo xtask ci schema-codegen ; cargo xtask platform wave gate --slice T-242
═══ MANUAL ═══
None.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
