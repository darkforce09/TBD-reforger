# Staging fleet procedure

The `staging_fleet` procedure: waves of fleet commands issued in Server Control on the five
staging instances, the effects each wave must show within its deadline, and the observations the
receipt carries. The wave table and the case list are in the
[staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md#fleet-procedure-staging_fleet);
the walkthrough is the
[fleet procedure runbook](/documentation/runbooks/staging_verification/fleet_procedure.md).

## Contents

```text
tools/commands/staging_procedures/src/fleet_procedure/
├── fixture_identities.rs   the manifest's fixture identities and the fleet's own preflight checks
├── fleet_cases.rs          the declared cases: eight per server, the two-client pair, the fleet-wide ones
├── fleet_reads.rs          committed queries answering one JSON object per row; unit and config reads
├── judge_mapping.rs        the receipt's observations: server ids, client count, judge scenarios
├── mod.rs                  `FleetProcedure`: the `StagingProcedure` of the fleet check
├── operator_lists.rs       the numbered action list and the recovery list
├── single_server_reads.rs  W9–W14's committed queries: the operator's link, command leases, credentials
├── tests/                  recorded observer answers, the waves on the fake clock, a recorded receipt
└── waves/                  W1–W14: the wave table and the steps built from it
```

## How it works

`FleetProcedure` implements `StagingProcedure` from
`tools/commands/staging_procedures/src/procedure_runner/procedure.rs`. The plan declares 50 cases
(`server<N>_{stop,start,restart,custom_console,list_players,same_terrain,cross_terrain,runtime_session_succession}`
for N = 1 to 5; `kick_targets_one_of_two_clients` and `same_terrain_carries_two_clients`, recorded
`NOT RUN (missing: second game client)` because the operator is the fleet's only game client;
`return_to_origin_terrain`; then `identity_link`, `kick`, `rejection_ended_session_kick`,
`machine_credential_rotation_host_agent`, `machine_credential_rotation_mod_runtime`,
`lost_acknowledgement_claim_response` and `lost_acknowledgement_result_response`), one step per
wave for W1–W8 and one to three steps per wave for W9–W14, stopped at 6,900 s. A fleet other than
five instances (`TBD_FLEET_INSTANCES`), or one without a relay instance
(`TBD_FLEET_RELAY_INSTANCE`) or an operator account (`TBD_STAGING_OPERATOR_DISCORD_ID`), is
refused before the recording begins.

Each of W1–W8 is one `chrome_action` step: the orchestrator performs it in Server Control on all
five servers at once. Its request is the first matching row (`fleet_commands.requested_at`, or
`mission_deployments.requested_at` for W6–W8) stamped at most 5 s before the step's `AWAIT` line,
and every effect's deadline counts from that row, so each server is held to at most the deadline
from its own request. Every effect judges one server's newest row since the step began:

| Wave | Per-server effects | Case |
|---|---|---|
| W1 stop | command `succeeded`; unit `inactive` | `server<N>_stop` |
| W2 start, W3 restart | command `succeeded`; the unit runs a process (W3: another than W2's); a generation started after the request heartbeats | `server<N>_start`, `server<N>_restart` |
| W2, W3 | the new generation is the one before it plus 1; the one before it ended with a recorded reason (`superseded`, `expired`, `ended_by_runtime` or `credential_revoked`); the server has no second open session | `server<N>_runtime_session_succession` |
| W4 console `#players` | the command `succeeded` with its response recorded | `server<N>_custom_console` |
| W5 list players | the listing `succeeded` with its players recorded | `server<N>_list_players` |
| W6 Everon | a `scenario_restart` of "TBD Staging Everon"; `load_mission` `succeeded`; `confirmed`; then the process W3 measured still runs | `server<N>_same_terrain` |
| W7 Arland | a `host_restart` of "TBD Staging Arland"; `restart_with_mission` `succeeded`; confirmed by a session started after the request; the config's `scenarioId` names the deployment's scenario; a process other than W6's | `server<N>_cross_terrain` |
| W8 Everon | as W7, back to the Everon scenario | `return_to_origin_terrain` |

W9–W14 each drive one server (`waves/wave_table.rs`, `SINGLE_SERVER_STEPS`); a `host_action`
step's command runs as the step starts, and its deadline counts from the step's start:

| Step | Server, actor | Effects | Case |
|---|---|---|---|
| `w9_identity_link` | 1, operator and orchestrator | the link code issued since the step began consumed, the account linked to the Arma id the code carried; an `identity.link` audit row; a listing of server 1 naming that id; the saved `GET /api/v1/me/link/status` reading `linked` with it | `identity_link` |
| `w10_kick` | 1, orchestrator | the kick succeeded naming the linked Arma id; `console.log` holds `[TBD][Fleet] kicked command=<id>` | `kick` |
| `w10_ended_session_kick` | 1, orchestrator | the saved answer is 409 `RUNTIME_SESSION_ENDED`; no kick but W10's first recorded since the step began | `rejection_ended_session_kick` |
| `w11_stage_…`, `w11_revoke_…`, `w11_promote_host_agent_credential` | 1, harness, orchestrator, harness | one new credential beside one live one; the live one revoked and the agent's claims answered 401 `machine credential revoked` in its unit journal; after the promotion and the agent's restart the new credential authenticates and the revoked one authenticated nothing after its revocation | `machine_credential_rotation_host_agent` |
| `w12_stage_…`, `w12_revoke_…`, `w12_promote_mod_runtime_credential` | 2, harness, orchestrator, harness | as W11 for the game runtime: the revoked credential's session ends `credential_revoked` and `console.log` shows the session loop stopped; the promotion rewrites the profile's `TBD_BackendConfig.json` with the staging deploy's own profile writer (`instance_profile_commands`) and restarts the game server; a newer generation opened with the new credential heartbeats | `machine_credential_rotation_mod_runtime` |
| `w13_lost_claim_answer` | relay instance, harness then orchestrator | the relay withheld the restart's claim answer (fencing token 1); the ledger requeued it at least one 30 s claim lease after the request and it succeeded under fencing token 2; the unit started once | `lost_acknowledgement_claim_response` |
| `w14_lost_result_answer` | relay instance, harness then orchestrator | the relay withheld the result answer; the restart succeeded under fencing token 1 without a requeue; the agent's retried report was refused `STALE_FENCING_TOKEN`, never accepted; the unit started once | `lost_acknowledgement_result_response` |

A journal read starts at the step's request row (the host's clock) less 5 s; a single start holds
once 60 s have passed after the unit's first start, and a second start contradicts it at once.

Row times are the database's own clock; a command's finish, a heartbeat or a confirmation is
judged at the time its row carries, a unit or config read at the time it was observed. A
comparison with an earlier wave's process needs that wave's measured PID, and says so when it has
none. The receipt's observations (`judge_mapping.rs`) name a per-server judge scenario only when
all five servers' cases of that name are `ok`, `lost_acknowledgement` only when both
lost-acknowledgement cases are `ok`; `client_count` is the largest number of distinct Arma ids in
one player listing; `server_ids` are the ids the waves observed.

Before the first step the manifest takes the five servers' ids, the two staging missions with
every artifact digest, the fleet scenario rows and the Workshop version from instance 1's
`console.log`. `staging preflight` adds three fleet checks: the five servers registered once each,
both missions live with an artifact and both fleet scenario rows present, and every server's open
runtime session heartbeating within 60 s on an Everon artifact (without one, W6 would be a
`host_restart`). The recovery list disarms the relay, starts any game server left stopped, and
deploys Everon back onto any server left on another terrain, and finishes a credential rotation
left midway.

## Boundaries

- Depends on: `tools/commands/staging_procedures/src/procedure_runner/` (steps, runner, recording);
  `remote_observers/` (`database_reader`, `unit_state_reader`, `unit_journal_reader`,
  `console_log_reader`); `remote_actions/` (`relay_control.rs`, `host_fixture_commands.rs`);
  `environment_identity/build_identity.rs`; `tools/commands/deployment/src/` (the toolchain
  line and the credential file name W12's promotion uses).
- Used by: `tools/commands/staging_procedures/src/staging_dispatch.rs`.
- Rules: every read only reads (the config read prints only the `scenarioId` value, never the
  passwords the config holds); a wave's deadline counts from its first observed request row; the
  harness never reads stdin; the unit tests are `cargo test -p staging_procedures --locked
  fleet_procedure::`.
