# Xtask command groups

One folder per `cargo xtask` command group: each owns its clap declarations, its dispatch and the
work its commands do. The tree above them lives in `tools_v2/xtask/src/cli/`, the checks the
`verify` group runs live in `tools_v2/xtask/src/verifications/`, and shared plumbing lives in
`tools_v2/xtask/src/core/`.

## Contents

```text
tools_v2/xtask/src/commands/
├── agent_context/  `ai`: the agent tool-call guard hook and the filtered command runner
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
├── reproduction/   `repro`: the mission-version upload reproduction and its helpers
├── schema/         `schema`: contract codegen, contract and map-asset gates, mission-file tools
├── setup/          `setup`: server profiles, Workbench, the MCP game root and client addons
├── ticket/         `ticket` and `registry-get`: the ticket registry commands over ticket-engine
├── verify/         `verify`: the command adapters of the repository verifications
└── wave/           `wave` and `slice-collisions`: the wave lock over ticket-engine
```

## How it works

A group folder holds a `cli.rs` with the group's clap `Subcommand` enum, a `dispatch.rs` whose
`run` matches it, and the modules that do the work; `db` keeps both in `db/operations.rs`, and
`mk`, `ci` and `slice-collisions` take their arguments raw. `tools_v2/xtask/src/cli/mod.rs` names
every group in its `TopCmd` enum and `tools_v2/xtask/src/cli/dispatch.rs` calls the group's `run`,
which returns the process exit code as `Result<u8>`.

Groups that wrap a library pass it the checkout root and keep its result: `ticket` and `wave` call
`ticket-engine` for [ticket](/documentation_v2/glossary/n_to_z.md#ticket) storage and the
[wave](/documentation_v2/glossary/n_to_z.md#wave) lock; `map` and the map rows of `ci` call
`developer-tools` for engine-backed map work and blueprint compilation, as a library or through its
binaries; `verify` calls the checks under `tools_v2/xtask/src/verifications/`. Each group's own
README gives its commands, flags and exit codes.

## Public surface

- The command groups above, reached only through `cargo xtask <group>`: the groups' modules are
  `pub(crate)` or private to the crate, except `map`, `ticket` and `wave`, which `mod.rs` declares
  `pub`.
- Within the crate, a few groups serve others: `ci`'s task table feeds `schema list-gates`, the
  platform wave gate and `verify ci-schema-parity`; `build`'s recipes feed `ci help` and the
  target-dir checks in `tools_v2/xtask/src/core/`; `ticket`'s `load_registry` feeds
  `registry-get`; `generate`'s codegen feeds `schema codegen` and `ci verify-codegen-fresh`.

## Boundaries

- Depends on: `tools_v2/xtask/src/core/` (repository root and layout, host execution, the target
  directory); `tools_v2/xtask/src/verifications/`; the `ticket-engine`, `developer-tools` and
  `verification-core` crates; the host tools each group names in its README.
- Used by: `tools_v2/xtask/src/cli/dispatch.rs`; `tools_v2/xtask/src/verifications/`, which reads
  the `ci` table; and, through the command line, people, the GitHub workflows in
  `.github/workflows/`, the systemd units in `tools_v2/xtask/deploy/systemd/` and the agent hook in
  `.claude/settings.json`.
- Rules: a group's parsing stays in its own `cli.rs` and its routing in its `dispatch.rs`;
  ticket persistence, wave-lock compilation and engine-backed map work stay in their libraries,
  never copied here; unit tests live in each folder's `tests/`, wired by `#[path]`
  (`tooling_test_modules_live_in_separate_files` in
  `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`).
