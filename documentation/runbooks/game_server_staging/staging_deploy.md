**Status:** live

# Deploy the checkout to the staging game server fleet

`cargo xtask deploy staging` checks that the website API answers on the staging host and that the
fleet's secret files are there, syncs the checkout, writes every instance's profile and server
config on the host, checks each instance's game-runtime routes, installs the fleet's template
units, restarts every game server, and proves from each instance's own log that it loaded the
checkout it just synced; then it starts the relay and the host agents and runs the log check of
every instance. It starts nothing of the website: the API, its Postgres and Caddy come from
`cargo xtask deploy website`. Run it after every change the staging fleet should run; a deploy
takes a few minutes, most of it the rsync and the engines' boot.

## Prerequisites

- A prepared host with the website deployed, so the API answers `/healthz` at `TBD_BACKEND_URL`
  ([host preparation](/documentation/runbooks/game_server_staging/host_preparation.md),
  [website deployment](/documentation/runbooks/website_deployment.md)).
- `deploy/deploy.env` filled in with the settings below; no machine credential,
  RCON password or join password goes in it.
- On the host, every instance's two credential files from `cargo xtask staging provision-fleet`,
  and the join password in `~/tbd/fleet/join-password`
  ([machine credentials](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)).
- A source for `game.mods[]` (see Settings).

## Settings

The deploy reads `deploy.env` (or the file a `DEPLOY_ENV` variable names) as `KEY=VALUE` lines
(an `export ` prefix and quotes are stripped) and never executes it, so `$(…)` stays literal text
and a line that is not an assignment stops the deploy with `<path>:<line>`. A key the file sets
beats the process environment, and an empty value there counts as unset; a key the file never
sets may come from the environment.

