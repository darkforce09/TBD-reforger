# Systemd user unit templates

The systemd user units of the home server and its game server fleet: the website API, the dedicated
game server of each fleet instance, the [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent)
of each instance, the acknowledgement-dropping relay in front of one agent, and the nightly
database backup with its weekly restore drill. Every unit runs as the deploy user from
`~/.config/systemd/user/`, never as root; a template unit (`name@.service`) runs once per instance
number, `name@N.service`.

## Contents

```text
tools/xtask/deploy/systemd/
├── acknowledgement-dropping-relay@.service  the loopback relay between one instance's host agent and the API
├── fleet-host-agent@.service          the host agent of fleet instance N, with that instance's agent.toml
├── tbd-reforger@.service              the dedicated game server of fleet instance N, from its own config and profile
├── tbd-website-api.service            the website API release binary, with its asset and runtime storage paths pinned
├── tbd-website-backup-drill.service   one restore drill of the newest backup into a scratch database
├── tbd-website-backup-drill.timer     runs the restore drill every Sunday at 04:10
├── tbd-website-backup.service         one verified Postgres dump, with retention by count
└── tbd-website-backup.timer           runs the backup every night at 03:20
```

## How it works

A unit reaches the host in one of three ways. `cargo xtask deploy staging` installs the three fleet
templates itself; `cargo xtask deploy website` restarts the API unit by name and prints its install
command; the operator installs the backup pair and the drill pair by hand. A template names its
repository root as `/TBD_REPO_DIR_PLACEHOLDER`, an absolute path, so the committed file passes
`systemd-analyze verify --user` as it stands; an install replaces it with the checkout's path on
the host. A unit copied without that step fails at once and names the placeholder in the journal.

```text
tbd-reforger@.service, fleet-host-agent@.service, acknowledgement-dropping-relay@.service
                          ── include_str! in deploy/staging/fleet_units.rs ──▶ written by deploy staging
tbd-website-api.service   ── sed by hand (deploy website prints the line) ──▶ restarted by deploy website
tbd-website-backup*.{service,timer} ── sed and cp by hand ──▶ timers run `deploy db backup|drill`
```

## Configuration

- `tbd-website-api.service`: `WorkingDirectory` is `apps/website/api_v2` of the checkout;
  `EnvironmentFile` is that folder's `.env`, the server's own secrets; `MAP_ASSETS_DIR` and
  `GLYPH_ASSETS_DIR` are pinned to the checkout's `assets/terrains` and `assets/glyphs`,
  because the API's fallback resolves them against its working directory and a wrong root answers
  404 without an error; `StateDirectory=tbd-website-api`, `UPLOAD_DIR=%S/tbd-website-api/uploads`
  and `EQUIPMENT_DATA_DIR=%S/tbd-website-api/equipment` keep uploads and the imported equipment
  data out of the rsynced checkout, the API creating `uploads/` at boot and `equipment/` on its
  first import, so neither needs a `mkdir`; the `.env` leaves all four directories out, because
  systemd lets an `EnvironmentFile` value override an `Environment=` line for the same variable
  and, outside development, a relative `UPLOAD_DIR` or `EQUIPMENT_DATA_DIR` stops the boot;
  `ExecStart` is `target/release/api`; it restarts on failure after 5 s.
- `tbd-reforger@.service`: placeholders `/TBD_SERVER_DIR_PLACEHOLDER` (the experimental server
  install, Steam app 1890870) and `/TBD_ADDONS_STAGING_PLACEHOLDER`, replaced by `TBD_SERVER_DIR`
  and `TBD_ADDONS_STAGING` without their leading slash; it starts `ArmaReforgerServer` with
  `-addonsDir`, `-config %h/tbd/fleet/instance-%i/server.config.json` and
  `-profile %h/tbd/fleet/instance-%i/profile`, never `-addons`; it restarts on failure after 10 s.
- `fleet-host-agent@.service`: runs `%h/.local/bin/fleet-host-agent` with
  `%h/.config/fleet-host-agent/instance-%i/agent.toml`; restarts on failure after 5 s except on
  exit 78, an invalid configuration; gives the agent 200 s to stop.
- `acknowledgement-dropping-relay@.service`: reads `RELAY_LISTEN` and `RELAY_UPSTREAM` from
  `%h/tbd/fleet/instance-%i/relay.env` and runs `%h/.local/bin/acknowledgement-dropping-relay serve`
  with its control socket in the mode-700 runtime folder
  `%t/acknowledgement-dropping-relay-%i/control.sock`; restarts on failure after 2 s.
