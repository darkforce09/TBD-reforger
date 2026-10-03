# Tooling foundation crates

The lowest tier of the repository tooling: the libraries every tool builds on. They hold the
vocabulary a check concludes in, the way a child process is run, and the structural engineering
laws, so `xtask`, the `api` engineering-law tests and the other tooling crates share one
implementation of each.

## Contents

```text
tools/foundation/
├── process_runner/     `process_runner`: child processes in their own process group, the container-to-host bridge, the ssh transport
├── repository_laws/    `repository_laws`: file length, test placement, exemptions, engine layers, crate directions, workspace laws
├── repository_layout/  `repository_layout`: the checkout-root walk and the repository locations the tools share
└── verification_core/  `verification_core`: fail-closed verdicts, pattern scans, the run report and the verification lock
```

## How it works

`verification_core` and `repository_layout` are tier 0 and depend on no workspace crate.
`repository_layout` finds the checkout root and names the locations more than one tool reads.
`verification_core` defines what a check can conclude (held, failed, or did not run, with the
`NotRun` cause). `process_runner` and `repository_laws` are tier 1 and report through it: a child
that died on a signal or ran out of time is a `NotRun`, and so is a law whose input is missing.

```text
xtask ──▶ process_runner ──▶ verification_core
  │                            ▲
  └────▶ repository_laws ──────┘   ◀── api (dev-dependency: the engineering_laws tests)

xtask, ticket_engine, developer_tools, ticketboard ──▶ repository_layout
```

## Boundaries

- Depends on: external crates only (`regex`, `libc`, `thiserror`), and within this folder on
  lower tiers alone.
- Used by: `xtask` (every crate); `ticket_engine`, `developer_tools` and `ticketboard`
  (`repository_layout`); the `api` package's `engineering_laws` tests
  (`verification_core` and `repository_laws`, as dev-dependencies).
- Rules: each crate declares `category = "tools/foundation"` and its tier
  (`cargo xtask verify crate-tiers`), keeps the crate anatomy (`cargo xtask verify crate-anatomy`),
  and depends only on lower `tools/foundation` crates
  (`foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`).
