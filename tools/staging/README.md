# Staging tool crates

The libraries the staging verification procedures run on: the load plan and report shapes, the load
generator that drives the API with them, and the relay that withholds one fleet executor answer to
inject a fault. The two engines run tokio and an HTTP client, so they reach xtask only as
`developer_tools` executables; xtask links the tokio-free plan crate alone.

## Contents

```text
tools/staging/
├── acknowledgement_dropping_relay/  `acknowledgement_dropping_relay`: the loopback relay that withholds one fleet executor answer, and its executable's command line (`acknowledgement-dropping-relay`)
├── staging_load_generator/          `staging_load_generator`: the member load's virtual clients, address guard and account rotation, and the `staging-load` executable's command line
└── staging_load_plan/               `staging_load_plan`: the member load's plan, request catalog, pacing, records, report and process-boundary codec, without tokio
```

## How it works

```text
staging_procedures load procedure ── staging_load_plan (plan, checks, judges)
        │ plan JSON on stdin / report JSON on stdout
        ▼
developer_tools `staging-load` ── staging_load_generator ── staging_load_plan

staging_procedures fleet procedure ── ssh ── developer_tools `acknowledgement-dropping-relay`
                                              └── acknowledgement_dropping_relay
```

`staging_load_plan` is tier 1, the generator above it tier 2, and the relay tier 1; none depends on
xtask; `staging_procedures` depends only on the plan crate, which keeps tokio, axum and reqwest out
of the xtask binary's dependency closure.

## Boundaries

- Depends on: `newtype_ids`, `deterministic_random` (the plan's seeded streams) and `time_source`
  (the relay's drop stamps) among the workspace crates; tokio, reqwest, axum (the relay) and clap
  in the engines.
- Used by: `developer_tools`' `staging-load` and `acknowledgement-dropping-relay` binaries; the
  `staging_procedures` load procedure (the plan crate).
- Rules: a crate here sits at `tools/staging/<name>` and declares `category = "tools/staging"`
  (`cargo xtask verify crate-tiers`); an engine never writes a credential to disk, a log or its
  report.

## Related documentation

- [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) —
  the member load and the relay end to end.
- [Staging verification runbooks](/documentation/runbooks/staging_verification/README.md) — how
  the receipts are recorded against the staging host.
