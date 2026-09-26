# Kit and vehicle aliases

The table that turns an [Enfusion](/documentation_v2/glossary/a_to_f.md#enfusion) resource name into the
alias the [mod](/documentation_v2/glossary/g_to_m.md#mod) spawns: a placed character's `kit:` alias, a
placed vehicle's `veh:` alias, and each side's default kit and preset. The table is
`contracts_v2/rules/kit-aliases.json`, embedded at compile time; the module is exposed as
`data::scenario::kit`.

## Contents

```text
apps/website/map-engine/src/data/scenario/compiler/kit/
├── aliases.rs  `KitAliases` and `load_kit_aliases`: the embedded table, parsed once
├── mod.rs      the module tree; re-exports `KitAliases` and `load_kit_aliases`
└── tests/      unit tests for kit, vehicle and side-default resolution
```

## How it works

`load_kit_aliases` parses the embedded JSON on first use into a process-wide `OnceLock` and hands
out `&'static KitAliases`; the embedded copy is committed, so a parse failure is a build bug and
panics. `kit_for_resource` and `vehicle_for_resource` look up the full resource name, `{GUID}`
prefix included, and answer `None` for anything else. `faction_default` answers a lowercased side
key's `(kit, preset)` pair, the `fallbackFaction` side's pair for an unknown key, and two empty
strings when the table has neither.

## Boundaries

- Depends on: `contracts_v2/rules/kit-aliases.json` through `include_str!`; `serde` and
  `serde_json`.
- Used by: `crate::data::scenario::flatten`, which resolves every
  [slot](/documentation_v2/glossary/n_to_z.md#slot)'s kit, every vehicle's alias and every side's default
  kit and preset; the [API](/documentation_v2/glossary/a_to_f.md#api), which re-exports `KitAliases` and
  `load_kit_aliases` from `apps/website/api_v2/src/missions/contract/mod.rs`.
- Rules: a resource name matches only exactly, and an unknown vehicle resolves to nothing rather
  than to a substitute (`resolves_known_kits_and_faction_defaults`,
  `resolves_known_vehicles_and_refuses_unknown` in `tests/cases_1.rs`); every side the mod knows
  (`blufor`, `opfor`, `indfor`, `civ`) has its own default
  (`every_mod_faction_key_has_its_own_default`); the table's kits mirror the `kit:` entries of the
  mod's spawn registry, `apps/mod/tbd-framework/Data/registry.json` (`cargo xtask schema validate`).

## Related documentation

- [Contract rules](/contracts_v2/rules/README.md) — the kit-alias table and the other lookup
  tables beside it.
