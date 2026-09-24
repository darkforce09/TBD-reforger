# T-299 — Single-faction compile ships a phantom opfor

Owner: command center. Found by T-186's compiled→mod boot lane. Operator authorization 2026-09-04 covers the .c edit.

## Claude Code prompt — T-299

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-299 && pwd && git branch --show-current   # must be slice/T-299
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
crates/map-engine-core/src/mission/flatten.rs:2430-2500, mission.schema.json factions block, Backend/TBD_MissionValidator.c faction checks, docs/plans/t-299_plan.md
═══ PROBLEM ═══
A stub opfor is padded into every single-faction compile; it shows in briefing/ORBAT and nobody can join it.
═══ SHIPPED ═══
T-181.46 endOn hard-reject fix (unrelated, keep), T-186 boot lane (the proof harness).
═══ LANGUAGE GATE ═══
Rust + JSON schema + Enforce script. No scripts.
═══ LOCKED ═══
- Two-faction output byte-identical.
- generated/ regenerated via schema-codegen only.
═══ DO ═══
1. One-faction fixture; paste the padded stub. 2. Schema minItems 1 + golden. 3. Remove the pad. 4. Validator accepts one.
5. Perturb (restore pad) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits to briefing/ORBAT .c files (report found_not_fixed instead).
═══ VERIFY ═══
cargo test -p map-engine-core --all-features mission::flatten ; cargo xtask ci schema-validate ; cargo xtask ci schema-codegen ; cargo xtask mod compile ; cargo xtask platform wave gate --slice T-299
═══ MANUAL ═══
Human checklist: boot a one-faction mission; briefing and ORBAT show one side.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
