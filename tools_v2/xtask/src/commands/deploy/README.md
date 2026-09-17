# Deployment Operations (`xtask/src/commands/deploy`)

> Planned architecture scaffold. Phase one keeps the live Rust module layout; this directory does not yet implement the structure described below.

Deployment orchestrators for remote staging and production environments.

---

## Submodules

- **`staging.rs`**: Full staging server deploy pipeline (`rsync`, systemd, compose).
- **`website.rs`** (formerly `gate_deploy_website.rs`): Deploys static frontend assets and REST API.
- **`db_backup.rs`**: Creates timestamped PostgreSQL dumps over SSH.
- **`db_restore.rs`**: Guarded database restore utility.
- **`db_drill.rs`**: Automated disaster-recovery backup and restore test drill.
