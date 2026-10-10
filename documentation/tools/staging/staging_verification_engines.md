**Status:** live

# Staging verification engines

The two engines behind the staging receipts that the staging
harness cannot run from its own process: the member load generator, which drives the staging API
with a hundred synthetic members for the `staging_load` receipt, and the acknowledgement-dropping
relay, which loses one fleet executor answer on the staging host for the lost-acknowledgement cases
of the `staging_fleet` receipt.

## Where it lives

- Code: three crates under [`tools/staging/`](/tools/staging/README.md):
  [`staging_load_generator`](/tools/staging/staging_load_generator/README.md) (`load_run.rs`
  holds `run`, `command_line.rs` the `staging-load` executable), its tokio-free
  [`staging_load_plan`](/tools/staging/staging_load_plan/README.md) (`workload_plan.rs` the plan,
  `load_report.rs` the report, `process_boundary.rs` the JSON both cross the process boundary in)
  and [`acknowledgement_dropping_relay`](/tools/staging/acknowledgement_dropping_relay/README.md)
  (`relay.rs` holds `start` and `serve`, `drop_policy.rs` the arming).
- Entry: the `staging-load` executable, which the xtask staging load procedure builds and runs
  as a child process with the `LoadRunPlan` as JSON on its standard input, reading the
  `LoadReport` as one JSON line from its standard output; the relay's executable
  `acknowledgement-dropping-relay`, whose `serve` command the unit
  `acknowledgement-dropping-relay@N` runs and whose `control` command the fleet procedure runs on
  the host ([executables README](/tools/developer_tools/src/bin/README.md#acknowledgement-dropping-relay)).
  Both executables are `developer_tools` binaries of one line each.
- Related features: the [staging design note](/documentation/crates/api/api_server/design_notes/staging.md),
  which defines the procedures, the cases and the receipts; the
  [staging verification runbooks](/documentation/runbooks/staging_verification/README.md), which
  record them against the staging host.

## Behaviour

### Member load

1. The load procedure builds a `LoadRunPlan`: the committed workload (decoded with
   `deny_unknown_fields` and checked), the target origin, the source addresses, the account file of
   refresh tokens the host tool seeded, and the fixture events with their slots. The check refuses
   a ramp shorter than `clients / (addresses × auth ceiling × 0.8)` seconds (50 s for 100 clients
   over five addresses at one refresh per 2 s) and names that minimum.
2. The procedure hands the plan to `staging-load`, whose `run` checks the plan, confirms every
   source address belongs to this machine, reads the account file once, and gives each virtual client its accounts (client `c` holds accounts `c`,
   `c + clients`, …) and one HTTP client bound to its source address, with at most one idle
   connection and no proxy.
3. On a runtime of its own, each client runs two lanes side by side. Its account lane signs in at
   `ramp · c / clients`, so the sign-ins keep every address at or under 80 % of its auth ceiling,
   and hands the token over as soon as it arrives. Its member lane fires open loop: slot `n` is a
   seeded phase inside the client's own `1 / clients` stratum of the period plus `n · P` with
   `P = clients / rate` (100 / 27 ≈ 3.7 s), jittered by ±5 % around that fixed grid, so each
   address's clients spread evenly across the period. A slot that finds no account yet is skipped,
   never sent late: a client joins the member load with its first slot after its token.
4. Every `hold` the account lane switches account through `POST /api/v1/auth/refresh`: it
   refreshes the next account half a hold before the switch and hands it over at the switch
   instant, so no member request waits on a refresh; a refresh still running at the switch instant
   leaves the client on its current account until it ends, and the report counts that switch as
   late. Rotated pairs stay in memory only; a failed refresh retires that account instead of
   replaying a token the API may have spent. A slot sends the next step of a weighted template as
   the current account, so a bookmark or a registration toggle always sends the write its state
   allows, on the account's own event and slot. A client has at most one member request and one
   refresh in flight.
5. Every send first reserves an instant from its address's guard, which holds each address under
   8 requests a second and under 0.5 a second on `/api/v1/auth/`; a refresh is reserved only where
   the auth ceiling already lets it leave, so it never stalls the member requests queued behind it.
6. The report counts, inside the measured window, the completed requests, the class latencies
   (nearest-rank p95, from the scheduled instant to the end of the body; refreshes in the `session`
   class) and the concurrency census (the fewest distinct clients completing an expected request in
   any 10 s window); the unexpected errors, refreshes, late switches, addresses and templates cover
   the whole run. An unexpected error is a status outside the step's list, a transport error, a
   timeout or an incomplete refresh answer.

### Lost acknowledgement

1. The staging deploy installs the relay for the relay instance (instance 5): the unit listens on
   `127.0.0.1:18085` and forwards to the loopback API origin, and that instance's host agent polls
   the relay instead of the API. The relay refuses any listen address or upstream off loopback, and
   its control socket is mode 600 in a mode-700 runtime folder.
2. Disarmed, the relay passes every exchange through: the same method, path, query, end-to-end
   headers and body upstream, and the upstream's status, end-to-end headers and body back.
3. For wave W13 the fleet procedure runs `control … arm drop-next-claim-response` on the host and
   asks the operator to restart server 5. The API claims the restart for the agent and answers
   `200`; the relay records the command id and fencing token, disarms, holds the answer for 30 s
   and then closes the connection without a byte of it. The agent's 20 s request timeout has
   expired by then, so the agent never learns of the claim; the lease lapses, the command is
   requeued with fencing token 2, and the agent claims and runs it once.
4. For wave W14 it arms `drop-next-result-response`: the API records the agent's result and answers
   `200`, the relay withholds that answer the same way, and the agent's retried result report
   meets `409` with the command's state `succeeded`, again with one start.
5. `control … status` prints the arming, the counts and the last withheld answer's command id and
   fencing token as one JSON line, which the procedure journals as its observation; a `204` claim
   answer (nothing claimable) never spends the arming, so an idle poll cannot use it up.

## Data

- `POST /api/v1/auth/refresh`: the load engine's sign-in and account switch; it sends a
  `SessionTokenRequest` and expects a `SessionTokenPair` (`session-token.schema.json`), and
  rotating a refresh token spends the old one.
- The load templates' routes come from the committed workload the load procedure passes in; the
  request catalog refuses game-runtime, fleet-executor and ingest routes, so the member load never
  touches game traffic.
- `POST /api/v1/fleet-executor/commands/claim` and
  `POST /api/v1/fleet-executor/commands/{commandId}/result`: the two answers the relay can withhold;
  it reads `command_id` and `fencing_token` from a `ClaimedFleetCommand` answer and `fencing_token`
  from an `ExecutionResult` report (`fleet-command.schema.json`), and nothing else.
- The relay's status document: `arming`, `listen`, `upstream`, `withhold_milliseconds`,
  `forwarded_count`, `drop_count` and `last_drop` (`response`, `command_id`, `fencing_token`,
  `upstream_status`, `withheld_at_unix_ms`), with the types in `drop_policy.rs`.

## Design

No user interface: the load engine prints its report as one JSON line and the relay prints its
status. Both engines
keep credentials in memory only: the load engine's tokens never reach a log, an error or the
report, and the relay forwards the agent's `Authorization` header without storing, logging or
reporting it.

## Open work

None.

## Decisions

- The load engine runs as a child process of the load procedure, not inside it: the engine needs
  tokio and an HTTP client, and xtask's dependency closure admits neither, so the procedure links
  only the tokio-free plan crate and the two sides share one JSON codec, which round-trips a plan
  and a report byte for byte.

- The relay withholds the answer after the API has committed the claim or the result: losing the
  acknowledgement, not the request, is the failure the lease and the fencing token exist for, and
  withholding the request would prove nothing about them.
- The hold is 30 s against the agent's 20 s timeout, and the connection then closes without a
  byte: the agent sees what a lost answer looks like on the network, a request that times out,
  rather than an answer of any kind.
- Only a `200` spends an arming: the agent's claim loop polls continually, and a `204` spending it
  would drop nothing and leave the wave without its lost answer.
- The relay binds and forwards on loopback only and is armed over a mode-600 Unix socket, not over
  HTTP: nothing off the host, and no other user on it, can make it drop an answer.
- The load engine is open loop with a per-address guard: a closed loop would slow down with the API
  and hide the latency the p95 cases judge, and the guard keeps each source address under the
  workload's per-address ceilings whatever the jitter does.
- The harness's own pacing never shows as server latency: a member request is sent only as an
  account the client already holds, account switches are prefetched on a lane of their own, and
  the stratified phases keep the paced load under every address's all-requests ceiling. A client
  that joined before holding a token would carry the wait for its sign-in, throttled by the auth
  ceiling, into the p95 the cases judge.
