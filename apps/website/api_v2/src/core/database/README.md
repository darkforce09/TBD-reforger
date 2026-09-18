# Core Database (`core/database/`)

PostgreSQL connection pool management, migration execution, and transaction lifecycle abstractions.

---

## 1. Modules

### `connection_pool.rs` (<320 LOC)
- **Purpose**: Initializes and maintains the `sqlx::PgPool` instance.
- **Invariants**:
  - Sets connection timeouts (e.g., 5s acquire timeout), min/max pool sizes (5..50 connections), and idle connection reap intervals.
  - Exposes health probe functions (`ping_database`) used by liveness checks and metrics collectors.

### `migrations.rs` (<250 LOC)
- **Purpose**: Runs pending database schema migrations on application startup.
- **Invariants**:
  - Uses `sqlx::migrate!("./migrations")` to guarantee database schema consistency before binding HTTP listeners.

### `transaction.rs` (<300 LOC)
- **Purpose**: Ergonomic transaction helpers and advisory lock managers.
- **Key Functions**:
  - `run_in_transaction`: Executes a closure inside a `BEGIN ... COMMIT` block with automatic rollback on error.
  - `acquire_advisory_lock`: Obtains PostgreSQL application advisory locks (e.g., Gate G7b registration locks).
