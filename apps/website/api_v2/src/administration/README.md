# Administration Subsystem (`administration/`)

Personnel roster management, disciplinary actions, role hierarchy resolution, and immutable administrative audit logging.

---

## 1. Subsystem Topology & Responsibilities

The `administration/` domain decomposes the 1,175 LOC `admin.rs` monolith, externalizes RCON to `server_infrastructure/`, and encapsulates audit streaming:

```text
src/administration/
├── README.md                           <-- Domain documentation (this document)
├── routes.rs                           <-- /api/v1/admin sub-router (<70 LOC)
│
├── models/
│   ├── mod.rs
│   ├── audit.rs                        <-- AuditLog, AuditSeverity enum (<80 LOC)
│   └── user.rs                         <-- User, UserRole, DiscordRole, Warning (<115 LOC)
│
├── handlers/
│   ├── mod.rs
│   ├── personnel_roster.rs             <-- Member list, rank, total deployments, warnings (<120 LOC)
│   ├── disciplinary.rs                 <-- Ban, unban, warning issuance, token revocation (<200 LOC)
│   ├── role_management.rs              <-- Role assignment & resync triggers (<100 LOC)
│   └── audit_logs.rs                   <-- Keyset query, CSV export, live NOTIFY stream (<270 LOC)
│
├── services/
│   ├── audit_writer.rs                 <-- Asynchronous best-effort audit logger (<45 LOC)
│   ├── audit_notifier.rs               <-- Postgres LISTEN audit_log distributor (<290 LOC)
│   └── tests/audit_notifier.rs         <-- Sibling unit tests (<50 LOC)
│
└── tests/                              <-- Non-inline sibling unit tests
    ├── personnel_roster.rs
    ├── disciplinary.rs
    ├── role_management.rs
    └── audit_logs.rs
```

---

## 2. HTTP Route Catalog

| Verb | Path | Handler | Auth Extractor | Description |
|:---|:---|:---|:---|:---|
| `GET` | `/api/v1/admin/users` | `personnel_roster::list_users` | `AdminUser` | Filterable member roster with ranks, deployments, and warning totals. |
| `PATCH` | `/api/v1/admin/users/{discordId}` | `role_management::update_user` | `AdminUser` | Update user platform role; records audit log. |
| `POST` | `/api/v1/admin/users/{discordId}/ban` | `disciplinary::ban_user` | `AdminUser` | Ban user; immediately revokes all unexpired refresh tokens. |
| `DELETE`| `/api/v1/admin/users/{discordId}/ban` | `disciplinary::unban_user` | `AdminUser` | Lift ban; clears ban metadata. |
| `POST` | `/api/v1/admin/users/{discordId}/warnings`| `disciplinary::issue_warning` | `AdminUser` | Issue formal disciplinary warning. |
| `POST` | `/api/v1/admin/roles/sync` | `role_management::resync_roles` | `AdminUser` | Trigger full community role reconciliation against stored snapshots. |
| `GET` | `/api/v1/admin/audit-logs` | `audit_logs::list_audit_logs` | `AdminUser` | Keyset-paginated audit trail (`id < $1`). |
| `GET` | `/api/v1/admin/audit-logs/export.csv` | `audit_logs::export_audit_logs_csv` | `AdminUser` | Full CSV audit export with formula injection sanitization. |
| `GET` | `/api/v1/admin/audit-logs/stream` | `audit_logs::stream_audit_logs` | `AdminUser` (SSE)| Real-time audit log stream driven by Postgres `NOTIFY`. |

---

## 3. Key Invariants & Security Controls

### 3.1 Immediate Token Revocation on Ban
When a user is banned via `disciplinary::ban_user`, an atomic SQL statement immediately invalidates all active sessions:
```sql
UPDATE refresh_tokens SET revoked_at = now()
WHERE discord_id = $1 AND revoked_at IS NULL
```
This ensures banned users are disconnected immediately upon access token expiration rather than surviving until refresh token expiration.

### 3.2 CSV Formula Injection Prevention (`escape_csv_formula`)
When exporting audit logs to CSV (`audit_logs::export_audit_logs_csv`), any cell starting with `=`, `+`, `-`, or `@` is prepended with a single quote (`'`). This neutralizes formula injection attacks when audit files are opened in Microsoft Excel or Google Sheets.

### 3.3 Postgres `LISTEN/NOTIFY` Audit Notification Dispatcher
`audit_notifier.rs` pins a dedicated connection per `PgPool` running `LISTEN audit_log` triggered by database trigger `0025_audit_notify.sql`. If disconnected, it provides a linear backoff reconnect loop (250ms to 5s) while broadcasting a fallback signal so connected SSE clients gracefully degrade to 2s polling.
