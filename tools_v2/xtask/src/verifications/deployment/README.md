# Deployment Verifications (`verifications/deployment`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Validates deployment infrastructure configurations and container composition files.

---

## Verifications

- **`staging_compose_path.rs`** (formerly `gate_t438.rs`): Verifies that remote staging deploy drivers point `docker compose -f` at `docker-compose.staging.yml`.