| Setting | Default | Meaning |
|---|---|---|
| `TBD_SSH_HOST` | required | `user@host` of the staging host; the user's home, `/home/<user>`, is where the folders below default |
| `TBD_SSH_PASS`, `TBD_SSH_IDENTITY_FILE` | none | password through `sshpass`, else a key file, else plain `ssh` |
| `TBD_REMOTE_DIR`, `TBD_ADDONS_STAGING` | `tbd/repo`, `tbd/addons-staging` in `/home/<user>`; required when `TBD_SSH_HOST` names no user | the checkout, and the `-addonsDir` folder every instance shares |
| `TBD_SERVER_DIR` | `steam/arma-reforger-server` in `/home/<user>`; required when `TBD_SSH_HOST` names no user | the install of Steam app 1890870, the experimental dedicated server every instance runs |
| `TBD_PROFILE_DIR` | `tbd/profile` in `/home/<user>` | the single server's profile, which `--migrate-single-instance` archives with the `server.config.json` beside it; no instance uses it |
| `TBD_FLEET_INSTANCES` | `5` | instances 1 to this number run; 1 to 5 |
| `TBD_FLEET_GAME_PORT_BASE`, `TBD_FLEET_A2S_PORT_BASE`, `TBD_FLEET_RCON_PORT_BASE` | `2000`, `17776`, `19998` | instance N's game, A2S and loopback RCON ports are the base plus N |
| `TBD_FLEET_RELAY_INSTANCE`, `TBD_FLEET_RELAY_PORT` | none; the example sets `5` and `18085` | the instance whose host agent polls through the relay on `127.0.0.1:<port>`; the port is required once the instance is set, and without the instance no relay runs |
| `TBD_HOST_AGENT_API_URL` | `TBD_BACKEND_URL` | the origin the host agents poll and the relay forwards to; https, or http on a loopback host, and loopback http when a relay runs |
| `TBD_BACKEND_URL` | `http://127.0.0.1:8080` | the `backendUrl` every instance's mod calls, without a trailing `/`; `cargo xtask staging` reads the same value |
| `TBD_ADDON_GUID` | `B2C3D4E5F6A78901` | must equal the GUID in `apps/mod/tbd-framework/addon.gproj` |
| `TBD_SCENARIO` | `{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf` | the [mission header](/documentation/glossary/g_to_m.md#mission-header) a new instance boots; an instance whose server config exists keeps its own `scenarioId` |
| `TBD_WORKSHOP_MOD_ID`, `TBD_WORKSHOP_MOD_NAME` | none, `TBD_Framework` | the single-mod `game.mods[]` entry |
| `TBD_MODPACK_JSON` | none | a file holding a `GET /api/v1/modpacks/current` body; its mods become `game.mods[]` |
| `TBD_MODPACK_URL`, `TBD_MODPACK_TOKEN` | none | fetch that body from the API; the route needs a user's bearer access token |
| `TBD_PUBLIC_ADDRESS` | the first IPv4 address `TBD_SSH_HOST` resolves to on the development machine at deploy time | the address every instance's backend room advertises (`publicAddress`); must be IPv4; set it only when players reach the host at another address |
| `TBD_ADMIN_PASSWORD`, `TBD_MAX_PLAYERS` | fixed in `config.rs`, `64` | shared by every instance; set your own admin password |
| `TBD_ADMIN_IDENTITY_IDS` | empty | comma-separated identityIds or 17-digit SteamIDs; they become every instance's `game.admins[]` |
| `TBD_BOOT_VERIFY_TIMEOUT` | `180` | seconds to wait for every instance's room registration |

`game.mods[]` comes from the first source set: `TBD_MODPACK_JSON`, then `TBD_MODPACK_URL`, then
`TBD_WORKSHOP_MOD_ID`. For `tbd-framework` alone, the Workshop id equals the addon GUID
`B2C3D4E5F6A78901`; with `-addonsDir` the engine loads the checkout for that id. The keys
`TBD_STAGING_DB_CONTAINER`, `TBD_STAGING_OPERATOR_DISCORD_ID`, `TBD_STAGING_PARTNER_GUILD_ID`,
`TBD_STAGING_PARTNER_ROLE_ID`, `TBD_LOAD_TARGET_ORIGIN` and `TBD_LOAD_SOURCE_ADDRESSES` belong to
`cargo xtask staging`, which reads the same file; the deploy ignores them.

No machine credential, RCON password or join password is a setting. Before anything else the
deploy refuses, all at once and with exit 1,
every retired single-server setting the file or the process environment still assigns, naming
what replaced it and never printing its value:

```text
<path>:<line>: TBD_MOD_RUNTIME_CREDENTIAL: is no longer read (each instance's mod_runtime credential lives on the host in ~/tbd/fleet/instance-N/secrets/mod-runtime-credential); delete the assignment
```

| Retired setting | What the fleet reads instead |
|---|---|
| `TBD_MOD_RUNTIME_CREDENTIAL`, `TBD_HOST_AGENT_CREDENTIAL` | each instance's `mod-runtime-credential` and `host-agent-credential` under `~/tbd/fleet/instance-N/secrets/` |
| `TBD_RCON_PASSWORD` | each instance's `secrets/rcon-password`, generated on the host |
| `TBD_GAME_PORT`, `TBD_A2S_PORT`, `TBD_RCON_PORT` | `TBD_FLEET_GAME_PORT_BASE`, `TBD_FLEET_A2S_PORT_BASE` and `TBD_FLEET_RCON_PORT_BASE` plus N |
| `TBD_INSTALL_HOST_AGENT` | every instance runs its host agent |
| `TBD_SERVER_MODE` | every instance starts with `-config` |
| `TBD_SERVER_NAME` | instance N is named "TBD Staging N" |
| `TBD_SERVER_CONFIG_REMOTE` | `~/tbd/fleet/instance-N/server.config.json` |

Then, before anything is sent, it refuses with exit 1:

- a missing required setting (`<KEY> is not set: add it to <path>`), a value it cannot use
  (`<path>:<line>: <KEY>: …`), or no `deploy.env` at all;
- no `TBD_PUBLIC_ADDRESS` and no IPv4 address for `TBD_SSH_HOST` from the development machine
  (`TBD_PUBLIC_ADDRESS is unset and <host> has no IPv4 address from here (…): run avahi-daemon on
  the host or set TBD_PUBLIC_ADDRESS in <path>`);
- a `TBD_FLEET_INSTANCES` outside 1 to 5; a relay instance outside the fleet, or without
  `TBD_FLEET_RELAY_PORT`; a host agent origin that is neither https nor http on a loopback host,
  and a relay whose upstream is not http on a loopback host;
- a `TBD_ADDON_GUID` that differs from the gproj;
- a `TBD_REMOTE_DIR` containing `prairielearn`;
- ports that break the fleet's rules (`Refusing to deploy: <problem>.`): a port above 65535, a
  port two instances or two roles share (`port <port> is both instance <a>'s <kind> port and
  instance <b>'s <kind> port`), and a relay port that is 0, a fleet port or the API's own port;
- no mod source, or an admin id matching neither the identityId pattern (lowercase) nor the
  17-digit SteamID pattern, the engine's own two patterns.

Every instance's server config is rendered on the development machine, read back and checked
before it is pushed: valid JSON, the required keys, `a2s.port` different from `bindPort`, a
`scenarioId` the engine's pattern accepts, and a non-empty `game.mods[]` whose entries carry a
`modId` and a `name`. Instance N's config binds its game port (`bindPort`, `publicPort`) and A2S
port, carries an `rcon` block on `127.0.0.1` at its RCON port with the `admin` permission and two
clients, names the server "TBD Staging N", and is `visible` for instance 1 only. A rendered config
holds no password: `rcon.password` and `game.password` carry the placeholders
`TBD_RCON_PASSWORD_FROM_HOST_FILE` and `TBD_JOIN_PASSWORD_FROM_HOST_FILE`, which the host replaces
from its own files.

## Steps

1. Render every instance's server config into a local folder and check it, without touching the
   host. It needs the filled `deploy.env`, and renders `TBD_SCENARIO` rather than any instance's
   live `scenarioId`.

   ```bash
   cargo xtask deploy staging --render-only <local directory>
   ```

   Expected: `==> render every instance's server config (local only, no deploy) -> <local directory>`,
   the `  modpack source: …` line, then for each instance
   `  instance <N>: TBD Staging <N> -> <local directory>/instance-<N>/server.config.json` and
   `  config VALID: 1 mod(s) -> TBD_Framework=B2C3D4E5F6A78901` (one pair per mod), exit 0.

2. Print the plan: every step for every instance, with nothing sent to the host.

   ```bash
   cargo xtask deploy staging --dry-run
   ```

   Expected, in order: `==> publicAddress <IPv4 address>`,
   `[dry-run] curl -sSf <TBD_BACKEND_URL>/healthz on the host; the deploy stops unless it answers`,
   `[dry-run] check on the host: ~/tbd/fleet/join-password and each instance's two credential files (mode 600, expected shape, never printed)`,
   `[dry-run] refuse while tbd-reforger.service or fleet-host-agent.service is installed`,
   `[dry-run] rsync -avz --delete ... <TBD_REMOTE_DIR>/`, `[dry-run] game.mods[] from: <source>`,
   two lines per instance, the first
   `[dry-run] instance <N>: "TBD Staging <N>" game <port> A2S <port> RCON 127.0.0.1:<port> (admin), visible <true|false>, folder ~/tbd/fleet/instance-<N>, agent polls <origin>`,
   the unit install line ending with every `tbd-reforger@N.service`,
   `[dry-run] boot verdict per instance over the console.log of its new boot`, the relay line
   `[dry-run] relay: acknowledgement-dropping-relay@5 on 127.0.0.1:18085 forwarding to <origin>`,
   the host agent line ending with every `fleet_host_agent@N.service`, and
   `[dry-run] mod remote-logs --file over each instance's console.log`, exit 0. With
   `--migrate-single-instance` the refusal line gives way to
   `[dry-run] migrate: stop and disable tbd-reforger.service and fleet-host-agent.service; …`.

3. The first fleet deploy of a host retires the single-instance server: without the flag, the
   deploy refuses while its units are installed, because instance 1 takes their ports. On a fresh
   host the migration finds nothing and the deploy goes on. The migration also carries the
   [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent)'s names from the
   single server's kebab-case `fleet-host-agent.service`, `~/.config/fleet-host-agent/` and
   `~/.local/bin/fleet-host-agent` to the fleet's snake_case `fleet_host_agent@N.service`,
   `~/.config/fleet_host_agent/instance-N/` and `~/.local/bin/fleet_host_agent`; no other step
   renames them. A host whose checkout is not yet in the layout of restructure stage S2 first
   takes the one-time host step S1 of the [S2 operator steps](/documentation/archive/restructure_agent_briefs/s2_a2_to_a6.md#oc-deploy-operator-steps).

   ```bash
   cargo xtask deploy staging --migrate-single-instance
   ```

   Expected: after the rsync, `==> migrate the single-instance server` with
   `  stopped and disabled <unit>` and `  archived <path>` lines and
   `  single-instance files archived in <home>/tbd/retired/single-instance-<UTC time>`, or
   `  nothing of the single-instance server is left to archive`; then the stages below and
   `==> deploy complete: 5 instance(s)`, exit 0.

4. Every later deploy.

   ```bash
   cargo xtask deploy staging
   ```

   Expected: the stages below, then `==> deploy complete: 5 instance(s)`, exit 0. A stage that
   fails stops the deploy with its exit code, except that every instance gets its boot verdict
   before a failed one stops it; a website API that does not answer stops it with 1.

The stages, in order:

| Stage | What runs |
|---|---|
| website API | on the host, `curl -sSf --max-time 10 <TBD_BACKEND_URL>/healthz`; any failure, a 503 from an API whose database is down included, stops the deploy with exit 1 before anything on the host changes, naming `cargo xtask deploy website` |
| secret files | checks `~/tbd/fleet/join-password` and each instance's `mod-runtime-credential` and `host-agent-credential`: a regular file with no group or other permission, holding the expected shape; one line per file (`  ok      <label>`, `  MISSING …`, `  INVALID …`), never a value |
| single-instance units | without `--migrate-single-instance`: refuses while `tbd-reforger.service` or `fleet-host-agent.service` is installed |
| rsync | the checkout to `TBD_REMOTE_DIR` with `--delete`, excluding `.git/`, `target/`, `node_modules/`, the reference mods, `apps/mod/tbd-export/`, `apps/mod/tbd-emcp/`, the terrain, scratch and equipment asset trees, `apps/api/.env`, the app the website deploy built (`apps/frontend/dist/`), `deploy.env`, and what only the development machine holds, which the website deploy excludes too (the other cargo `target-*/` folders, worktrees, and the local files of Claude Code, Codex and `.mcp.json`); excluded paths on the host survive `--delete` |
| migration | with `--migrate-single-instance`: stops and disables the two single-instance units and moves their unit files, the whole `~/.config/fleet-host-agent/` folder, the `~/.local/bin/fleet-host-agent` binary, `TBD_PROFILE_DIR` and the `server.config.json` beside it into one new folder `~/tbd/retired/single-instance-<UTC time>/`, deleting nothing; refuses a `TBD_PROFILE_DIR` inside `~/tbd/fleet`, and ends by proving neither unit is loaded |
| instance files, per instance | keeps the live config's `scenarioId` (`  keeping the deployed scenario … (TBD_SCENARIO seeds only a new instance)`), renders and checks the config; on the host, makes the folders mode 700, links `TBD_ADDONS_STAGING/tbd-framework` to the synced `apps/mod/tbd-framework`, generates the RCON password once (32 lowercase hex digits, mode 600), runs `cargo xtask setup server-profile ~/tbd/fleet/instance-N/profile` with the instance's `mod_runtime` credential in its environment, sets `backendUrl`, fills both passwords into the config and moves it into place, mode 600 |
| game-runtime smoke, per instance | with the instance's credential, V2: `GET /api/v1/game-runtime/deployment` answers 200, or 404 `NO_DEPLOYMENT`; V3: with a deployment, the artifact's bytes hash to `artifact_sha256`; V4: the read without a credential answers 401 |
| units and restart | writes the three template units to `~/.config/systemd/user/`, enables linger, reloads, disables and stops the units of instances above `TBD_FLEET_INSTANCES` and every relay unit but the relay instance's, then enables and restarts every `tbd-reforger@N.service` |
| boot verdicts | waits up to `TBD_BOOT_VERIFY_TIMEOUT` for each instance's new boot, pulls its log and runs the verdict of [boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md); prints `==> boot verdicts` and `  instance <N>: PASS` or `FAIL`, and stops with exit 1 when any failed |
| relay | with a relay instance, the install under Host agent |
| host agents | the install under Host agent |
| V6, per instance | on the development machine, `cargo xtask mod remote-logs --file` over a fresh pull of the instance's new `console.log`; 0 (a player was seated) and 2 (booted, nobody joined yet) pass, 1 and 3 fail the deploy |

The deploy writes the templates `tbd-reforger@.service`, `fleet_host_agent@.service` and
`acknowledgement-dropping-relay@.service` of
[the systemd folder](/deploy/systemd/README.md), embedded in `xtask` when it is
built: the game server template with `TBD_SERVER_DIR` and `TBD_ADDONS_STAGING` in place of its
two placeholders, the other two byte for byte. Its `ExecStart`, where `%h` is the deploy account's
home and `%i` the instance number:

```text
<TBD_SERVER_DIR>/ArmaReforgerServer -addonsDir <TBD_ADDONS_STAGING> -config %h/tbd/fleet/instance-%i/server.config.json -profile %h/tbd/fleet/instance-%i/profile -maxFPS 60 -logStats 30000 -nothrow
```

## Host agent

Every instance runs its own [fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent)
as `fleet_host_agent@N.service`. It claims the host side of its server's
[fleet commands](/documentation/glossary/a_to_f.md#fleet-command) in
[server control](/documentation/glossary/n_to_z.md#server-control): `start`, `stop`, `restart`
and `list_players`, `console_command`, and the restart of a
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) on another terrain,
which rewrites the config's `scenarioId` to that terrain's
[mission header](/documentation/glossary/g_to_m.md#mission-header). The
[game runtime](/documentation/glossary/g_to_m.md#game-runtime) runs `broadcast`, `kick` and
`load_mission`. No action changes the map by name: a mission on another terrain reaches the
instance through its mission deployment. The
[fleet command ledger](/documentation/apps/api/verification_evidence/fleet_command_ledger.md#commands)
and [fleet command execution](/documentation/apps/fleet_host_agent/fleet_command_execution.md#who-runs-what)
hold the executor table. The install stage:

- builds `fleet_host_agent` in the synced checkout and installs it as `~/.local/bin/fleet_host_agent`;
- writes every instance's `~/.config/fleet_host_agent/instance-N/agent.toml` (mode 600, folder
  mode 700), which names files, never secrets: the instance's `secrets/host-agent-credential` and
  `secrets/rcon-password`, its unit `tbd-reforger@N.service` and its `server.config.json`, RCON on
  `127.0.0.1` at its RCON port, the origin it polls and a poll interval of 5 s;
- enables and restarts every `fleet_host_agent@N.service` and reads each state back after 3 s:
  `  fleet_host_agent@N.service active`, or `FAIL: fleet_host_agent@N.service is '<state>', not
  active.` with the unit's last 20 journal lines, which fails the deploy.

With `TBD_FLEET_RELAY_INSTANCE` set, the relay stage runs just before: it builds and installs
`acknowledgement-dropping-relay` into `~/.local/bin/`, writes that instance's `relay.env` (mode 600,
`RELAY_LISTEN` on `127.0.0.1` at `TBD_FLEET_RELAY_PORT` and `RELAY_UPSTREAM`, the agents' API
origin; no secret), restarts `acknowledgement-dropping-relay@N.service` and reads its state back:
`  acknowledgement-dropping-relay@N.service active on 127.0.0.1:<port>, forwarding to <origin>`. That
instance's agent polls the relay, which passes every request through to the API until it is armed
over its control socket, `$XDG_RUNTIME_DIR/acknowledgement-dropping-relay-N/control.sock`, in a
folder of mode 700.

The console command: the "Server console" form of a server's card in `/admin/server` sends one line
as the fleet action `console_command`, which the instance's host agent sends to the game server
over RCON, logged in with the `admin` permission on loopback. The line is 1 to 256 bytes, one line
without control characters, and never starts with `@`. The command is not idempotent and the line
is never resent, not even after a new RCON login; its execution window is 30 s; its outcome is the
server's reply, cut at 4096 bytes with `response_truncated` saying so, and a line with no reply
fails with "no RCON response; the command may or may not have run". The request's audit row
carries the line; [fleet command execution](/documentation/apps/fleet_host_agent/fleet_command_execution.md#rcon)
has the full rules.

Then issue **restart** from `/admin/server` on one instance's server; the command's receipt goes
`queued`, `claimed`, `executing`, `succeeded`, with the unit's `active_state`.

## Verify

On the host, every unit of the default five-instance fleet runs, the game and A2S ports are bound
on every address and the RCON ports on loopback only:

```bash
ssh <TBD_SSH_HOST> 'systemctl --user is-active tbd-reforger@{1..5}.service fleet_host_agent@{1..5}.service acknowledgement-dropping-relay@5.service && ss -ulnp | grep -E ":(200[1-5]|1777[7-9]|1778[01]|19999|2000[0-3]) "'
```

Expected: eleven `active` lines, then one UDP listener on `0.0.0.0` for each game port 2001 to
2005 and each A2S port 17777 to 17781, and one on `127.0.0.1` for each RCON port 19999 to 20003.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `Missing …deploy.env — copy from deploy/deploy.env.example` | no deploy file | [host preparation](/documentation/runbooks/game_server_staging/host_preparation.md) step 6 |
| `<path>:<line>: <KEY>: is no longer read (…); delete the assignment`, exit 1 | the deploy file, or the process environment, still assigns a retired single-server setting | delete the line or unset the variable; credentials and passwords live on the host ([machine credentials](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)) |
| `ERROR: <TBD_BACKEND_URL>/healthz does not answer on the staging host (probe exit <n>).`, exit 1 | the website is not deployed on the host or its API is down: 7 means nothing listens, 22 an error status such as the 503 of an API without its database | `cargo xtask deploy website` ([website deployment](/documentation/runbooks/website_deployment.md)), then deploy staging again |
| `TBD_PUBLIC_ADDRESS is unset and <host> has no IPv4 address from here …` | the development machine cannot resolve `TBD_SSH_HOST` to IPv4: avahi is not running on the host, or mDNS does not reach this machine | start avahi-daemon on the host, or set `TBD_PUBLIC_ADDRESS` |
| `The fleet's -config servers need TBD_WORKSHOP_MOD_ID …` | no mod source | set `TBD_WORKSHOP_MOD_ID=B2C3D4E5F6A78901`, or a modpack source |
| `FAIL: TBD_MODPACK_URL is set but TBD_MODPACK_TOKEN is empty.` | the modpack route needs a user's bearer token | set the token, or save the body and use `TBD_MODPACK_JSON` |
| `NOTE: TBD_ADMIN_IDENTITY_IDS is empty, so game.admins[] will be [].` | no admins configured | add the admins' identityIds; `passwordAdmin` does not feed `game.admins[]` |
| `Refusing to deploy: port <port> is both …` | two instances or two roles share a port | change a `TBD_FLEET_*_PORT_BASE` or `TBD_FLEET_RELAY_PORT` |
| `  MISSING <label>: <file> is absent, not a regular file, or open to other users`, then `FAIL: <n> secret file(s) under …/tbd/fleet are not ready.` | the fleet is not provisioned, the join password is not written, or a file is readable by group or others | `cargo xtask staging provision-fleet`, the join password ([machine credentials](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md) steps 1 and 2), or `chmod 600` on the file |
| `  INVALID <label>: <file> does not hold the expected shape` | a credential that is not `tbdm_<32 hex>_<64 hex>`, or a join password outside its characters | rotate the credential, or write the join password again |
| `FAIL: the single-instance units are still installed: … rerun with --migrate-single-instance …` | the host still runs the single server, whose ports instance 1 takes | step 3 |
| `FAIL: TBD_PROFILE_DIR <path> is inside the fleet root …; refusing to archive it` | `TBD_PROFILE_DIR` points into `~/tbd/fleet` | unset it, or point it at the single server's profile |
| `game.scenarioId … is rejected by the engine's schema` | the value is not a bracketed 16-hex GUID followed by a path; one that stops right after the GUID was cut by shell brace parsing before it reached the deploy | write the whole `{GUID}Missions/….conf` value in `deploy.env`, which the deploy parses without a shell |
| `FAIL: …/secrets/rcon-password does not hold 32 lowercase hex digits; delete it to generate a new one` | the file does not hold a password the deploy generated | delete that file on the host and deploy again |
| the smoke stage fails with a `curl` error | nothing answers on the host's `127.0.0.1:8080`, which the smoke reads whatever `TBD_BACKEND_URL` names | deploy the website to this host ([host preparation](/documentation/runbooks/game_server_staging/host_preparation.md) step 12) |
| `V2 instance <N> game-runtime deployment: HTTP 401` | the instance's credential is revoked or another server's | rotate it ([machine credentials](/documentation/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md)) and deploy again |
| `FAIL: instance <N> wrote no new log folder under …/logs.` | the unit never started | `systemctl --user status tbd-reforger@<N>.service` on the host |
| `FAIL: instance <N> is NOT serving what was deployed, or is not joinable; log kept at <path>` | its boot verdict failed | read the FAIL lines above it; [boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md) |
| `FAIL: acknowledgement-dropping-relay@<N>.service is '<state>', not active.` | the relay did not start | the journal lines printed below it name the cause |
| `FAIL: fleet_host_agent@<N>.service is '<state>', not active.` | the agent did not start: exit 78 is a configuration it refuses, which a restart cannot fix | the journal lines below it; the instance's `agent.toml` and secret files |
| `V6 ENVIRONMENT — the log could not be obtained …`, or `V6 instance <N>: could not pull …` | no log was examined | check SSH, then `cargo xtask mod remote-logs --instance <N>` |

## Related

- [Staging deploy code](/tools/xtask/src/commands/deploy/staging/README.md) — the module
  layout, the settings check and the tests that pin each payload.
- [Deploy files](/deploy/README.md) — `deploy.env.example` and the systemd units.
- [Fleet host agent](/documentation/apps/fleet_host_agent/README.md) — the agent's configuration and
  command execution.
- [Game server staging](/documentation/runbooks/game_server_staging/README.md) — the index.
