# Administration Domain Services (`administration/services/`)

Internal asynchronous domain services supporting audit logging and real-time database change notifications.

---

## 1. Services Catalog

### `audit_writer.rs` (<45 LOC)
- **Primary Function**:
  ```rust
  pub async fn write_audit(
      pool: &PgPool,
      severity: AuditSeverity,
      actor_id: Option<&str>,
      actor_name: &str,
      action: &str,
      message: &str,
      target_type: &str,
      target_id: &str,
  )
  ```
- **Properties**:
  - Asynchronous and non-blocking.
  - Fail-safe / best-effort: Errors during audit row insertion are logged via `tracing::error!` but never bubble up to abort the calling administrative transaction.
  - Automatically triggers the `audit_log` Postgres notification event.

### `audit_notifier.rs` (<290 LOC)
- **Primary Function**:
  - Centralized dispatcher connecting to Postgres via `sqlx::postgres::PgListener`.
  - Listens on topic `audit_log` triggered by database trigger `0025_audit_notify.sql`.
  - Pins a single dedicated connection per `PgPool`.
  - Dispatches `AuditSignal` items over a Tokio broadcast channel (`BROADCAST_CAPACITY = 256`):
    - `AuditSignal::Row(i64)`: Carries the newly inserted sequence ID.
    - `AuditSignal::Resync`: Informs clients to perform a full reload.
    - `AuditSignal::Down`: Informs clients that the listener connection dropped.
- **Reconnect Loop**:
  - Exponential / linear backoff: 250ms doubling up to 5s.
  - Informs connected SSE clients of degradation so they can fall back to 2s polling.

---

## 2. Invariants & Rules

1. **Law 7 Compliance**: `audit_notifier.rs` keeps production code under 290 LOC; its 29 lines of unit tests are extracted to `administration/tests/audit_notifier.rs`.
2. **Resource Isolation**: Only one persistent connection is pinned per database pool for notifications, preventing connection exhaustion.
