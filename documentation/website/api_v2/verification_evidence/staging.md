**Status:** live

# Staging acceptance: fleet, Discord and load receipts

Design for the three operational requirements of the acceptance register: `staging_fleet`,
`staging_discord` and `staging_load`, and for the Discord identity requirements that also cite
`staging_discord` (`identity_discord_rest_reconciliation`, `identity_discord_resilience`,
`identity_discord_revocation`). An operational check has no command in the register: an external
runner observes the staging environment and writes a receipt, and `cargo xtask verify
api-readiness` judges that receipt against the thresholds in
`tools/xtask/src/verifications/api_readiness/operational.rs`. This note fixes how the runner
observes, what it records and when a receipt may pass. Acceptance evidence is the receipt set in
`target/api-readiness/` and the execution record in `progress_checkpoint.md`.

## Operator decisions (2026-09-29)

| Topic | Decision |
|---|---|
| Staging stack | The stack on the staging host `dooley` (public site `tbd.icanteam.com`) is pre-launch staging; experiments run with cleanup |
| Game servers | Five real dedicated-server instances, all on `dooley`, on the Experimental server app 1890870, updated before the runs |
| Game clients | One (the operator). `staging_fleet` runs every scenario one client allows and records a failing receipt naming the missing second client |
| Discord | The existing application's bot; the real TBD guild as the main guild and a partner guild owned by the operator. No subject accounts: `role_demotion` and `departure_to_guest` are recorded as not run; the operator's own account carries the other scenarios, and its Command Staff role is never touched |
| Faults | Outage: an `HTTPS_PROXY` blackhole drop-in on the API unit. Rate limit: the real Get Guild Member bucket spent with the bot token. Lost acknowledgement: a loopback relay that withholds one executor answer |
| Console | A new fleet action `console_command`, sent over RCON at most once |
| Load | The workstation drives Caddy on the LAN from five source addresses; 1,100 synthetic verified members seeded on the host and deleted afterwards; no traffic through the tunnel |
| Actions | Every administrator or member action is made in the real web UI by the orchestrator through the operator's browser; the harness holds no session of the operator |
| Witnessing | Automated only: a fact without machine evidence is recorded as missing |
| Confirmation | One operator approval per procedure run, over a numbered list of its real actions |

## Topology

| Part | Where | Identity recorded in each receipt |
|---|---|---|
| API | user unit `tbd-website-api` on `dooley`, release build, `APP_ENV=production`, loopback :8080 | binary SHA-256, `tbd_build_info`, migration head |
| Proxy | Caddy container `tbd_staging_caddy`, host network, :3080; Cloudflare tunnel for the public name | Caddy version, trusted proxies |
| Database | Postgres container `tbd_staging_db` (`tbd_reforger`) | server version |
| Game servers | units `tbd-reforger@1` … `@5`: game port 2000+N, A2S 17776+N, RCON 19998+N on loopback | app 1890870 build id, game version from `console.log`, Workshop mod version |
| Host agents | units `fleet-host-agent@1` … `@5`, one per instance; instance 5 reaches the API through `acknowledgement-dropping-relay@5` on 127.0.0.1:18085 | agent binary SHA-256 |
| Load generator | the workstation, wired, 192.168.0.117 plus 192.168.0.240–243 | CPU, memory, interface, round-trip time |
| Discord | the TBD guild (main) and the partner guild; the bot of the platform's application | guild ids, bot application id |

Instances 2–5 are hidden from the server browser, and every instance takes a join password from a
file on the host, so only the planned client joins. Each instance has its own profile folder and
its own two machine credentials in mode-600 files under `~/tbd/fleet/instance-N/secrets/`; nothing
secret leaves the host.

## Evidence model

### The receipt

`tools/xtask/src/verifications/api_readiness/operational_recording.rs` records every operational
run. At the start it reads the register, snapshots the source and configuration fingerprints, the
start time and the tool versions. At the end it:

1. recomputes both fingerprints; a drift fails the run and the receipt keeps the start digests, so
   the judge rejects it too;
