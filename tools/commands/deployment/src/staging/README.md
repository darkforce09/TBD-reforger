# Game server staging deploy

The implementation of `cargo xtask deploy staging`: it puts the checkout on the staging host, runs
the fleet of dedicated game servers on the [mod](/documentation/glossary/g_to_m.md#mod) it just
synced, one host agent per server, and proves each boot from the server's own log instead of
assuming it. The entry point, `Paths`, the argument parser and the mode order live in
`tools/commands/deployment/src/staging.rs`, which declares every module here.

## Contents

```text
tools/commands/deployment/src/staging/
├── acknowledgement_relay.rs             the relay install in front of the relay instance's host agent
├── boot/                                the addon GUID read, the three boot assertions, `--verify-boot` and its self-test
├── boot.rs                              the `Out` sink that prints or captures; declares boot/ and re-exports its functions
├── config.rs                            `Env`: the settings from deploy.env, their defaults, the retired settings, the settings check
├── fleet_instances.rs                   the API origin; the instances: port rules, names, visibility, folders, units, the relay settings
├── fleet_server_config.rs               one `server.config.json` per instance with password placeholders; `--render-only`
├── fleet_units.rs                       the three template units, their install and the game server restart
├── host_agent.rs                        each instance's `agent.toml` and the host agents' install
├── legacy_single_instance_migration.rs  `--migrate-single-instance` and the check that refuses without it
├── payloads.rs                          the secret file check, each instance's files and profile writer, its V2–V4 smoke
├── pycompat.rs                          the JSON and message behaviours of Python that the render's output reproduces
├── remote/                              the fleet pipeline, the ssh and rsync argv, the boot verdict per instance
├── remote.rs                            `Runner`, which drives the shared ssh transport and prints instead of spawning on dry runs
├── render.rs                            the modpack source, `game.mods[]` and the server config check
└── tests/                               unit tests of every module here
```

## How it works

`run` parses the flags left to right: an unknown option exits 2 at once, a value flag takes the
next argument whatever it is, and `--help` prints the usage. The two boot modes run before
`deploy.env` is read, so they need no staging settings; everything else goes to
`remote::deploy`.

```text
staging::run
  ├─ --verify-boot-selftest ─▶ boot::selftest
  ├─ --verify-boot <log>    ─▶ boot::verify_boot_cli
  └─ remote::deploy
       ├─ config::Env::load (retired settings refused) and Env::validate
       ├─ --render-only <directory> ─▶ fleet_server_config::render_only (instance-N/server.config.json)
       ├─ --dry-run ─▶ the plan of every step for every instance, no socket
       └─ website API check, secret files, single-instance check or --migrate-single-instance,
          rsync, per instance: files and smoke; units, restart, boot verdict per instance,
          relay, host agents, log check per instance
```

The fleet is `TBD_FLEET_INSTANCES` instances (default 5, at most 5). Instance N listens on game
port `TBD_FLEET_GAME_PORT_BASE + N` (2000), A2S port `TBD_FLEET_A2S_PORT_BASE + N` (17776) and a
loopback RCON port `TBD_FLEET_RCON_PORT_BASE + N` (19998); all must be distinct. It is named
`TBD Staging N`, only instance 1 is listed in the server browser, and its files live on the host
under `~/tbd/fleet/instance-N/`: `server.config.json`, `profile/`, and `secrets/` holding
`mod-runtime-credential`, `host-agent-credential` and `rcon-password`, folders mode 700 and files
mode 600. It runs as `tbd-reforger@N.service` from the experimental server install
(`TBD_SERVER_DIR`, Steam app 1890870) with the shared `-addonsDir`, and its host agent as
`fleet_host_agent@N.service` with `~/.config/fleet_host_agent/instance-N/agent.toml`. The agent of
`TBD_FLEET_RELAY_INSTANCE` polls `127.0.0.1:TBD_FLEET_RELAY_PORT`, where
`acknowledgement-dropping-relay@N.service` forwards to the API.

No secret is a setting: `Env::from_environment` refuses every key of `RETIRED_SETTINGS`, among
them the three credentials and passwords, and names what the fleet reads instead.
The credentials are written on the host by `cargo xtask staging provision-fleet`, the RCON
passwords are generated there by the deploy, and the join password is the operator's
`~/tbd/fleet/join-password`. The deploy proves all of them present, private and well-formed before
it changes anything, reads them into shell variables on the host, and fills them into the server
config there: a render on the development machine carries only the placeholders
`TBD_RCON_PASSWORD_FROM_HOST_FILE` and `TBD_JOIN_PASSWORD_FROM_HOST_FILE`.

`Env::validate` refuses, before anything is sent: a `TBD_ADDON_GUID` that differs from
`apps/mod/tbd-framework/addon.gproj`; a `TBD_REMOTE_DIR` containing `prairielearn`; ports that
break the fleet's port rules; a missing mod source (`TBD_WORKSHOP_MOD_ID`, `TBD_MODPACK_JSON` or
`TBD_MODPACK_URL`); admin ids outside the engine's two patterns. The settings load refuses an
agent origin that is neither https nor loopback http, and a relay whose upstream is not loopback.

