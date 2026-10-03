# Compiled mission rows

The rows of the compiled [mission](/documentation/glossary/g_to_m.md#mission) document the
[mod](/documentation/glossary/g_to_m.md#mod) loads, each written to a definition of
`contracts/definitions/mission.schema.json`.

## Contents

```text
crates/mission/mission_model/src/compiled/
├── entities.rs  compiled entity, vehicle, seat, slot, loadout, ORBAT group and radio net rows
├── mission.rs   compiled meta, factions, environment, flow, zones, win rule, settings, briefings
└── mod.rs       the module tree
```

## How it works

The `Mod*` rows are serialise-only and project the `$defs` of
`contracts/definitions/mission.schema.json` (many doc comments name the definition). Keys use the
schema's spelling (`unitName`, `leaderSlotId`, `type`, and `freqMHz`, which the mod binds by exact
name), and an optional key is left out when it is empty, so the document carries only what was
authored or derived. Every id field is a newtype of `crate::ids` that serialises as its bare
string. The whole document, `ModMissionDocument`, sits in the game-document compiler
(`mission_compiler`), which builds these rows.

## Boundaries

- Depends on: `serde`; `crate::ids` for the id fields and `crate::objectives::win_conditions` for
  `WinConditionParams`, the parameters of `ModWinConditions`.
- Used by: `mission_compiler`, which emits the rows; the API's `ModSlot` and the Mission
  Creator's and document store's zone tests name them here.
- Rules: a compiled slot carries exactly the keys `$defs/slot` allows
  (`a_compiled_slot_carries_exactly_these_keys` in
  `crates/mission/mission_compiler/src/game_document/tests/cases_2.rs`); a net's frequency is
  `freqMHz` on the wire, never `freqMhz` (`radio_plan_is_derived_from_the_orbat`).

## Related documentation

- [Mission schema](/contracts/definitions/mission.schema.json) — the compiled document's
  definitions these rows project.
