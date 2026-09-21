# Deployment configuration and systemd unit templates

Configuration files and unit templates the `cargo xtask deploy` commands read, carry to a server,
or tell an operator to install. Nothing here is executable; every path is declared once in
`tools_v2/xtask/src/core/repository_layout.rs` and joined onto a checkout root by the command that
reads it.

## Files

| File | Who reads it |
|---|---|
| `deploy.env.example` | An operator copies it to `deploy.env` beside it and fills in the host, credentials and remote paths. |
| `deploy.env` | `cargo xtask deploy website`, `cargo xtask deploy staging`, `cargo xtask mod bootstrap-staging`, `cargo xtask mod remote-logs` and `cargo xtask debug direct-join`. Never committed: it holds credentials, and both rsync lanes exclude it so a development machine cannot overwrite the server's copy. |
| `Caddyfile.website` | Caddy on the server; `cargo xtask deploy website` prints the reload command for it, and `apps/website/api_v2/tests/forwarded_for_trust.rs` pins its loopback upstream. |
| `systemd/tbd-website-api.service` | `cargo xtask deploy website` restarts the installed unit by name and prints the one-time install command that renders this template for the remote directory. |
| `systemd/tbd-reforger.service` | `cargo xtask deploy staging` installs it as the dedicated game server's user unit. |
| `systemd/tbd-website-backup.service` | An operator installs it; it runs `cargo xtask deploy db backup`. |
| `systemd/tbd-website-backup.timer` | Triggers the backup nightly. |
| `systemd/tbd-website-backup-drill.service` | An operator installs it; it runs `cargo xtask deploy db drill`, which restores the backup on disk into a scratch database. |
| `systemd/tbd-website-backup-drill.timer` | Triggers the restore drill weekly. |

## Installing a unit

Each unit template spells its repository root as `/TBD_REPO_DIR_PLACEHOLDER`, an absolute path so
that the committed template passes `systemd-analyze verify --user` as it stands. Substitute it for
the real directory when installing; each file's own header carries the exact command.