`render::validate_server_config` re-reads every rendered file, so a value that breaks the JSON is
caught before the push. `game.mods[]` comes from `TBD_MODPACK_JSON` (the body of
`GET /api/v1/modpacks/current`), else from that route at `TBD_MODPACK_URL` with `TBD_MODPACK_TOKEN`
as a bearer token on curl's stdin, else from the single `TBD_WORKSHOP_MOD_ID`.

## Public surface

- `run` (in `tools/commands/deployment/src/staging.rs`): the entry of `cargo xtask deploy
  staging`, called by `tools/commands/deployment/src/deploy_dispatch.rs`. Every module here is
  private to it.

## Boundaries

- Depends on: `tool_test_support` and `deploy_settings` (the settings
  file, the deploy host and the remote folder defaults); `process_runner`; `serde_json` and
  `regex`; the three template units of `deploy/systemd/`, embedded by
  `fleet_units.rs`; on the host, the website API that `cargo xtask deploy website` runs there, the
  credential files `cargo xtask staging provision-fleet` writes, `cargo xtask setup server-profile`,
  the `fleet_host_agent` crate, the `acknowledgement-dropping-relay` executable of
  `tools/developer_tools` and the dedicated server; and `cargo xtask mod remote-logs --file` for
  the last check.
- Used by: `tools/commands/deployment/src/deploy_dispatch.rs`; people deploying the staging fleet;
  `cargo xtask staging` (`tools/commands/staging_procedures/src/staging_settings.rs`) and the
  instance-aware debug commands (`tools/commands/remote_debugging/src/debug/staging_fleet_instance.rs`),
  which address the fleet, its instance folders and units and the API origin (`backend_url`)
  through `fleet_instances.rs`; and the fleet procedure's `mod_runtime`
  credential promotion
  (`tools/commands/staging_procedures/src/fleet_procedure/waves/credential_waves.rs`), which
  rewrites an instance's profile with `payloads.rs`' `instance_profile_commands`.
- Rules: `deploy.env` is parsed, never executed, and a command line in it stops the deploy
  (`a_command_line_in_the_deploy_file_is_refused_and_never_run` in `tests/config/tests.rs`); its
  values beat the process environment (`deploy_env_file_beats_the_process_environment`); a retired
  setting is refused without printing its value
  (`a_retired_setting_is_refused_with_its_replacement_and_without_its_value`); the port rules hold
  (`five_instances_take_their_ports_from_the_three_bases`,
  `a_shared_or_overflowing_port_breaks_the_rules` in `tests/fleet_instances/tests.rs`); a render
  for five instances carries its own ports, visibility and placeholders
  (`render_only_writes_every_instance_of_the_five` in `tests/fleet_server_config/tests.rs`); the
  unit texts are pinned (`tests/fleet_units/tests.rs`); the migration's argv and effect are pinned,
  its effect under a local bash (`tests/legacy_single_instance_migration/tests.rs`); the secret file
  check fails closed and prints no secret under a local bash
  (`the_secret_file_check_runs_and_never_prints_a_secret` in `tests/payloads/tests.rs`); an
  instance's `TBD_BackendConfig.json` has one writer, `instance_profile_commands`, which the
  instance files carry (`instance_files_are_written_from_host_secrets_and_never_carry_one`); its
  `backendUrl` is `TBD_BACKEND_URL`'s one reading, without a trailing `/`
  (`the_backend_url_has_one_reading_without_a_trailing_slash` in `tests/fleet_instances/tests.rs`,
  `the_backend_url_is_the_one_reading_without_a_trailing_slash` in `tests/config/tests.rs`); no argv
  and no dry-run line holds a secret
  (`the_ssh_password_is_in_no_argv_and_only_in_the_child_environment`,
  `the_dry_run_plan_walks_every_instance_and_prints_no_secret` in `tests/remote/tests.rs`); the
  boot of the previous run never passes for the new one
  (`the_log_folder_newest_before_the_restart_never_counts` in
  `tests/remote/instance_boot_verdict_tests.rs`); the pipeline runs no compose command
  (`cargo xtask verify staging-compose-paths`).

## Related documentation

- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the staging
  runbooks' index: the host, the fleet and the ports.
- [Staging deploy](/documentation/runbooks/game_server_staging/staging_deploy.md) — every
  `deploy.env` setting, the refusals, the deploy's stages and the host agent install.
- [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md)
  — running `--verify-boot` and its self-test by hand, and what each assertion proves.
