# Equipment data viewer contracts

The response schemas of the equipment data viewer's debug
[API](/documentation/glossary/a_to_f.md#api): one schema per kind of page the viewer reads,
from the dataset status to the resource cards. The API's models are generated from them, and the
frontend's DTO parity tests check the fixture corpus against the same shapes.

## Contents

```text
contracts/definitions/equipment-data-viewer/
├── dataset.schema.json            the dataset status and overview (`EquipmentDatasetStatus`)
├── field-inventory.schema.json    one page of the field inventory (`EquipmentFieldPage`)
├── relationships.schema.json      one page of resource relationships (`EquipmentRelationshipPage`)
├── resource-cards.schema.json     one page of resource cards and their source facts
├── resources.schema.json          one page of the resource list (`EquipmentResourcePage`)
└── source-inspection.schema.json  one page of the source inspector (`EquipmentSourcePage`)
```

## How it works

`cargo xtask ci schema-codegen` generates one Rust module per schema into
`crates/contracts/contract_schema_types/src/generated/community_content/equipment_data_viewer/`,
and the handlers in `apps/api/src/community_content/handlers/equipment_data_viewer/` answer the
`GET /api/v1/debug/equipment-data/…` routes with those types:

| Schema | Root type | Routes under `/api/v1/debug/equipment-data/` |
|---|---|---|
| `dataset.schema.json` | `EquipmentDatasetStatus` | `status`, `overview` |
| `resources.schema.json` | `EquipmentResourcePage` | `resources` |
| `relationships.schema.json` | `EquipmentRelationshipPage` | `relationships` |
| `field-inventory.schema.json` | `EquipmentFieldPage` | `fields` |
| `source-inspection.schema.json` | `EquipmentSourcePage` | `selection`, `containers`, `properties`, `values`, `documents` |
| `resource-cards.schema.json` | `EquipmentResourceCardPage` | `resource-cards` |

`download` answers with the dataset file itself and has no schema here. The frontend mirrors each
root type in `apps/frontend/src/foundation/transport/dto/equipment_data_viewer/`, and
`apps/frontend/src/foundation/transport/dto/tests/equipment_data_viewer_parity.rs` decodes every
`positive/` fixture of `contracts/fixtures/equipment-data-viewer/` into its DTO, claiming every
wire field, and refuses every `negative/` one.

## Format

- Encoding: UTF-8 JSON Schema (draft-07), one file per page kind, named in lowercase hyphenated
  words after the page with the `.schema.json` suffix; each `$id` is
  `https://tbd.local/contracts/equipment-data-viewer/<file>`.
- Schema: each file's root is the page type its `title` names; `resource-cards.schema.json` also
  defines `EquipmentResourceCard` and `EquipmentSourceFact`. Every root is closed
  (`"additionalProperties": false`), so an unknown key fails validation.
- Adding a file: add the schema here, register it in the schema list of
  `tools/commands/schema_tooling/src/generate/schema_types.rs`, run `cargo xtask ci schema-codegen`, add
  a `positive/` and a `negative/` fixture, and mirror the type in the frontend DTO module.

## Producers and consumers

- Producers: people; the schemas are written by hand alongside the viewer's handlers.
- Consumers:
  - `cargo xtask ci schema-codegen`, which generates the API models;
  - the API handlers and services under `apps/api/src/community_content/`, through
    the generated models;
  - `apps/api/tests/contract_parity_equipment_viewer.rs`, which imports a committed
    export, validates every route's answer against its schema and requires it to equal its
    golden, the `positive/` fixtures among them;
  - the frontend DTO parity tests named above, and the equipment data viewer bench in
    `apps/frontend/src/workspaces/debug/data_viewer/`, through the DTOs.

## Boundaries

- Depends on: the equipment dataset the API serves, which the Workbench equipment export
  produces (`contracts/definitions/equipment-vehicle-export.schema.json`).
- Used by: the generated API models, the API's contract parity suite, the frontend DTOs and their
  parity tests, and `contracts/fixtures/equipment-data-viewer/`.
- Rules: a schema change regenerates the API models in the same change and keeps the fixture
  corpus and the frontend DTOs in step; the file names are pinned in `schema_types.rs`, so a
  rename updates it too.
