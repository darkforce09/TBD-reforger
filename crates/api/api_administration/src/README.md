# API administration source

The [API](/documentation/glossary/a_to_f.md#api)'s
[administration](/documentation/glossary/a_to_f.md#administration) domain: the member roster and the
moderation taken against it (bans, ban lifts, warnings), the Discord
[role](/documentation/glossary/n_to_z.md#role) resync, the membership grace extension, and the audit
log that records every privileged action, with its live feed. The identity behind a member (sign-in,
tokens, the account row) belongs to `api_identity_and_access`.

## Contents

```text
crates/api/api_administration/src/
├── error.rs    `Error` and `Result`: a read or write failure, converting into `ApiError`
├── handlers/   the roster, discipline, role, grace and audit log handlers
├── lib.rs      the crate root; re-exports `routes`, `Error` and `Result`
├── models/     the audit log line with its severity, the audit stream events, the roster page, and the warning
├── prelude.rs  the audit services and the audit line a caller imports with `prelude::*`
├── routes.rs   the domain's `/api/v1` route table
└── services/   the audit notifier, the publication sequence and the live delivery stream
```

## How it works

A request reaches a handler through `routes.rs`; every route sits under `/api/v1/admin/*`, where only an
administrator may read at all. The handlers take `AdminUser`, apart from the grace extension, which
takes `AuthUser` and lets the identity service require verified administrator authority, so it
still works while Discord is unreachable. A member's role follows their Discord roles: the role edit
route refuses every change with 409, and the resync route re-applies the `discord_roles` mapping.

The roster answers one page at a time, `{items, page, per_page, total}`, in `lower(username)`
then `discord_id` order, so every member appears on exactly one page.

Privileged writes across the API leave a row in `audit_logs`, either inside their own
transaction (`crates/api/api_audit_log/src/required_audit.rs`) or best-effort (`crates/api/api_audit_log/src/audit_writer.rs`). Committed
rows get a durable publication number, and the audit stream delivers them in that order,
replaying from a client's `Last-Event-ID`. The stream opens with `event: ready`; a cursor it cannot
replay, beyond the tail or below the retained floor that migration 0057 keeps in
`audit_publication_state`, becomes `event: reset` and the stream continues from the tail. The audit
logs page connects, waits for `ready`, loads the history through the list route, merges the two by
audit id, and reloads the history on `reset`.

## Public surface

- `routes::routes()`: the table the API's router (`api_server::router`) merges under `/api/v1`, every route `AdminUser`
  unless marked:
  - `GET /api/v1/admin/users?q&page&per_page`: one page of the searchable roster,
    `{items, page, per_page, total}`.
  - `PATCH /api/v1/admin/users/{discordId}`: refuses a website role change (409).
  - `POST` and `DELETE /api/v1/admin/users/{discordId}/ban`: ban with a reason, lift the ban.
  - `POST /api/v1/admin/users/{discordId}/warnings`: issue a warning.
  - `POST /api/v1/admin/users/{discordId}/membership-grace`: `AuthUser`, with verified
    administrator authority checked in the service; extend cached permissions by 1 to 48 hours.
  - `POST /api/v1/admin/roles/sync`: re-apply the Discord role mapping.
  - `GET /api/v1/admin/audit-logs`: the filtered, keyset-paged audit list.
  - `GET /api/v1/admin/audit-logs/export.csv`: the CSV export.
  - `GET /api/v1/admin/audit-logs/stream`: the live [SSE](/documentation/glossary/n_to_z.md#sse)
    feed: `ready`, unnamed row events, and `reset`.
- `services::audit_publication::publish_audit_batch`, run by the audit publication worker.
- `services::audit_delivery::audit_delivery_stream`, the stream of `AuditStreamItem`s behind the
  live feed, and `handlers::audit_logs::audit_row_stream`, its rows alone.
- `models::audit_log::AuditLog`: the audit line the list, the export and the stream read.

## Boundaries

- Depends on: the API crates `api_state` (the application state), `api_foundation` (errors,
  pagination, wire formats), `api_http_layer` (extractors, the authorized SSE stream),
  `api_audit_log` (the audit log writers and `AuditSeverity`), `api_caller_identity` (`UserRole`
  and the account locks) and `api_member_activity` (the reservation re-evaluation queue); the domain
  crate `api_identity_and_access` (the role resync and the grace
  extension).
- Used by:
  - the API's router (`crates/api/api_server/src/router.rs`), which merges the route table, and the
    `audit_publication_worker` in `crates/api/api_background_workers/src/`;
  - the API's audit integration suites in `crates/api/api_server/tests/`, which drive the services directly;
  - over HTTP, the [personnel](/documentation/glossary/n_to_z.md#personnel) and
    [audit logs](/documentation/glossary/a_to_f.md#audit-logs) pages in
    `crates/frontend/pages/administration_pages/src/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges; every handler
  carries its `/// @route` tag; no Rust code outside `api_audit_log`
  inserts into `audit_logs`, which keeps one audit path for the API's code; the wire shapes follow
  `contracts/definitions/personnel-roster.schema.json` and
  `contracts/definitions/audit-log.schema.json`.

## Related documentation

- [API overview](/documentation/crates/api/api_server/api_overview.md) — every domain's routes.
- [Administration and community content](/documentation/crates/api/api_server/verification_evidence/administration_and_content.md)
  — the roster paging, the audit stream's replay, reset and recovery semantics.
- [Personnel roster page](/documentation/crates/frontend/pages/administration_pages/personnel/personnel_roster_page.md)
  — the roster, discipline and resync as administrators use them.
- [Audit logs page](/documentation/crates/frontend/pages/administration_pages/audit_logs/audit_logs_page.md)
  — the audit console that reads these routes.