2. considers a pass only when every declared case is `ok`, no case is `NOT RUN`, the observations
   meet `operational.rs` and the run stayed inside the check's timeout;
3. runs the judge itself on the finished receipt and log, and turns any rejection into a failure
   with the judge's reason;
4. publishes `<check>.log`, `<check>.fixture.json` and `<check>.json` into `target/api-readiness/`
   atomically, and exits 0 only on a pass.

A failing receipt exits 1, carries no success marker, keeps the real observations (for example a
client count of 1 and only the scenarios actually observed) and names every missing dependency.
The same command records it again once the dependency exists.

### The log

```text
staging-run: <check> run=<id> started=<unix> command=<argv>
environment: <key>=<value>
fixture: sha256=<hex> manifest=<check>.fixture.json
observation: <step> <observer> <summary> sha256=<raw artifact digest>
case <check>_<name> ... ok | FAILED (<why>) | NOT RUN (missing: <dependency>)
missing: <dependency>
<check>: PASS <ok>/<declared>
<check>: FAIL <ok>/<declared> (<reasons>)
```

Case names are lower-case words joined by `_`. Only the `ok` form matches the register's case
pattern, and the `PASS` line is written only on full acceptance.

### The fixture manifest

`fixture_sha256` is the SHA-256 of the manifest: the procedure definition (steps, deadlines,
declared cases), the server ids, the mission and artifact digests, the fleet scenario rows, the
guild ids, the Workshop version and the digests of the committed data files. `workload_sha256` for
the load check is the SHA-256 of the two committed workload files, each length-framed.

### Binding a run to the tree

The receipt binds to the source fingerprint (tracked and untracked source files under `apps/`,
`tools/`, `contracts/`, `.github/`, `.cargo/` and this folder) and to the configuration
fingerprint (the local `.env` files, `tools/xtask/deploy/deploy.env` and the verifying process's
environment, PATH and CARGO_TARGET_DIR included). Hence:

- every recording and every verification runs as `hcargo xtask …` from the repository root, which
  fixes PATH and CARGO_TARGET_DIR;
- the code, the register, this note and `deploy.env` are final and committed before the first
  recording, and nothing changes them until `verify api-readiness --execute` has finished;
- the three recordings and `--execute` fit inside 24 hours of the first recording's start.

### The journal and the browser inbox

Each run keeps a JSONL journal and every raw artifact with its SHA-256 under
`target/staging/<check>/<run>/`. Observations taken in the operator's browser (the response body of
the single-page app's own `/api/v1/me` poll, the text of the staleness banner) are saved unchanged
from the browser tool into `browser_inbox/<step>.json` with their capture time; the harness accepts
an entry only inside its step's window and labels it a browser page read. The access token is never
read.

## Witness rules

- Observers: read-only database transactions over SSH (`default_transaction_read_only=on`, only
  committed queries), unit journals, game-server console logs, the API's `/metrics` read on the
  host, bot member reads on the host, and browser page reads.
- The harness never waits for a person to type: every step waits for a machine-observable effect
  with a deadline. A deadline counts from the observed request row (for example
  `fleet_commands.requested_at`), so the time an approval takes never fails a step.
- A staged precondition (the aged membership snapshot of the Discord procedure) is recorded as such
  in the receipt's environment, the fixture manifest and its own observation, and never presented
  as elapsed time.
- A fact the observers cannot see is recorded as missing, never assumed.

## Fleet procedure (`staging_fleet`)

The orchestrator issues each command in Server Control; the harness waits for the effect. Waves
run on all five servers at once. The run stops itself at 6,900 s, inside the check's 7,200 s.

