# Equipment data viewer fixtures

Sample pages of the equipment data viewer's development-only debug
[API](/documentation_v2/glossary/a_to_f.md#api), one per response schema: pages each schema must
accept and pages it must refuse. The API reproduces every accepted page from a committed export,
and the frontend decodes each one into its DTO.

## Contents

```text
contracts_v2/fixtures/equipment-data-viewer/
├── negative/  one page per schema that its closed schema and the frontend DTO refuse
└── positive/  one page per schema, exactly as the API answers it for the committed export
```

## How it works

Both folders hold the same six file names, one per schema in
`contracts_v2/definitions/equipment-data-viewer/`. The API's
`apps/website/api_v2/tests/contract_parity_equipment_viewer.rs` imports the committed export under
`apps/website/api_v2/tests/fixtures/equipment_data_viewer/` through the production importer, boots
a development router and requires the answer of each route that has a sample here to equal the
`positive/` page, validate against its schema and decode into the generated type. The frontend's
`apps/website/frontend/src/v2/core/api/dto/tests/equipment_data_viewer_parity.rs` decodes every
`positive/` page into its DTO, claiming every wire field, and requires every `negative/` page to
fail decoding.

## Format

- Encoding: UTF-8 JSON, one page per file, named after its schema without the `.schema.json`
  suffix (`dataset.json` for `dataset.schema.json`).
- Schema: the root type of the matching schema in `contracts_v2/definitions/equipment-data-viewer/`.
- Adding a file: add the schema first, then a page in each folder under the schema's name; a
  `positive/` page is the API's answer for the committed export, so capture it from the live route
  and add its row to the golden list of `contract_parity_equipment_viewer.rs`.

## Producers and consumers

- Producers: `positive/` pages are captures of the API's answers for the committed export;
  `negative/` pages are written by hand.
- Consumers: the API's `contract_parity_equipment_viewer` test binary (`positive/` only) and the
  frontend DTO parity tests named above.

## Boundaries

- Depends on: the schemas in `contracts_v2/definitions/equipment-data-viewer/`, the committed
  export under `apps/website/api_v2/tests/fixtures/equipment_data_viewer/`, and the API's importer
  and query code that turn that export into pages.
- Used by: the API's contract parity suite and the frontend's DTO parity tests.
- Rules: a `positive/` page changes only with the API's answer or the committed export, never by
  hand to make a test pass (`contract_parity_equipment_viewer`); every page here has a schema and
  every schema a page in each folder.

## Related documentation

- [Equipment data viewer contracts](/contracts_v2/definitions/equipment-data-viewer/README.md) —
  the schemas these pages sample and the routes that answer them.
