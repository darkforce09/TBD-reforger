# Command Center Domain Services (`command_center/services/`)

Internal asynchronous domain services supporting career statistics recalculations.

---

## 1. Services Catalog

### `user_stats.rs` (<135 LOC)
- **Primary Function**:
  ```rust
  pub async fn recompute_user_stats(pool: &PgPool, discord_id: &str) -> sqlx::Result<()>
  ```
- **Operations**:
  - Recomputes member's `total_deployments` from `event_registrations WHERE state = 'attended'`.
  - Recalculates `attendance_rate` as ratio of attended missions vs registered missions.
  - Updates `users.total_deployments` and `users.attendance_rate`.
  - Invokes best-effort asynchronous materialized view refresh.
- **Trigger Points**:
  - Post-match result ingestion (`match_telemetry::match_results`).
  - Arma account linking/unlinking (`identity_and_access::arma_linking`).

---

## 2. Invariants & Rules

1. **Non-Blocking Execution**: Statistics recomputation runs detached in a Tokio task so telemetry and linking requests return immediately.
2. **Deterministic Rates**: Attendance rate handles zero registered operations cleanly without division-by-zero errors (`COALESCE(..., 0.0)`).
