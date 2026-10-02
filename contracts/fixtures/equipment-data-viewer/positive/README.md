# Accepted equipment data viewer pages

One page per equipment data viewer schema, exactly as the development-only debug routes answer it
for the export committed under `apps/api/tests/fixtures/equipment_data_viewer/`.

## Contents

```text
contracts/fixtures/equipment-data-viewer/positive/
└── *.json  one accepted page per schema, named after it
```

## How it works

| Page | Route under `/api/v1/debug/equipment-data/` | Query |
|---|---|---|
| `dataset.json` | `status` | `dataset=diagnostic` |
| `field-inventory.json` | `fields` | `dataset=diagnostic` |
| `relationships.json` | `relationships` | `dataset=diagnostic`, one resource, `view=all` |
| `resource-cards.json` | `resource-cards` | `dataset=diagnostic`, one resource |
| `resources.json` | `resources` | `dataset=diagnostic` |
| `source-inspection.json` | `properties` | `dataset=diagnostic`, one resource and component node |

`apps/api/tests/contract_parity_equipment_viewer.rs` holds the exact queries and
requires each live answer to equal its page as a JSON value, with nothing normalised.

## Format

- Encoding: UTF-8 JSON, pretty-printed, one page per file.
- Schema: the root type of the schema of the same name in
  `contracts/definitions/equipment-data-viewer/`.
- Adding a file: capture the route's answer for the committed export and add its row to the
  golden list of `contract_parity_equipment_viewer.rs`.

## Producers and consumers

- Producers: captures of the API's answers for the committed export.
- Consumers: the API's `contract_parity_equipment_viewer` test binary and the frontend's
  `apps/frontend/src/v2/core/api/dto/tests/equipment_data_viewer_parity.rs`.

## Boundaries

- Depends on: the committed export, the API's equipment data viewer importer and queries, and the
  schemas in `contracts/definitions/equipment-data-viewer/`.
- Used by: the API's contract parity suite and the frontend's DTO parity tests.
- Rules: a page equals the API's live answer (`contract_parity_equipment_viewer`); the frontend
  DTO claims every key it holds.
