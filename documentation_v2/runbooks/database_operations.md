# Database Operations Runbook

This runbook outlines PostgreSQL database operations, schema migrations, backup verification gates, and automated disaster recovery drills.

---

## 1. Schema Migrations (SQLx)

Migrations reside in `apps/website/api_v2/migrations/` and follow strictly monotonic numeric prefixes:
`YYYYMMDDHHMMSS_name.sql`

- **Execution**: Migrations run automatically on Axum API startup via `sqlx::migrate!()`.
- **Integrity**: Each migration runs in a transaction. The `_sqlx_migrations` table records applied checksums.
- **Rules**: Migrations are append-only. Never edit an existing migration file that has already shipped to staging or production.

---

## 2. Production & Staging Backups (`cargo xtask deploy db backup`)

Backups are created using PostgreSQL custom archive format (`pg_dump -Fc`).

```bash
cargo xtask deploy db backup
```

### Safety & Verification Gate (Check 5):
`pg_restore --list` verifies only the archive header and table of contents, failing to detect mid-stream data corruption. Therefore, the backup command enforces **Check 5**:
- Restores the archive in `--data-only` mode into a temporary pipeline.
- Counts table rows and verifies zero CRC errors.
- Emits a checksum digest file alongside `<timestamp>_tbd_reforger.dump`.

---

## 3. Database Restore Safety (`cargo xtask deploy db restore`)

Restoring a dump requires explicit double-confirmation to prevent accidental overwrite of production databases:

```bash
cargo xtask deploy db restore <dump-file> --target <dbname> --confirm <dbname>
```

- Refuses to execute against the active production database unless `--confirm` matches verbatim.
- Terminating existing active connections before dropping/re-creating the target database.

---

## 4. Automated Backup Restore Drill (`cargo xtask deploy db backup-drill`)

Backups are mathematically useless unless proven restorable. The repository includes an automated restore drill harness:

```bash
cargo xtask deploy db backup-drill
```

### Drill Steps:
1. Locates the latest backup `.dump` file.
2. Creates an isolated scratch database: `tbd_drill_scratch_<uuid>`.
3. Restores the dump completely into the scratch database.
4. Executes validation queries:
   - Verifies `_sqlx_migrations` matches the codebase migrations.
   - Counts rows across critical tables (`users`, `missions`, `events`, `orbat_slots`).
5. Drops `tbd_drill_scratch_<uuid>` and reports success.
6. Automated via systemd timer: `tbd-website-backup-drill.timer` (runs weekly).
