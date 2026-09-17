# Database Operations (`xtask/src/commands/db`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Manages PostgreSQL database containers and integration test databases.

---

## Submodules

- **`up.rs`**: Starts local PostgreSQL Docker container on port 5434.
- **`down.rs`**: Stops container while preserving data volumes.
- **`seed.rs`**: Applies development SQL seeds (`seeds/*.sql`).
- **`test_it.rs`**: Spawns clean test databases and runs backend integration tests.
