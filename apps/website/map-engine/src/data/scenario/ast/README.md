# Mission data shapes

The [mission](/documentation_v2/glossary/g_to_m.md#mission) shapes on both sides of the compiler: the
editor payload it parses, and the rows of the compiled document the
[mod](/documentation_v2/glossary/g_to_m.md#mod) loads, each written to a definition of
`contracts_v2/definitions/mission.schema.json`.
The [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) templates the
[API](/documentation_v2/glossary/a_to_f.md#api) reads live here too.

## Contents

```text
apps/website/map-engine/src/data/scenario/ast/
├── authoring.rs  `EditorPayload` and its rows: the authored input the compiler parses
├── entities.rs   compiled entity, vehicle, seat, slot, loadout, ORBAT group and radio net rows
├── factions/     the ORBAT templates read from a saved payload, exposed as `orbat`
├── mod.rs        the module tree
└── scenario.rs   compiled meta, factions, environment, flow, zones, win rule, settings, briefings
```

## How it works

`authoring.rs` is the input side: `EditorPayload` with its `editor` graph of factions, squads and
[slots](/documentation_v2/glossary/n_to_z.md#slot), and its zones, entities, vehicles and settings,
visible only inside `data::scenario`. Every field has a default and unknown keys are dropped, so a
missing key never fails a parse. `environment` and the authored blocks (`winConditions`, `tasks`,
`radioPlan`, `weatherTimeline`, `audio`, `spawnModules`, `tacticalGraphics`) stay
`serde_json::Value`: stored payloads never change, so a wrongly typed value must not make a stored
version uncompilable. The compiler types them later, dropping a bad environment value and
reporting a refused block, which `crate::data::scenario::extensions` types.

`entities.rs` and `scenario.rs` are the output side: the `Mod*` rows, serialise-only, which
project the `$defs` of `contracts_v2/definitions/mission.schema.json` (many doc comments name the
definition). Keys use the schema's spelling (`unitName`, `leaderSlotId`, `type`, and `freqMHz`,
which the mod binds by exact name), and an optional key is left out when it is empty, so the
document carries only what was authored or derived. The whole document, `ModMissionDocument`,
sits in `crate::data::scenario::flatten`, which re-exports these rows.

## Public surface

- `factions`, exposed as `data::scenario::orbat`: `OrbatSquadTemplate`, `OrbatSlotTemplate`,
  `parse_orbat_template`, `derive_orbat_from_editor` and `validate_faction_join_key`, read by the
  [operations](/documentation_v2/glossary/n_to_z.md#operations) and
  [missions](/documentation_v2/glossary/g_to_m.md#missions) domains of the API.
- `entities` and `scenario`: the compiled rows, which code outside `data::scenario` names through
  `data::scenario::flatten` (the API's `ModSlot`, and `ModZoneShape` in the tests of the
  [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)).

## Boundaries

- Depends on: `serde` and `serde_json`; `crate::data::scenario::extensions` for `AUTHORED_BLOCKS`,
  which selects the authored blocks, and `WinConditionParams`, the parameters of
  `ModWinConditions`.
- Used by: `crate::data::scenario::flatten`, which parses `authoring` and emits the rows;
  `crate::data::scenario::compile`, which derives the export payload's ORBAT through `factions`;
  outside the crate, the users named under Public surface.
- Rules: a compiled slot carries exactly the keys `$defs/slot` allows
  (`a_compiled_slot_carries_exactly_these_keys` in
  `apps/website/map-engine/src/data/scenario/compiler/flatten/tests/cases_2.rs`); a wrongly typed
  identity key or authored block never fails the compile
  (`wrong_typed_identity_keys_still_compile`,
  `a_malformed_authored_block_falls_back_to_the_derivation_and_reports`); a net's frequency is
  `freqMHz` on the wire, never `freqMhz` (`radio_plan_is_derived_from_the_orbat`).

## Related documentation

- [Mission schema](/contracts_v2/definitions/mission.schema.json) — the compiled document's
  definitions these rows project.
- [Mission editor payload schema](/contracts_v2/definitions/mission-editor-payload.schema.json) —
  the save-time contract of the payload `authoring.rs` reads.
