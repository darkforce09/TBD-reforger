**Status:** live

# Record the fleet receipt

How the orchestrator and the operator run `cargo xtask staging fleet --record`: the checks
before it, the approval of its numbered waves, what to do at each `AWAIT` line of waves W1 to W8
(every server at once) and W9 to W14 (one server each), and how to read the receipt. What each wave proves is in the
[staging design note](/documentation_v2/website/api_v2/verification_evidence/staging.md#fleet-procedure-staging_fleet).

## Prerequisites

- The [setup checklist](/documentation_v2/runbooks/staging_verification/setup_checklist.md) is
  done: the five servers "TBD Staging 1" to "TBD Staging 5" are registered and active, the
  missions "TBD Staging Everon" and "TBD Staging Arland" are live with artifacts, and the fleet
  scenario rows for `everon` and `arland` exist.
- Every fleet server runs a confirmed deployment of "TBD Staging Everon". W6 deploys Everon onto
  servers that run Everon, which the platform makes a `scenario_restart` only when the running
  session loaded an Everon artifact; a server booted from the mission header alone would take a
  `host_restart` and fail W6.
- The relay of instance 5 is disarmed and every game server unit is active
  (`cargo xtask staging status`).
- `deploy.env` sets `TBD_STAGING_OPERATOR_DISCORD_ID` (the account W9 links) and
  `TBD_FLEET_RELAY_INSTANCE`; without either the recording is refused before its first `AWAIT`.
- The operator can join "TBD Staging 1" with the game client, and the operator's account is not
  linked to an Arma identity yet (unlink it in the account settings first).
- The host checkout holds the release build of `staging-fixtures` (W11 and W12 stage and promote
  credentials with it).
- Every command runs as `hcargo xtask …` from the repository root, inside the run day's 24-hour
  window ([run day](/documentation_v2/runbooks/staging_verification/run_day.md)).

## Steps

1. Check the preconditions.

   ```bash
   cargo xtask staging preflight
   ```

   Expected: the harness checks met, then `fleet servers registered`, `fleet missions and
   scenarios` and `fleet runs the Everon deployment` met. An unmet check names each server or row
   at fault; deploy "TBD Staging Everon" in Server Control onto any server it names.

2. Print the numbered waves and the declared cases, and have the operator approve the waves.

   ```bash
   cargo xtask staging action-list fleet
   cargo xtask staging action-list fleet --cases
   ```

   Expected: one numbered orchestrator action per W1–W8 wave, each naming the five servers, the
   awaited effect and the deadline, then one numbered action per W9–W14 step naming its server
   and actor, with the exact host command of each harness step; the 50 declared cases, eight per
   server, the two two-client cases `NOT RUN (missing: second game client)`,
   `return_to_origin_terrain`, and the seven cases of W9–W14.

3. Start the recording in the background and follow its output.

   ```bash
   cargo xtask staging fleet --record
   ```

   Expected: `staging-run: staging_fleet run=<id> journal=<folder>`, then `AWAIT w1_stop: W1: in
   Server Control, Stop on TBD Staging 1, 2, 3, 4 and 5; …`. The harness never reads input; it
   polls the host every 5 s and stops itself at 6,900 s.

4. At each `AWAIT` line, the orchestrator performs the wave in Server Control on all five servers,
   one server after another without waiting between them. A wave's deadline counts from its first
   request row, and the harness waits up to 15 minutes for that row to appear.

   | `AWAIT` step | In Server Control, on each server | The harness waits for | Deadline |
   |---|---|---|---|
   | `w1_stop` | Stop | the stop command `succeeded`; `tbd-reforger@N` inactive | 150 s |
   | `w2_start` | Start | the command `succeeded`; the unit runs a process; a new runtime-session generation heartbeats: the previous generation plus 1; the previous generation ended with a recorded reason (`superseded`, `expired`, `ended_by_runtime` or `credential_revoked`); no second open session | 240 s |
   | `w3_restart` | Restart | as W2, with a process other than W2's | 240 s |
   | `w4_console_command` | Console command, line `#players` | the command `succeeded` with the server's response recorded | 60 s |
   | `w5_list_players` | List players | the listing `succeeded` with its players recorded | 60 s |
   | `w6_same_terrain` | Deploy the mission "TBD Staging Everon" | a `scenario_restart`; `load_mission` `succeeded`; the deployment confirmed; the same process as after W3 | 600 s |
   | `w7_cross_terrain` | Deploy the mission "TBD Staging Arland" | a `host_restart`; `restart_with_mission` `succeeded`; the config names the Arland scenario; a new process; confirmed by a session started after the request | 1,200 s |
   | `w8_return_to_origin` | Deploy the mission "TBD Staging Everon" | as W7, back to the Everon scenario | 1,200 s |

   Expected: after each wave, one `<step>.server<N>_<effect> ok` line per effect, or `FAILED`
   with what was last seen; the next `AWAIT` line follows once every effect of the wave is decided.

5. From W9 on, each wave drives one server. A step whose actor is the harness runs its host
   command as its `AWAIT` line prints; the orchestrator then acts only where the step names it.
   Where a step reads the browser, the harness prints the inbox path: save the Chrome tool's raw
   output there as `{"captured_at_unix_ms": <now>, "output": <raw output>}`, never a token.

   | `AWAIT` step | Who acts, and what | The harness waits for | Deadline |
   |---|---|---|---|
   | `w9_identity_link` | the operator joins "TBD Staging 1"; the orchestrator requests an Arma link code in the account settings; the operator types `#tbd link <code>` in the game chat; the orchestrator runs List players on "TBD Staging 1" and saves the page's `GET /api/v1/me/link/status` answer | the code consumed and the account linked to the Arma id it carried; an `identity.link` audit row; the listing naming that id; the saved status `linked` with it | 600 s from the code (its lifetime) |
   | `w10_kick` | Kick on "TBD Staging 1": the linked Arma id, the runtime session it runs now, a reason | the kick `succeeded` naming the linked id; `console.log` holds `[TBD][Fleet] kicked command=<id>` | 120 s |
   | `w10_ended_session_kick` | Kick again, naming the runtime session that confirmed W7's Arland deployment (W8 ended it); save the page's answer | the answer is 409 `RUNTIME_SESSION_ENDED`; no second kick recorded | 900 s from the step's start |
   | `w11_stage_host_agent_credential` | the harness stages a new `host_agent` credential of "TBD Staging 1" | one new credential beside one live one | 120 s from the step's start |
   | `w11_revoke_host_agent_credential` | "TBD Staging 1", Credentials: Revoke the older `host_agent` credential with a reason | the old one revoked; the agent's claims answered 401 `machine credential revoked` in `fleet-host-agent@1`'s journal | 120 s |
   | `w11_promote_host_agent_credential` | the harness promotes the staged credential and restarts `fleet-host-agent@1` | the new credential authenticates; the revoked one authenticated nothing after its revocation | 180 s from the step's start |
   | `w12_stage_mod_runtime_credential` | the harness stages a new `mod_runtime` credential of "TBD Staging 2" | as W11 | 120 s from the step's start |
   | `w12_revoke_mod_runtime_credential` | "TBD Staging 2", Credentials: Revoke the older `mod_runtime` credential | the old one revoked; its session ended `credential_revoked`; `console.log` shows the session loop stopped over the machine credential | 180 s |
   | `w12_promote_mod_runtime_credential` | the harness promotes the staged credential, rewrites the instance profile's `TBD_BackendConfig.json` with the staging deploy's own profile commands, and restarts `tbd-reforger@2` | a newer generation opened with the new credential heartbeats | 600 s from the step's start |
   | `w13_lost_claim_answer` | the harness arms the relay to withhold the next claim answer; then Restart on "TBD Staging 5" | the relay withheld the restart's claim answer (fencing token 1); the ledger requeued it at least 30 s after the request and it `succeeded` under fencing token 2; `tbd-reforger@5` started exactly once | 600 s |
   | `w14_lost_result_answer` | the harness arms the relay to withhold the next result answer; then Restart on "TBD Staging 5" | the relay withheld the result answer; the restart `succeeded` under fencing token 1 without a requeue; the agent's retried report refused `STALE_FENCING_TOKEN`; one start | 600 s |

   Expected: one `<step>.<effect> ok` line per effect. A single start holds once 60 s have
   passed after the unit's first start, so W13 and W14 end about a minute after the restart.

6. Read the verdict line the recording ends with.

   Expected: with one game client the run ends `staging_fleet: FAIL 48/50` at best (the two
   two-client cases are `NOT RUN`), naming the acceptance reason (the judge needs two clients),
   and exit 1. The receipt still carries the
   observed server ids, the client count, the judge scenarios every server passed and the fixture
   digest.

## Verify

The receipt is `target/api-readiness/staging_fleet.json`, its log `staging_fleet.log` and its
manifest `staging_fleet.fixture.json` beside it; the raw observations are in the journal folder the
first line printed (`journal.jsonl` and `artifacts/`). Each `observation:` line of the log cites
the SHA-256 of an archived artifact in that folder.

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `the fleet procedure cannot be recorded` before any `AWAIT` | `deploy.env` declares a fleet other than five instances | set `TBD_FLEET_INSTANCES=5`; the earlier receipt is untouched |
| `… not observed within 900 s of the step's start` | no matching command or deployment row appeared | the wave was not issued, or issued before its `AWAIT` line; the receipt keeps the failure |
| W6 fails with `is a host_restart, not a scenario_restart` | the server's open session runs no Everon artifact | deploy "TBD Staging Everon" onto it before the next run |
| W2 or W3 fails with `follows generation …, not generation …` or `has 2 open runtime sessions` | the session ledger skipped a generation or kept two sessions open | the receipt records the observed rows; read the session rows in the journal |
| a later wave fails with `measured no process of …` | an earlier wave did not measure the server's process | fix the earlier wave's failure first; the comparison is refused, not guessed |
| W9 fails with `the listing names …, not the linked Arma id …` | the account was linked to another identity, or another player was listed | unlink in the account settings and link from the game client on "TBD Staging 1" |
| W10's second kick fails with `is not the 409 RUNTIME_SESSION_ENDED refusal` | the kick named the open session, or the saved output is another request's | name the session that confirmed W7's Arland deployment; save the answer of that request |
| W11 fails with `no claim answered 401` | the revoked credential was not the agent's live one | revoke the older credential, not the one the harness staged |
| W12 fails with `opened no runtime session since the step began` | the game server did not read the promoted credential | `cargo xtask staging action-list fleet --recovery`, item "Finish a credential rotation" |
| W13 fails with `succeeded without a lease-lapse requeue` or W14 with `relay is … with … drop(s)` | the relay was not armed, or another command spent the arming | check `cargo xtask staging status` shows the relay disarmed before the run; issue no other command on "TBD Staging 5" during W13 and W14 |
| the run stopped early | a crash, or the hard stop | [recovery and cleanup](/documentation_v2/runbooks/staging_verification/recovery_and_cleanup.md) with `cargo xtask staging action-list fleet --recovery` |

## Related

- [Run day](/documentation_v2/runbooks/staging_verification/run_day.md) — where the fleet
  recording sits among the three receipts.
- [Recovery and cleanup](/documentation_v2/runbooks/staging_verification/recovery_and_cleanup.md)
  — the fleet recovery list: relay disarm, stopped units, the Everon deployment, a credential
  rotation left midway.
- [Staging design note](/documentation_v2/website/api_v2/verification_evidence/staging.md) — the
  witness rules and the receipt format.