| Wave | Action | Effect observed | Deadline |
|---|---|---|---|
| W1 | Stop | command `succeeded`; unit inactive | 150 s |
| W2 | Start | unit active; a new runtime-session generation heartbeats; the previous one has ended (`expired`, `ended_by_runtime` or `superseded`) | 240 s |
| W3 | Restart | as W2 | 240 s |
| W4 | Console `#players` | the response is in the command outcome | 60 s |
| W5 | List players | outcome recorded | 60 s |
| W6 | Deploy the Everon mission | same-terrain transition (`scenario_restart`): `load_mission` succeeds in the running process; deployment confirmed | 600 s |
| W7 | Deploy the Arland mission | cross-terrain transition (`host_restart`): the mission header in the config changes; new process and session; deployment confirmed | 1,200 s |
| W8 | Deploy the Everon mission again | as W7 | 1,200 s |
| W9 | Server 1: the operator joins, requests a link code on the site and types `#tbd link <code>` | link status linked; `identity.link` audit row; the linked Arma id equals the id in the player list | — |
| W10 | Kick the operator; then a kick naming an ended session | kick outcome and mod log line; then a 409 `RUNTIME_SESSION_ENDED` refusal | — |
| W11 | Instance 1 host-agent credential: new credential staged on the host, the old one revoked in Server Control, the agent switched | the old credential answers 401; claims succeed with the new one | — |
| W12 | Instance 2 game-runtime credential: the same | the session ends `credential_revoked`; a new generation heartbeats | — |
| W13 | Relay drops the next claim answer; Restart server 5 | the claim lease lapses; the command is requeued with fencing token 2; the unit starts exactly once | — |
| W14 | Relay drops the next result answer; Restart server 5 | the retried result gets 409 with state `succeeded`; exactly one start | — |

Declared cases (50): for each server N, `serverN_stop`, `serverN_start`, `serverN_restart`,
`serverN_custom_console`, `serverN_list_players`, `serverN_same_terrain`, `serverN_cross_terrain`
and `serverN_runtime_session_succession` (the generation rises by one, the predecessor carries a
recorded end reason, never two open sessions); then `identity_link`, `kick`,
`kick_targets_one_of_two_clients`, `same_terrain_carries_two_clients`,
`lost_acknowledgement_claim_response`, `lost_acknowledgement_result_response`,
`machine_credential_rotation_host_agent`, `machine_credential_rotation_mod_runtime`,
`rejection_ended_session_kick` and `return_to_origin_terrain`. The two cases that need a
second client are `NOT RUN (missing: second game client)`.

The observations name the judge's scenarios: `start`, `stop`, `restart`, `custom_console`,
`same_terrain` and `cross_terrain` only when all five servers passed the case; `kick` and
`identity_link` from their cases; `lost_acknowledgement` when both lost-acknowledgement cases
passed. `client_count` is the largest number of distinct Arma ids in one player list.

## Discord procedure (`staging_discord`)

Preconditions: the bot token set on the host, a healthy reconciliation of the operator's
membership, the partner guild with its role "Partner Member" held by the operator, the bot invited
to both guilds, and a database backup.

