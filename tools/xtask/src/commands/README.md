# Xtask command groups

One folder per `cargo xtask` command group: each owns its clap declarations, its dispatch and the
work its commands do. The tree above them lives in `tools/xtask/src/cli/`, the checks the
`verify` group runs live in `tools/xtask/src/verifications/`, and shared plumbing lives in
`tools/xtask/src/core/`.

## Contents

```text
tools/xtask/src/commands/
├── agent_context/  `ai`: the agent tool-call guard hook and the filtered command runner
├── ballistics/     `ballistics`: the game ballistics catalog and calibration fixtures from the export
├── build/          `mk`: build, lint, test and development-server recipes, and the target-dir checks
├── ci/             `ci` and `help`: the CI, schema, verify and map task table and its runner
├── db/             `db`: local Postgres, seeds, backups, integration tests, milestone announcement
├── debug/          `debug`: server-join probes; the remote log reader behind `mod remote-logs`
├── deploy/         `deploy`: the website and staging game server deploys, database backup, restore
├── fetch/          `fetch`: mirrors of the vanilla script sources and the Script API reference
├── generate/       `gen`: the font table generator, and the contract type codegen `schema` runs
├── map/            `map`: terrain export classification, building blueprints, BVH and LOS parity
├── mcp/            `mcp`: the Enfusion MCP daemon, tool calls, Workbench calls, logs and selftest
├── mod.rs          the module tree
├── mod_ops/        `mod`: compile gates, dev and playtest servers, mission tests, world boot, waves
├── platform/       `platform`: preflight, slice worktrees, slice runs, the platform wave lifecycle
├── refactor/       `refactor`: manifest-driven relocation of tracked paths and their references
├── reproduction/   `repro`: the mission-version upload reproduction and its helpers
├── schema/         `schema`: contract codegen, contract and map-asset gates, mission-file tools
├── setup/          `setup`: server profiles, Workbench, the MCP game root and client addons
├── staging/        `staging`: the staging acceptance harness, its host actions and recorded receipts
├── ticket/         `ticket` and `registry-get`: the ticket registry commands over ticket_engine
├── verify/         `verify`: the command adapters of the repository verifications
└── wave/           `wave` and `slice-collisions`: the wave lock over ticket_engine
```

## How it works

A group folder holds a `cli.rs` with the group's clap `Subcommand` enum, a `dispatch.rs` whose
`run` matches it, and the modules that do the work; `db` keeps both in `db/operations.rs`, and
`mk`, `ci` and `slice-collisions` take their arguments raw. `tools/xtask/src/cli/mod.rs` names
every group in its `TopCmd` enum and `tools/xtask/src/cli/dispatch.rs` calls the group's `run`,
which returns the process exit code as `Result<u8>`.

Groups that wrap a library pass it the checkout root and keep its result: `ticket` and `wave` call
`ticket_engine` for [ticket](/documentation/glossary/n_to_z.md#ticket) storage and the
[wave](/documentation/glossary/n_to_z.md#wave) lock; `map` and the map rows of `ci` call
`developer_tools` for engine-backed map work and blueprint compilation, as a library or through its
binaries; `verify` calls the checks under `tools/xtask/src/verifications/`. Each group's own
README gives its commands, flags and exit codes.

## Public surface

- The command groups above, reached only through `cargo xtask <group>`: the groups' modules are
  `pub(crate)` or private to the crate, except `map`, `ticket` and `wave`, which `mod.rs` declares
  `pub`.
- Within the crate, a few groups serve others: `ci`'s task table feeds `schema list-gates`, the
  platform wave gate and `verify ci-schema-parity`; `build`'s recipes feed `ci help` and the
  target-dir checks in `tools/xtask/src/core/`; `ticket`'s `load_registry` feeds
  `registry-get`; `generate`'s codegen feeds `schema codegen` and `ci verify-codegen-fresh`.

## Boundaries

- Depends on: `tools/xtask/src/core/` (repository root and layout, host execution, the target
  directory); `tools/xtask/src/verifications/`; the `ticket_engine`, `developer_tools` and
  `verification_core` crates; the host tools each group names in its README.
- Used by: `tools/xtask/src/cli/dispatch.rs`; `tools/xtask/src/verifications/`, which reads
  the `ci` table; and, through the command line, people, the GitHub workflows in
  `.github/workflows/`, the systemd units in `tools/xtask/deploy/systemd/` and the agent hook in
  `.claude/settings.json`.
- Rules: a group's parsing stays in its own `cli.rs` and its routing in its `dispatch.rs`;
  ticket persistence, wave-lock compilation and engine-backed map work stay in their libraries,
  never copied here; unit tests live in each folder's `tests/`, wired by `#[path]`
  (`tooling_test_modules_live_in_separate_files` in
  `tools/xtask/src/tests/tooling_dependency_boundaries.rs`).
