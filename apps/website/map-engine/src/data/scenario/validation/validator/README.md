# Mission validation rules

The rule engine that checks a [mission](/documentation_v2/glossary/g_to_m.md#mission) editor payload and
answers the findings the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s
validation panel shows: an ordered list of rules, each gated on the payload's shape and on the
facts the caller supplies. The module is exposed as `data::scenario::validate`.

## Contents

```text
apps/website/map-engine/src/data/scenario/validation/validator/
├── assets.rs    `ASSET-RESOLVES`: every placed slot, vehicle and entity asset is in the catalog
├── cargo.rs     the cargo rules: required equipment, the vehicle cargo ceiling, garment capacity
├── context.rs   `EvalContext`, `LoadoutPolicy`, `Finding`, `Severity` and `Primitive`
├── loadout.rs   the loadout rules: a uniform, a vest, the magazine floor
├── mod.rs       the module tree; re-exports the context types, the rule list and the entry points
├── orbat.rs     the ORBAT rules: slots resolve; squads are named, led, uniquely called, filled
├── registry.rs  `Rule` and `Registry`: ordered evaluation and the self-check on trip fixtures
├── rules.rs     `default_registry`, `validate_editor_payload` and the total payload readers
├── scenario.rs  rules V1 to V4: a spawnable slot, at most four sides, slots in bounds, the version
└── tests/       unit tests for every rule, the context gates and the self-check
```

## How it works

`default_registry()` builds the sixteen rules in a fixed order, and
`Registry::evaluate_with_context(payload, ctx)` runs each whose `applies` gate holds and returns
every finding of every rule, with no early exit. A rule carries an id, a `Severity` (`Error`,
`Warning` or `Info`), a `Primitive` (V1 required entity, V2 cardinality, V3 per-object invariant,
V4 field shape), its evaluator, and a trip fixture with an optional trip context that it must fire
on. `Registry::self_check` reports a rule whose gate excludes its own fixture or that stays silent
on it, and `Registry::new` panics on a duplicate id. A `Finding` names its rule, severity and
primitive, a message, the payload path it concerns (`/editor/slots/3/position`) and, when one
exists, the id of the object at fault, such as a [slot](/documentation_v2/glossary/n_to_z.md#slot)'s, so
the panel can select it.

`EvalContext` carries the facts a rule may need: `known_asset_ids`, `cargo_phys` (the
`wire_safety::CargoPhysCatalog`) and a `LoadoutPolicy` (`min_magazines`, `required_equipment`,
`max_vehicle_cargo_items`). A rule whose fact is absent stays inactive.

| Rule | Severity | Applies when | Fires on |
|---|---|---|---|
| `V1-PLAYER-SPAWN` | error | a faction is declared | a payload with no slot |
| `V2-FACTION-MAX` | warning | always | more than four factions |
| `V3-SLOT-IN-BOUNDS` | error | always | a slot outside its terrain's `terrain_bounds` |
| `V4-SCHEMA-VERSION` | error | always | a `schemaVersion` that is not a positive integer |
| `ORBAT-SLOT-RESOLVES` | error | a squad is declared | a slot with no role, or in no squad |
| `ORBAT-IDENTITY-FILLED` | warning | a squad is declared | a squad with neither callsign nor name |
| `ORBAT-SQUAD-HAS-LEADER` | warning | a squad is declared | a manned squad whose `leaderSlotId` names none of its slots |
| `ORBAT-CALLSIGN-UNIQUE` | warning | a squad is declared | two squads of one side with one callsign, ignoring case |
| `ORBAT-TEMPLATE-COVERAGE` | warning | a squad is declared | a role its `template.requiredRoles` lists and no slot fills |
| `ASSET-RESOLVES` | error | `known_asset_ids` is set | a placed asset missing from that set |
| `LOADOUT-HAS-UNIFORM` | warning | a slot has a loadout | a loadout with no `wear.jacket` |
| `LOADOUT-HAS-VEST` | warning | a slot has a loadout | a loadout with neither `wear.vest` nor `wear.armoredVest` |
| `LOADOUT-MAG-COUNT` | warning | a loadout, and `min_magazines` | a primary weapon with fewer magazines, loaded one included |
| `LOADOUT-HAS-EQUIPMENT` | warning | a loadout, and `required_equipment` | a loadout missing a required equipment kind |
| `VEHICLE-CARGO-POLICY` | warning | vehicles, and `max_vehicle_cargo_items` | a vehicle carrying more cargo items |
| `CARGO-OVER-CAPACITY` | error | a loadout, and `cargo_phys` | cargo over a garment's catalogued capacity |

## Boundaries

- Depends on: `crate::data::scenario::compile` (`terrain_bounds`) and
  `crate::data::scenario::wire_safety` (`CargoPhysCatalog` and `scan_cargo_capacity`);
  `serde_json`.
- Used by:
  - the Mission Creator's validation panel
    (`apps/website/frontend/src/v2/apps/editor/ui/inspector/validation_panel/`), which evaluates
    the compiled payload with `known_asset_ids` from the item
    [registry](/documentation_v2/glossary/n_to_z.md#registry) and no other fact, and renders every
    `Finding`, compile findings included;
  - `crate::data::scenario::flatten`, whose compile findings are `Finding` values, and
    `crate::editing::commands`, which summarises those findings for the compiled export;
  - the [API](/documentation_v2/glossary/a_to_f.md#api), which re-exports `Finding` and `Severity` from
    `apps/website/api_v2/src/missions/services/mission_compile.rs` and runs no rule.
- Rules:
  - every rule fires on its own trip fixture, and the ids are distinct
    (`engine_self_check_passes_for_the_seed_registry`,
    `every_seed_rule_has_a_distinct_id_and_a_known_primitive` in `tests/cases_1.rs`);
  - evaluation never stops at the first finding and never panics on a malformed payload
    (`evaluate_returns_all_findings_across_rules_never_early_exits`,
    `orbat_rules_never_panic_on_garbage`, `loadout_rules_never_panic_on_garbage`);
  - a context rule stays silent without its fact
    (`asset_resolves_skips_when_no_catalogue_is_supplied`,
    `cargo_over_capacity_skips_without_a_catalogue`), and the capacity rule
    agrees with the standalone scan (`cargo_over_capacity_agrees_with_the_standalone_scanner`).
