# Staging load plan

The `staging_load_plan` crate: the plan and the report of the staging member load, apart from the
engine that runs it. It holds the committed workload's shape and a run's bindings with their
checks, the compiled request mix, the seeded pacing, the records an exchange leaves, the report
folded from them, and the JSON both cross the process boundary in. Nothing here holds an async
runtime or an HTTP client, so the `staging_procedures` load procedure builds plans and judges
reports through it while the tokio generator runs in a process of its own.

## Contents

```text
tools/staging/staging_load_plan/
├── Cargo.toml  the `staging_load_plan` library package: `newtype_ids`, `deterministic_random`, serde, `url`, `thiserror`, layout tier 1
└── src/        the workload and run plan, the request catalog, the pacing, the records, the report and the codec
```

## How it works

```text
committed workload JSON ─▶ WorkloadPlan ─┐
run bindings ─▶ LoadRunPlan ─▶ validate ─▶ encode_plan ─▶ staging-load ─▶ decode_report ─▶ LoadReport ─▶ judges
```

The `staging_procedures` load procedure decodes the committed workload, binds it to the run (origin, source
addresses, account file, fixture events), validates it before asking to run, and hands it to the
`staging-load` executable through `encode_plan`; the load generator decodes it, runs it on the
pacing and catalog defined here, assembles the `LoadReport` with `load_report::assemble` and prints
it through `encode_report`. `src/README.md` describes each module.

## Getting started

Run from the repository root:

```bash
cargo build -p staging_load_plan   # the plan checks, the catalog, the pacing, the records, the census and the codec
```

## Configuration

No feature and no environment variable.

## Public surface

- At the crate root: `WorkloadPlan`, `LoadRunPlan`, `FixtureEvent`, `LoadReport`, `ClassSummary`,
  `reachable_member_accounts`, `verify_source_addresses`, `encode_plan`, `decode_plan`,
  `encode_report`, `decode_report`, `Error` and `Result`.
- The modules `workload_plan`, `run_settings`, `request_catalog`, `pacing`, `latency_recording`,
  `concurrency_census`, `client_outcome`, `source_addresses`, `load_report`, `identifiers` and
  `process_boundary`.
- `prelude`: the plan and report types, the two checks, the codec and `Error`.

## Boundaries

- Depends on: `newtype_ids` (the typed ids), `deterministic_random` (the SplitMix64 generator
  behind the seeded streams), `serde`, `serde_json`, `url` and `thiserror`.
- Used by: `staging_load_generator`, which runs a plan and assembles its report; the
  `staging_procedures` load procedure, which builds plans, checks addresses and judges reports.
- Rules: tier 1 of `tools/staging` (`cargo xtask verify crate-tiers`); no async runtime and no
  HTTP client, since xtask depends on it (the tokio firewall of the crate-tier law); a plan and a
  report round-trip through the codec byte for byte.

## Related documentation

- [Staging tool crates](/tools/staging/README.md) — the three staging crates.
- [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) —
  the member load and the relay end to end.
- [Staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md) — the load
  procedure and how the report maps onto its cases.
