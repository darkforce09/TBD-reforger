# T-940.6 — Plan

## Context
audit.rs:161 polls every 2 s; no audit rows for event create, mission delete, slot kick. events.rs belongs to
.1/.2/.3/.12, so the rows come from a trigger plus the services layer.

## Approach
1. Verify on main: creating an event yields no audit row; paste the red.
2. `migrations/0025_audit_notify.sql`: trigger function on events insert, missions deleted_at, registrations kicked; pg_notify.
3. `services/audit_notify.rs` (new, in services/mod.rs): PgListener stream with reconnect backoff.
4. audit.rs streams from it; poll kept as fallback.
5. Perturbation: drop the mission-delete branch → test red; restore, touch, green.

## Risks
- Listener holds a dedicated connection: one per process, not per client.

## Verification
- `cargo xtask db test-it`
- `cargo xtask platform wave gate --slice T-940.6`
