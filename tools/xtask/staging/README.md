# Staging load data

The committed inputs of the `staging_load` receipt: the member workload the load engine drives
against the staging API, and the synthetic population and fixture events it drives it with.
`workload_sha256` in the receipt is their digest.

## Contents

```text
tools/xtask/staging/
├── load_population.json  the synthetic accounts and the [Load fixture] events the host tool seeds
└── load_workload.json    the load engine's workload: pacing, ceilings and the weighted request mix
```

## How it works

`load_workload.json` is a `WorkloadPlan` of
`tools/staging/staging_load_plan/src/workload_plan.rs`, decoded with
unknown fields refused: the seed, a 60 s ramp, 1,800 measured seconds, 100 clients over 5 source
addresses at 27 requests a second, 11 accounts per client switched every 160 s, ±5 % jitter, a 10 s
request timeout, 10 s census windows, and the per-address ceilings (8 requests in any second, one
authentication request in any 2 s). The mix is 80 % JSON reads (events, the event hub, missions, the
dashboard, leaderboards, the wiki, vehicles, announcements, `/me`, servers, ballistics catalogs) and
20 % writes (a bookmark toggle on the fixture mission, a registration and withdrawal on the
account's own slot, and a fire-mission save whose client solution is the solver's own answer for the
committed `vanilla_mortars` catalog version 1, so the server's re-solve agrees).

`load_population.json` states what `staging-fixtures` seeds: 1,100 accounts from the first reserved
Discord id with the Discord role `Player`, and 10 events titled `[Load fixture] NN` with an ORBAT of
2 factions × 8 squads × 8 slots, starting 14 days ahead, attached to the live mission
`TBD Staging Everon`. The account count is the workload's clients × accounts per client, and each
event has a slot for every account that writes to it (account `k` takes event `k mod 10` and slot
`k div 10`).

`workload_sha256` is the SHA-256 of both files, each framed by its length as an 8-byte big-endian
integer, workload first.

## Boundaries

- Read by: `tools/commands/staging_procedures/src/load_procedure/` (`staging load --record`,
  `--rehearse-local`, `seed-load`, `action-list load`).
- Rules: the population's fixture plan equals the host tool's fixed plan in
  `tools/staging/staging_fixtures/src/load_fixture_events/fixture_plan.rs`, which a unit
  test of the load procedure holds; no template reaches the game-runtime, fleet-executor or ingest
  routes, which the load engine refuses.

## Related documentation

- [Staging load procedure](/documentation/runbooks/staging_verification/load_procedure.md) — the
  runbook that seeds, records and cleans with these files.
- [Staging design note](/documentation/crates/api/api_server/design_notes/staging.md) — the
  load procedure and its ten cases.
