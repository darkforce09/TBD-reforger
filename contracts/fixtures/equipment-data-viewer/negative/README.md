# Refused equipment data viewer pages

One page per equipment data viewer schema that the schema and the frontend's DTO must refuse.

## Contents

```text
contracts/fixtures/equipment-data-viewer/negative/
└── *.json  one refused page per schema, named after it
```

## How it works

Every page carries an `unexpected` key that no closed schema root admits. All but
`resource-cards.json` also lack the page's required fields; `resource-cards.json` is a complete
page with the extra key, so it fails on the unknown key alone.

## Format

- Encoding: UTF-8 JSON, one page per file.
- Schema: the root type of the schema of the same name in
  `contracts/definitions/equipment-data-viewer/`, broken on purpose.
- Adding a file: write a page the matching schema refuses, named after the schema, and add its
  refusal to `apps/website/frontend/src/v2/core/api/dto/tests/equipment_data_viewer_parity.rs`.

## Producers and consumers

- Producers: people.
- Consumers: `apps/website/frontend/src/v2/core/api/dto/tests/equipment_data_viewer_parity.rs`,
  which requires every page to fail decoding into its DTO.

## Boundaries

- Depends on: the closed roots of the schemas in `contracts/definitions/equipment-data-viewer/`.
- Used by: the frontend's DTO parity tests.
- Rules: a page that starts decoding means a DTO stopped refusing unknown or missing fields, and
  is never edited just to pass.
