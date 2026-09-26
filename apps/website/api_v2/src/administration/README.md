# Administration domain

The [API](/documentation_v2/glossary/a_to_f.md#api)'s
[administration](/documentation_v2/glossary/a_to_f.md#administration) domain: the member roster and the
moderation taken against it (bans, ban lifts, warnings), the Discord
[role](/documentation_v2/glossary/n_to_z.md#role) resync, the membership grace extension, and the audit
log that records every privileged action, with its live feed. The identity behind a member (sign-in,
tokens, the account row) belongs to `identity_and_access`.

## Contents

```text
apps/website/api_v2/src/administration/
├── handlers/  the roster, discipline, role, grace and audit log handlers
├── mod.rs     the module tree; re-exports `routes`
├── models/    the audit log line with its severity, and the warning
├── routes.rs  the domain's `/api/v1` route table
└── services/  the audit writers, the publication sequence and the live delivery stream
```

## How it works

A request reaches a handler through `routes.rs`; every route sits under `/api/v1/admin/*`, where only an
administrator may read at all. The handlers take `AdminUser`, apart from the grace extension, which
takes `AuthUser` and lets the identity service require verified administrator authority, so it
still works while Discord is unreachable. A member's role follows their Discord roles: the role edit
route refuses every change with 409, and the resync route re-applies the `discord_roles` mapping.

Privileged writes across the crate leave a row in `audit_logs`, either inside their own
transaction (`services/required_audit.rs`) or best-effort (`services/audit_writer.rs`). Committed
rows get a durable publication number, and the audit stream delivers them in that order,
replaying from a client's `Last-Event-ID`.

## Public surface

- `routes::routes()`: the table `core::http_router` merges under `/api/v1`, every route `AdminUser`
  unless marked:
  - `GET /api/v1/admin/users`: the paginated, searchable roster.
  - `PATCH /api/v1/admin/users/{discordId}`: refuses a website role change (409).
  - `POST` and `DELETE /api/v1/admin/users/{discordId}/ban`: ban with a reason, lift the ban.
  - `POST /api/v1/admin/users/{discordId}/warnings`: issue a warning.
  - `POST /api/v1/admin/users/{discordId}/membership-grace`: `AuthUser`, with verified
    administrator authority checked in the service; extend cached permissions by 1 to 48 hours.
  - `POST /api/v1/admin/roles/sync`: re-apply the Discord role mapping.
  - `GET /api/v1/admin/audit-logs`: the filtered, keyset-paged audit list.
  - `GET /api/v1/admin/audit-logs/export.csv`: the CSV export.
  - `GET /api/v1/admin/audit-logs/stream`: the live [SSE](/documentation_v2/glossary/n_to_z.md#sse) feed.
- `services::required_audit`: `append_required_audit`, `append_actor_audit`,
  `append_actor_audit_with_severity` and `append_system_audit`, the transactional audit append
  every other domain writes through.
- `services::audit_writer`: `write_audit`, the best-effort append, and `actor_display_name`.
- `services::audit_publication::publish_audit_batch`, run by the audit publication worker.
- `models::audit_log::AuditSeverity`: the severity every audit call passes.

## Boundaries

- Depends on: `core` (the application state, errors, extractors, pagination, the authorized SSE
  stream, wire formats); `identity_and_access` (`UserRole`, the account locks, the role resync and
  the grace extension); the reservation re-evaluation queue in `operations::services`.
- Used by:
  - `core::http_router`, which merges the route table, and the `audit_publication_worker` in
    `apps/website/api_v2/src/background_workers/`;
  - every other domain, through the audit services and `AuditSeverity`;
  - over HTTP, the [personnel](/documentation_v2/glossary/n_to_z.md#personnel) and
    [audit logs](/documentation_v2/glossary/a_to_f.md#audit-logs) pages in
    `apps/website/frontend/src/v2/pages/administration/`.
- Rules: handlers never import another domain's handlers, and `routes.rs` exports the table the
  router merges (`apps/website/api_v2/src/tests/architecture_rules.rs` checks both); every handler
  carries its `/// @route` tag (`cargo xtask verify route-tags`); no Rust code outside `services/`
  inserts into `audit_logs`, which keeps one audit path for the crate's code.

## Related documentation

- [API overview](/documentation_v2/website/api_v2/api_overview.md) — every domain's routes.
- [Personnel roster page](/documentation_v2/website/frontend/pages/administration/personnel/personnel_roster_page.md)
  — the roster, discipline and resync as administrators use them.
- [Audit logs page](/documentation_v2/website/frontend/pages/administration/audit_logs/audit_logs_page.md)
  — the audit console that reads these routes.
