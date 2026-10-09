# Ballistics catalog handlers

The HTTP handlers of the game ballistics catalogs: the administrator upload of a catalog with its
calibration bundle, and the public reads the mortar calculator and the offline service worker
fetch.

## Contents

```text
crates/api/api_operations/src/handlers/ballistics_catalogs/
├── mod.rs     the module tree
├── reads.rs   the catalog list and one stored version's document, with its ETag
└── upload.rs  the multipart upload: parts, limits, refusals and the upload report
```

## How it works

| Route | Tier | Answer |
|---|---|---|
| `GET /api/v1/ballistics-catalogs` | public | 200 `BallisticsCatalogList`, ordered by catalog then version |
| `GET /api/v1/ballistics-catalogs/{catalogId}/versions/{version}` | public | 200 the catalog document; 304 on a matching `If-None-Match`; 404 unknown |
| `POST /api/v1/ballistics-catalogs` | administrator | 201 `CatalogUploadReport`; 422 the report under `details`; 409; 400/413/415 |

- `upload.rs` reads the multipart parts `catalog` (at most 1 MiB) and `calibration` (at most
  16 MiB) under the route's own body limit `MAX_CATALOG_UPLOAD_BODY_BYTES` (both caps plus 64 KiB
  of framing); the caps hold the committed vanilla pair (10.6 kB and 6.4 MB) more than twice
  over. It decodes both, refuses a stored version or stored bytes with 409 before the calibration
  runs, runs the calibration on the blocking pool, and stores an accepted version with its audit
  line `ballistics_catalog.uploaded` in one transaction.
- Every refusal is the `{error, details}` envelope with a stable `details.code`:
  `request_too_large` (413), `missing_part` and `repeated_part` (400), `catalog_undecodable`,
  `calibration_undecodable` and `invalid_catalog_field` (400), `catalog_version_exists` and
  `catalog_bytes_exist` (409). A request that is not `multipart/form-data`, or a part declaring
  a content type other than JSON or octet-stream, answers 415. A calibration failure answers 422
  with the whole report, `accepted` false, as `details`.
- `reads.rs` answers a stored version with the strong ETag `"<catalog_sha256>"` and
  `Cache-Control: public, max-age=31536000, immutable`: a stored version never changes.

## Boundaries

- Depends on: `api_operations::services::ballistics_catalogs` for the judgement and the store;
  `api_operations::models::ballistics_catalog` for the summaries; `api_http_layer` for `AdminUser`,
  `api_foundation` for `PathParams` and `ApiError`.
- Used by: `crates/api/api_operations/src/routes.rs`; over HTTP, the mortar calculator in
  `crates/frontend/pages/field_tools_pages/src/mortar/` and the offline service worker; the
  test `crates/api/api_server/tests/game_ballistics_catalog_upload.rs`.
- Rules: the reads take no identity extractor; the upload takes `AdminUser`, so a lower caller is
  refused before the body is read; a refused upload stores nothing and writes no audit line.

## Related documentation

- [Ballistics catalog contract](/contracts/definitions/ballistics-catalog.schema.json) — the
  catalog document, the summaries and the upload report.
- [Vanilla mortar calibration bundle](/contracts/fixtures/ballistics/vanilla_mortars.v1/README.md)
  — the committed pair and its refused variants.
