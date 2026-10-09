# Contract check modules

The submodules of `tools/commands/schema_tooling/src/schema_checks.rs`: one module per contract
gate that `cargo xtask schema` runs, and the helpers they share. The gates check the JSON Schemas
in `contracts/definitions/` and the documents that must follow them.

## Contents

```text
tools/commands/schema_tooling/src/schema_checks/
├── ballistics_validation.rs    the suite's ballistics section: catalog and calibration schemas, provenance, coverage
├── contract_validation/        the full validation suite and the one-file mission check
├── contract_validation.rs      declares the suite and the one-file check; re-exports both
├── kit_registry_references.rs  the `kit:` and `preset:` aliases a mission cites, and the registry's alias set
├── map_glyphs.rs               `schema map-glyphs`: glyph icon coverage, SVGs, render fields, the built atlas
├── mission_validation.rs       the suite's mission sections: goldens, unread wire fields, ceiling, kits, negatives
├── object_enumerations.rs      `schema map-object-enums`: every map-object class and kind is in the closed enums
├── object_type_inventory.rs    `schema type-inventory`: the inventory invariants and the instance-kind lockstep
├── read_json.rs                JSON reading, the contracts root, and the shared OK or FAIL verdict printer
├── registry_validation.rs      the suite's registry, items and compat sections with their reference walks
└── wire_field_readers.rs       the schemaVersion 1.3 wire fields and their pinned reader counts in the mod
```

## How it works

`schema_checks.rs` holds the constants several modules read (`IGNORE_DIRS`, the imported `INSTANCE_KINDS`,
`KNOWN_UNRESOLVABLE_KITS`) and wires the unit tests; each module takes the checkout root from
`repository_layout` and prints its own report.

| Gate | Checks |
|---|---|
| `schema validate` | the suite in `contract_validation/`, with the [mission](/documentation/glossary/g_to_m.md#mission), registry and ballistics sections here; the ballistics section prints `NOT RUN`, never `PASS`, while neither the catalog nor its calibration bundle is committed, and fails when only one is |
| `schema map-object-enums` | the golden prefabs, `contracts/rules/prefab-classify.json`, the Everon region sample and the glyph manifest keys use only the kinds and classes of `map-object-enums.schema.json` |
| `schema type-inventory` | the prefab catalogue's `INSTANCE_KINDS` matches the schema's kinds, then every committed type inventory passes its schema and invariants I1 to I5 and I7 (kind sums, class sums, closed class keys, a complete census, manifest counts) |
| `schema map-glyphs` | every icon key the golden prefabs and the committed Everon catalog use has a glyph, each glyph's SVG exists with a view box and sane render fields, and a built atlas, when present, matches the manifest |

Two pins inside the suite fail on purpose when the [mod](/documentation/glossary/g_to_m.md#mod) changes: `UNREAD_WIRE_FIELDS` requires each
schemaVersion 1.3 wire field to keep exactly its baseline count of identifiers in the mod's
comment- and string-stripped `.c` sources, so a new reader must update the schema's "no reader"
wording; and every `kit:` alias a golden mission cites must exist in
`mod/tbd-framework/Data/registry.json`, apart from the rows of `KNOWN_UNRESOLVABLE_KITS`. An
unresolved `preset:` alias is printed but does not fail, since no spawn path reads presets.

The gates print
`<gate>: OK` or `<gate>: FAIL (<n>)` and exit 0 or 1. An unreadable input aborts the gate with an
error, which exits 1.

## Public surface

- `validate_all`, `validate_file`, `map_object_enums`, `type_inventory` and
  `map_glyphs`, re-exported by `schema_checks.rs` and the crate root for `tools/xtask/src/commands/schema/dispatch.rs`
  and the `ci` task table.

## Boundaries

- Depends on: `repository_layout` (contract, definition, catalog, fixture, glyph
  and terrain paths) and `prefab_catalog::instance_kinds::INSTANCE_KINDS`; `jsonschema`;
  `regex`; `walkdir`; `serde_json`.
- Used by: `schema_checks.rs`, and through the crate root the `schema` command group and the `schema-validate`
  row of `tools/commands/ci_task_catalog/src/task_definitions.rs`.
- Rules:
  - `INSTANCE_KINDS` stays in lockstep with the schema, checked at run
    time and by `tools/commands/schema_tooling/src/tests/schema_checks/instance_kind_lockstep_tests.rs`.
  - A new unresolvable kit gets a `KNOWN_UNRESOLVABLE_KITS` row only with the reason the registry
    cannot gain the entry.
