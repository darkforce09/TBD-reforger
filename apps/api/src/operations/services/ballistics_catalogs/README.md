# Ballistics catalog services

The logic behind the game ballistics catalog routes: the judgement of an uploaded catalog and its
calibration bundle by the `ballistics_calibration` crate, and the immutable catalog store.

## Contents

```text
apps/api/src/operations/services/ballistics_catalogs/
├── catalog_store.rs      duplicate check, audited insert, list, document read, decoded catalog
├── mod.rs                the module tree
├── tests/                unit tests for the contract field checks and the upload report
└── upload_validation.rs  decode both parts, check catalog identity fields, run the calibration
```

## How it works

`upload_validation.rs` decodes the catalog with `BallisticsCatalog::from_json_slice` (pinned with
the SHA-256 of its bytes) and the bundle with `CalibrationBundle::from_json_slice`, then checks the
catalog's identity fields against the contract patterns the `ballistics_catalogs` table also
enforces: `catalog_id` a slug of at most 64 characters, `catalog_version` from 1 to the largest
`integer`, a title of 1 to 200 characters, a dotted `game_build` and a 16-digit uppercase
`export_generation_id`. `judge_upload` runs `calibration::evaluate` and wraps its report as the
`CatalogUploadReport`: `accepted` beside every field of the calibration report (`cases`,
`failures`, `forward_samples_not_judged`).

`catalog_store.rs` binds both documents as the uploaded text and lets Postgres cast them to
`jsonb`, so every number keeps its uploaded digits. The insert and its audit line
(`ballistics_catalog.uploaded`, target `ballistics_catalog` `<catalog_id>/<catalog_version>`)
share one transaction; a unique violation on either key is a `CatalogDuplicate`, as the pre-check
reports it. `load_catalog` decodes a stored version for a route that pins one.

## Boundaries

- Depends on: `ballistics_model` (`catalog`) and `ballistics_calibration`;
  `operations::models::ballistics_catalog`; `administration` for the audit row; `core` for the
  digest and the unique-violation check; the `ballistics_catalogs` table of migration 0060.
- Used by: `apps/api/src/operations/handlers/ballistics_catalogs/` and the fire-mission
  save, which pins a stored version.
- Rules: a version is stored only after its calibration lists no failure; the table's trigger
  refuses every update and delete, so a stored version never changes.

## Related documentation

- [Ballistics catalog handlers](/apps/api/src/operations/handlers/ballistics_catalogs/README.md)
  — the routes, limits and refusals.
