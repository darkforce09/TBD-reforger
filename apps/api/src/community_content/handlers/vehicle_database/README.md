# Vehicle database handlers

The routes of the vehicle database, the identification table on the doctrine pages: the list and
the single row any signed-in member reads, and the create, replace, partial edit and removal an
administrator uses. The domain's `routes.rs` registers each handler; this folder holds the
handlers, the one validator the three writes share and the statements they run.

## Contents

```text
apps/api/src/community_content/handlers/vehicle_database/
├── create_and_replace.rs  `POST /api/v1/vehicle-database` (201) and `PUT /api/v1/vehicle-database/{id}`
├── delete.rs              `DELETE /api/v1/vehicle-database/{id}`: soft-delete a row, answer it
├── mod.rs                 the module tree and the handler and body re-exports
├── patch.rs               `PATCH /api/v1/vehicle-database/{id}`: change the fields the body names
├── reads.rs               `GET /api/v1/vehicle-database` and `GET /api/v1/vehicle-database/{id}`
├── tests/                 unit tests of the validator
├── validation.rs          the write bodies and the one validator of POST, PUT and PATCH
└── vehicle_rows.rs        every statement on `vehicle_databases`, the row lock and the audit line
```

## How it works

| Route | Method | Caller | Answer |
|---|---|---|---|
| `/api/v1/vehicle-database` | GET | `AuthUser` | 200 `{"data": [...]}`, live rows by `name`, then `id` |
| `/api/v1/vehicle-database` | POST | `AdminUser` | 201 with the new row |
| `/api/v1/vehicle-database/{id}` | GET | `AuthUser` | 200 with the row |
| `/api/v1/vehicle-database/{id}` | PUT, PATCH, DELETE | `AdminUser` | 200 with the stored row |

- **Validation.** `VehicleWriteBody` (POST, PUT) and `VehiclePatchBody` (PATCH) refuse unknown
  keys. Every value is trimmed before its rule applies: `name` (at most 120 characters), `faction`
  (60) and `armor_type` (60) are required and never blank; `amphibious` (60) and
  `primary_threat` (120) are optional; `profile_image_url` is empty or a safe image URL under
  `core::text::content_url_policy` (`https://…` or a site path `/…`). An empty optional value is
  stored as null. PUT replaces every field, so an absent optional field is cleared. In a PATCH an
  absent key leaves its field as stored, `null` clears an optional field, and `null` or a blank
  value on a required field answers 400. An unreadable body answers through
  `ApiError::from_json_rejection`: 413 with `details.code = request_too_large` over the request
  limit, 415 without a JSON content type, else 400 with the reason.
- **Writes.** Each accepted write runs in one transaction: PUT, PATCH and DELETE lock the live row
  `FOR UPDATE` (a missing or deleted row answers 404), then write it, then append the audit line
  through `administration::services::required_audit::append_actor_audit` (actions
  `vehicle.created`, `vehicle.replaced`, `vehicle.updated` with the changed keys, and
  `vehicle.deleted`; target type `vehicle`). The creator, editor and deleter stamps
  (`created_by`, `updated_by`, `deleted_by`) are the caller's Discord id.
- **Soft deletion.** DELETE sets `deleted_at` and `deleted_by` and keeps the row. A deleted row
  leaves the list and answers 404 on every route that names it; a malformed id answers 400.
- **Rows.** Each answer is a `VehicleDatabase`, which leaves empty optional columns off the wire;
  the lifecycle columns never reach it. Rows written before those columns existed keep them null.

## Boundaries

- Depends on: `models::VehicleDatabase` and `models::vehicle_database::VehicleDatabaseList` in the
  domain; `administration::services::required_audit` for the transactional audit line; `core` for
  the application state, the `AuthUser` and `AdminUser` extractors, `ApiError` and the content URL
  policy; the `vehicle_databases` table (lifecycle columns from migration 0058).
- Used by: the domain's `routes.rs`, which registers the re-exported handlers; over HTTP, the
  vehicle database page under `apps/frontend/src/pages/doctrine_and_info/vehicles/`.
- Rules: reads take `AuthUser` and writes `AdminUser`; every handler carries its `/// @route` tag
  (`cargo xtask verify route-tags`); every statement on `vehicle_databases` lives in
  `vehicle_rows.rs`, and every select list `COALESCE`s the optional columns
  (`apps/api/tests/null_tolerance_select_scan.rs`); the wire shapes are
  `contracts/definitions/vehicle-database.schema.json`.

## Related documentation

- [Administration and community content design](/documentation/apps/api/verification_evidence/administration_and_content.md)
  — the vehicle semantics these handlers implement.
- [Vehicle database page](/documentation/apps/frontend/pages/doctrine_and_info/vehicles/vehicle_database_page.md)
  — the page that reads and writes these routes.
- [API overview](/documentation/apps/api/api_overview.md) — every domain's routes.
