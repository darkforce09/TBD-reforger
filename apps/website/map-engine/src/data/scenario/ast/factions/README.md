# ORBAT templates

The [ORBAT](/documentation_v2/glossary/n_to_z.md#orbat) as a saved
[mission](/documentation_v2/glossary/g_to_m.md#mission) payload states it: an ordered list of squads, each
with its ordered [slots](/documentation_v2/glossary/n_to_z.md#slot) (role, loadout summary, tag), read
from the payload's top-level `orbat` array or derived from its editor graph. The module is exposed
as `data::scenario::orbat`.

## Contents

```text
apps/website/map-engine/src/data/scenario/ast/factions/
├── mod.rs                  the module tree; re-exports the two templates and the three functions
├── orbat_slot_template.rs  the two templates, their parse and derivation, the faction key check
└── tests/                  unit tests for the precedence, the derivation order and the faction key
```

## How it works

`parse_orbat_template` returns the payload's top-level `orbat` array when it holds at least one
squad; otherwise it calls `derive_orbat_from_editor`, because a version saved from the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator) carries no `orbat` key. The
derivation walks `editor.factions` in array order, each faction's `squadIds`, and each squad's
slots sorted by `index`; a squad or slot id that resolves nothing is skipped. A slot's position in
its squad's list is its 1-based ORBAT number. Each slot's loadout is its loadout's `summary` when
set, otherwise the display names of its `primary` and `launcher` joined with `" + "` (the base
name of the resource path without `.et`), otherwise empty.

`validate_faction_join_key` refuses an empty or whitespace-only faction and one with leading or
trailing whitespace, and never trims: the [event](/documentation_v2/glossary/a_to_f.md#event) side
stores `faction` verbatim in `orbat_slots.faction`, which the
[API](/documentation_v2/glossary/a_to_f.md#api) matches byte for byte against the mission's
[armory](/documentation_v2/glossary/a_to_f.md#armory) rows.

## Boundaries

- Depends on: `serde` and `serde_json` only.
- Used by:
  - `crate::data::scenario::compile`, whose export payload embeds `derive_orbat_from_editor`'s
    result as `orbat`;
  - through the `orbat` alias, the API: `apps/website/api_v2/src/operations/services/mod.rs`
    re-exports the templates and `parse_orbat_template` for the event mission attachment
    (`apps/website/api_v2/src/operations/handlers/event_mission_attachment.rs`, which also calls
    `validate_faction_join_key`), the reservation restore in
    `apps/website/api_v2/src/operations/services/event_reservations/` and the
    [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) slot bindings in
    `apps/website/api_v2/src/missions/services/mission_deployments/`.
- Rules:
  - a non-empty top-level `orbat` wins, and a payload that does not decode derives from the editor
    graph or yields nothing (`legacy_orbat_wins`, `empty_payloads_yield_nothing` in
    `tests/cases_1.rs`); the derivation keeps faction, squad and `index` order and skips dangling
    ids (`derives_from_editor_sorted_by_index`, `skips_missing_refs`);
  - `faction` is required on the wire and refused, never trimmed, when blank or padded
    (`orbat_squad_faction_is_required_on_wire`, `faction_join_key_require_and_refuse`);
  - the derived loadout is read from the slot, never hard-coded empty
    (`derive_fills_loadout_from_summary`, `derive_fills_loadout_from_weapons`);
    `cargo xtask verify editor-orbat-coherency` fails on `loadout: String::new()` in
    `orbat_slot_template.rs`.
