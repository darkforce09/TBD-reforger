**Status:** live

# Give the staging server its credentials and a mission

Connects the staging game server to the platform: its
[machine credentials](/documentation_v2/glossary/g_to_m.md#machine-credential), the
[event](/documentation_v2/glossary/a_to_f.md#event) it serves, and the
[mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) it runs. The credentials are
issued once per server; binding an event and deploying a
[mission](/documentation_v2/glossary/g_to_m.md#mission) repeat whenever either changes.

## Prerequisites

- The API running on the staging host
  ([host preparation](/documentation_v2/runbooks/game_server_staging/host_preparation.md)).
- An administrator login on the website, and the staging server registered in
  [server control](/documentation_v2/glossary/n_to_z.md#server-control) (`/admin/server`); its server
  uuid is on its card.
- An approved [artifact](/documentation_v2/glossary/a_to_f.md#artifact) of the mission to deploy.
- For the `curl` forms: `$ADMIN_TOKEN`, an administrator's access token, and `$API`, the API
  origin (`http://127.0.0.1:8080` on the host).

## Steps

1. Issue the server's `mod_runtime` credential: in `/admin/server`, select the server, open its
   credentials and issue one for "Game runtime", or call the route.

   ```bash
   curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"executor_kind":"mod_runtime","label":"staging runtime"}' "$API/api/v1/servers/<server uuid>/credentials"
   ```

   Expected: `201` with `credential` and `secret`. The secret, shaped
   `tbdm_<32 hex>_<64 hex>`, is shown this once. Put it in `tools_v2/xtask/deploy/deploy.env` as
   `TBD_MOD_RUNTIME_CREDENTIAL`; every deploy writes it into
   `$TBD_PROFILE_DIR/profile/TBD_BackendConfig.json` as `machineCredential`, and the deploy
   refuses to run without a well-formed one.

2. Only when the deploy installs the [fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent)
   (`TBD_INSTALL_HOST_AGENT=1`): issue its separate `host_agent` credential ("Host agent" in the
   credentials panel).

   ```bash
   curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"executor_kind":"host_agent","label":"staging host agent"}' "$API/api/v1/servers/<server uuid>/credentials"
   ```

   Expected: `201` with a second secret. Put it in `deploy.env` as `TBD_HOST_AGENT_CREDENTIAL`.
   Each executor claims only its own [fleet commands](/documentation_v2/glossary/a_to_f.md#fleet-command):
   the host agent starts, stops and restarts the unit and lists players over
   [RCON](/documentation_v2/glossary/n_to_z.md#rcon); the
   [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) runs `broadcast`, `kick` and
   `load_mission`.

3. For an event: bind it to this server. An event bound elsewhere answers the server's roster
   read with 403, and a deployment of its event mission here is refused.

   ```bash
   curl -s -X PATCH -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"server_id":"<server uuid>"}' "$API/api/v1/events/<event uuid>"
   ```

   Expected: the updated event, with `server_id` set; an unknown server answers
   `server_id does not name a known server`.

4. Deploy the mission to the server, from the server control page or the route; add
   `"event_mission_id"` for an event's mission. In game, a listed admin can do the same with
   `#tbd missions`, then `#tbd mission <n>`.

   ```bash
   curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" -H 'Content-Type: application/json' -d '{"mission_id":"<mission uuid>","artifact_id":"<approved artifact uuid>"}' "$API/api/v1/servers/<server uuid>/deployments"
   ```

   Expected: the deployment record. A server already on the mission's terrain loads the artifact
   and restarts the scenario in process; another terrain needs the host agent to restart the
   server on that terrain's scenario.

## Verify

After the first [staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md),
check the game-runtime routes the way the [mod](/documentation_v2/glossary/g_to_m.md#mod) calls them, from any machine that reaches the API.

```bash
TBD_API_BASE="$API" TBD_MACHINE_CREDENTIAL='<mod_runtime secret>' cargo xtask mod test-game-runtime-api
```

Expected: one `ok` line per check and `game-runtime API: every check passed`, exit 0. The checks:
the deployment read answers 200 (or 404 `NO_DEPLOYMENT` before the first deployment); with a
deployment, the artifact's bytes hash to its `artifact_sha256` and its `ETag`, and an event's
roster answers in wire version 2; without the credential the deployment read answers 401. Exit 1
is a failed check, 2 a usage error, 3 an API that gave no answer.

## What the mod does with the credential

The [game runtime transport](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) lists every
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
- The service token (`serverToken`, from `TBD_GAME_SERVER_TOKEN`) serves only
  `/api/v1/ingest/link-confirm` and `/api/v1/ingest/match-results`.
- The deploy rewrites `TBD_BackendConfig.json` on every run, so hand edits do not survive it. The
  mod re-reads the file while it waits on the platform, so a running server picks up a changed
  credential without a restart.

Check the roster the way the server reads it, on the host, with `CREDENTIAL` set to the server's
`mod_runtime` secret:

```bash
curl -s -w '\n%{http_code}\n' -H "Authorization: Bearer $CREDENTIAL" "http://127.0.0.1:8080/api/v1/game-runtime/events/<event uuid>/roster"
```

Expected: `{"version":2,"eventId":…,"missionId":…,"assignments":[…],"slots":[…]}` and `200`.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| the deploy stops with `TBD_MOD_RUNTIME_CREDENTIAL is not a machine credential (tbdm_<32 hex>_<64 hex>)` | the value is truncated or not a platform credential (an empty one stops at `deploy.env: line 35: TBD_MOD_RUNTIME_CREDENTIAL: …`) | step 1, then copy the whole secret |
| the smoke or `mod test-game-runtime-api` answers 401 with the credential | the credential is revoked or belongs to another server | issue a fresh one (step 1) and deploy again |
| the roster answers 403 | the event is not bound to this server | step 3 |
| `[TBD][Mission] NO MISSION - …` (ERROR) in the server log | nothing is deployed to this server; the server stays in LOADING | step 4 |
| `[TBD][Runtime] runtime session loop STOPPED` (ERROR) | another runtime started a session with the same credential, or the credential was revoked or rejected | give each server its own credential, then restart the game server |
| every player hears that the event roster has not loaded on this server yet | the roster fetch fails; the `[TBD][Roster]` ERROR says why | fix the cause (no `machineCredential`, 403, 401); a running server retries on its own |
| a host command in server control stays `queued` until it expires | no host agent claims it | install the agent ([staging deploy](/documentation_v2/runbooks/game_server_staging/staging_deploy.md)) |

## Related

- [Server control page](/documentation_v2/website/frontend/pages/administration/server_control/server_control_page.md)
  — the credentials, deployments and fleet command panels.
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — the
  credential, session and fleet command routes.
- [Fleet command execution](/documentation_v2/fleet_host_agent/fleet_command_execution.md) — how a
  command moves from `queued` to `succeeded`.
- [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)
  — the log lines of each step above.
