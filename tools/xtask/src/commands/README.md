# Xtask command groups

One folder per `cargo xtask` command group: each owns its clap declarations, its dispatch and the
work its commands do. The tree above them lives in `tools/xtask/src/cli/`, the checks the
`verify` group runs live in the check and command crates under `tools/`, and the shared plumbing
lives in the `tools/foundation` crates (the checkout root and layout, the deploy settings, child
processes, and the test locks).

## Contents

```text
tools/xtask/src/commands/
├── agent_context/  `ai`: the agent tool-call guard hook and the filtered command runner
├── fetch/          `fetch`: the command line of the vanilla source and Script API mirrors in `enfusion_script_index`
├── map/            `map`: terrain export classification, building blueprints, BVH and LOS parity
├── mod.rs          the module tree
├── refactor/       `refactor`: the `relocate` command line over the repository_relocation crate
├── schema/         `schema`: contract codegen, contract and map-asset gates, mission-file tools
├── ticket/         `ticket` and `registry-get`: the ticket registry commands over ticket_registry
├── verify/         `verify`: the command adapters of the repository verifications
└── wave/           `wave` and `slice-collisions`: the wave lock over ticket_wave_lock
```

## How it works

A group folder holds a `cli.rs` with the group's clap `Subcommand` enum, a `dispatch.rs` whose
`run` matches it, and the modules that do the work; `mk`, `ci` and `slice-collisions` take their
arguments raw. The `db` and `deploy` groups live in the `database_operations` and `deployment`
crates under `tools/commands/`, and the `ci`, `help` and `mk` lanes in the `ci_task_catalog`
crate there, which `tools/xtask/src/cli/dispatch.rs` calls directly. `tools/xtask/src/cli/mod.rs` names
every group in its `TopCmd` enum and `tools/xtask/src/cli/dispatch.rs` calls the group's `run`,
which returns the process exit code as `Result<u8>`.

Groups that wrap a library pass it the checkout root and keep its result: `ticket` and `wave` call
the ticket crates in `tools/tickets/` for [ticket](/documentation/glossary/n_to_z.md#ticket)
storage and the [wave](/documentation/glossary/n_to_z.md#wave) lock; `map` calls
`blueprint_compiler`, `map_asset_verification` and `world_export_pipeline` in `tools/map_assets/`
for blueprint compilation, the line-of-sight probe, the terrain export driver (which runs the
`world` binary of `developer_tools`) and the map tile index; `refactor` calls `repository_relocation` in `tools/commands/` for the relocation
modes; `ballistics` and `gen` dispatch straight to `ballistics_oracle_tooling` and
`schema_tooling` in `tools/commands/`, and `schema` calls `schema_tooling` for the codegen, the
contract gates and the flattening; the `setup` command line dispatches straight to the `workstation_setup` crate in
`tools/commands/`, the `debug` and `repro` command lines to the `remote_debugging` crate there,
the `mod` command line to the `mod_operations` crate there, and `verify api-readiness` calls `api_readiness_checks` there; `verify` calls
the check crates under `tools/checks/` and the checks the command crates carry. Each group's own
README gives its commands, flags and exit codes.

## Public surface

- The command groups above, reached only through `cargo xtask <group>`: the groups' modules are
  `pub(crate)` or private to the crate, except `map`, `ticket` and `wave`, which `mod.rs` declares
  `pub`.
- Within the crate, `ticket`'s `load_registry` feeds `registry-get`.

## Boundaries

- Depends on: the `repository_layout`, `deploy_settings` and `process_runner` crates (the
  checkout root and layout, the deploy settings, child processes and host execution);
  the check and command crates under `tools/checks/` and `tools/commands/` (`ticket run` calls
  `platform_execution`'s slice runner); the ticket crates in
  `tools/tickets/`, the map asset crates `blueprint_compiler`, `map_asset_verification` and
  `world_export_pipeline`, and `verification_core`; the host
  tools each group names in its README.
- Used by: `tools/xtask/src/cli/dispatch.rs`; and, through the command line, people, the GitHub workflows in
  `.github/workflows/`, the systemd units in `deploy/systemd/` and the agent hook in
  `.claude/settings.json`.
- Rules: a group's parsing stays in its own `cli.rs` and its routing in its `dispatch.rs`;
  ticket persistence, wave-lock compilation and engine-backed map work stay in their libraries,
  never copied here; unit tests live in each folder's `tests/`, wired by `#[path]`
  (`tooling_test_modules_live_in_separate_files` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`).
