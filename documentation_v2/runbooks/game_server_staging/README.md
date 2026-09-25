**Status:** live

# Game server staging

How to run the Arma Reforger dedicated server of the staging host on the checkout: preparing the
host once, issuing its [machine credentials](/documentation_v2/glossary.md#machine-credential),
deploying with `cargo xtask deploy staging`, proving the boot from the server's own log, and
joining it from a client. Developers and agents working on the [mod](/documentation_v2/glossary.md#mod)
or on the platform's [game runtime](/documentation_v2/glossary.md#game-runtime) routes read it.

## Contents

```text
documentation_v2/runbooks/game_server_staging/
├── boot_and_log_verification.md                    the boot verdict, `mod remote-logs` and the log lines
├── client_join_and_mod_updates.md                  Direct Join, launch flags, ports, script changes
├── host_preparation.md                             one-time host setup: deploy.env, server, API, firewall
├── machine_credentials_and_mission_deployment.md   credentials, event binding, mission deployment
└── staging_deploy.md                               `deploy staging`: settings, stages, host agent
```

## How it works

The staging host is the machine `TBD_SSH_HOST` names in `tools_v2/xtask/deploy/deploy.env`. It runs
the platform's API and Postgres and one dedicated game server; the server's mod asks the API which
[mission deployment](/documentation_v2/glossary.md#mission-deployment) it runs and fetches that
[mission](/documentation_v2/glossary.md#mission)'s [artifact](/documentation_v2/glossary.md#artifact).

```text
development machine                         staging host (TBD_SSH_HOST)
───────────────────                         ───────────────────────────
cargo xtask deploy staging ── rsync ──────▶ TBD_REMOTE_DIR (the checkout)
                           ── ssh ────────▶ profile, addon link, server config, tbd-reforger.service
cargo xtask mod remote-logs ◀── console.log  │
                                            ▼
                             ArmaReforgerServer ── game UDP 2001, A2S UDP 17777
                               │ -addonsDir TBD_ADDONS_STAGING (links the checkout's tbd-framework)
                               │ -config server.config.json (game.mods[], game.admins[])
                               └── machineCredential ──▶ API 127.0.0.1:8080 ──▶ Postgres 127.0.0.1:5432
                             fleet-host-agent (optional) ─ RCON 127.0.0.1:19999, polls the API
Arma client ── Direct Join <public address>:2001 ──▶ ArmaReforgerServer
```

Run the topic runbooks in this order the first time; afterwards a redeploy is
[staging_deploy.md](/documentation_v2/runbooks/game_server_staging/staging_deploy.md) alone.

| Order | Runbook | When |
|---|---|---|
| 1 | [Host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md) | once per host |
| 2 | [Machine credentials and mission deployment](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md) | once per server, then per mission |
| 3 | [Staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md) | every change to the checkout |
| 4 | [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md) | after a deploy, or over any saved `console.log` |
| 5 | [Client join and mod updates](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md) | joining, and when players need a script change |

Facts every topic relies on:

- **Two launch modes.** `TBD_SERVER_MODE=config`, the default, launches with `-addonsDir` and
  `-config` together: the server registers a backend room (joinable), takes `game.admins[]` from
  the config, and loads the checkout the deploy synced. `TBD_SERVER_MODE=addons` launches with
  `-server` and `-addons`: it loads the checkout and reaches LOBBY but registers no room and has no
  admins, so it serves headless log checks only.
- **`-addonsDir` is not `-addons`.** The engine refuses `-addons` together with `-config`
  ("-config cannot be used together with addons!"); `-addonsDir` only says where to look and
  combines with `-config`. Without `-addonsDir`, a `-config` server satisfies `game.mods[]` from the
  Workshop copy of `tbd-framework`, which is published under the same id as the addon GUID
  `B2C3D4E5F6A78901` in `apps/mod/tbd-framework/addon.gproj`, and runs that copy instead of the
  checkout.
- **Two ids for one mod.** `game.mods[].modId` is the Workshop id; the addon GUID is the gproj's.
  For `tbd-framework` both are `B2C3D4E5F6A78901`. A modpack row carries both (`workshop_id` and
  `mod_guid`), because another mod's two ids can differ.
- **Ports.** Game `2001`, A2S `17777`, [RCON](/documentation_v2/glossary.md#rcon) `19999`. The A2S
  port must differ from the game port: equal ports make the engine log `NETWORK (E): Unable to start replication` and exit with status
  0, so `Restart=on-failure` does not restart it.
- **Log format.** Every current mod line reads `[TBD][<channel>] …`; a line with no channel tag
  comes from an old Workshop build. The stage machine prints both `[TBD][Stage] <from> -> <to>` and
  the flat `[TBD] Stage → <stage>`, so a check matches either.
- **Host paths.** Everything the deploy writes lives under the deploy account's `tbd/` folder;
  the deploy refuses a `TBD_REMOTE_DIR` containing `prairielearn`, the neighbouring project on the
  same host.

## Code

- [Staging deploy](/tools_v2/xtask/src/commands/deploy/staging/) — `cargo xtask deploy staging`:
  the settings check, the remote payloads, the unit and the boot verdict.
- [Debug commands](/tools_v2/xtask/src/commands/debug/) — `debug direct-join` and the
  `mod remote-logs` verdict.
- [Setup commands](/tools_v2/xtask/src/commands/setup/) — `setup server-profile`,
  `setup client-addons` and the `mod bootstrap-staging` discovery.
- [Mod commands](/tools_v2/xtask/src/commands/mod_ops/) — `mod bootstrap-staging`,
  `mod remote-logs`, `mod test-game-runtime-api` and `mod playtest`.
- [Deploy files](/tools_v2/xtask/deploy/) — `deploy.env.example` and the systemd units.
- [Game runtime transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/) — what the mod sends
  with its machine credential.
- [Fleet host agent](/apps/fleet_host_agent/) — the optional process supervisor on the host.

## Boundaries

- Depends on: the [runbook template](/documentation_v2/standards/templates/runbook.md); the xtask
  `deploy`, `mod`, `setup` and `debug` command trees; `tools_v2/xtask/deploy/deploy.env.example`;
  `apps/website/docker-compose.staging.yml`; the mod's log lines under
  `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: `cargo xtask mod bootstrap-staging` and `cargo xtask mod dev-server`, which print this
  README's path (`STAGING_SERVER_RUNBOOK` in `tools_v2/xtask/src/core/repository_layout.rs`); code
  comments in `TBD_Log.c`, `TBD_FrameworkManager.c`, `modpack_admin.rs` and
  `tools_v2/xtask/src/commands/mod_ops/playtest_server/usage_fail.rs`; the READMEs of the code
  folders above and of `apps/mod/`; the glossary; the fleet host agent and two-client playtest docs.
- Rules: this README keeps its path, because the xtask layout pins it; a topic file stays at or
  under 500 lines; every command in a topic file is checked against the xtask source; no document
  here writes a host address or a secret.

## Related documentation

- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — running the API and
  Postgres on the same host, which the staging deploy needs.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — a joinable,
  mod-loaded server on a development machine with `cargo xtask mod playtest`.
- [Fleet host agent](/documentation_v2/fleet_host_agent/README.md) — the agent the deploy can
  install, and how it runs fleet commands.
- [Mod documentation](/documentation_v2/mod/README.md) — the addons the server loads.
- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) and
  [Enfusion MCP tooling](/documentation_v2/runbooks/enfusion_mcp_tooling.md) — the
  [Workbench](/documentation_v2/glossary.md#workbench)-side checks before a deploy.
