# T-242 — Plan

## Context
Re-scoped 2026-09-05: every T-216 schema delta is already declared (slot tag/callsign/rank/stance :411-439, group
leaderSlotId :312, vehicles :105). What remains is that flatten drops the four slot fields (DIAG_DROP_SLOT_*,
flatten.rs:699-702). leaderSlotId is T-674.1's.

## Approach
1. `crates/map-engine-core/src/mission/flatten.rs`: golden with all four slot fields authored → red (dropped).
2. Emit them in the slot writer; delete the four DIAG_DROP_SLOT_* rows and their diagnostic ids.
3. `packages/tbd-schema/schema/mission.schema.json`: only if a description says "dropped by flatten" — update the
   text; `cargo xtask ci schema-validate`, then `cargo xtask ci schema-codegen` (generated/ is never hand-edited).
4. Perturbation: skip `stance` → golden test red; restore, `touch`, green.
## Risks
- Mod readers for rank/stance may not exist; emitting is still correct (schema-declared) — report found_not_fixed
  for the reader side with the file that should read them.

## Verification
- `cargo test -p map-engine-core --all-features mission::flatten` · `cargo xtask ci schema-validate`
- `cargo xtask ci schema-codegen` · `cargo xtask platform wave gate --slice T-242`
