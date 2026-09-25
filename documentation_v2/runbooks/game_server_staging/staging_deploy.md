**Status:** live

# Deploy the checkout to the staging game server

`cargo xtask deploy staging` syncs the checkout to the staging host, refreshes the
[mod](/documentation_v2/glossary.md#mod)'s server profile,
checks the platform's game-runtime routes, renders the server config, restarts the dedicated
server, and then proves from the server's own log that it loaded the checkout it just synced. Run
it after every change the staging server should run; a deploy takes a few minutes, most of it the
rsync and the engine's boot.

## Prerequisites

- A prepared host with the API answering on its `127.0.0.1:8080`
  ([host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md)).
- `tools_v2/xtask/deploy/deploy.env` filled in, including `TBD_MOD_RUNTIME_CREDENTIAL`
  ([machine credentials](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)).
- In config mode, a source for `game.mods[]` (see Settings).

## Settings

The deploy reads `deploy.env` as `KEY=VALUE` lines (an `export ` prefix and quotes are stripped)
and never executes it, so `$(…)` stays literal text. A key the file sets beats the process
environment; a key it leaves out may come from the environment.

| Setting | Default | Meaning |
|---|---|---|
| `TBD_SSH_HOST` | required | `user@host` of the staging host |
| `TBD_SSH_PASS`, `TBD_SSH_IDENTITY_FILE` | none | password through `sshpass`, else a key file, else plain `ssh` |
| `TBD_REMOTE_DIR`, `TBD_PROFILE_DIR`, `TBD_ADDONS_STAGING` | required | the checkout, the `-profile` folder and the `-addonsDir` folder on the host |
| `TBD_SERVER_DIR` | `steam/arma-reforger-server` in the deploy account's home | the dedicated server install |
| `TBD_GAME_SERVER_TOKEN` | required | the API's `SERVICE_TOKEN`; becomes `serverToken` |
| `TBD_MOD_RUNTIME_CREDENTIAL` | required | the server's `mod_runtime` credential; becomes `machineCredential` |
| `TBD_BACKEND_URL` | `http://127.0.0.1:8080` | the `backendUrl` the mod calls |
| `TBD_SERVER_MODE` | `config` | `config` (`-addonsDir` + `-config`, joinable) or `addons` (`-server` + `-addons`, log checks only) |
| `TBD_ADDON_GUID` | `B2C3D4E5F6A78901` | must equal the GUID in `apps/mod/tbd-framework/addon.gproj` |
| `TBD_SCENARIO` | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` | the [mission header](/documentation_v2/glossary.md#mission-header) a new server boots; a server config that exists keeps its own `scenarioId` |
| `TBD_WORKSHOP_MOD_ID`, `TBD_WORKSHOP_MOD_NAME` | none, `TBD_Framework` | the single-mod `game.mods[]` entry |
| `TBD_MODPACK_JSON` | none | a file holding a `GET /api/v1/modpacks/current` body; its mods become `game.mods[]` |
| `TBD_MODPACK_URL`, `TBD_MODPACK_TOKEN` | none | fetch that body from the API; the route needs a user's bearer access token |
| `TBD_GAME_PORT`, `TBD_A2S_PORT` | `2001`, `17777` | must differ |
| `TBD_BIND_IP`, `TBD_PUBLIC_ADDRESS` | a LAN address fixed in `config.rs`, then `TBD_BIND_IP` | the address the backend room advertises; set it to the host's |
| `TBD_SERVER_NAME`, `TBD_ADMIN_PASSWORD`, `TBD_MAX_PLAYERS` | `TBD Staging POC`, fixed in `config.rs`, `64` | server identity; set your own admin password |
| `TBD_ADMIN_IDENTITY_IDS` | empty | comma-separated identityIds or 17-digit SteamIDs; they become `game.admins[]` |
| `TBD_SERVER_CONFIG_REMOTE` | `server.config.json` beside `TBD_PROFILE_DIR` | where the rendered config goes on the host |
| `TBD_BOOT_VERIFY_TIMEOUT` | `180` | seconds to wait for the room registration |
| `TBD_INSTALL_HOST_AGENT` | `0` | `1` installs the host agent (see Host agent) |

`game.mods[]` comes from the first source set: `TBD_MODPACK_JSON`, then `TBD_MODPACK_URL`, then
`TBD_WORKSHOP_MOD_ID`. For `tbd-framework` alone, the Workshop id equals the addon GUID
`B2C3D4E5F6A78901`; with `-addonsDir` the engine loads the checkout for that id.

Before anything is sent, the deploy refuses, with exit 1:

- a missing required setting (`deploy.env: line <n>: <KEY>: …`), or no `deploy.env` at all;
- a `TBD_ADDON_GUID` that differs from the gproj;
- a `TBD_REMOTE_DIR` containing `prairielearn`;
- a runtime or host-agent credential not shaped `tbdm_<32 hex>_<64 hex>`;
- a `TBD_SERVER_MODE` other than `config` or `addons`;
- in config mode: no mod source, equal game and A2S ports, or an admin id matching neither the
  identityId pattern (lowercase) nor the 17-digit SteamID pattern, the engine's own two patterns;
- with the host agent: config mode missing, an [RCON](/documentation_v2/glossary.md#rcon) password outside 3 to 256 bytes or holding
  whitespace, quotes or backslashes, an RCON port outside 1 to 65535, or an agent API URL that is
  neither https nor http on a loopback host.

In config mode the server config is rendered on the development machine, read back and checked
before it is pushed: valid JSON, the required keys, `a2s.port` different from `bindPort`, a
`scenarioId` the engine's pattern accepts, and a non-empty `game.mods[]` whose entries carry a
`modId` and a `name`.

## Steps

1. Render the server config to a local file and check it, without touching the host. It needs
   the filled `deploy.env` and config mode, and renders `TBD_SCENARIO` rather than the host's live
   `scenarioId`.

   ```bash
   cargo xtask deploy staging --render-only <local path>
   ```

   Expected: `==> render server config (local only, no deploy) -> <local path>`, then
   `  config VALID: 1 mod(s) -> TBD_Framework=B2C3D4E5F6A78901` (one pair per mod), exit 0.
   Addons mode exits 2 with `--render-only requires TBD_SERVER_MODE=config`.

2. Print the plan: every stage of the deploy, with nothing sent to the host.

   ```bash
   cargo xtask deploy staging --dry-run
   ```

   Expected, in order: `==> rsync to <TBD_REMOTE_DIR>`, `==> remote profile + addon symlink`,
   `==> docker compose (API + Postgres)`, `==> game-runtime smoke (V2–V4)`,
   `==> systemd user service + restart game server (mode: config)` with the
   `[dry-run] ExecStart=…` line, `==> host agent (fleet-host-agent)`, `==> V6 remote log grep` and
   `[dry-run] cargo run -q -p xtask -- mod remote-logs`, exit 0.

3. Deploy.

   ```bash
   cargo xtask deploy staging
   ```

   Expected: the stages below, then `==> deploy complete`, exit 0. Any stage that fails stops the
   deploy with its exit code.

The stages, in order:

| Stage | What runs on the host |
|---|---|
| rsync | the checkout to `TBD_REMOTE_DIR` with `--delete`, excluding `.git/`, `target/`, `node_modules/`, the reference mods, `apps/mod/tbd-export/`, `apps/mod/tbd-emcp/`, the terrain, scratch and equipment asset trees, `apps/website/api_v2/.env` and `deploy.env`; excluded paths on the host survive `--delete` |
| profile and addon link | links `TBD_ADDONS_STAGING/tbd-framework` to the synced `apps/mod/tbd-framework`, runs `cargo xtask setup server-profile TBD_PROFILE_DIR` with the token and credential, and sets `backendUrl` |
| compose | `docker compose -f apps/website/docker-compose.staging.yml up -d --build`, which starts Postgres only: the compose file's `api` service needs `--profile api` |
| game-runtime smoke | V2: `GET /api/v1/game-runtime/deployment` with the credential answers 200, or 404 `NO_DEPLOYMENT`; V3: with a deployment, the artifact's bytes hash to `artifact_sha256`; V4: the read without a credential answers 401 |
| server config and unit | config mode keeps the live config's `scenarioId` (`keeping the deployed scenario …`), renders, checks and pushes the config; then writes `~/.config/systemd/user/tbd-reforger.service` with the mode's `ExecStart`, reloads, enables and restarts it |
| boot verdict | polls the newest `TBD_PROFILE_DIR/logs/logs_*/console.log` every 10 s for `Server registered with address:`, up to `TBD_BOOT_VERIFY_TIMEOUT`, pulls the log and runs the verdict of [boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md); addons mode checks the addon only |
| host agent | with `TBD_INSTALL_HOST_AGENT=1`, the install below; otherwise `[SKIP] host agent — TBD_INSTALL_HOST_AGENT=1 to install it.` |
| V6 | runs `cargo xtask mod remote-logs` on the development machine; 0 (a player was seated) and 2 (booted, nobody joined yet) pass, 1 and 3 fail the deploy |

The unit the deploy writes is its own: `tools_v2/xtask/deploy/systemd/tbd-reforger.service` is a
template no code reads. The written `ExecStart`, in config mode:

```text
<TBD_SERVER_DIR>/ArmaReforgerServer -addonsDir <TBD_ADDONS_STAGING> -config <TBD_SERVER_CONFIG_REMOTE> -profile <TBD_PROFILE_DIR> -maxFPS 60 -logStats 30000 -nothrow
```

In addons mode it is `-profile … -addonsDir … -addons <TBD_ADDON_GUID> -server "<TBD_SCENARIO>"
-bindIP 0.0.0.0 -bindPort <game port> -a2sPort <A2S port>` with the same last three flags.

## Host agent

The [fleet host agent](/documentation_v2/glossary.md#fleet-host-agent) runs the start, stop,
restart and player-list [fleet commands](/documentation_v2/glossary.md#fleet-command) of
[server control](/documentation_v2/glossary.md#server-control) on the host, and restarts the server
on another terrain's scenario for a [mission](/documentation_v2/glossary.md#mission) on that terrain. Without it those commands stay
`queued` until they expire. To install it, set in `deploy.env`: `TBD_INSTALL_HOST_AGENT=1`,
`TBD_HOST_AGENT_CREDENTIAL` (its `host_agent` credential), `TBD_RCON_PASSWORD`, and optionally
`TBD_RCON_PORT` (default `19999`) and `TBD_HOST_AGENT_API_URL` (default `TBD_BACKEND_URL`). Then
deploy as in step 3. The rendered config gains an `rcon` block on `127.0.0.1` with monitor
permission and two clients, and the install stage:

- builds `fleet-host-agent` in the synced checkout and installs it as `~/.local/bin/fleet-host-agent`;
- writes `~/.config/fleet-host-agent/agent.toml`, `machine-credential` and `rcon-password` (mode
  600, folder mode 700), with a poll interval of 5 s;
- installs `fleet-host-agent.service` byte for byte from
  `tools_v2/xtask/deploy/systemd/fleet-host-agent.service`, enables linger, and restarts the unit;
- reads the unit's state back after 3 s: `  fleet-host-agent.service active, polling <url>`, or
  `FAIL: fleet-host-agent.service is '<state>', not active.` with the unit's last 20 journal lines,
  which fails the deploy.

Then issue **restart** from `/admin/server`; the command's receipt goes `queued`, `claimed`,
`executing`, `succeeded`, with the unit's `active_state`.

## Verify

On the host, the unit runs and both UDP ports are bound:

```bash
ssh <TBD_SSH_HOST> 'systemctl --user is-active tbd-reforger.service && ss -ulnp | grep -E ":2001|:17777"'
```

Expected: `active`, then one listener on 2001 and one on 17777.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Missing …deploy.env — copy from tools_v2/xtask/deploy/deploy.env.example` | no deploy file | [host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md) step 1 |
| `TBD_SERVER_MODE=config requires TBD_WORKSHOP_MOD_ID …` | no mod source in config mode | set `TBD_WORKSHOP_MOD_ID=B2C3D4E5F6A78901`, or a modpack source |
| `FAIL: TBD_MODPACK_URL is set but TBD_MODPACK_TOKEN is empty.` | the modpack route needs a user's bearer token | set the token, or save the body and use `TBD_MODPACK_JSON` |
| `NOTE: TBD_ADMIN_IDENTITY_IDS is empty, so game.admins[] will be [].` | no admins configured | add the admins' identityIds; `passwordAdmin` does not feed `game.admins[]` |
| `game.scenarioId … is rejected by the engine's schema` | the value is not a bracketed 16-hex GUID followed by a path; one that stops right after the GUID was cut by shell brace parsing before it reached the deploy | write the whole `{GUID}Missions/….conf` value in `deploy.env`, which the deploy parses without a shell |
| the smoke stage fails with a `curl` error | nothing answers on the host's `127.0.0.1:8080`: the compose step starts no API | start the API ([host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md) step 4) |
| `V2 game-runtime deployment: HTTP 401` | the credential is revoked or another server's | issue a fresh one and deploy again |
| `FAIL: the server produced no log directory under …/logs after <n>s.` | the unit never started | `systemctl --user status tbd-reforger.service` on the host |
| `DEPLOY FAILED ITS OWN ACCEPTANCE CHECK.` | the boot verdict failed | read the FAIL lines above it; [boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md) |
| `V6 ENVIRONMENT — the log could not be obtained …` | `mod remote-logs` read no log | check SSH and `TBD_PROFILE_DIR`, then run it again |
| `TBD_INSTALL_HOST_AGENT=1 needs TBD_SERVER_MODE=config …` | the agent restarts missions by rewriting the config's `scenarioId` | use config mode |
| the server exits right after `Starting RPL server` and systemd does not restart it | equal A2S and game ports: `Unable to start replication`, exit status 0 | keep `TBD_A2S_PORT` different from `TBD_GAME_PORT` |

## Related

- [Staging deploy code](/tools_v2/xtask/src/commands/deploy/staging/README.md) — the module
  layout, the settings check and the tests that pin each payload.
- [Deploy files](/tools_v2/xtask/deploy/README.md) — `deploy.env.example` and the systemd units.
- [Fleet host agent](/documentation_v2/fleet_host_agent/README.md) — the agent's configuration and
  command execution.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index.
