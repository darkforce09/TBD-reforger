# Staging load generator source

The engine behind the `staging_load` receipt: open-loop virtual clients that sign in with
synthetic member accounts, spread over several source addresses, and send a weighted mix of JSON
reads and writes to the staging API for a fixed measured window, then report the rate, latency,
concurrency and errors the load cases are judged on.

## Contents

```text
tools/staging/staging_load_generator/src/
├── account_rotation.rs     the account file, each client's ring and request state, and the refresh exchange
├── account_switch_lane.rs  one client's sign-in, prefetched account switches, and the handover of its account
├── command_line.rs         the `staging-load` executable: `--plan` / standard input in, `--report` / standard output out
├── error.rs                `Error` and `Result`: plan refusals, the account file, the runtime, the plan and report files
├── guarded_exchange.rs     one exchange through the address guard: reserve, send, read to the end, classify
├── http_client.rs          `http_client_builder`: every virtual client's HTTP client, built after the rustls ring provider is installed
├── lib.rs                  the crate root: module header, `mod` lines and the re-exports
├── load_run.rs             `run`: checks the plan, builds the runtime and the clients, gathers the report
├── member_request_lane.rs  one client's paced slots, each sent as the account it holds
├── prelude.rs              `run`, `entrypoint` and `Error` for glob import
├── source_address_pool.rs  the source addresses and their per-address ceilings
├── tests/                  unit tests, the command line, and runs against a stub API from 127.0.0.2 to 127.0.0.6
└── virtual_client.rs       one client: its address-bound connection and its two lanes side by side
```

The plan, the request catalog, the pacing, the records, the census and the report the engine runs
on live in the tokio-free crate
[`staging_load_plan`](/tools/staging/staging_load_plan/README.md).

## How it works

```text
LoadRunPlan ─▶ run ─▶ checks: workload (ramp ≥ its sign-in minimum), templates, origin, addresses, events
                 │    account file read once ─▶ client c holds accounts c, c+clients, …
                 ▼
          own tokio runtime ── one task per client, two lanes side by side ───────────────────┐
                 │                                                                            │
   account switches: sign-in refresh at ramp·c/clients ─▶ hand the token over at once         │
                 │   switch m at sign-in + m·hold: refresh the next account half a hold       │
                 │   earlier, hand it over at the switch (or when a late refresh ends)        │
   member requests:  slots at phase + n·P from the run start; a slot sends its template's     │
                 │   next step as the handed-over account, or is skipped when none is held    │
                 ▼                                                                            │
   address guard: ≤ N sends in any window W (all requests; auth requests) ─▶ send ────────────┘
                 │
                 ▼
   records (scheduled, sent, finished, outcome) ─▶ LoadReport
```

Each client fires open loop: slot `n` is `phase + n·P` from the run start, with `P = clients /
rate`, the phase drawn once inside the client's own `1 / clients` stratum of the period, and a
±jitter drawn per slot around that fixed grid, so the offered rate never drifts and every address's
clients spread evenly across the period instead of bunching into its all-requests ceiling.

The harness never delays a member request with its own pacing. Client `c` signs in at `ramp · c /
clients`, and the plan check refuses a ramp shorter than `clients / (addresses × auth ceiling ×
0.8)` seconds, so the sign-ins stay at or under 80 % of every address's auth ceiling. A client
joins the member load only once its sign-in has handed it a token: a slot that comes due while it
holds no account is skipped, never sent late, so no member latency carries a wait for a token.
Account switches run on a lane of their own beside the member requests: half a hold before each
switch the client refreshes its next account, and the new account takes over at the switch
instant. A refresh that has not ended by then leaves the client on its current account until it
does (never a gap), and the report counts that switch as late. A refresh's latency joins the
`session` class only.

A client has at most one member request and one refresh in flight. When an answer is slow, the
next slot starts as soon as it ends and its latency still runs from its scheduled instant to the
end of its body. Once the window closes no further event starts, and slots still waiting are
reported as unsent.

A refresh spends the old refresh token and keeps the new pair in memory only; a failed refresh
retires that account rather than replay a token the API may already have spent, and the next
candidate is refreshed at once. A template is a cycle of steps each account walks, so a bookmark
or a registration toggle always sends the write its state allows, on the account's own event and
slot (account `k` takes event `k mod events` and slot `k div events`).

Every send first reserves an instant from its address's guard, which keeps each address under the
workload's ceilings (and requests under `/api/v1/auth/` under the stricter one) with a small
margin. An auth request is reserved only where its auth ceiling already lets it leave, so the
address's queue never stands still behind a refresh. The report then measures the busiest window
from the instants requests actually left.

The report counts `completed_requests`, the class latencies and the census inside the measured
window `[ramp, ramp + measured)`; unexpected errors, refreshes, late switches, addresses and
templates cover the whole run. An unexpected error is a status outside the step's list, a
transport error, a timeout, or a refresh answer that is not a complete token pair.

## Public surface

- `run(&LoadRunPlan) -> Result<LoadReport>`: blocks for the run on a runtime of its own.
- `entrypoint() -> ExitCode`: the `staging-load` executable. `--plan <path>` reads the plan from
  a file, standard input otherwise; `--report <path>` writes the report to a file, standard output
  otherwise, as one JSON line. Exit 0 the report was written, 1 the load did not run or its report
  was not written (the reason on standard error as one `staging-load: …` line), 2 a usage error.
- `Error` and `Result`; `prelude`.

The account file is JSON, `{"accounts": [{"discord_id": "…", "refresh_token": "…"}, …]}`, with
account `k` at position `k` and exactly `clients × accounts_per_client` entries. A path or body
string of a step may name the placeholders `{account_index}`, `{discord_id}`, `{event_id}`,
`{event_mission_id}`, `{mission_id}` and `{slot_id}`.

## Boundaries

- Depends on: `staging_load_plan` for the plan, catalog, pacing, records, report and the
  process-boundary codec; `reqwest` (one client per virtual client, bound to its source address,
  no proxy, no redirects, at most one idle connection), `tokio` (its runtime, timers and `watch`
  handover), `clap` (the command line), `serde`, `serde_json` and `thiserror`; the tests also use
  `axum` and `futures` for the stub API and `staging_load_plan`'s `test_fixtures` samples.
- Used by: `developer_tools`' `staging-load` binary, which the `staging_procedures` load procedure runs
  as a child process for the recorded run and the local rehearsal.
- Rules:
  - The engine never writes a token to disk, a log, an error message or the report; the account
    file is only read.
  - The refresh request and answer follow `session-token.schema.json` (the `@contract` tags in
    `account_rotation.rs`, checked by the contract-citation gate).
  - No template reaches the game-runtime, fleet-executor or ingest routes: the catalog refuses
    them, so the member load stays apart from the game operations it is measured beside.
  - The tests bind 127.0.0.2 to 127.0.0.6 as source addresses, which every Linux loopback
    interface answers for.

## Related documentation

- [Staging design note](/documentation/apps/api/verification_evidence/staging.md) — the
  load procedure, its ten cases and how the report maps onto them.
- [Staging load generator](/tools/staging/staging_load_generator/README.md) — the crate this
  folder is the source of.
- [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) —
  the member load and the relay end to end.
