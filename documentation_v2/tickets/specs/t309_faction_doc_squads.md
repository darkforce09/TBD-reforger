# T-309 — FactionDoc squad level for Apply Template

Owner: command center. The fix T-217 could not make: a squad level in the faction library contract and both editor ops.

## Claude Code prompt — T-309

```text
═══ PREFLIGHT ═══
cd .ai/artifacts/worktrees/T-309 && pwd && git branch --show-current   # must be slice/T-309
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target
═══ READ ═══
packages/tbd-schema/schema/faction-library.schema.json, apps/website/frontend/src/core/dto.rs:730-800, state/operations/entity.rs:2330-2420 and :2900-2960, docs/plans/t-309_plan.md
═══ PROBLEM ═══
FactionDoc and its schema are flat; save-as-template then Apply cannot round-trip N squads (T-217 refuses).
═══ SHIPPED ═══
T-217 loud refusal; orbat_add_squad/orbat_add_slot/orbat_add_vehicle mutators — reuse them.
═══ LANGUAGE GATE ═══
Rust/Leptos + JSON schema. No scripts.
═══ LOCKED ═══
- Old flat templates keep validating and apply as one squad.
- Schema golden added; `cargo xtask ci schema-validate` green before any Rust change.
═══ DO ═══
1. Round-trip test first (three squads); paste the T-217 refusal as the red. 2. Schema + golden. 3. dto. 4. Both ops.
5. Perturb (skip squads emit) → red → restore → touch → green. 6. Gates.
═══ DO NOT ═══
No git add -A, no git stash, no ci-local; no edits to mission.schema.json or flatten.rs.
═══ VERIFY ═══
cargo xtask ci schema-validate ; cargo xtask mk ci-local-leptos ; cargo xtask mk leptos-gates ; cargo xtask platform wave gate --slice T-309
═══ MANUAL ═══
Operator: ORBAT with three squads → Save as template → new mission → Apply → three squads.
═══ RETURN ═══
Report schema per brief. Ready for Cursor doc sync.
```
