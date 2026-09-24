# Spawn modules

The check on a [mission](/documentation_v2/glossary.md#mission)'s authored `spawnModules` block:
the AI groups the game spawns once the round is live, either as a `wave` that restocks on an
interval or as a `garrison` that spawns once and holds. The module is exposed as
`data::scenario::spawn_modules`.

## Contents

```text
apps/website/map-engine/src/data/scenario/extensions/modules/
├── mod.rs     the module tree; re-exports the module row, the vocabularies, the cap and the checks
├── spawns.rs  `parse` and `validate` for the block; `KINDS`, `FACTION_KEYS`, `MAX_ALIVE`, placement
└── tests/     unit tests for the parse, each refusal and the block's way onto the payload root
```

## How it works

`parse` reads a non-empty array of modules as `contracts_v2/definitions/mission.schema.json`
shapes each one in `$defs/spawnModule`: `{id, kind, factionKey, groupTemplate, x and z or zoneId,
count, intervalSeconds?, maxAlive?, triggerId?}`. `kind` is one of `KINDS` (`wave`, `garrison`),
and `factionKey` one of `FACTION_KEYS` (`blufor`, `opfor`, `indfor`, `civ`), the four sides the
game's spawner maps to engine factions, in the order of `EngineFactionKey` in
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_SpawnManager.c`; the schema's
pattern alone would admit any lowercase key. Placement is exclusive (`placement_is_exclusive`): a
world `x` and `z` together or a `zoneId`, never both and never neither. `count` and `maxAlive` are
whole numbers from 1 to `MAX_ALIVE` (32), `intervalSeconds` is above zero, strings are non-empty,
ids are unique, and a key the schema does not declare refuses the block. Whether `zoneId` names an
authored zone and `triggerId` an authored trigger is not checked here. The first problem found is
the answer, a sentence with its path.

`validate` is `parse` with the value dropped: the check of the `spawnModules` row of
`AUTHORED_BLOCKS` in `crate::data::scenario::extensions`. The compile carries a valid block
verbatim to the compiled document's root, and in the game
`apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/TBD_DynamicSpawner.c` spawns the groups
under the same cap of 32.

## Boundaries

- Depends on: `serde_json`.
- Used by: `crate::data::scenario::extensions`, whose `spawnModules` row calls `validate`; the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s spawn modules panel
  (`apps/website/frontend/src/v2/apps/editor/ui/inspector/spawn_modules.rs`), which offers `KINDS`
  and `FACTION_KEYS`, bounds counts by `MAX_ALIVE` and checks each edit with `validate`, and whose
  tests use `placement_is_exclusive`.
- Rules: a module is placed by position or by zone, exactly one of them
  (`both_position_and_zone_are_refused`, `neither_position_nor_zone_is_refused` and
  `incomplete_position_is_refused` in `tests/cases_1.rs`); a count outside 1 to 32 is refused
  (`zero_and_over_cap_counts_are_refused`); the vocabularies and the cap are pinned
  (`spawn_modules_is_registered_on_the_carrier`), and `MAX_ALIVE` equals the
  [mod](/documentation_v2/glossary.md#mod) spawner's own `MAX_ALIVE`; a mission that authors no
  module compiles with no `spawnModules` key
  (`an_unauthored_payload_still_omits_the_spawn_modules_key`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — `spawnModules` and
  `$defs/spawnModule`, the shape this module checks.
