**Status:** live

# Verify a staging boot from the server's log

Two checks read a dedicated server's `console.log`. The boot verdict,
`cargo xtask deploy staging --verify-boot`, says whether the server runs the checkout that was
deployed, registered a joinable room and accepted its admins. The log verdict,
`cargo xtask mod remote-logs`, says whether the [mod](/documentation_v2/glossary/g_to_m.md#mod) booted
healthy and seated a player. A deploy runs both; this runbook runs them by hand, over the staging
server's newest log or any saved one. Each takes seconds.

## Prerequisites

- For the remote log: `TBD_SSH_HOST` and `TBD_PROFILE_DIR`, in the environment or in
  `tools_v2/xtask/deploy/deploy.env`; here the environment wins. The server writes its log to
  `$TBD_PROFILE_DIR/logs/logs_<time>/console.log` (`mod remote-logs` also looks under
  `$TBD_PROFILE_DIR/profile/logs/`).
- For the boot verdict: a local copy of the log, and the `-addonsDir` path the server ran with.

## Steps

1. Prove the boot verdict can fail. It needs no log and no deploy settings.

   ```bash
   cargo xtask deploy staging --verify-boot-selftest
   ```

   Expected: `==> boot verdict selftest (local only, no deploy, no ssh)`, fifteen `  PASS  …`
   lines (a Workshop copy that won must fail, the checkout that won must pass, addons mode must
   fail the room check, a missing log must fail, a rival pak found on disk is reported, and so
   on), then `BOOT VERDICT SELFTEST: 15 passed, 0 failed`, exit 0.

2. Run the boot verdict over a log. `TBD_ADDONS_STAGING` is required; `TBD_ADMIN_COUNT` (default
   0), `TBD_PROFILE_DIR` and `TBD_ADDON_GUID` (default: the gproj's GUID) are optional.

   ```bash
   TBD_ADDONS_STAGING=<addonsDir> TBD_ADMIN_COUNT=<n> cargo xtask deploy staging --verify-boot <console.log>
   ```

   Expected: `==> boot verdict: <console.log>`, then
   `  PASS  deployed checkout won: <addonsDir>/tbd-framework/addon.gproj`,
   `  PASS  backend room registered: Server registered with address: …`,
   `  PASS  server config accepted by the engine, carrying <n> admin id(s)`, one `  NOTE` line
   about the rival Workshop copy, and `BOOT VERDICT: PASS`, exit 0. A failed assertion prints
   `FAIL: …` and `BOOT VERDICT: FAILED`, exit 1; without `TBD_ADDONS_STAGING` it exits 2.

3. Prove the log verdict can fail.

   ```bash
   cargo xtask mod remote-logs --selftest
   ```

   Expected: five lines `ok   selftest <case> -> <code>` (`stale-1.0.1-must-fail -> 1`,
   `healthy-no-player-is-partial -> 2`, `mission-invalid-must-fail -> 1`,
   `seated-flat-format-is-pass -> 0`, `seated-tagged-format-is-pass -> 0`), then `SELFTEST: PASS`,
   exit 0.

4. Run the log verdict over the staging server's newest log.

   ```bash
   cargo xtask mod remote-logs
   ```

   Expected: `Remote log: <host>:<path>`, the last 80 mod and error lines between `---` rulers,
   `[TBD][ tagged lines: <n>`, one `ok   <check>` line per required line, and a verdict:
   `VERDICT: PASS — boot healthy and at least one player was seated.` (exit 0) or
   `VERDICT: PARTIAL — boot healthy, no player has joined yet (join a client to finish V6).`
   (exit 2).

5. To judge a log you already have, without SSH, point the same verdict at the file.

   ```bash
   cargo xtask mod remote-logs --file <console.log>
   ```

   Expected: as in step 4, without the `Remote log:` line.

## Verify

Read the exit code, never `!= 0`. `mod remote-logs` has four outcomes, and so do
`cargo xtask mcp wb-logs` and `cargo xtask mod spawn-verify`:

| Exit | Meaning | Deploy |
|---|---|---|
| 0 | PASS: healthy boot and a player was seated | passes |
| 2 | PARTIAL: healthy boot, nobody joined yet; the normal state right after a deploy | passes |
| 1 | FAIL: a required line is missing, or an error class is present; an unset `TBD_SSH_HOST` or `TBD_PROFILE_DIR` also exits 1 | fails |
| 3 | ENVIRONMENT: no log was examined, so it says nothing about the mod | fails |

## What each verdict proves

The boot verdict's three assertions, each over the last matching block of the log:

- **The checkout won.** Among the engine's `Loaded addons:` lines, the entry with
  `guid: '<GUID>'` must name `<addonsDir>/tbd-framework/addon.gproj`. A Workshop copy that won
  names `<profile>/addons/TBDFramework_<GUID>/addon.gproj` with the same GUID and an equally
  healthy log, so only the path tells them apart.
- **A room registered.** A `Server registered with address:` line. Addons mode reaches LOBBY with
  the right mod and never registers one, so Direct Join answers "No server found".
- **The config was accepted.** `Server config loaded.` and `JSON is Valid`. With an admin count of
  0 it passes with a `WARN` that every `#tbd` command will answer "TBD: admin only.".

It then says whether the win was a contest: a Workshop pak mounted or downloaded this boot, or one
on the host's disk at `<profile>/addons/TBDFramework_<GUID>/data.pak` that lost; otherwise
`WEAK EVIDENCE`, because there was nothing to beat. A deploy measures that pak on the host; a
local `--verify-boot` looks for it under the local `TBD_PROFILE_DIR`.

The log verdict fails on a log with no `[TBD][` line (`FAIL: STALE BUILD`), on a missing
`[TBD][Mission] loaded id=`, `[TBD][Slots] Slot-` or LOBBY stage line, and on any
`Can't compile`, `Unknown class TBD_…` or `RequestSpawn failed`; it passes with a seated player
and is PARTIAL without one. Below `TBD_MIN_TAGGED` tagged lines (default 20) it only warns.

The two answer different questions. A zero `[TBD][` count means an old build that predates the
tagged format; a current Workshop copy prints the same tagged lines as the checkout, so the count
cannot tell which of the two runs. Only the boot verdict's gproj path answers that. Never turn the
tagged-line count into a pass threshold: it varies by mission and grows as the mod gains lines.

## Log lines to match

Match the stable prefix, never a whole sentence: everything after it varies with the
[mission](/documentation_v2/glossary/g_to_m.md#mission) and the outcome, and a prefix stays stable only up to its last tag or `key=`.

| Expect | Prefix | Printed by |
|---|---|---|
| mission document loaded | `[TBD][Mission] loaded id=` | `TBD_Log.MissionLoaded` |
| mission passed validation | `[TBD][Validate] mission result=PASS` | `TBD_Log.ValidationResult` |
| registry aliases loaded | `[TBD] Registry loaded` (flat format) | `TBD_Registry.c` |
| one line per slot body | `[TBD][Slots] Slot-` | `TBD_SpawnManager.c` |
| all bodies materialized | `[TBD][Slots] materialized` | `TBD_SpawnManager.c` |
| loadouts applied | `[TBD][Loadout][Slot]` | the slot-body loadout pass |
| spawn opened | `[TBD][Slots] loadout settle` | `TBD_SpawnManager.c` |
| reached LOBBY | `[TBD][Stage]` … `LOBBY`, or the flat `[TBD] Stage →` … `LOBBY` | `TBD_Log.Stage`, `TBD_FrameworkManager.c` |
| a player was seated | `[TBD] SpawnManager: assigned slot` | `TBD_SpawnManager.c` |

The loadout tag is `[TBD][Loadout][Slot]`; no line prints `[TBD][Loadout][Player]`. A mission
that authors no loadouts prints no `[Loadout][Slot]` line, which `mod remote-logs` notes and does
not fail.

The [game runtime](/documentation_v2/glossary/g_to_m.md#game-runtime) lines of a server with a
[machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential):

| Expect | Prefix | When |
|---|---|---|
| no credential yet | `[TBD][Runtime] no runtime session until a machine credential is configured` | legal on a host without one |
| session held | `[TBD][Runtime] session-started session=` | a credential is configured and the artifact loaded |
| session lost | `[TBD][Runtime] runtime session loop STOPPED` (ERROR) | another runtime took over, or the credential was revoked; restart after fixing |
| SHA-256 self-test | `[TBD][Sha256] self-test-passed vectors=` | once per process; `self-test FAILED` (ERROR) means every artifact will be refused |
| deployment read | `[TBD][Mission] deployment deployment=` | a [mission deployment](/documentation_v2/glossary/g_to_m.md#mission-deployment) exists |
| artifact verified | `[TBD][Mission] artifact-verified artifact=` … `from=` | the bytes hashed to the published digest |
| mission loaded | `[TBD][Mission] loaded id=` … `source=platform` (or `cache`, `last-verified-cache`) | the artifact parsed and validated |
| platform down at boot | `[TBD][Mission] RUNNING THE LAST VERIFIED ARTIFACT` (WARNING) | the cached artifact runs |
| nothing deployed | `[TBD][Mission] NO MISSION -` (ERROR) | the server stays in LOADING |
| not loaded yet | `[TBD][Mission] NO MISSION YET -` (ERROR, once per cause) | no credential and no cache, no answer, a refusal or a SHA-256 mismatch; retried |
| fleet commands | `[TBD][Fleet]` with `claimed`, `executing`, `reported` or `ABANDONED` | a command for this runtime; `load_mission` ends in a restart line |
| roster loaded | `[TBD][Roster] loaded event=` | the deployment names an [event](/documentation_v2/glossary/a_to_f.md#event) |
| roster after seating | `[TBD][Roster] slot-table-loaded event=` | it arrived after seating settled |
| no event | `[TBD][Roster] the running mission is deployed for no event` | not a failure |
| roster refused | `[TBD][Roster] roster of event` … `not loaded` (ERROR) | 401, 403, 404, wrong wire version or no credential; retried every 60 s |
| roster unreachable | `[TBD][Roster] roster fetch failed` | network or 5xx; retried with backoff |
| spawn authorization | `[TBD][Deployment]` with `authorization-requested`, `deployment-allowed`, `deployment-denied`, `deployment-refused`, `life-ended`, `life-end-reported` | a player deploys into, or leaves, an event seat |

Refusals also reach the admin audit trail (`#tbd audit`, the admin screen) as `DEPLOYMENT: …`
entries.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `FAIL: STAGING IS VALIDATING A BUILD IT DID NOT DEPLOY.` | a Workshop copy won: the unit lacks `-addonsDir` | on the host, `systemctl --user cat tbd-reforger.service` must show `-addonsDir` and `-config`; redeploy in config mode |
| `FAIL: the engine never reported loading addon <GUID> at all.` | neither copy loaded | check the link `TBD_ADDONS_STAGING/tbd-framework` and the rsync |
| `FAIL: no backend room registered — the server is NOT joinable.` | addons mode, or the boot died before registering | use config mode; read the log's `(E)` lines |
| `FAIL: the engine did not report the server config as schema-valid.` | a config value the engine rejects; the listed `RegEx Pattern` lines name it | fix the setting in `deploy.env` and redeploy |
| `FAIL: STALE BUILD — zero '[TBD][' lines.` | an old Workshop build ran | boot with `-addonsDir` (config mode does) |
| `MISSING: mission document loaded` | no mission loaded: nothing deployed, or the artifact was refused | read the `[TBD][Mission]` lines; [deploy a mission](/documentation_v2/runbooks/game_server_staging/machine_credentials_and_mission_deployment.md) |
| `ENVIRONMENT: no console.log found under …` | wrong `TBD_PROFILE_DIR`, SSH failed, or the server never wrote a log | fix the setting or the host, and run again |
| `--verify-boot needs TBD_ADDONS_STAGING …`, exit 2 | the boot verdict needs the `-addonsDir` path | export it, as in step 2 |

## Related

- [Remote log verdict code](/tools_v2/xtask/src/commands/debug/remote_logs/README.md) — the
  patterns and the four outcomes.
- [Staging deploy code](/tools_v2/xtask/src/commands/deploy/staging/README.md) — where the boot
  verdict runs in the deploy.
- [Spawn determinism](/documentation_v2/runbooks/spawn_determinism.md) — the spawn checks over
  Workbench logs.
- [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md) — the index.
