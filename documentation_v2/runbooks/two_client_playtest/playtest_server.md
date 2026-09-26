**Status:** live

# Start the playtest server

Starts a joinable dedicated server on the development machine with `cargo xtask mod playtest`: it
stages a run folder, deploys the authored [mission](/documentation_v2/glossary/g_to_m.md#mission) through
the platform, boots the server with the checkout's `tbd-framework`, and prints the join details
only once a backend room registered and the local addon won. The admin list needs the player's
identity id, which the engine prints only when that player joins, so the server starts twice:
once without an admin, and again with the id. Each start takes a few minutes.

## Prerequisites

- The stack of [Stack and mission](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md)
  running, and the mission id as `MID`; or, with no API, a compiled document for
  `--artifact-file=<path>`, such as `contracts_v2/fixtures/missions/valid/bridgehead-at-levie.json`.
- The firewall open on TCP and UDP 2001 and UDP 17777, for example
  `sudo firewall-cmd --add-port=2001/tcp --add-port=2001/udp --add-port=17777/udp` (add
  `--permanent` to keep it; `firewall-cmd --list-ports` shows what is open).
- No other playtest server running from the same run folder.

## Steps

1. Render and validate everything without booting.

   ```bash
   cargo xtask mod playtest --mission=$MID --dry-run
   ```

   Expected: `==> staging <run dir>`, `[dry-run] would deploy mission <MID> through http://127.0.0.1:8080`,
   `rendered <run dir>/server.json (mods=[B2C3D4E5F6A78901] admins=[])`, a `NOTE: no --admin given`
   block, then `[dry-run] cd "<server dir>" && ./ArmaReforgerServer -addonsDir <run dir>/addons -config <run dir>/server.json -profile <run dir>/profile -maxFPS 60 -logStats 30000 -nothrow`
   and `[dry-run] would advertise: <lan address>:2001`; exit 0. A dry run provisions nothing on
   the platform.

2. Start the server without `--admin`. It stays in the foreground, following the log, until
   Ctrl-C.

   ```bash
   cargo xtask mod playtest --mission=$MID
   ```

   Expected: `    deployment <id> of artifact <id> on server <server id> (scenario <id>)`, the
   `NOTE: no --admin given` block (every `#tbd` command answers "TBD: admin only." until step 4),
   progress lines `    ... <s>s — <phase>`, then the banner:

   ```text
     SERVER UP — second client joins with:
       Multiplayer -> Direct Join -> <lan address>:2001
       or Direct Join Code:          <code>
   ```

   followed by `==> waiting for the runtime to confirm deployment <id>` and, within 180 s,
   `    CONFIRMED: runtime session <id> loaded artifact <id>`. Keep `<server id>` as `SID`: it is
   the "TBD Playtest" server row, the one an event must be bound to.

3. Join the server from the first client (session step S2 in
   [Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md)),
   then read your identity id from the log. The engine prints it when a client authenticates.

   ```bash
   grep -iE 'identityId' "$LOG" | tail -5
   ```

   Expected: your identity id, a lowercase UUID. `game.admins[]` accepts it or a 17-digit
   SteamID and nothing else; the playtest checks both engine patterns
   (`^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`, `^[0-9]{17}$`) before
   booting, because the engine rejects a bad entry as a fatal config error about 90 s into the
   boot.

4. Stop the server: Ctrl-C in its terminal.

   Expected: `==> stopping server`, `==> stopped (process group confirmed gone)` and
   `==> revoked this run's machine credential`. Wait for the second line before starting
   another server. A `STRAY SERVER` block instead means the stop could not be confirmed: run the
   command it prints, and check `pgrep -af '[A]rmaReforgerServer'` comes back empty, because a
   survivor holds 2001 and 17777 and the next boot dies on `Unable to start replication`.

5. With an [event](/documentation_v2/glossary/a_to_f.md#event), bind it to `SID` now (step 16 of
   [Stack and mission](/documentation_v2/runbooks/two_client_playtest/stack_and_mission.md)).
   Then restart with the admin id, and with the event mission when there is one; `--admin` is
   repeatable, so add the second player's id once they have joined.

   ```bash
   cargo xtask mod playtest --mission=$MID --event-mission=$EMID --admin=<your identity id>
   ```

   Expected: step 2's output without the `NOTE: no --admin given` block. The Direct Join Code is
   minted again on every boot, so read it from this run.

6. With an event, check the roster the game server reads, using the credential this run wrote
   into the backend config.

   ```bash
   curl -s -w '\n%{http_code}\n' -H "Authorization: Bearer $(sed -n 's/.*"machineCredential": *"\([^"]*\)".*/\1/p' "$HOME/tbd-playtest/profile/profile/TBD_BackendConfig.json")" "http://127.0.0.1:8080/api/v1/game-runtime/events/$EID/roster"
   ```

   Expected: `{"version":2,"eventId":"…","missionId":"…","assignments":[…],"slots":[…]}` and `200`.
   `403` means the event is not bound to this server, `401` a wrong or revoked credential.
   `assignments` is keyed on each user's linked game identity and stays empty until a player
   links theirs in game (session step S6); an empty list is legal and seats everyone
   round-robin.

## What the command does

`cargo xtask mod playtest` replaces the hand-assembled server: the
[playtest server README](/tools_v2/xtask/src/commands/mod_ops/playtest_server/README.md) holds its
full order.

- It stages `<run dir>` (default `$HOME/tbd-playtest`): the profile from
  `cargo xtask setup server-profile`, the backend config rewritten from
  `apps/mod/tbd-framework/Data/backend.example.json` on every start (`backendUrl`, `serverToken`,
  `machineCredential`; hand edits do not survive), `addons/tbd-framework` linking the checkout,
  and `server.json` rendered from `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`
  with the ports, `visible`, `maxPlayers`, `admins` and one mod entry keyed by the GUID in
  `apps/mod/tbd-framework/addon.gproj`.
- With `--mission`, it logs in through the [dev login](/documentation_v2/glossary/a_to_f.md#dev-login)
  as an administrator, takes the mission's approved [artifact](/documentation_v2/glossary/a_to_f.md#artifact)
  (submitting and approving the current version when there is none), makes sure Everon has its
  [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario), issues this run a `mod_runtime`
  [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential), and requests the
  [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment), bound to
  `--event-mission` when given. It revokes the credential when the server stops.
- With `--artifact-file`, it stages the document as the mod's last verified artifact instead: no
  API and no credential; the log then reads `source=last-verified-cache`.
- It launches with `-addonsDir` and `-config` together, the only combination that both loads the
  checkout and registers a joinable room
  ([launch flags and ports](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md#launch-flags-and-ports)).
- It waits up to 300 s for a verdict. `tbd-framework` is also published to the Workshop under
  the same id as the addon GUID, so the engine can load that copy instead; the engine's
  `Loaded addons:` block must name `<run dir>/addons/tbd-framework/addon.gproj`, or the run prints
  `FAILED: the STALE Workshop copy won, not your checkout.`, kills the server and exits 1.
- A second start while a server from the same run folder lives is refused and touches nothing.
  To run two at once, give the second its own `--run-dir`, `--port` and `--a2s-port`.
- Exit codes: 0 booted and joinable; 1 the server died, refused its config, loaded the wrong
  addon copy or could not be confirmed stopped; 2 usage; 3 environment.

## Verify

Once the banner is up, the log carries the registration and the mission:

```bash
grep -E 'Server registered with address|Direct Join Code|\[TBD\]\[Mission\] loaded|\[TBD\]\[Validate\] mission result=' "$LOG"
```

Expected: `Server registered with address: <lan address>:2001` with the machine's LAN address,
not a loopback or container one; `Direct Join Code: <code>`;
`[TBD][Mission] loaded id=<id> name='…' slots=<N> source=platform` (or `source=cache`, the
deployment's own artifact from the profile cache); and
`[TBD][Validate] mission result=PASS errors=0 warnings=<W>`. From the second player's machine,
`ping -c2 <lan address>` must answer.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `FAILED: the world came up but the server never registered a backend room.` with the fingerprint `Attempting online Game Config instead.` | a registration hang: the backend handshake stalled; the engine logs no error for it | run the same command again; registration time varies from seconds to never within one boot |
| `FAILED: the STALE Workshop copy won, not your checkout.` | the engine loaded the Workshop copy under the addon GUID | delete the cached copy the run names, then start again |
| `FAILED: the engine refused the config or could not start.` | a config value the engine rejects, such as an admin id | check the `--admin` values and the dump of engine errors |
| `could not provision the deployment on the platform`, exit 3 | the API is down or not in development mode | start the stack, or boot offline with `--artifact-file` |
| `[TBD][Mission] NO MISSION YET` | no credential, no answer, a refusal or a SHA-256 mismatch; the line names the cause and the mod retries | read the `[TBD][Mission]` lines; with `--mission`, check the deployment in `/admin/server` |
| `[TBD][Mission] NO MISSION -` | nothing is deployed to this server | start with `--mission` |
| `[TBD][Validate] mission result=FAIL` | the mission failed validation; the server stays in LOADING | `#tbd validate` in chat replays the findings |
| about 79 vanilla `DEFAULT`/`MATERIAL`/`RESOURCES` `(E)` lines | the vanilla floor of every boot | ignore; the run counts and suppresses them |
| Direct Join answers "No server found" | no `Server registered with address:` line in this boot | see the first row; [join diagnosis](/documentation_v2/runbooks/game_server_staging/client_join_and_mod_updates.md) |

## Related

- [Playtest server code](/tools_v2/xtask/src/commands/mod_ops/playtest_server/README.md) — the
  staging order, the boot verdict and the lifecycle guards.
- [Session: join to deploy](/documentation_v2/runbooks/two_client_playtest/session_join_to_deploy.md)
  — the next runbook.
- [Boot and log verification](/documentation_v2/runbooks/game_server_staging/boot_and_log_verification.md)
  — the same boot verdict and log lines on the staging server.
- [Staging deploy: host agent](/documentation_v2/runbooks/game_server_staging/staging_deploy.md#host-agent)
  — the optional fleet host agent behind the Server Control start, stop and restart commands; a
  playtest server is not a systemd unit, so the session does not need it.
- [Two-client playtest](/documentation_v2/runbooks/two_client_playtest/README.md) — the index.
