# Systemd user unit templates

The systemd user units of the home server and the game server: the website API, the dedicated game
server, the [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent), and the nightly
database backup with its weekly restore drill. Every unit runs as the deploy user from
`~/.config/systemd/user/`, never as root.

## Contents

```text
tools_v2/xtask/deploy/systemd/
├── fleet-host-agent.service           the fleet host agent of one game server, run from the user's ~/.local/bin
├── tbd-reforger.service               a reference unit for the dedicated game server in addons mode
├── tbd-website-api.service            the website API release binary, with its asset and upload paths pinned
├── tbd-website-backup-drill.service   one restore drill of the newest backup into a scratch database
├── tbd-website-backup-drill.timer     runs the restore drill every Sunday at 04:10
├── tbd-website-backup.service         one verified Postgres dump, with retention by count
└── tbd-website-backup.timer           runs the backup every night at 03:20
```

## How it works

A unit reaches the host in one of three ways. `cargo xtask deploy staging` installs the host agent
unit itself; `cargo xtask deploy website` restarts the API unit by name and prints its install
command; the operator installs the backup pair and the drill pair by hand. A template names its
repository root as `/TBD_REPO_DIR_PLACEHOLDER`, an absolute path, so the committed file passes
`systemd-analyze verify --user` as it stands; an install replaces it with the checkout's path on
the host. A unit copied without that step fails at once and names the placeholder in the journal.

```text
fleet-host-agent.service  ── include_str! in deploy/staging/host_agent.rs ──▶ written by deploy staging
tbd-website-api.service   ── sed by hand (deploy website prints the line) ──▶ restarted by deploy website
tbd-website-backup*.{service,timer} ── sed and cp by hand ──▶ timers run `deploy db backup|drill`
tbd-reforger.service      ── read by nothing; deploy staging writes its own unit ──▶ reference only
```

## Configuration

- `tbd-website-api.service`: `WorkingDirectory` is `apps/website/api_v2` of the checkout;
  `EnvironmentFile` is that folder's `.env`, the server's own secrets; `MAP_ASSETS_DIR` and
  `GLYPH_ASSETS_DIR` are pinned to the checkout's `assets_v2/terrains` and `assets_v2/glyphs`,
  because the API's fallback resolves them against its working directory and a wrong root answers
  404 without an error; `StateDirectory=tbd-website-api` and `UPLOAD_DIR=%S/tbd-website-api/uploads`
  keep uploads out of the rsynced checkout; `ExecStart` is `target/release/api`; it restarts on
  failure after 5 s.
- `tbd-reforger.service`: placeholders `TBD_SERVER_DIR_PLACEHOLDER`, `TBD_PROFILE_DIR_PLACEHOLDER`,
  `TBD_ADDONS_STAGING_PLACEHOLDER`, `TBD_ADDON_GUID_PLACEHOLDER` and `TBD_SCENARIO_PLACEHOLDER`,
  named after the `deploy.env` keys they stand for; it starts `ArmaReforgerServer` with
  `-addonsDir`, `-addons` and `-server`, bind port 2001 and A2S port 17777.
- `fleet-host-agent.service`: runs `%h/.local/bin/fleet-host-agent` with
  `%h/.config/fleet-host-agent/agent.toml`; restarts on failure after 5 s except on exit 78, an
  invalid configuration; gives the agent 200 s to stop.
- `tbd-website-backup.service`: runs `cargo run -q -p xtask -- deploy db backup` from the checkout
  with `TBD_BACKUP_KEEP=14` (dumps kept, by count), `TBD_BACKUP_DB=tbd_reforger`,
  `TBD_BACKUP_DIR=%h/tbd-backups/website` and `TBD_DB_CONTAINER=tbd_reforger_db`, read by
  `tools_v2/xtask/src/commands/deploy/database_backup.rs`; oneshot, no restart, 30-minute limit,
  umask 0077.
- `tbd-website-backup-drill.service`: runs `cargo run -q -p xtask -- deploy db drill` with the same
  database, folder and container, and `TBD_DRILL_DB=tbd_drill_probe`, a scratch name inside the
  restore guard's allow-list; it drills the dump already on disk and takes no new one.
- Timers: `OnCalendar=*-*-* 03:20:00` with up to 5 minutes of random delay for the backup,
  `OnCalendar=Sun *-*-* 04:10:00` with up to 10 minutes for the drill; both are `Persistent=true`,
  so a run missed while the host was off fires when it is back.

## Installed by

- `fleet-host-agent.service`: `cargo xtask deploy staging` with `TBD_INSTALL_HOST_AGENT=1` embeds
  the file at build time (`tools_v2/xtask/src/commands/deploy/staging/host_agent.rs`), writes it
  to `~/.config/systemd/user/fleet-host-agent.service` on the game host with the agent's
  `agent.toml`, credential and RCON password beside it, then enables and restarts it.
- `tbd-website-api.service`: installed by hand once, by replacing `TBD_REPO_DIR_PLACEHOLDER` with
  `TBD_REMOTE_DIR` minus its leading slash and writing the result to
  `~/.config/systemd/user/`; when the restart fails, `cargo xtask deploy website` prints that exact
  command (`install_command` in `tools_v2/xtask/src/commands/deploy/website/systemd_unit.rs`). Each
  deploy then runs `systemctl --user restart` on it, or on `TBD_WEBSITE_SYSTEMD_UNIT`.
- `tbd-website-backup.service` and `.timer`, `tbd-website-backup-drill.service` and `.timer`:
  installed by hand as each service file's header shows (substitute the placeholder with its
  leading slash, copy the timer, `systemctl --user enable --now` the timer), with
  `loginctl enable-linger` so the timers run while nobody is logged in.
- `tbd-reforger.service`: no command installs it. `cargo xtask deploy staging` writes its own
  `tbd-reforger.service` from `unit_payload` in `tools_v2/xtask/src/commands/deploy/staging/payloads.rs`,
  in the launch mode `TBD_SERVER_MODE` selects.

## Boundaries

- Depends on: systemd user instances with lingering on the host; the release `api` binary that
  `cargo xtask deploy website` builds; the `fleet-host-agent` binary that `deploy staging` builds
  from `apps/fleet_host_agent/`; the `deploy db backup` and `deploy db drill` commands; the
  `tbd_reforger_db` Postgres container.
- Used by: `cargo xtask deploy website` and `cargo xtask deploy staging` (through
  `SYSTEMD_UNITS_DIR` and `WEBSITE_API_UNIT` in `tools_v2/xtask/src/core/repository_layout.rs`, and
  the `include_str!` of the host agent unit); the fleet host agent, which restarts
  `tbd-reforger.service` by name.
- Rules: the repository placeholder keeps its leading slash, so the templates verify as they stand;
  the API unit's file name is the default unit `deploy website` restarts (`default_unit_name`);
  the API unit's `.env` stays on the host and is never rsynced.

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — installing the API unit,
  Caddy and the backup timers.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the game server
  unit and the host agent.
