**Status:** live

# Game server staging

How to run the staging host's fleet of Arma Reforger dedicated servers on the checkout: preparing
the host once, provisioning each instance's
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), deploying the
instances with `cargo xtask deploy staging`, proving each boot from the instance's own log, and
joining an instance from a client. Developers and agents working on the
[mod](/documentation/glossary/g_to_m.md#mod) or on the platform's
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) routes read it.

## Contents

```text
documentation/runbooks/game_server_staging/
├── boot_and_log_verification.md                    the boot verdict per instance, `mod remote-logs --instance` and the log lines
├── client_join_and_mod_updates.md                  Direct Join to an instance, the join password, launch flags, ports, script changes
├── host_preparation.md                             one-time host setup: name, deploy.env, build tools, experimental server, API, fleet folder, firewall
├── machine_credentials_and_mission_deployment.md   provisioning and rotating credentials, the first fleet deploy, events, mission deployment
└── staging_deploy.md                               `deploy staging`: settings, refusals, stages, host agents, relay, console command
```

## How it works

The staging host is the machine `TBD_SSH_HOST` names in `deploy/deploy.env`. It runs
the platform's API and Postgres and a fleet of up to five dedicated game servers, the instances.
Each instance's mod asks the API which
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) it runs and fetches
that [mission](/documentation/glossary/g_to_m.md#mission)'s
[artifact](/documentation/glossary/a_to_f.md#artifact), and each instance's
[game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) runs the host side of its
server's [fleet commands](/documentation/glossary/a_to_f.md#fleet-command).

```text
development machine                            staging host (TBD_SSH_HOST)
───────────────────                            ───────────────────────────
cargo xtask deploy website ──────── ssh ─────▶ API 127.0.0.1:8080, Postgres, Caddy; the host tools
                                               staging-fixtures and acknowledgement-dropping-relay
cargo xtask staging provision-fleet ── ssh ──▶ servers "TBD Staging 1" … "TBD Staging 5";
                                               ~/tbd/fleet/instance-N/secrets/ (two credentials each)
cargo xtask deploy staging ── rsync ─────────▶ TBD_REMOTE_DIR (the checkout)
                           ── ssh ───────────▶ per instance N: profile, server config, RCON password;
                                               the template units; a boot verdict per instance
cargo xtask mod remote-logs --instance N ◀──── ~/tbd/fleet/instance-N/profile/logs/…/console.log

tbd-reforger@N ── ArmaReforgerServer (Steam app 1890870 in TBD_SERVER_DIR), N = 1 … 5
  │ game UDP 2000+N, A2S UDP 17776+N, RCON 127.0.0.1:19998+N
  │ -addonsDir TBD_ADDONS_STAGING (links the checkout's tbd-framework)
  │ -config ~/tbd/fleet/instance-N/server.config.json (game.mods[], game.admins[], join password)
  └── machineCredential ──▶ API 127.0.0.1:8080 ──▶ Postgres
game_server_host_agent@N ── RCON 127.0.0.1:19998+N, polls the API
  (instance 5 through acknowledgement-dropping-relay@5 on 127.0.0.1:18085, which forwards to the API)
Arma client ── Direct Join <public address>:2000+N, join password ──▶ instance N
```

Run the topic runbooks in this order the first time; afterwards a redeploy is
[staging_deploy.md](/documentation/runbooks/game_server_staging/staging_deploy.md) alone.

| Order | Runbook | When |
|---|---|---|
| 1 | [Host preparation](/documentation/runbooks/game_server_staging/host_preparation.md) | once per host |
| 2 | [Machine credentials and mission deployment](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md) | once per fleet: provisioning, the join password and the first fleet deploy; then per credential rotation, event and mission |
| 3 | [Staging deploy](/documentation/runbooks/game_server_staging/staging_deploy.md) | every change to the checkout |
| 4 | [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md) | after a deploy, or over any saved `console.log` |
| 5 | [Client join and mod updates](/documentation/runbooks/game_server_staging/client_join_and_mod_updates.md) | joining, and when players need a script change |

A fresh host and a host that ran the single server take the first fleet deploy in one order:
`cargo xtask deploy website`, `cargo xtask staging provision-fleet`, the operator's join password
in `~/tbd/fleet/join-password`, then `cargo xtask deploy staging --migrate-single-instance`, which
also retires the single server's kebab-case host agent names (`fleet-host-agent.service`,
`~/.config/fleet-host-agent/`) for the fleet's `game_server_host_agent@N.service`
([staging deploy](/documentation/runbooks/game_server_staging/staging_deploy.md) step 4). A host
whose host agents still run as `fleet_host_agent@N.service` first takes
`cargo xtask deploy staging --migrate-host-agent-name` (step 3), and a host whose API `.env` still
sits outside `crates/api/api_server/` moves it there before any deploy
([website deployment](/documentation/runbooks/website_deployment.md)). The
[setup checklist](/documentation/runbooks/staging_verification/setup_checklist.md) runs this
order as its steps 3 to 5.

Facts every topic relies on:

- **Five instances.** `TBD_FLEET_INSTANCES` instances run (default 5, at most 5). Instance N runs
  as the user units `tbd-reforger@N.service` and `game_server_host_agent@N.service` and is named
  "TBD Staging N". Only instance 1 is listed in the server browser; instances 2 to 5 carry
  `visible: false` and take Direct Join only. The relay instance (`TBD_FLEET_RELAY_INSTANCE`, 5)
  also runs `acknowledgement-dropping-relay@N.service` on `127.0.0.1:TBD_FLEET_RELAY_PORT`
  (18085): its host agent polls the relay, which forwards to the API.
- **Ports.** Instance N listens on game port `TBD_FLEET_GAME_PORT_BASE + N` (2001 to 2005), A2S
  port `TBD_FLEET_A2S_PORT_BASE + N` (17777 to 17781) and
  [RCON](/documentation/glossary/n_to_z.md#rcon) port `TBD_FLEET_RCON_PORT_BASE + N` (19999 to
  20003, on `127.0.0.1` only); instance 1 keeps the single server's ports 2001, 17777 and 19999.
  Every port of the fleet is distinct: equal A2S and game ports make the engine log
  `NETWORK (E): Unable to start replication` and exit with status 0, so `Restart=on-failure` does
  not restart it, and the deploy refuses any port two instances or two roles share.
- **One launch line.** Every instance runs the install of Steam app 1890870, the Arma Reforger
  experimental dedicated server, in `TBD_SERVER_DIR`, with the shared
  `-addonsDir TBD_ADDONS_STAGING`, its own `-config ~/tbd/fleet/instance-N/server.config.json` and
  `-profile ~/tbd/fleet/instance-N/profile`, and `-maxFPS 60 -logStats 30000 -nothrow`. `-config`
  registers a backend room (joinable) and supplies `game.admins[]`; `-addonsDir` loads the
  checkout the deploy synced.
- **`-addonsDir` is not `-addons`.** The engine refuses `-addons` together with `-config`
  ("-config cannot be used together with addons!"); `-addonsDir` only says where to look and
  combines with `-config`. Without `-addonsDir`, a `-config` server satisfies `game.mods[]` from the
  Workshop copy of `tbd-framework`, which is published under the same id as the addon GUID
  `B2C3D4E5F6A78901` in `apps/mod/tbd-framework/addon.gproj`, and runs that copy instead of the
  checkout.
- **Two ids for one mod.** `game.mods[].modId` is the Workshop id; the addon GUID is the gproj's.
  For `tbd-framework` both are `B2C3D4E5F6A78901`. A modpack row carries both (`workshop_id` and
  `mod_guid`), because another mod's two ids can differ.
- **Secrets live on the host.** Each instance's `mod_runtime` and `host_agent` credentials are
  files under `~/tbd/fleet/instance-N/secrets/`, written by `cargo xtask staging provision-fleet`;
  its RCON password (the `admin` permission, loopback only) is generated there by the deploy; the
  join password every instance requires is the operator's `~/tbd/fleet/join-password`. Folders
  are mode 700 and files mode 600. None of them is a `deploy.env` setting: the deploy refuses the
  retired credential, password, port and launch mode settings, naming what replaced each.
- **Log format.** Every current mod line reads `[TBD][<channel>] …`; a line with no channel tag
  comes from an old Workshop build. The stage machine prints both `[TBD][Stage] <from> -> <to>` and
  the flat `[TBD] Stage → <stage>`, so a check matches either.
- **Host paths.** Everything the deploy writes lives under the deploy account's home: by default
  `/home/<user>/tbd/repo`, `…/tbd/addons-staging` and `…/tbd/fleet`, and the server in
  `/home/<user>/steam/arma-reforger-server`, where `<user>` is the user of `TBD_SSH_HOST`;
  `…/tbd/profile` is the single server's profile, which the first fleet deploy archives under
  `…/tbd/retired/`. The deploy refuses a `TBD_REMOTE_DIR` containing `prairielearn`, the
  neighbouring project on the same host.
- **One host name.** `TBD_SSH_HOST` in `deploy.env` is the only place the host is named: the
  deploy derives the servers' `publicAddress` from it, and the probes and hints resolve it too.

## Code

- [Staging deploy](/tools/commands/deployment/src/staging/) — `cargo xtask deploy staging`:
  the settings check, the fleet's instances and units, the remote payloads, the single-instance
  migration and the boot verdict per instance.
- [Debug commands](/tools/commands/remote_debugging/src/debug/) — `debug direct-join --instance` and the
  `mod remote-logs` verdict.
- [Setup commands](/tools/commands/workstation_setup/src/) — `setup server-profile`,
  `setup client-addons` and the `mod bootstrap-staging` discovery.
- [Mod commands](/tools/commands/mod_operations/src/) — `mod bootstrap-staging`,
  `mod remote-logs`, `mod test-game-runtime-api` and `mod playtest`.
- [Deploy files](/deploy/) — `deploy.env.example` and the systemd units.
- [Staging fixtures host tool](/tools/staging/staging_fixtures/src/README.md) — the
  `provision-fleet` and `rotate-credential` subcommands `cargo xtask staging` runs on the host.
- [Game runtime transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/) — what the mod sends
  with its machine credential.
- [Game server host agent](/crates/fleet/game_server_host_agent/) — the process supervisor each instance runs.

## Boundaries

- Depends on: the [runbook template](/documentation/standards/templates/runbook.md); the xtask
  `deploy`, `staging`, `mod`, `setup` and `debug` command trees;
  `deploy/deploy.env.example`; the website on the same host, which
  `cargo xtask deploy website` puts there with the host tools; the mod's log lines under
  `apps/mod/tbd-framework/Scripts/Game/TBD/`.
- Used by: `cargo xtask mod bootstrap-staging` and `cargo xtask mod dev-server`, which print this
  README's path (`STAGING_SERVER_RUNBOOK` in `tools/foundation/repository_layout/src/documentation_locations.rs`); code
  comments in `TBD_Log.c`, `TBD_FrameworkManager.c`, `modpack_admin.rs` and
  `tools/commands/mod_operations/src/playtest_server/usage_fail.rs`; the READMEs of the code
  folders above and of `apps/mod/`; the glossary; the game server host agent, two-client playtest and
  staging verification docs.
- Rules: this README keeps its path, because the xtask layout pins it; a topic file stays at or
  under 500 lines; every command in a topic file is checked against the xtask source; no document
  here writes a host address or a secret.

## Related documentation

- [Website deployment](/documentation/runbooks/website_deployment.md) — running the API,
  Postgres and Caddy on the same host and building the host tools; the staging deploy checks that
  the API answers before it changes anything.
- [Staging verification](/documentation/runbooks/staging_verification/README.md) — the setup
  checklist and the run-day procedures that drive the fleet.
- [Two-client playtest](/documentation/runbooks/two_client_playtest/README.md) — a joinable,
  mod-loaded server on a development machine with `cargo xtask mod playtest`.
- [Game server host agent](/documentation/crates/fleet/game_server_host_agent/README.md) — the agent every instance runs,
  and how it runs fleet commands.
- [Mod documentation](/documentation/apps/mod/README.md) — the addons the servers load.
- [Spawn determinism](/documentation/runbooks/spawn_determinism.md) and
  [Enfusion MCP tooling](/documentation/runbooks/enfusion_mcp_tooling.md) — the
  [Workbench](/documentation/glossary/n_to_z.md#workbench)-side checks before a deploy.
