# T-310 — Plan

## Context
Arsenal attachment picks persist in SlotLoadoutV2 (T-197) but never compile: mod_slot_loadout reads
weapon/optic/magazine only and the gear schema has no list. Same defect class T-193 removed controls for.

## Approach
1. Fixture loadout with a suppressor → compile on main → paste the gear block without it.
2. `packages/tbd-schema/schema/mission.schema.json` gear: `attachments` (array of wireSafeString, optional);
   golden; `schema-validate`; `schema-codegen`.
3. `crates/map-engine-core/src/mission/flatten.rs` mod_slot_loadout: read the edges, emit `attachments`.
4. `Gamemode/TBD_LoadoutEquipHelper.c`: after optic/magazine, TryInsert each attachment into
   SCR_WeaponAttachmentsStorageComponent; `[TBD][Equip] attach=<res> result=<ok|failed>`; `cargo xtask mod compile`.
5. Perturbation: skip the emit → golden red; restore, `touch`, green.
## Risks
- Attachment slot compatibility varies per weapon; a failed mount logs and continues (never blocks the spawn).

## Verification
- flatten tests · `schema-validate` · `schema-codegen` · `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-310`
- Human checklist: spawn with a suppressor picked; it is on the rifle.
