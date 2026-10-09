# Staging load procedure

The `staging_load` procedure: the synthetic population, 30 measured minutes of member load from
five source addresses, and the game operations measured beside it; the `seed-load` and
`clean-load` actions; and the local rehearsal of the load path. The workload and the cases are in
the
[staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md#load-procedure-staging_load);
the operator's steps are in the
[load procedure runbook](/documentation/runbooks/staging_verification/load_procedure.md).

## Contents

```text
tools/commands/staging_procedures/src/load_procedure/
├── committed_load_data.rs         the committed workload and population, their agreement and `workload_sha256`
├── game_operations.rs             game-route counter deltas with a p95 bound, and the heartbeat census judge
├── load_action_lists.rs           the action list, the recovery list and the preflight checks
├── load_fixture_orchestration.rs  `seed-load` and `clean-load`, and the mode-600 token file
├── load_preconditions.rs          proxy trust, bot token, keying probe, population, fixture event and token file judges
├── load_queries.rs                the committed database reads and their row parsers
├── load_report_judges.rs          the member-load cases judged on the engine's report, and their thresholds
├── load_run.rs                    the run: workstation actions between the plan's polled effects
├── load_steps.rs                  the plan: ten declared cases, five steps, deadlines and the hard stop
├── local_rehearsal.rs             `load --rehearse-local` against the local stack, recording nothing
├── mod.rs                         `LoadProcedure` and the entry points `staging_dispatch.rs` calls
├── tests/                         unit tests: committed data, judges, the recorded run, the actions, the rehearsal, the child-process seam
└── workstation_load.rs            the keying refresh through `curl` and the member load as the `staging-load` child process
```

## How it works

```text
seed-load ─▶ live mission "TBD Staging Everon" ─▶ seed-load-population ─▶ seed-load-fixture-events
          ─▶ host account file ─▶ --token-file (exclusive, mode 600) ─▶ host copy removed

load --record ─▶ population        synthetic accounts, fixture events + slots, token file shape
              ─▶ keying_probe      one invalid refresh per source address (401) ─▶ strict|<address> rows
              ─▶ game_operations_baseline   /metrics game-route counters
              ─▶ member_load       engine thread (60 s ramp + 1,800 s)  ║ census every 60 s
              ─▶ game_operations_delta      /metrics again ─▶ counts by status, p95 bound
```

`LoadProcedure::plan` builds five observation steps whose effects decide the ten cases:
`population_seeded` (the database holds 1,100 enlisted synthetic accounts and ten `[Load fixture]`
events of 128 slots on the committed mission, and the token file holds the 1,100 accounts in
order), `refresh_paced` (every keying refresh answered 401 with its own strict bucket, and in the
report every refresh succeeded, none was throttled and no address passed its ceilings),
`sustained_rate`, `concurrency`, `member_accounts`, `zero_unexpected_errors`, `p95_json_reads`
and `p95_json_writes` (the engine's report against the register's thresholds),
`game_servers_heartbeating` (each census sample sees every fleet server heartbeat within 60 s)
and `game_operations_measured` (both counter reads, game-runtime requests since the baseline, no
game route answering 5xx).

The member load is not an awaited effect, so `LoadProcedure::run` replaces the generic runner with
`load_run.rs`: before each step's effects are polled through the engine's `observe`, it performs
the step's workstation action and stores what it measured (the token file's Discord ids, the
keying answers, the report and the census) as measurements, which the report and census effects
read through `measurement:<name>` probes. A workstation action that fails fails the effects that
needed it, naming why; the receipt then carries zeros and an unreachable p95.

`seed-load` resolves the one live mission titled as the population says, seeds the population and
then the fixture events, reads the account file the host tool wrote, writes it into the operator's
token file (refused when it exists) and removes the host copy; `clean-load` removes the fixture
events, then the population, then any host copy. `--rehearse-local` seeds the same population
into the local database, keys 127.0.0.2 to 127.0.0.6 against the local API, runs the committed
workload for the shortest ramp its sign-ins allow (`clients / (addresses × auth ceiling × 0.8)`,
50 s) and 60 measured seconds, prints each member-load case, and always cleans. It judges
`concurrency` and `member_accounts` against rehearsal thresholds scaled to that short window (every
client, and the accounts the clients reach in it: 100), printed as rehearsal thresholds; the
recorded run keeps the acceptance thresholds (100 clients, 1,000 member accounts).

## Boundaries

- Depends on: `tools/commands/staging_procedures/src/procedure_runner/` (the plan, `observe`, the
  journal records), `remote_observers/` (database and `/metrics` reads), `remote_actions/`
  (`staging-fixtures` command lines), `staging_dispatch.rs` (confirmed actions); `staging_load_plan` for
  the plan and report types and their JSON codec; the load engine
  (`tools/staging/staging_load_generator/`) as `developer_tools`' `staging-load` binary, which
  `workstation_load.rs` builds with cargo and runs with the plan on standard input; `curl` on the
  workstation;
  the committed data in `tools/xtask/staging/`.
- Used by: `tools/commands/staging_procedures/src/staging_dispatch.rs`.
- Rules: no token reaches an argument, the output, the journal or the receipt (the token file is
  read for its Discord ids only, the engine reads it once); `--rehearse-local` records nothing;
  the tests are `cargo test -p staging_procedures --locked load_procedure::`.