- `tbd-website-backup.service`: runs `cargo run -q -p xtask -- deploy db backup` from the checkout
  with `TBD_BACKUP_KEEP=14` (dumps kept, by count), `TBD_BACKUP_DB=tbd_reforger`,
  `TBD_BACKUP_DIR=%h/tbd-backups/website` and `TBD_DB_CONTAINER=tbd_reforger_db`, read by
  `tools/xtask/src/commands/deploy/database_backup.rs`; oneshot, no restart, 30-minute limit,
  umask 0077.
- `tbd-website-backup-drill.service`: runs `cargo run -q -p xtask -- deploy db drill` with the same
  database, folder and container, and `TBD_DRILL_DB=tbd_drill_probe`, a scratch name inside the
  restore guard's allow-list; it drills the dump already on disk and takes no new one.
- Timers: `OnCalendar=*-*-* 03:20:00` with up to 5 minutes of random delay for the backup,
  `OnCalendar=Sun *-*-* 04:10:00` with up to 10 minutes for the drill; both are `Persistent=true`,
  so a run missed while the host was off fires when it is back.

## Installed by

- `tbd-reforger@.service`, `fleet-host-agent@.service` and `acknowledgement-dropping-relay@.service`:
  `cargo xtask deploy staging` embeds the three files at build time
  (`tools/xtask/src/commands/deploy/staging/fleet_units.rs`), writes them to
  `~/.config/systemd/user/` on the game host, enables and restarts `tbd-reforger@N` for every
  instance, the relay unit for the relay instance and `fleet-host-agent@N` for every instance, each
  agent with its `~/.config/fleet-host-agent/instance-N/agent.toml`, and disables and stops the
  instances above the fleet size. The fleet runs only these templates, and this folder holds no
  single-instance unit: a host that still has `tbd-reforger.service` or `fleet-host-agent.service`
  installed is refused until `cargo xtask deploy staging --migrate-single-instance` stops,
  disables and archives them under `~/tbd/retired/`.
- `tbd-website-api.service`: installed by hand once, by replacing `TBD_REPO_DIR_PLACEHOLDER` with
  `TBD_REMOTE_DIR` minus its leading slash and writing the result to
  `~/.config/systemd/user/`; when the restart fails, `cargo xtask deploy website` prints that exact
  command (`install_command` in `tools/xtask/src/commands/deploy/website/systemd_unit.rs`). Each
  deploy then runs `systemctl --user restart` on it, or on `TBD_WEBSITE_SYSTEMD_UNIT`.
- `tbd-website-backup.service` and `.timer`, `tbd-website-backup-drill.service` and `.timer`:
  installed by hand as each service file's header shows (substitute the placeholder with its
  leading slash, copy the timer, `systemctl --user enable --now` the timer), with
  `loginctl enable-linger` so the timers run while nobody is logged in.
## Boundaries

- Depends on: systemd user instances with lingering on the host; the release `api` binary that
  `cargo xtask deploy website` builds; the `fleet-host-agent` binary and the
  `acknowledgement-dropping-relay` executable that `deploy staging` builds from
  `apps/fleet_host_agent/` and `tools/developer_tools/`; the `deploy db backup` and `deploy db drill` commands; the
  `tbd_reforger_db` Postgres container.
- Used by: `cargo xtask deploy website` and `cargo xtask deploy staging` (through
  `SYSTEMD_UNITS_DIR` and `WEBSITE_API_UNIT` in `tools/xtask/src/core/repository_layout.rs`, and
  the `include_str!` of the three fleet templates); each fleet host agent, which restarts its
  instance's `tbd-reforger@N.service` by name.
- Rules: the repository placeholder keeps its leading slash, so the templates verify as they stand;
  the API unit's file name is the default unit `deploy website` restarts (`default_unit_name`);
  the API unit's `.env` stays on the host and is never rsynced; the template it is copied from,
  `apps/website/api_v2/.env.example`, sets none of the variables the API unit pins
  (`the_env_template_sets_none_of_the_variables_the_unit_pins` in
  `tools/xtask/src/commands/deploy/tests/website/tests.rs`).

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — installing the API unit
  and the backup timers; Caddy runs from the staging compose file, not from a unit here.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the game server
  unit and the host agent.
