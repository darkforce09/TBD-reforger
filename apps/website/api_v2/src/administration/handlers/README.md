# Administration Handlers (`administration/handlers/`)

HTTP endpoint handlers for personnel roster management, disciplinary actions, role updates, and administrative audit trails.

---

## 1. Handlers & Route Mappings

### `personnel_roster.rs` (<120 LOC)
- **`GET /api/v1/admin/users`** (`list_users`):
  - **Auth**: `AdminUser`
  - **Query**: `RosterQuery` (`q: Option<String>`, `limit: Option<i64>`, `offset: Option<i64>`).
  - **SQL**: Selects `users.*` with scalar subquery `(SELECT count(*) FROM warnings w WHERE w.discord_id = users.discord_id) AS warnings`.
  - **Search**: Multi-column `ILIKE` across `username`, `discord_handle`, `arma_id`, and `arma_character`.

### `disciplinary.rs` (<200 LOC)
- **`POST /api/v1/admin/users/{discordId}/ban`** (`ban_user`):
  - **Auth**: `AdminUser`
  - **Body**: `BanInput` (`reason: String`, trimmed, non-empty required).
  - **Action**: Updates `users.is_banned = true`, records ban actor, reason, timestamp.
  - **Token Invalidation**: Immediately revokes all unrevoked `refresh_tokens` for the user snowflake.
  - **Audit**: Appends `user.ban` at `AuditSeverity::Warn`.
- **`DELETE /api/v1/admin/users/{discordId}/ban`** (`unban_user`):
  - **Auth**: `AdminUser`
  - **Action**: Sets `users.is_banned = false`, clears ban metadata.
  - **Audit**: Appends `user.unban` at `AuditSeverity::Info`.
- **`POST /api/v1/admin/users/{discordId}/warnings`** (`issue_warning`):
  - **Auth**: `AdminUser`
  - **Body**: `WarnInput` (`reason: String`, non-whitespace required).
  - **Action**: Inserts into `warnings` table; writes `user.warn` audit log.

### `role_management.rs` (<100 LOC)
- **`PATCH /api/v1/admin/users/{discordId}`** (`update_user`):
  - **Auth**: `AdminUser`
  - **Body**: `UpdateUserInput` (`role: String`, validated against `UserRole` closed set).
  - **Action**: Updates `users.role`; writes `user.role_change` audit log.
- **`POST /api/v1/admin/roles/sync`** (`resync_roles`):
  - **Auth**: `AdminUser`
  - **Action**: Reconciles all members against stored `user_discord_roles` snapshots using `services::role_synchronizer`.

### `audit_logs.rs` (<270 LOC)
- **`GET /api/v1/admin/audit-logs`** (`list_audit_logs`):
  - **Auth**: `AdminUser`
  - **Query**: `AuditFilter` (`severity`, `q`, `before: Option<i64>`, `limit`).
  - **SQL**: Keyset pagination `WHERE id < $1 ORDER BY id DESC LIMIT $2`.
- **`GET /api/v1/admin/audit-logs/export.csv`** (`export_audit_logs_csv`):
  - **Auth**: `AdminUser`
  - **Action**: Streams CSV with formula sanitization (`escape_csv_formula`).
- **`GET /api/v1/admin/audit-logs/stream`** (`stream_audit_logs`):
  - **Auth**: `AdminUser` (SSE)
  - **Action**: Real-time SSE stream fed by Postgres `NOTIFY audit_log`.

---

## 2. Invariants & Security Controls

1. **Law 7 Compliance**: Every handler file is structured strictly under 300 LOC.
2. **Formula Injection Sanitization**: Any exported CSV cell beginning with `=`, `+`, `-`, or `@` is prepended with `'`.
3. **Fail-Closed Role Verification**: Arbitrary role strings return HTTP 400 Bad Request; unconfigured permissions fail closed.
