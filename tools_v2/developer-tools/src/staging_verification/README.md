# Staging verification engines

The engines the staging verification harness runs against the staging host, each behind one
operational receipt of the API acceptance register: the member load from the workstation, and the
acknowledgement-dropping relay on the staging host itself.

## Contents

```text
tools_v2/developer-tools/src/staging_verification/
├── acknowledgement_relay/  the relay of the staging fleet receipt's lost-acknowledgement cases
├── load_generation/        the member load of the staging load receipt: virtual clients, report
└── mod.rs                  declares the engines
```

## How it works

Each engine is a self-contained submodule with one entry point, its own inputs and its own
report, and builds its own tokio runtime, so a synchronous xtask command calls it directly. The
harness owns everything around a run: it reads the committed staging data, prepares the run's
bindings on the host, journals the engine's report as an observation and judges the receipt. The
relay is reached differently: its unit runs `acknowledgement-dropping-relay serve` on the staging
host, and the harness runs the same executable's `control` command there over ssh and journals the
status it prints.

## Public surface

- `load_generation`: `run`, `LoadRunPlan`, `WorkloadPlan`, `FixtureEvent`, `LoadReport` and
  `verify_source_addresses` (see the
  [member load generation README](/tools_v2/developer-tools/src/staging_verification/load_generation/README.md)).
- `acknowledgement_relay`: `entrypoint` (the `acknowledgement-dropping-relay` executable),
  `start`, `serve`, `RelaySettings`, `send_control_command` and the `RelayStatus` document (see
  the [acknowledgement-dropping relay README](/tools_v2/developer-tools/src/staging_verification/acknowledgement_relay/README.md)).

## Boundaries

- Depends on: `tokio`, `reqwest`, `serde`, `serde_json` and `anyhow`; the relay also on `axum`
  and `clap`.
- Used by: the `acknowledgement-dropping-relay` executable, which runs the relay; the xtask
  staging procedures are the callers the load engine is built for.
- Rules: an engine never writes a credential to disk, a log or its report; each engine's tests
  live in its own `tests/` folder.

## Related documentation

- [Staging design note](/documentation_v2/website/api_v2/verification_evidence/staging.md) — the
  procedures, the receipts and the cases the engines serve.
- [Staging verification runbooks](/documentation_v2/runbooks/staging_verification/README.md) — how
  the receipts are recorded against the staging host.
