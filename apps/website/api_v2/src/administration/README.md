# `administration/`

The member roster and the moderation actions taken against it: web-role assignment, Discord role
resync, bans, ban lifts, warnings, and the immutable audit log every one of those actions writes a
row to. Reading and searching the roster belongs here; the identity behind a member (sign-in,
tokens, the account row itself) belongs to `identity_and_access`.

## Public surface

- **`routes::routes()`** — the domain's `/api/v1` table, merged by `core::http_router::api_v1_routes`
  and nested under `/api/v1`. The literals in `routes.rs` are the public URLs:
  `/admin/users`, `/admin/users/{discordId}`, `/admin/users/{discordId}/ban`,
  `/admin/users/{discordId}/warnings`, `/admin/roles/sync`, `/admin/audit-logs`,
  `/admin/audit-logs/export.csv`, `/admin/audit-logs/stream`.
- **`services::audit_writer`** — `write_audit` (append one papertrail row) and
  `actor_display_name` (resolve the name a row is attributed to). Every domain that performs a
  privileged write calls it; it is the crate's single audit-append path.
- **`models::audit_log::AuditSeverity`** — the severity vocabulary those call sites pass.

## Dependency rules

- Handlers here never import another domain's handlers. Cross-domain reuse goes through a service
  or a model; `src/tests/architecture_rules.rs` enforces it.
- This domain imports `identity_and_access` (the account row and role enum) and `core`. Nothing in
  `core` imports it.
- Audit writing has exactly one home: `services/audit_writer.rs`. A domain that needs a papertrail
  row calls it rather than inserting its own.

## Files

```text
mod.rs                                 Domain module tree; re-exports `routes`.
routes.rs                              The `/api/v1` route table for administration.
handlers/
  mod.rs                               One module per administrative surface.
  audit_logs.rs                        The audit console: filtered list, CSV export, live SSE feed.
  disciplinary.rs                      Disciplinary actions against a member: bans, ban lifts, warnings.
  personnel_roster.rs                  Paginated, searchable projection of `users` with each member's standing.
  role_management.rs                   Setting a member's web role, and re-applying the Discord mapping.
  tests/
    audit_logs.rs                      Sibling unit tests for `audit_logs.rs`.
    personnel_roster.rs                Sibling unit tests for `personnel_roster.rs`.
services/
  mod.rs                               The audit-log writer and the Postgres notification listener.
  audit_notifier.rs                    Postgres `LISTEN audit_log`: the push behind the admin audit stream.
  audit_writer.rs                      Appends audit rows and resolves the actor display name.
  tests/
    audit_notifier.rs                  Sibling unit tests for `audit_notifier.rs`.
models/
  mod.rs                               Administration wire/database models.
  audit_log.rs                         The admin papertrail row and its severity ENUM.
  warning.rs                           The disciplinary warning record.
```

Unit tests live in the sibling files above, declared from the production file as
`#[cfg(test)] #[path = "tests/<file>.rs"] mod tests;`. Cross-cutting behaviour is covered by the
integration suites under the crate's `tests/` directory.
