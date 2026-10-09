# Spawn modules

The check on a [mission](/documentation/glossary/g_to_m.md#mission)'s authored `spawnModules` block:
the AI groups the game spawns once the round is live, either as a `wave` that restocks on an
interval or as a `garrison` that spawns once and holds. The module is `mission_model::spawn_modules`.

## Contents

```text
crates/mission/mission_model/src/spawn_modules/
├── mod.rs     the module tree; re-exports the module row, the vocabularies, the cap and the checks
├── spawns.rs  `parse` and `validate` for the block; `KINDS`, `FACTION_KEYS`, `MAX_ALIVE`, placement
└── tests/     unit tests for the parse, each refusal and the block's way onto the payload root
```

## How it works

`parse` reads a non-empty array of modules as `contracts/definitions/mission.schema.json`
shapes each one in `$defs/spawnModule`: `{id, kind, factionKey, groupTemplate, x and z or zoneId,
count, intervalSeconds?, maxAlive?, triggerId?}`. `kind` is one of `KINDS` (`wave`, `garrison`),
and `factionKey` one of `FACTION_KEYS` (`blufor`, `opfor`, `indfor`, `civ`), the four sides the
game's spawner maps to engine factions, in the order of `EngineFactionKey` in
`mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Slots/TBD_SlotBodyMaterializer.c`; the schema's
pattern alone would admit any lowercase key. Placement is exclusive (`placement_is_exclusive`): a
world `x` and `z` together or a `zoneId`, never both and never neither. `count` and `maxAlive` are
whole numbers from 1 to `MAX_ALIVE` (32), `intervalSeconds` is above zero, strings are non-empty,
ids are unique, and a key the schema does not declare refuses the block. Whether `zoneId` names an
authored zone and `triggerId` an authored trigger is not checked here. The first problem found is
the answer, a sentence with its path.

`validate` is `parse` with the value dropped: the check of the `spawnModules` row of
`AUTHORED_BLOCKS` in `crate::authored_blocks`. The compile carries a valid block
verbatim to the compiled document's root, and in the game
`mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/Dynamic/TBD_DynamicSpawner.c` spawns the groups
under the same cap of 32.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::authored_blocks`, whose `spawnModules` row calls `validate`; the
  [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s spawn modules panel
  (`crates/frontend/workspaces/mission_creator_workspace/src/ui/inspector/spawn_modules.rs`), which offers `KINDS`
  and `FACTION_KEYS`, bounds counts by `MAX_ALIVE` and checks each edit with `validate`, and whose
  tests use `placement_is_exclusive`.
- Rules: a module is placed by position or by zone, exactly one of them
  (`both_position_and_zone_are_refused` and `neither_position_nor_zone_is_refused` in
  `tests/cases_1.rs`); a count outside 1 to 32 is refused (`zero_and_over_cap_counts_are_refused`);
  `MAX_ALIVE` equals the
  [mod](/documentation/glossary/g_to_m.md#mod) spawner's own `MAX_ALIVE`; a mission that authors no
  module compiles with no `spawnModules` key
  (`an_unauthored_payload_omits_every_block_key` in
  `crates/mission/mission_payload/src/tests/extension_round_trips.rs`).

## Related documentation

- [Mission schema](/contracts/definitions/mission.schema.json) — `spawnModules` and
  `$defs/spawnModule`, the shape this module checks.
