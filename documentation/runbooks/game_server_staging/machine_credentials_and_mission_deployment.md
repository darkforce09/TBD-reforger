**Status:** live

# Give the staging fleet its credentials and a mission

Connects the staging fleet's game servers to the platform: each instance's
[machine credentials](/documentation/glossary/g_to_m.md#machine-credential), the
[event](/documentation/glossary/a_to_f.md#event) a server serves, and the
[mission deployment](/documentation/glossary/g_to_m.md#mission-deployment) it runs.
`cargo xtask staging provision-fleet` registers the servers and writes their credentials on the
host once, before the first fleet deploy; rotating a credential, binding an event and deploying a
[mission](/documentation/glossary/g_to_m.md#mission) repeat whenever one of them changes.

## Prerequisites

- The website deployed on the staging host by `cargo xtask deploy website`
  ([host preparation](/documentation/runbooks/game_server_staging/host_preparation.md) step 12).
  Its remote step `cargo build --release: the staging host tools (staging-fixtures,
  acknowledgement-dropping-relay)` builds the host tool `staging-fixtures`, which provisioning and
  rotation run on the host, into `<TBD_REMOTE_DIR>/target/release/`; `TBD_SKIP_API_BUILD=1` skips
  that build together with the API's.
- The operator's Discord account with administrator authority: the host tool issues every
  credential in the operator's name, and refuses an actor without that authority.
- An administrator login on the website, and an approved
  [artifact](/documentation/glossary/a_to_f.md#artifact) of the mission to deploy.
- For the `curl` forms: `$ADMIN_TOKEN`, an administrator's access token, and `$API`, the API
  origin (`http://127.0.0.1:8080` on the host).

## Steps

The first fleet deploy takes one order on a fresh host and on a host that ran the single server:
the website deploy above, steps 1 to 3 here, then every later change through the
[staging deploy](/documentation/runbooks/game_server_staging/staging_deploy.md). The
[setup checklist](/documentation/runbooks/staging_verification/setup_checklist.md) runs the same
order as its steps 3 to 5.

1. Register the fleet's servers and write their credentials on the host.

   ```bash
   cargo xtask staging provision-fleet
   ```

   Expected: five new servers "TBD Staging 1" … "TBD Staging 5" in
   [server control](/documentation/glossary/n_to_z.md#server-control), registered at the
   host's address on game ports 2001 to 2005, each with a `mod_runtime` and a `host_agent`
   credential, and ten credential files of mode 600 in mode-700 folders:
   `~/tbd/fleet/instance-N/secrets/mod-runtime-credential` and `host-agent-credential`. No secret
   is printed. The work is the host tool's `staging-fixtures provision-fleet`, which writes every
   file before it commits the servers and refuses a server name already registered, a credential
   file that already exists, and a secrets folder open to group or others. Each executor claims
   only its own [fleet commands](/documentation/glossary/a_to_f.md#fleet-command): the
   [game server host agent](/documentation/glossary/g_to_m.md#game-server-host-agent) runs the unit and
   [RCON](/documentation/glossary/n_to_z.md#rcon) actions, and the
   [game runtime](/documentation/glossary/g_to_m.md#game-runtime) runs `broadcast`, `kick` and
   `load_mission` (the
   [fleet command ledger](/documentation/crates/api/api_server/verification_evidence/fleet_command_ledger.md#commands)
   lists every action and its executor).

2. On the host, write the join password every instance requires.

   ```bash
   umask 077; cat > ~/tbd/fleet/join-password
   ```

   Expected: `cat` waits for input; type the password, press Enter, then Ctrl-D.
   `stat -c %a ~/tbd/fleet/join-password` prints `600`. The password is 3 to 64 of
   `A-Z a-z 0-9 . _ ~ + = : @ % -`
   ([the fleet folder](/documentation/runbooks/game_server_staging/host_preparation.md#the-fleet-folder));
   hand it to the players, who type it when they join.

3. Deploy the fleet, retiring the single-instance server where one ran, together with its
   kebab-case host agent unit, configuration folder and binary.

   ```bash
   cargo xtask deploy staging --migrate-single-instance
   ```

   Expected: the stages of the staging deploy, ending with `==> deploy complete: 5 instance(s)`,
   exit 0. Before anything changes on the host, the deploy refuses a missing, open or malformed
   credential file or join password, one line per file and no value, then
   `FAIL: <n> secret file(s) under …/tbd/fleet are not ready.`

4. For an event: bind it to one instance's server. An event bound elsewhere answers the server's
   roster read with 403, and a deployment of its event mission here is refused. In
   `/admin/events`, select the event, choose "Edit Selected Operation", pick the server
   ("TBD Staging N") under "Game server" and "Save Changes", or call the route.

   ```bash
   curl -s -X PATCH -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"server_id":"<server uuid>"}' "$API/api/v1/events/<event uuid>"
   ```

   Expected: the updated event, with `server_id` set; an unknown server answers
   `server_id does not name a known server`.

5. Deploy the mission to one instance's server, from the server control page or the route; add
   `"event_mission_id"` for an event's mission. In game, a listed admin can do the same with
   `#tbd missions`, then `#tbd mission <n>`.

   ```bash
   curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"mission_id":"<mission uuid>","artifact_id":"<approved artifact uuid>"}' "$API/api/v1/servers/<server uuid>/deployments"
   ```

   Expected: the deployment record. An instance already on the mission's terrain loads the
   artifact and restarts the mission in process; another terrain needs the instance's host agent
   to restart it on that terrain's
   [mission header](/documentation/glossary/g_to_m.md#mission-header).

## Rotate a credential

A rotation replaces one instance's `host_agent` or `mod_runtime` credential without a gap: stage
the new one beside the live one, promote it, let the instance read it, then revoke the old one.

1. Stage a new credential.

   ```bash
   cargo xtask staging rotate-credential --instance <N> --executor <host_agent|mod_runtime> --stage
   ```

   Expected: a new credential of that executor issued to "TBD Staging <N>" and written beside the
   live file as `host-agent-credential.staged` or `mod-runtime-credential.staged` (mode 600); the
   live credential keeps working, and no secret is printed.

2. Promote it.

   ```bash
   cargo xtask staging rotate-credential --instance <N> --executor <host_agent|mod_runtime> --promote
   ```

   Expected: the staged file replaces the live one, once the host tool has checked that it holds
   an unrevoked credential of that server and executor.

3. Let the instance read it. The host agent reads its credential file when
   `game_server_host_agent@<N>.service` starts; the mod reads the copy in its profile's
   `TBD_BackendConfig.json`, which every staging deploy writes from the live file. A deploy covers
   both, because it rewrites every profile and restarts every host agent.

   ```bash
   cargo xtask deploy staging
   ```

   Expected: `==> deploy complete: 5 instance(s)`, exit 0, with the instance's line
   `V2 instance <N> game-runtime deployment: HTTP 200` (or `HTTP 404` before its first
   deployment). For a `host_agent` rotation alone, `systemctl --user restart game_server_host_agent@<N>.service`
   on the host is enough.

4. Revoke the old credential. No command: in `/admin/server`, select "TBD Staging <N>", open
   "Credentials", and "Revoke" the older credential of that program with a reason.

   Expected: the sheet lists the old credential as revoked, and the instance's commands and
   runtime session keep working on the new one.

The credentials sheet still issues and revokes the credentials of any registered server, showing
an issued secret once. A fleet instance reads its credentials only from its files under
`~/tbd/fleet/instance-N/secrets/`, never from `deploy.env`, which refuses the credential settings,
so a fleet instance takes a new credential through the rotation above.

## Verify

After the first fleet deploy, check one instance's game-runtime routes the way the
[mod](/documentation/glossary/g_to_m.md#mod) calls them, on the host, from the checkout
(`TBD_REMOTE_DIR`), with the instance's credential file:

```bash
TBD_MACHINE_CREDENTIAL="$(cat ~/tbd/fleet/instance-<N>/secrets/mod-runtime-credential)" cargo xtask mod test-game-runtime-api
```

Expected: one `ok` line per check and `game-runtime API: every check passed`, exit 0. The checks:
the deployment read answers 200 (or 404 `NO_DEPLOYMENT` before the first deployment); with a
deployment, the artifact's bytes hash to its `artifact_sha256` and its `ETag`, and an event's
roster answers in wire version 2; without the credential the deployment read answers 401. Exit 1
is a failed check, 2 a usage error, 3 an API that gave no answer. `TBD_API_BASE` overrides the
API origin, `http://127.0.0.1:8080` by default.

## What the mod does with the credential

The [game runtime transport](/mod/tbd-framework/Scripts/Game/TBD/API/README.md) lists every
call; these are the ones an operator meets.

- At boot the mod reads `GET /api/v1/game-runtime/deployment`: the deployment in flight, else
  the latest confirmed one. It fetches `GET /api/v1/game-runtime/artifacts/{artifactId}`, loads the
  bytes only when their SHA-256 equals `artifact_sha256`, and caches them in
  `$profile:TBD_MissionArtifactCache/`.
- It starts a runtime session (`POST /api/v1/game-runtime/sessions`) once the artifact has loaded
  and reports it, which confirms the deployment; a new session supersedes the server's previous
  one. Heartbeats follow every 15 seconds, and while a session is open the mod claims fleet
  commands every 5 seconds.
- For an event it reads `GET /api/v1/game-runtime/events/{eventId}/roster` and asks the platform
  before each player spawns into an event seat. Until the roster loads, every such spawn is
  refused and the player keeps the seat. A fetch with no answer is retried with backoff from 2 s
  to 60 s; a refusal (401, 403, 404, a wire version other than 2) is logged at ERROR once and
  retried every 60 s.
- The profile names no mission and no event: with no deployment the server runs no mission
  (ERROR), and with the platform unreachable at boot it runs the last verified cached artifact
  (WARNING) and reports it once a session starts.
- The backend config holds `backendUrl` and `machineCredential` only: the machine credential
  also authenticates `/api/v1/ingest/*`, including `/api/v1/ingest/link-confirm`.
- Every staging deploy rewrites each instance's
  `~/tbd/fleet/instance-N/profile/profile/TBD_BackendConfig.json` from its
  `mod-runtime-credential` file, so hand edits do not survive it. The mod re-reads the file while
  it waits on the platform, so a running server picks up a changed credential without a restart.

Check the roster the way an instance reads it, on the host; the credential reaches `curl` on its
standard input, never in an argument:

```bash
printf 'Authorization: Bearer %s\n' "$(cat ~/tbd/fleet/instance-<N>/secrets/mod-runtime-credential)" | curl -s -w '\n%{http_code}\n' -H @- "http://127.0.0.1:8080/api/v1/game-runtime/events/<event uuid>/roster"
```

Expected: `{"version":2,"eventId":…,"missionId":…,"assignments":[…],"slots":[…]}` and `200`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the deploy stops with `<path>:<line>: TBD_MOD_RUNTIME_CREDENTIAL: is no longer read (…); delete the assignment` | `deploy.env` still holds a credential setting | delete the line; the credential lives in the instance's `secrets/` folder |
| the deploy stops with `  MISSING instance <N> mod_runtime credential: …` (or `host_agent`, or `join password`) | the fleet is not provisioned, the join password is not written, or the file is open to group or others | step 1 or 2, or `chmod 600` on the file |
| `  INVALID instance <N> … credential: … does not hold the expected shape` | the file does not hold a `tbdm_<32 hex>_<64 hex>` credential | rotate it (Rotate a credential) |
| `provision-fleet` refuses | a server of that name is already registered, a credential file exists, or a secrets folder is open to group or others | secret files are never overwritten: rotate a credential instead; fix the folder's mode |
| the smoke or `mod test-game-runtime-api` answers 401 with the credential | the credential is revoked or belongs to another server | rotate it and deploy again |
| the roster answers 403 | the event is not bound to this instance's server | step 4 |
| "Register server" shows "The address must be a literal IPv4 or IPv6 address — not a hostname, and not a /mask" | a server registered by hand in server control names a hostname, or carries a `/mask` | enter the host's public IP address as digits, with no mask |
| `[TBD][Mission] NO MISSION - …` (ERROR) in an instance's log | nothing is deployed to its server; the instance stays in LOADING | step 5 |
| `[TBD][Runtime] runtime session loop STOPPED` (ERROR) | another runtime started a session with the same credential, or the credential was revoked or rejected | rotate the instance's `mod_runtime` credential and deploy again |
| every player hears that the event roster has not loaded on this server yet | the roster fetch fails; the `[TBD][Roster]` ERROR says why | fix the cause (no `machineCredential`, 403, 401); a running server retries on its own |
| a host command in server control stays `queued` until it expires | the instance's host agent does not run | `systemctl --user status game_server_host_agent@<N>.service` on the host; deploy again ([staging deploy](/documentation/runbooks/game_server_staging/staging_deploy.md#host-agent)) |

## Related

- [Server control page](/documentation/crates/frontend/pages/administration_pages/server_control/server_control_page.md)
  — the servers, and the credentials, deployments and fleet command panels.
- [Event manager page](/documentation/crates/frontend/pages/administration_pages/event_manager/event_manager_page.md)
  — binding an operation to its game server.
- [Staging fixtures host tool](/tools/staging/staging_fixtures/src/README.md) — the
  `provision-fleet` and `rotate-credential` subcommands and their guards.
- [Server infrastructure domain](/crates/api/api_server_infrastructure/src/README.md) — the
  credential, session and fleet command routes.
- [Fleet command execution](/documentation/crates/fleet/game_server_host_agent/fleet_command_execution.md) — how a
  command moves from `queued` to `succeeded`.
- [Boot and log verification](/documentation/runbooks/game_server_staging/boot_and_log_verification.md)
  — the log lines of each step above.
