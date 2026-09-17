# Deployment Configuration & Systemd Units (`xtask/deploy`)

Deployment configuration files, web server configs, and systemd service templates.

Relocated from the legacy root `scripts/deploy/` directory, resolving the misnomer (these are configuration files and unit templates, not executable scripts).

---

## Files

- **`deploy.env.example`**: Template for remote staging environment variables (`TBD_SSH_HOST`, `TBD_REMOTE_DIR`, `TBD_STAGE_USER`, etc.).
- **`Caddyfile.website`**: Production and staging Caddy reverse-proxy configuration.
- **`systemd/`**:
  - `tbd-reforger.service`: Systemd service unit for the dedicated game server.
  - `tbd-website-backup.service`: Scheduled PostgreSQL database backup service.
  - `tbd-website-backup.timer`: Systemd timer triggering daily database backups.
  - `tbd-website-backup-drill.service`: Automated disaster recovery restore drill service.
  - `tbd-website-backup-drill.timer`: Systemd timer triggering weekly restore drills.