| Step | Action | Cases and evidence |
|---|---|---|
| 1 | Create a partner-only event and its partner group; register (refused `MEMBERSHIP_VERIFICATION_REQUIRED`, which enrols the partner snapshot); register again | `partner_membership`: a partner-guild snapshot row; the site role unchanged |
| 2 | Remove the operator's partner role in Discord | `eligibility_release`: the reservation released `eligibility_lost` with an `event.reservation_released` audit row; `partner_role_propagation_within_60_seconds`: the release follows the first bot read that shows the loss within 60 s |
| 3 | Install the outage drop-in and restart the API | `network_outage`: reconciliation outcomes `unavailable`, `last_error` set, membership unchanged |
| 4 | Read `/me` and the banner in the browser; read `/events` | `staleness_warning`, `cached_grace` (the cached role still applies), `non_blocking_during_outage` |
| 5 | Age the operator's snapshot to 49 h (staged precondition); read `/me` (Guest); grant the override from the banner form (48 h) | `admin_override`: the override row, the `membership.grace_extended` audit row, `/me` with the override active and the role restored |
| 6 | Remove the drop-in, restart the API, restore the partner role | `outage_recovery`: a fresh `verified_at` |
| 7 | Spend the Get Guild Member bucket 0.3 s before the next refresh (at most 50 requests; the start is computed on the host from `next_refresh_at`) | `rate_limit`: the reconcile log line with outcome `rate_limited` and the outcome counter above its baseline (the snapshot's `last_error` and the pushed-back schedule last too briefly to observe reliably); `rate_limit_recovery` |
| 8 | Delete the partner event | cleanup |

Declared cases (13): the eleven above plus `role_demotion` and `departure_to_guest`, both
`NOT RUN (missing: test subject Discord account)`.

## Load procedure (`staging_load`)

**Target.** `http://192.168.0.129:3080` (Caddy, then the API) from the workstation; the tunnel is
never loaded. The API limits each client address to 20 requests a second (burst 40) and to 1 a
second on `/api/v1/auth/`, so the 100 virtual clients are spread over five source addresses and
each address is held under 8 requests a second and 0.5 refreshes a second.

**Population.** A host tool seeds 1,100 synthetic accounts in a reserved id range as verified
members with the Player role, each with a refresh-only session, and then ten fixture events titled
`[Load fixture]` with 128 slots each (two factions of eight squads of eight), attached to the
approved Everon staging mission and starting 14 days ahead; cleanup removes the events before the
accounts. Seeding refuses while a bot token is set, because the
reconciler would ask Discord about every synthetic account and demote it to Guest.

**Workload.** `tools/xtask/staging/load_workload.json` and `load_population.json` hold the seed,
a 60 s ramp, 1,800 s of measurement, 100 clients, 5 addresses, a target of 27 requests a second, the
request mix and the expected statuses. The mix is about 80 % JSON reads (events, event hub,
missions, dashboard, leaderboards, wiki, vehicles, announcements, `/me`, servers, ballistics
catalogs) and 20 % writes (bookmark toggles, register and withdraw on the account's own slot,
fire-mission saves). Each client owns eleven accounts, switches every 160 s and refreshes through
`POST /api/v1/auth/refresh`; refreshed tokens live only in memory.

**Measurement.**

- Open loop: each client fires on a seeded schedule with at most one request in flight, and latency
  runs from the scheduled instant, so a slow answer cannot hide its own delay.
- p95 is the nearest rank of the sorted latencies, measured to the end of the body, for JSON reads
  and JSON writes separately; refreshes are their own class.
- `minimum_concurrent_clients` is the smallest number of distinct clients completing an expected
  request in any 10 s window; `member_accounts` counts accounts that refreshed and made at least one
  expected request.
- An unexpected error is any status outside the template's list, a transport error or a 10 s
  timeout.
- Game operations are measured apart from the member load: `/metrics` deltas for the game-runtime,
  fleet-executor and ingest routes (counts by status and a p95 bound from the histogram buckets)
  and a once-a-minute database census of the five heartbeating sessions.

**Preflight.** The trusted-proxy range covers the Caddy peer, the bot token is empty, the source
addresses answer, and a keying probe (one invalid refresh per address, each answered 401) leaves
one `strict|<address>` bucket per address.

Declared cases (10): `population_seeded`, `refresh_paced`, `sustained_rate`, `concurrency`,
`member_accounts`, `zero_unexpected_errors`, `p95_json_reads`, `p95_json_writes`,
`game_servers_heartbeating` and `game_operations_measured`.

## Tooling

| Part | Code |
|---|---|
| Receipt recorder | `tools/xtask/src/verifications/api_readiness/operational_recording.rs` |
| Harness and procedures | `tools/xtask/src/commands/staging/` (`cargo xtask staging …`) |
| Load engine and relay | `tools/developer_tools/src/staging_verification/` |
| Host tool | `apps/website/api_v2/src/bin/staging_fixtures/` (`staging-fixtures`, built on the host by the website deploy) |
| Multi-instance deploy | `tools/xtask/src/commands/deploy/staging/`, units in `tools/xtask/deploy/systemd/` |
| Console command | the fleet command ledger (migration 0061), the host agent's at-most-once RCON path, the Server Control console box |

The harness commands: `preflight`, `status`, `fingerprints` and `action-list` read only;
`backup`, `update-game-server`, `provision-fleet`, `rotate-credential`, `seed-load` and `clean-load`
change the staging host after approval; `fleet --record`, `discord --record` and `load --record`
write receipts; `load --rehearse-local` exercises the load path against the local stack and records
nothing.

### Console command

`console_command` takes one line of 1–256 bytes with no control character and no leading `@` (the
RCON session commands). The host agent sends it over RCON exactly once: it retries only the login,
never the command, because the RCON client otherwise resends after a reconnect. The response is kept
in the command outcome up to 4,096 bytes, with a truncation flag; no reply is a failure that says the
command may or may not have run. The action is administrator-only, process-changing (serialised
with start, stop, restart and deployments) and not idempotent, so a lapsed lease makes it
indeterminate rather than requeued. RCON binds to loopback with the `admin` permission and a
password generated on the host. The line never reaches a host shell.

## Safety and cleanup

- The host tool changes nothing without `--apply`, refuses a database other than `tbd_reforger`,
  never prints a secret and writes secret files create-exclusive with mode 600.
- Synthetic accounts live in the id range 9100000000000000000–9100000000000099999; cleanup deletes
  only those accounts and the `[Load fixture]` events; their audit rows remain, as the audit trail
  is append-only.
- A `pg_dump` of `tbd_reforger` precedes the load seeding and the Discord run; a restore needs the
  operator's approval.
- Left in place on purpose: the five servers and their credentials, the two approved missions and
  their fleet scenario rows, the vanilla ballistics catalog, the partner guild, the membership
  override (it expires within 48 h) and every audit row. The earlier single server is deactivated
  with its history kept.
- The outage drop-in and the relay's armed state are removed at the end of their procedures, and
  the secondary source addresses are removed after the load run.

## Findings

| Id | Finding | Class | Outcome |
|---|---|---|---|
| S-F1 | The game runtime counted only 200, 201 and 202 as success, so every 204 "no command" answer to a fleet claim was logged as a failure every 5 s | FIX | 204 is its own no-content answer; the claim poller is quiet on it |
| S-F2 | Behind the tunnel every visitor probably shared one rate-limit bucket, keyed on cloudflared's loopback address | FIX (verified first) | Caddy trusts the tunnel's peer, so the API keys on the real client address; LAN clients stay keyed per address |
| S-F3 | RCON ran with the `monitor` permission | FIX | the console work sets `admin` on loopback |
| S-F4 | `fleet_command_ledger.md` listed a nonexistent `change_map` and said kick used RCON | FIX | corrected; the console row added |
| S-F5 | Discord reconciliation wrote no logs or metrics | FIX | one structured log line and `tbd_discord_reconcile_outcomes_total{outcome}` |
| S-F6 | The staging game server ran an out-of-date Experimental build | FIX | updated before the runs |
| S-F7 | The recorder's run discipline read the environment of whatever process ran it, so under `verify api-readiness --execute`, which gives every check `PROPTEST_RNG_SEED`, the recorder and receipt tests were refused | FIX | `RecordingSession::begin` reads the variables its caller hands it: the harness passes its own process environment, a test passes a clean one; a real recording with `PROPTEST_*` set is still refused |
| S-F8 | The fencing property dropped a refused step's transaction, and sqlx sends that rollback only when the pool takes the connection back; until then the row lock hid the command from the next claim's `SKIP LOCKED` read, so the property failed now and then | FIX (test); NOTE (production) | the property rolls back every refused step before the next one; in production the same window costs one empty claim poll, and the agent claims the command on its next poll |

## Register

The register lists each staging requirement's implementation paths and adds the checks that prove
the tooling itself: `staging_harness` (`cargo test -p xtask --locked commands::staging::`),
`staging_verification_engines` (`cargo test -p developer_tools --locked staging_verification::`),
`staging_fixture_tool` (the `staging_fixtures_*` cases of `cargo xtask db test-it`) and the
recorder's tests inside `readiness_self_tests`. The console command is its own requirement,
`fleet_console_command`. The operational minimums are the declared case counts (50, 13 and 10), so a
receipt can only pass with every declared case observed.

## Results

Recorded here after the runs: each receipt's verdict, counts, timestamps and log paths, the
missing dependencies, and the verdict of the first `verify api-readiness --execute`.
