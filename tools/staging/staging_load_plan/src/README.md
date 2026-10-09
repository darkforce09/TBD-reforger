# Staging load plan source

The plan and the report of the staging member load, apart from the engine that runs it: the
committed workload's shape and a run's bindings with their checks, the compiled request mix, the
pacing that decides how many member accounts a window reaches, the source-address check, the
records an exchange leaves, and the report folded from them. Nothing here holds an async runtime
or an HTTP client.

## Contents

```text
tools/staging/staging_load_plan/src/
├── client_outcome.rs      what one virtual client leaves behind: its address, records and slot counts
├── concurrency_census.rs  distinct clients per census window, and the member-account count
├── error.rs               `Error` and `Result`: workload reads, refusals, the process boundary, source addresses
├── identifiers.rs         the template, Discord account, fixture event, attachment, mission and slot ids
├── latency_recording.rs   the record of every exchange, its outcome, and nearest-rank percentiles
├── lib.rs                 the crate root: module header, `mod` lines and the types the `staging_procedures` load procedure names
├── load_report.rs         the report: rates, class latencies, census, errors, switches, addresses, templates
├── pacing.rs              seeded streams, each client's sign-in, switches and slots, and the reachable accounts
├── prelude.rs             the plan and report types, the checks and the codec for glob import
├── process_boundary.rs    the JSON a plan and a report cross between xtask and `staging-load` in
├── request_catalog.rs     the compiled request mix: weighted picks, placeholders, resolved requests
├── run_settings.rs        the workload's numbers as a run applies them: durations and ceiling windows
├── source_addresses.rs    the check that every source address is this machine's, and the busiest window
├── tests/                 unit tests, and the sample workloads behind the `test_fixtures` feature
└── workload_plan.rs       the committed workload's shape and the run's bindings, with their checks
```

## How it works

```text
committed workload JSON ─▶ WorkloadPlan ─┐
run bindings (origin, addresses, accounts, events) ─▶ LoadRunPlan ─▶ checked settings, origin, catalog
                                                                     │
     staging-load (tokio, its own process) runs the clients ◀───────┘
                                                                     │
                     one ClientOutcome per client ─▶ load_report::assemble ─▶ LoadReport (JSON)
```

`LoadRunPlan` checks the workload (a ramp at least its sign-in minimum), the templates (the
catalog refuses the game-runtime, fleet-executor and ingest routes), the origin (no credentials)
and the events before any client exists; `RequestCatalog::compile` turns the request mix into weighted
picks. The pacing turns a seed and a client index into that client's sign-in, switch and slot
instants, which is also how `reachable_member_accounts` counts the accounts a window reaches. The
report counts completed requests, class latencies and the census inside the measured window, and
unexpected errors, refreshes, late switches, addresses and templates over the whole run.

## Public surface

- `WorkloadPlan`, `LoadRunPlan` and `FixtureEvent`, with the nested ceiling, template and step
  types in `workload_plan`, and the checked `RunSettings` and `CeilingWindow` in `run_settings`.
- `LoadReport` and `ClassSummary`, with the other summaries and `assemble` in `load_report`.
- `reachable_member_accounts(&WorkloadPlan)` and `verify_source_addresses(&[IpAddr])`.
- `encode_plan`, `decode_plan`, `encode_report` and `decode_report`, the process-boundary codec.
- For the load generator: `RequestCatalog`, `AccountBinding`, `ResolvedRequest` and `resolve` in
  `request_catalog`; the schedules and `SeededRandom` in `pacing`; `RequestRecord` and
  `RequestOutcome` in `latency_recording`; `ClientOutcome`; `GUARD_MARGIN` and `busiest_window` in
  `source_addresses`; the ids in `identifiers`.
- `sample_plans`, behind the `test_fixtures` feature.

## Boundaries

- Depends on: `newtype_ids`, `deterministic_random`, `serde`, `serde_json`, `url` and `thiserror`; no async runtime and
  no HTTP client.
- Used by: the crate root's re-exports, read by `staging_load_generator` (which runs a plan and
  assembles its report) and by the `staging_procedures` load procedure (which builds plans, checks addresses and
  judges reports).
- Rules: a report holds counts, latencies, addresses and template ids, never a token, a header or
  a body; a plan and a report round-trip through the codec byte for byte
  (`tests/process_boundary_tests.rs`); unit tests live in `tests/` files declared with `#[path]`.

## Related documentation

- [Staging load plan](/tools/staging/staging_load_plan/README.md) — the crate this folder is the
  source of.
- [Staging load generator](/tools/staging/staging_load_generator/README.md) — the engine that runs
  a plan.
- [Staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md) — the load
  procedure and how the report maps onto its cases.
