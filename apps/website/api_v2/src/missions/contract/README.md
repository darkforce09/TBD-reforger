# Mission contract validation

The [missions](/documentation_v2/glossary.md#missions) domain's contract layer: JSON Schema
validation of every document the domain accepts or serves, against the schemas in
`contracts_v2/definitions/`, and the Rust types projected from those schemas.

## Contents

```text
apps/website/api_v2/src/missions/contract/
├── generated/             types generated from the registry, editor payload and faction schemas
├── loadout_projection.rs  `LoadoutExport`, the loadout-export document model, written by hand
├── mod.rs                 the module tree; re-exports the validators and the kit-alias table
├── schema_validators.rs   the embedded schemas and their `validate_*` entry points
├── tests/                 unit tests for the validators, the zone pass and the loadout round trips
└── zone_quantisation.rs   refuses at save a zone the compiled mission document would reject
```

## How it works

`schema_validators.rs` embeds five schemas at compile time and compiles each one once, on first
use, into a draft 2020-12 validator. Every entry point answers `Ok` with no findings for a valid
document, `Ok` with findings (the instance path, then the validator's message) for an invalid one,
and `Err(ContractError)` only when a schema itself fails to compile. The findings are for people:
callers show them and never match on their text.

| Entry point | Checks | Called by |
|---|---|---|
| `validate_mission_editor_payload_with_catalog` | `mission-editor-payload.schema.json`, control characters in authored strings, cargo over capacity, the zone pass | `missions::handlers::mission_versions`, on every payload stored |
| `validate_mission_document` | `mission.schema.json` and the 8 MiB `MISSION_FILE_MAX_BYTES` ceiling | [artifact](/documentation_v2/glossary.md#artifact) compilation in `missions::services::mission_artifacts` |
| `validate_faction_library_doc` | `faction-library.schema.json` | `missions::handlers::faction_library` |
| `validate_registry_items_envelope` | `registry-items.schema.json` | `missions::services::registry_import` |
| `validate_registry_compat_envelope` | `registry-compat.schema.json` | `missions::services::registry_import` |

`zone_quantisation.rs` closes the gap between saving and compiling a
[mission](/documentation_v2/glossary.md#mission). The compile rounds zone coordinates to 0.1 m, so
a circle of radius 0.04 m is valid as authored and invalid once compiled. The zone pass validates
the row the compile will emit against `#/$defs/zone`, lifted from the same embedded
`mission.schema.json` bytes the compiled document is checked against, so a saved zone is a zone the
compiled document accepts; a zone the compile drops is not refused.

`loadout_projection.rs` models `loadout-export.schema.json` by hand, tagged by `loadoutVersion`
1 or 2, because the generated form of its versioned root `oneOf` loses fields; its tests round-trip
both sample exports in `contracts_v2/fixtures/registry/`.

## Boundaries

- Depends on: the schemas in `contracts_v2/definitions/`, embedded with `include_str!`; the
  `jsonschema` crate; `website_map_engine::data::scenario` for the `wire_safety` scans, the cargo
  catalog type and the kit-alias table.
- Used by: `missions::handlers` (`mission_versions`, `faction_library`) and `missions::services`
  (`mission_artifacts`, `registry_import`, whose import decodes envelopes into the generated
  [registry](/documentation_v2/glossary.md#registry) types).
- Rules: `generated/` is written by `cargo xtask ci schema-codegen` and never edited by hand
  (`cargo xtask ci verify-codegen-fresh` checks it), and `loadout_projection.rs` stays outside the
  generator; `MISSION_FILE_MAX_BYTES`, the [mod](/documentation_v2/glossary.md#mod) mission
  loader's 8 MiB limit, equals the schema's `x-tbd-missionFileMaxBytes`
  (`schema_x_tbd_mission_file_max_bytes_matches_mod_constant` in `tests/schema_validators.rs`); the
  zone pass rounds exactly as the compile does
  (`zone_quantisation_mirrors_flatten` in `tests/zone_quantisation.rs`).
