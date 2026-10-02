# Member load generation

The engine behind the `staging_load` receipt: open-loop virtual clients that sign in with
synthetic member accounts, spread over several source addresses, and send a weighted mix of JSON
reads and writes to the staging API for a fixed measured window, then report the rate, latency,
concurrency and errors the load cases are judged on.

## Contents

```text
tools/developer_tools/src/staging_verification/load_generation/
├── account_rotation.rs     the account file, each client's ring and request state, and the refresh exchange
├── account_switch_lane.rs  one client's sign-in, prefetched account switches, and the handover of its account
├── concurrency_census.rs   distinct clients per census window, and the member-account count
├── guarded_exchange.rs     one exchange through the address guard: reserve, send, read to the end, classify
├── latency_recording.rs    the record of every exchange, its outcome, and nearest-rank percentiles
├── load_report.rs          the report: rates, class latencies, census, errors, switches, addresses, templates
├── member_request_lane.rs  one client's paced slots, each sent as the account it holds
├── mod.rs                  `run`: checks the plan, builds the runtime and the clients, gathers the report
├── pacing.rs               seeded streams, each client's sign-in, switches and slots, and the reachable accounts
├── request_catalog.rs      the compiled request mix: weighted picks, placeholders, resolved requests
├── source_address_pool.rs  the source addresses, their per-address ceilings, and the busiest window
├── tests/                  unit tests and runs against a stub API from 127.0.0.2 to 127.0.0.6
├── virtual_client.rs       one client: its address-bound connection and its two lanes side by side
└── workload_plan.rs        the committed workload's shape and the run's bindings, with their checks
```

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
- `WorkloadPlan` (decoded from the committed workload with `deny_unknown_fields`, with
  `from_json_file`, `validate` and `minimum_ramp_seconds`) and `LoadRunPlan` (the workload plus
  the target origin, the source addresses, the account file and the `FixtureEvent` list), with the
  nested ceiling, template and step types in `workload_plan`.
- `reachable_member_accounts(&WorkloadPlan)`: the member accounts a run's window reaches when
  every refresh succeeds, which the local rehearsal scales its thresholds to.
- `LoadReport` and its summaries in `load_report`.
- `verify_source_addresses`: binds each address once, for the preflight's check that the source
  addresses are up.

The account file is JSON, `{"accounts": [{"discord_id": "…", "refresh_token": "…"}, …]}`, with
account `k` at position `k` and exactly `clients × accounts_per_client` entries. A path or body
string of a step may name the placeholders `{account_index}`, `{discord_id}`, `{event_id}`,
`{event_mission_id}`, `{mission_id}` and `{slot_id}`.

## Boundaries

- Depends on: `reqwest` (one client per virtual client, bound to its source address, no proxy, no
  redirects, at most one idle connection), `tokio` (its runtime, timers and `watch` handover),
  `serde` and `serde_json`, and `anyhow`; the tests also use `axum` and `futures-util` for the stub
  API.
- Used by: the xtask staging load procedure, for the recorded run and the local rehearsal.
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
- [Staging verification engines](/tools/developer_tools/src/staging_verification/README.md) —
  the folder this engine sits in.
