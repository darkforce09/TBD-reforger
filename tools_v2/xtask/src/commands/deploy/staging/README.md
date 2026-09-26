# Game server staging deploy

The implementation of `cargo xtask deploy staging`: it puts the checkout on the staging host, boots
the dedicated game server on the [mod](/documentation_v2/glossary/g_to_m.md#mod) it just synced, and
proves the boot from the server's own log instead of assuming it. The entry point, `Paths`, the
argument parser and the mode order live in `tools_v2/xtask/src/commands/deploy/staging.rs`, which
declares every module here.

## Contents

```text
tools_v2/xtask/src/commands/deploy/staging/
├── boot/          the addon GUID read, the three boot assertions, `--verify-boot` and its self-test
├── boot.rs        the `Out` sink that prints or captures; declares boot/ and re-exports its functions
├── config.rs      `Env`: deploy.env read as KEY=VALUE, required values, defaults and the settings check
├── host_agent.rs  the fleet host agent's settings, credential check, rcon block and install payload
├── payloads.rs    the remote `bash -s` scripts: profile and addon symlink, runtime smoke, unit install
├── pycompat.rs    the JSON and message behaviours of Python that the render's output reproduces
├── remote/        the deploy pipeline, the ssh and rsync argv, `ExecStart` and the remote boot wait
├── remote.rs      `SshBase` and `Runner`, the ssh transport that prints instead of spawning on dry runs
├── render.rs      the modpack source, `game.mods[]`, the server config render, its check, `--render-only`
└── tests/         unit tests for boot, config, the host agent, payloads, pycompat, remote and render
```

## How it works

`run` parses the flags left to right: an unknown option exits 2 at once, a value flag takes the
next argument whatever it is, and `--help` prints the usage. The two boot modes run before
`deploy.env` is read, so they need no staging credentials; everything else goes to
`remote::deploy`.

```text
staging::run
  ├─ --verify-boot-selftest ─▶ boot::selftest
  ├─ --verify-boot <log>    ─▶ boot::verify_boot_cli
  └─ remote::deploy
       ├─ config::Env::load (deploy.env beats the environment) and Env::validate
       ├─ --render-only <path> ─▶ render::render_only
       └─ rsync, profile, compose, smoke, server config, unit, boot verdict, host agent, log check
```

`Env::validate` refuses, before anything is sent: a `TBD_ADDON_GUID` that differs from
`apps/mod/tbd-framework/addon.gproj`; a `TBD_REMOTE_DIR` containing `prairielearn`; a runtime or
host-agent credential not shaped `tbdm_<32 hex>_<64 hex>`; a `TBD_SERVER_MODE` other than `config`
or `addons`. Config mode also needs a mod source (`TBD_WORKSHOP_MOD_ID`, `TBD_MODPACK_JSON` or
`TBD_MODPACK_URL`), an A2S port different from the game port, and admin ids that match the
engine's two patterns (a lowercase identityId or a 17-digit SteamID). The host agent needs config
mode, an RCON password of 3 to 256 bytes without whitespace, quotes or backslashes, a port, and an
API origin that is https or loopback http.

`render::render_server_config` builds `server.config.json` on the development machine from raw
values, then re-reads the file and validates it, so a value that breaks the JSON is caught before
the push. `game.mods[]` comes from `TBD_MODPACK_JSON` (the body of `GET /api/v1/modpacks/current`),
else from that route at `TBD_MODPACK_URL` with `TBD_MODPACK_TOKEN` as a bearer token, else from the
single `TBD_WORKSHOP_MOD_ID`; every source passes the same checks.

## Public surface

- `run` (in `tools_v2/xtask/src/commands/deploy/staging.rs`): the entry of `cargo xtask deploy
  staging`, called by `tools_v2/xtask/src/commands/deploy/dispatch.rs`. Every module here is
  private to it.

## Boundaries

- Depends on: `crate::core::repository_root` and `crate::core::repository_layout` (`DEPLOY_ENV`,
  `DEPLOY_ENV_EXAMPLE`); `verification_core::proc`; `serde_json` and `regex`;
  `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`, embedded by `host_agent.rs`; on the
  host, `cargo xtask setup server-profile`, `apps/website/docker-compose.staging.yml`, the
  `fleet-host-agent` crate and the dedicated server; and `cargo xtask mod remote-logs` for the last
  check.
- Used by: `tools_v2/xtask/src/commands/deploy/dispatch.rs`; people deploying the staging server.
- Rules: `deploy.env` is parsed, never executed (`source_no_longer_executes_the_env_file` in
  `tests/config/tests.rs`), and its values beat the process environment
  (`deploy_env_file_beats_the_process_environment`); the admin id patterns are the engine's
  (`admin_id_schema_is_the_engines`); an invalid rendered config fails before the push
  (`raw_substitution_can_emit_non_json_and_the_validator_catches_it` in `tests/render/tests.rs`); a
  modpack URL without a token fails before any network call
  (`modpack_url_without_a_token_fails_before_any_network_call`); the remote payloads are pinned
  byte for byte in `tests/payloads/tests.rs`.

## Related documentation

- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the staging
  runbooks' index: the host, the launch modes and the ports.
- [Staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md) — every
  `deploy.env` setting, the refusals, the deploy's stages and the host agent install.
- [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)
  — running `--verify-boot` and its self-test by hand, and what each assertion proves.
