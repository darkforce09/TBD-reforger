# Staging host actions

Every command the harness sends that changes the staging host, each paired with the read that
shows its state. The confirmed subcommands of `cargo xtask staging` and the procedures' host action
steps and recovery lists build their commands here.

## Contents

```text
tools/commands/staging_procedures/src/remote_actions/
├── database_backup.rs        verified `pg_dump -Fc` under `~/tbd/backups/<date>/`, named once verified
├── game_server_update.rs     steamcmd `app_update 1890870 validate`, refused while the fleet runs
├── host_fixture_commands.rs  every `staging-fixtures` subcommand, from the host checkout root
├── mod.rs                    the module tree
├── outage_dropin.rs          the API unit's `HTTPS_PROXY` drop-in: install, remove, state
├── relay_control.rs          the relay's `control` socket: arm, disarm, status
└── tests/                    unit tests for every action's command
```

## How it works

`staging-fixtures` runs as `cd <checkout> && ./target/release/staging-fixtures <subcommand> …
--confirm-database tbd_reforger --apply`, because its API env file defaults to a path under the
checkout root. `provision-fleet` passes the fleet size, the operator's Discord id, the host's IPv4
address, the fleet root and the game port base; `rotate-credential` passes the instance, the
executor, the fleet root and `--stage --actor <id>` or `--promote`. `spend_discord_member_bucket`
is the one builder of `spend-discord-member-bucket`: its start is a Unix time in milliseconds, or
the name of a variable the host script sets first (`SpendStart::HostVariable`), which the Discord
procedure's `bucket_spend` uses to start from the snapshot's `next_refresh_at` read on the host.

The backup script dumps to `<file>.partial`, checks it is non-empty and that `pg_restore --list`
reads it, then renames it and prints `backup: <path> (<n> bytes)`. The update script refuses while
any `tbd-reforger@*` unit is active, runs steamcmd into `TBD_SERVER_DIR`, and prints
`buildid: <id>` from the app manifest. The drop-in is
`~/.config/systemd/user/<api unit>.d/staging-discord-outage.conf` with
`HTTPS_PROXY=http://127.0.0.1:9`, installed or removed with a daemon reload and an API restart.
The relay is controlled through `$HOME/.local/bin/acknowledgement-dropping-relay control
--control-socket "$XDG_RUNTIME_DIR/acknowledgement-dropping-relay-<instance>/control.sock"`.

## Boundaries

- Depends on: `tools/commands/staging_procedures/src/remote_observers/remote_command.rs`; on the
  host, the `staging-fixtures` tool (`tools/staging/staging_fixtures/src/`), the relay
  (`tools/staging/acknowledgement_dropping_relay/`), docker, systemd and steamcmd.
- Used by: `tools/commands/staging_procedures/src/staging_dispatch.rs`, the procedures and
  `support_commands/status.rs`.
- Rules: no secret is an argument (the tool reads the bot token and writes credentials on the
  host); each builder states whether its command is a read or a change.
