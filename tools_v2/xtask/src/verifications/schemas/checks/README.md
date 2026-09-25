# Contract check modules

The submodules of `tools_v2/xtask/src/verifications/schemas/checks.rs`: one module per contract
gate that `cargo xtask schema` runs, and the helpers they share. The gates check the JSON Schemas
in `contracts_v2/definitions/`, the documents that must follow them, and the code that cites them.

## Contents

```text
tools_v2/xtask/src/verifications/schemas/checks/
├── contract_citations.rs       `schema citations`: every `@contract` tag in code resolves to a schema and pointer
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

`checks.rs` holds the constants several modules read (`CODE_EXTS`, `SCAN_ROOTS`, `INSTANCE_KINDS`,
`KNOWN_UNRESOLVABLE_KITS`) and wires the unit tests; each module takes the checkout root from
`crate::core::repository_root` and prints its own report.

| Gate | Checks |
|---|---|
| `schema citations` | walks `.c`, `.go`, `.js`, `.mjs`, `.rs`, `.ts` and `.tsx` files under `apps/` and `tools_v2/` (skipping `node_modules`, `dist`, `.git`, `build`, `coverage`, `vendor`) for `@contract <file>.schema.json#<pointer>`, and resolves each against `contracts_v2/definitions/`; a missing root or zero citations fails as an unexamined scan |
| `schema validate` | the suite in `contract_validation/`, with the [mission](/documentation_v2/glossary.md#mission) and registry sections here |
| `schema map-object-enums` | the golden prefabs, `contracts_v2/rules/prefab-classify.json`, the Everon region sample and the glyph manifest keys use only the kinds and classes of `map-object-enums.schema.json` |
| `schema type-inventory` | `INSTANCE_KINDS` matches the schema's kinds and the world-export pipeline's list, then every committed type inventory passes its schema and invariants I1 to I5 and I7 (kind sums, class sums, closed class keys, a complete census, manifest counts) |
| `schema map-glyphs` | every icon key the golden prefabs and the committed Everon catalog use has a glyph, each glyph's SVG exists with a view box and sane render fields, and a built atlas, when present, matches the manifest |

Two pins inside the suite fail on purpose when the [mod](/documentation_v2/glossary.md#mod) changes: `UNREAD_WIRE_FIELDS` requires each
schemaVersion 1.3 wire field to keep exactly its baseline count of identifiers in the mod's
comment- and string-stripped `.c` sources, so a new reader must update the schema's "no reader"
wording; and every `kit:` alias a golden mission cites must exist in
`apps/mod/tbd-framework/Data/registry.json`, apart from the rows of `KNOWN_UNRESOLVABLE_KITS`. An
unresolved `preset:` alias is printed but does not fail, since no spawn path reads presets.

`citations` exits 1 on a dangling citation or a scope failure; the other gates print
`<gate>: OK` or `<gate>: FAIL (<n>)` and exit 0 or 1. An unreadable input aborts the gate with an
error, which exits 1.

## Public surface

- `citations`, `validate_all`, `validate_file`, `map_object_enums`, `type_inventory` and
  `map_glyphs`, re-exported by `checks.rs` for `tools_v2/xtask/src/commands/schema/dispatch.rs`
  and the `ci` task table.

## Boundaries

- Depends on: `developer_tools::repository_layout` (contract, definition, catalog, fixture, glyph
  and terrain paths) and `developer_tools::world_export_pipeline::INSTANCE_KINDS`; `jsonschema`;
  `regex`; `walkdir`; `serde_json`.
- Used by: `checks.rs`, and through it the `schema` command group and the `schema-validate` and
  `verify-citations` rows of `tools_v2/xtask/src/commands/ci/task_definitions.rs`.
- Rules:
  - The citation scope is printed from `CODE_EXTS` and `SCAN_ROOTS`, and a scan that read nothing
    fails (`tools_v2/xtask/src/tests/citation_scope_tests.rs`).
  - `INSTANCE_KINDS` stays in lockstep with the schema and the export pipeline, checked at run
    time and by `tools_v2/xtask/src/verifications/schemas/tests/checks/instance_kind_lockstep_tests.rs`.
  - The unread-field gate must fire when a reader appears
    (`unread_gate_fires_when_a_reader_appears` in
    `tools_v2/xtask/src/verifications/schemas/tests/checks/unread_wire_field_tests.rs`).
  - A new unresolvable kit gets a `KNOWN_UNRESOLVABLE_KITS` row only with the reason the registry
    cannot gain the entry.
