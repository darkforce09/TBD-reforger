# Tooling foundation crates

The lowest tier of the repository tooling: the libraries every tool builds on. They hold the
vocabulary a check concludes in, the way a child process is run, and the structural engineering
laws, so `xtask`, the `api` engineering-law tests and the other tooling crates share one
implementation of each.

## Contents

```text
tools/foundation/
├── deploy_settings/    `deploy_settings`: the `deploy/deploy.env` reader, the deploy host, its remote folders and the ssh transport choice
├── process_runner/     `process_runner`: child processes in their own process group, the container-to-host bridge, the ssh transport, the `PATH` guard
├── repository_laws/    `repository_laws`: file length, test placement, exemptions, the apps' dependency directions, workspace laws
├── repository_layout/  `repository_layout`: the repository locations the tools share
├── tool_test_support/  `tool_test_support`: the environment and working-directory locks and the test checkout root (dev-dependency only)
└── verification_core/  `verification_core`: fail-closed verdicts, pattern scans, the run report and the verification lock
```

## How it works

`verification_core` is tier 0 and depends on no workspace crate. `repository_layout` is tier 1
over the foundation crate `repository_root` (`crates/foundation/repository_root`): it names the
locations more than one tool reads, the tools join them onto the checkout root that crate finds,
and its prelude re-exports the finder so the tool binaries reach the root through it.
`verification_core` defines what a check can conclude (held, failed, or did not run, with the
`NotRun` cause). `process_runner` and `repository_laws` are tier 1 and report through it: a child
that died on a signal or ran out of time is a `NotRun`, and so is a law whose input is missing.
`tool_test_support` is tier 1 over `repository_root` and reaches the other crates' tests only
through `[dev-dependencies]`. `deploy_settings` is tier 2: it reads the settings file
`repository_layout` names and chooses the `process_runner` ssh transport from it.

```text
xtask ──▶ process_runner ──▶ verification_core
  │                            ▲
  └────▶ repository_laws ──────┘   ◀── api (dev-dependency: the engineering_laws tests)

xtask, ticket crates, ticketboard ──▶ repository_layout ──▶ repository_root
xtask ──▶ deploy_settings ──▶ repository_layout, process_runner
xtask tests ──▶ tool_test_support ──▶ repository_root
```

## Boundaries

- Depends on: external crates (`regex`, `libc`, `thiserror`), the checkout-root finder
  `repository_root`, and within this folder on lower tiers alone.
- Used by: `xtask` (every crate, `tool_test_support` from its tests only); the ticket crates in `tools/tickets/`
  and `ticketboard` (`repository_layout`); the `api` package's `engineering_laws` tests
  (`verification_core` and `repository_laws`, as dev-dependencies).
- Rules: each crate declares `category = "tools/foundation"` and its tier
  (`cargo xtask verify crate-tiers`), keeps the crate anatomy (`cargo xtask verify crate-anatomy`),
  and depends only on lower `tools/foundation` crates and `repository_root`
  (`foundation_crates_depend_only_on_lower_foundation_crates` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`).
