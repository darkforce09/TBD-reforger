# Staging load generator

The `staging_load_generator` crate: the engine behind the `staging_load` receipt and the
`staging-load` executable that runs it. Open-loop virtual clients sign in with synthetic member
accounts, spread over several source addresses, send a weighted mix of JSON reads and writes to the
staging API for a fixed measured window, and report the rate, latency, concurrency and errors the
load cases are judged on.

## Contents

```text
tools/staging/staging_load_generator/
├── Cargo.toml  the `staging_load_generator` library package: `staging_load_plan`, tokio, reqwest, rustls (ring), clap, `thiserror`, layout tier 2
└── src/        the run, the virtual clients and their lanes, the address guard, the account rotation and the command line
```

## How it works

```text
staging_procedures load procedure ── encode_plan ──▶ staging-load (stdin) ── decode_plan ──▶ run ──▶ LoadReport
                     ◀── decode_report ── one JSON line (stdout) ◀── encode_report ──────────┘
```

`run` checks the plan (`staging_load_plan`), reads the account file once, confirms every source
address is this machine's, and drives one virtual client per client on a multi-threaded tokio
runtime of its own: each client signs in during the ramp, switches accounts through prefetched
refreshes, and fires its paced member slots through its address's guard. The clients' records fold
into the report. `src/README.md` describes the pacing, the lanes and the guard.

The `staging_procedures` load procedure never links this crate: it builds `developer_tools`' `staging-load`
binary and runs it as a child process, the plan on standard input and the report on standard
output, so tokio and the HTTP client stay out of xtask's dependency closure.

## Getting started

Run from the repository root:

```bash
cargo build -p staging_load_generator   # the account rotation, the guard, the command line and the run engine
```

## Configuration

No feature and no environment variable. The plan names everything a run needs: the workload, the
target origin, the source addresses, the account file and the fixture events.

## Public surface

- At the crate root: `run`, `entrypoint` (the `staging-load` command line: `--plan`, `--report`;
  exit 0 written, 1 the load did not run or its report was not written, 2 a usage error), `Error`
  and `Result`.
- `prelude`: `run`, `entrypoint` and `Error`.

## Boundaries

- Depends on: `staging_load_plan` (the plan, the catalog, the pacing, the records, the report and
  the process-boundary codec), `tokio`, `reqwest`, `rustls` (the ring provider), `clap`, `serde`, `serde_json` and `thiserror`.
- Used by: `developer_tools`' `staging-load` binary; the `staging_procedures` load procedure runs that
  binary for the recorded run and the local rehearsal.
- Rules: tier 2 of `tools/staging` (`cargo xtask verify crate-tiers`); never a dependency of
  xtask (the tokio firewall of the crate-tier law); no token reaches a log, an error, the report or
  standard output (`account_rotation.rs`).

## Related documentation

- [Staging tool crates](/tools/staging/README.md) — the three staging crates.
- [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) —
  the member load and the relay end to end.
- [Staging design note](/documentation/crates/api/api_server/design_notes/staging.md) — the load
  procedure, its ten cases and how the report maps onto them.
